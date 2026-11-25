//! The report routes against a stand-in: which right each asks for, that the
//! credential comes before the body, what each refusal says, the budget of
//! refused tokens, the slots, and that no answer holds a credential.
use super::*;
use crate::{
    boundary::{REJECTED, with_routes},
    browser::Peer,
    native::protocol,
};
use axum::{
    body::to_bytes,
    http::{Method, Request as HttpRequest},
    middleware,
};
use central_api::{PROTOCOL_HEADER, PROTOCOL_VERSION, reports::ReportIntake};
use std::{
    future::ready,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    time::Duration,
};
use tokio::sync::Notify;
use tower::ServiceExt;

const HOST: &str = "127.0.0.1:28780";
const REPORT: &str = "0192d0c57b3e7a1f8c2d3e4f5a6b7c8d";
const NOTE: &str = "0192d0c57b3e7a1f8c2d3e4f5a6b7c8e";
const TOKEN: [u8; 16] = [3; 16];

/// The service's stand-in: it knows one gateway's credential and one report
/// token with every right, and writes down what it is asked.
#[derive(Default)]
struct Stand {
    asked: Mutex<Vec<String>>,
    /// The token is refused, as an unknown one is.
    refuse: AtomicBool,
    /// Every call is refused by the rules.
    reject: AtomicBool,
    /// Calls wait for `go`.
    wait: AtomicBool,
    go: Notify,
    inside: Notify,
}

fn gateway() -> Token {
    Token::from_secret_bytes(zeroize::Zeroizing::new([0xcd; 32]))
}

fn token() -> Token {
    Token::from_secret_bytes(zeroize::Zeroizing::new([0xab; 32]))
}

impl Stand {
    fn note(&self, what: impl Into<String>) {
        self.asked.lock().unwrap().push(what.into());
    }

    fn taken(&self) -> Vec<String> {
        std::mem::take(&mut *self.asked.lock().unwrap())
    }

    /// Nothing was asked of it.
    fn unasked(&self) {
        assert_eq!(self.taken(), Vec::<String>::new());
    }

    fn good(&self, presented: &Token) -> Result<(), Error> {
        if presented.digest() == token().digest() && !self.refuse.load(Ordering::SeqCst) {
            Ok(())
        } else {
            Err(Error::Rejected)
        }
    }

    fn ruled(&self) -> Result<(), Error> {
        if self.reject.load(Ordering::SeqCst) {
            Err(Error::Rejected)
        } else {
            Ok(())
        }
    }
}

impl Reports for Stand {
    type Filing = bool;

    fn filing(
        &self,
        bearer: &ReportBearer,
        _: Option<Place>,
    ) -> impl Future<Output = Result<bool, Error>> + Send {
        ready(match bearer {
            ReportBearer::Gateway(presented) => {
                self.note("filing gateway");
                if presented.digest() == gateway().digest() {
                    Ok(false)
                } else {
                    Err(Error::Rejected)
                }
            }
            ReportBearer::Report(presented) => {
                self.note("filing token");
                self.good(presented).map(|()| true)
            }
        })
    }

    fn filer(_: &bool) -> [u8; 16] {
        TOKEN
    }

    fn file(&self, _: &bool, body: &[u8]) -> impl Future<Output = Result<[u8; 16], Error>> + Send {
        self.note(format!("file {}", body.len()));
        ready(
            self.ruled()
                .and_then(|()| central_api::decode::<ReportIntake>(body)?.validate())
                .map(|()| hex_id(REPORT).unwrap()),
        )
    }

    fn announced(
        &self,
        _: &bool,
        _: [u8; 16],
        _: Part,
    ) -> impl Future<Output = Result<u32, Error>> + Send {
        self.note("announced");
        ready(self.ruled().map(|()| 8))
    }

    fn store(
        &self,
        _: &bool,
        _: [u8; 16],
        _: Part,
        gzip: &[u8],
    ) -> impl Future<Output = Result<(), Error>> + Send {
        self.note(format!("store {}", gzip.len()));
        ready(Ok(()))
    }

    fn access(
        &self,
        presented: &Token,
        right: ReportScope,
        download: bool,
        _: Option<Place>,
    ) -> impl Future<Output = Result<[u8; 16], Error>> + Send {
        self.note(format!(
            "access {}{}",
            right.as_str(),
            if download { " download" } else { "" }
        ));
        ready(self.good(presented).map(|()| TOKEN))
    }

    async fn call(&self, _: [u8; 16], _: &Token, call: ReportCall) -> Result<ReportReply, Error> {
        self.note(format!("call {}", call.needs().as_str()));
        if self.wait.load(Ordering::SeqCst) {
            self.inside.notify_one();
            self.go.notified().await;
        }
        self.ruled()?;
        Ok(match call {
            ReportCall::Stats => ReportReply::Stats(report_api::Stats { days: Vec::new() }),
            ReportCall::Search(_) => ReportReply::Page(report_api::Page {
                reports: Vec::new(),
                total: 0,
            }),
            ReportCall::Bulk(_) | ReportCall::PurgeAll(_) => {
                ReportReply::Changed(report_api::Changed { changed: 1 })
            }
            _ => ReportReply::Done,
        })
    }

    fn download(
        &self,
        _: &Token,
        _: [u8; 16],
        part: Part,
    ) -> impl Future<Output = Result<(Vec<u8>, Option<RecordOrigin>), Error>> + Send {
        self.note(format!("download {}", part.as_str()));
        ready(self.ruled().map(|()| {
            (
                vec![0x1f; 3 * CHUNK + 1],
                (part == Part::Record).then_some(RecordOrigin::Client),
            )
        }))
    }
}

/// The routes as the native listener has them: behind its boundary and its
/// protocol's layer.
fn app(stand: &Arc<Stand>) -> Router {
    let routes =
        router(Arc::clone(stand), Arc::default(), None).layer(middleware::from_fn(protocol));
    with_routes(HeaderValue::from_static(HOST), 1_048_576, 32, routes)
}

/// A request as the local proxy passes it on for a client's address.
fn request(
    method: &Method,
    path: &str,
    authorization: Option<&str>,
) -> axum::http::request::Builder {
    let mut builder = HttpRequest::builder()
        .method(method)
        .uri(path)
        .header(header::HOST, HOST)
        .header(PROTOCOL_HEADER, PROTOCOL_VERSION)
        .extension(Peer("127.0.0.1:50000".parse().unwrap()));
    if let Some(value) = authorization {
        builder = builder.header(header::AUTHORIZATION, value);
    }
    builder
}

fn report_bearer() -> String {
    format!(
        "Bearer {}",
        central_api::report_tokens::token_text(&token()).as_str()
    )
}

async fn said(response: Response) -> (StatusCode, String) {
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1_048_576).await.unwrap();
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

async fn send(app: &Router, request: HttpRequest<Body>) -> (StatusCode, String) {
    said(app.clone().oneshot(request).await.unwrap()).await
}

const JSON: &str = "application/json";

fn intake(channel: &str) -> String {
    format!(
        r#"{{"gateway":{{"name":null,"url":null,"version":null}},"channel":"{channel}","reporter":"5bdc","kind":"bug","text":"t","game_id":null,"build":null,"crash":null,"client":null,"record":null}}"#
    )
}

/// Every route of a report token: its method, its path, the right it asks
/// for and, where it takes one, a good body.
fn routes() -> Vec<(Method, String, ReportScope, Option<String>)> {
    use ReportScope::{Purge, Read, Triage};
    let on = |rest: &str| format!("/v1/reports/{REPORT}{rest}");
    let ids = format!(r#"{{"reports":["{REPORT}"]}}"#);
    vec![
        (
            Method::POST,
            "/v1/reports/search".into(),
            Read,
            Some("{}".into()),
        ),
        (
            Method::POST,
            "/v1/reports/facets".into(),
            Read,
            Some("{}".into()),
        ),
        (
            Method::POST,
            "/v1/reports/crashes".into(),
            Read,
            Some("{}".into()),
        ),
        (Method::GET, "/v1/reports/stats".into(), Read, None),
        (Method::GET, on(""), Read, None),
        (Method::GET, "/v1/reports/admins".into(), Triage, None),
        (
            Method::PATCH,
            on(""),
            Triage,
            Some(r#"{"change":{"status":"triaged"},"expected_revision":0}"#.into()),
        ),
        (Method::POST, on("/trash"), Triage, None),
        (Method::POST, on("/restore"), Triage, None),
        (
            Method::POST,
            on("/notes"),
            Triage,
            Some(r#"{"text":"n"}"#.into()),
        ),
        (Method::DELETE, on(&format!("/notes/{NOTE}")), Triage, None),
        (
            Method::POST,
            "/v1/reports/bulk".into(),
            Triage,
            Some(format!(
                r#"{{"reports":["{REPORT}"],"bulk":{{"do":"trash"}}}}"#
            )),
        ),
        (Method::DELETE, on(""), Purge, None),
        (Method::POST, "/v1/reports/purge".into(), Purge, Some(ids)),
    ]
}

fn built(builder: axum::http::request::Builder, body: Option<&str>) -> HttpRequest<Body> {
    match body {
        Some(text) => builder
            .header(header::CONTENT_TYPE, JSON)
            .header(header::CONTENT_LENGTH, text.len().to_string())
            .body(Body::from(text.to_owned()))
            .unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    }
}

#[tokio::test]
async fn each_route_asks_for_its_right_and_then_for_its_call() {
    let stand = Arc::new(Stand::default());
    let app = app(&stand);
    for (method, path, right, body) in routes() {
        let sent = built(
            request(&method, &path, Some(&report_bearer())),
            body.as_deref(),
        );
        let (status, text) = send(&app, sent).await;
        assert!(
            status == StatusCode::OK || status == StatusCode::NO_CONTENT,
            "{method} {path}: {status} {text}"
        );
        assert_eq!(
            stand.taken(),
            [
                format!("access {}", right.as_str()),
                format!("call {}", right.as_str())
            ],
            "{method} {path}"
        );
    }
    // a download is a read, counted as one
    for (part, origin) in [("client", None), ("record", Some("client"))] {
        let sent = built(
            request(
                &Method::GET,
                &format!("/v1/reports/{REPORT}/{part}"),
                Some(&report_bearer()),
            ),
            None,
        );
        let response = app.clone().oneshot(sent).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::CONTENT_TYPE], "application/gzip");
        assert_eq!(
            response
                .headers()
                .get("x-record-origin")
                .map(|value| value.to_str().unwrap()),
            origin
        );
        let bytes = to_bytes(response.into_body(), 1_048_576).await.unwrap();
        assert_eq!(bytes.len(), 3 * CHUNK + 1);
        assert_eq!(
            stand.taken(),
            [
                "access read download".to_owned(),
                format!("download {part}")
            ]
        );
    }
}

/// A body that says whether it was asked for anything.
struct Watched(Arc<AtomicUsize>);

impl hyper::body::Body for Watched {
    type Data = Bytes;
    type Error = std::io::Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
    ) -> Poll<Option<Result<hyper::body::Frame<Bytes>, Self::Error>>> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Poll::Pending
    }

    fn size_hint(&self) -> hyper::body::SizeHint {
        hyper::body::SizeHint::with_exact(8)
    }
}

/// Nothing of a body is read for a credential that is not good, on any
/// route, whatever makes it not good; and every such answer is the same.
#[tokio::test]
async fn a_credential_that_is_not_good_is_refused_before_its_body_is_read() {
    let stand = Arc::new(Stand::default());
    let app = app(&stand);
    let hex = token().hex();
    let gateways = format!("Bearer {}", gateway().hex().as_str());
    let report_only = vec![
        // a gateway's credential, where a report token alone is one
        Some(gateways.clone()),
        // a report token's hex without its kind
        Some(format!("Bearer {}", hex.as_str())),
    ];
    let everywhere = vec![
        None,
        Some(String::new()),
        Some("Bearer".into()),
        Some("Basic YTpi".into()),
        Some("Bearer a.b.c".into()),
        Some(format!("Bearer bgt_{}", hex.as_str())),
        Some(format!("bearer brt_{}", hex.as_str())),
        Some(format!("Bearer brt_{}", hex.to_uppercase())),
        Some(format!("Bearer brt_{}", "0".repeat(64))),
    ];
    let mut tried: Vec<(Method, String, &str, Vec<Option<String>>)> = routes()
        .into_iter()
        .map(|(method, path, ..)| {
            (
                method,
                path,
                JSON,
                [everywhere.clone(), report_only.clone()].concat(),
            )
        })
        .collect();
    tried.push((Method::POST, "/v1/reports".into(), JSON, everywhere.clone()));
    tried.push((
        Method::PUT,
        format!("/v1/reports/{REPORT}/client"),
        "application/gzip",
        everywhere,
    ));
    for (method, path, kind, bearers) in tried {
        for bearer in bearers {
            let polls = Arc::new(AtomicUsize::new(0));
            // without a client's address, so that no budget ends the asking
            let sent = request(&method, &path, bearer.as_deref())
                .header(header::CONTENT_LENGTH, "8")
                .header(header::CONTENT_TYPE, kind)
                .body(Body::new(Watched(Arc::clone(&polls))))
                .unwrap();
            let (status, text) = send(&app, sent).await;
            assert_eq!(
                (status, text.as_str()),
                (StatusCode::UNAUTHORIZED, "account request rejected\n"),
                "{method} {path} {bearer:?}"
            );
            assert_eq!(
                polls.load(Ordering::SeqCst),
                0,
                "{method} {path} {bearer:?}"
            );
            assert!(
                !stand.taken().iter().any(|asked| asked.starts_with("call")
                    || asked.starts_with("file")
                    || asked.starts_with("store")
                    || asked.starts_with("announced")),
                "{method} {path} {bearer:?}"
            );
        }
    }
    // a gateway's credential is not even asked about where a token is
    for (method, path, ..) in routes() {
        let sent = built(request(&method, &path, Some(&gateways)), None);
        assert_eq!(send(&app, sent).await.0, StatusCode::UNAUTHORIZED);
        stand.unasked();
    }
    // a token without the right is refused as an unknown one is
    stand.refuse.store(true, Ordering::SeqCst);
    for (method, path, _, body) in routes() {
        let sent = built(
            request(&method, &path, Some(&report_bearer())),
            body.as_deref(),
        );
        assert_eq!(
            send(&app, sent).await,
            (
                StatusCode::UNAUTHORIZED,
                "account request rejected\n".into()
            )
        );
    }
}

#[tokio::test]
async fn two_credentials_are_none() {
    let stand = Arc::new(Stand::default());
    let app = app(&stand);
    for (method, path, _, body) in routes() {
        let sent = built(
            request(&method, &path, Some(&report_bearer()))
                .header(header::AUTHORIZATION, report_bearer()),
            body.as_deref(),
        );
        assert_eq!(send(&app, sent).await.0, StatusCode::UNAUTHORIZED);
        stand.unasked();
    }
}

#[tokio::test]
async fn another_protocol_is_told_to_update_before_anything_is_asked() {
    let stand = Arc::new(Stand::default());
    let app = app(&stand);
    for (method, path, _, body) in routes() {
        let mut sent = built(
            request(&method, &path, Some(&report_bearer())),
            body.as_deref(),
        );
        sent.headers_mut()
            .insert(PROTOCOL_HEADER, HeaderValue::from_static("1"));
        assert_eq!(send(&app, sent).await.0.as_u16(), 426, "{method} {path}");
        // nor does a page's request reach it
        let mut sent = built(
            request(&method, &path, Some(&report_bearer())),
            body.as_deref(),
        );
        sent.headers_mut().insert(
            header::ORIGIN,
            HeaderValue::from_static("https://baylee.app"),
        );
        assert_eq!(send(&app, sent).await.0, StatusCode::FORBIDDEN);
        stand.unasked();
    }
}

/// Past the credential a token is told what it did wrong by the status, and
/// nothing of its request comes back; a gateway hears what it always did.
#[tokio::test]
async fn past_the_credential_a_token_s_bad_request_is_one() {
    let stand = Arc::new(Stand::default());
    let app = app(&stand);
    let bad = |method: &Method, path: &str, body: Option<&str>| {
        built(request(method, path, Some(&report_bearer())), body)
    };
    let refused = (
        StatusCode::BAD_REQUEST,
        "report request rejected\n".to_owned(),
    );
    let canary = "SECRET_WORDS_CANARY";
    let (post, get) = (Method::POST, Method::GET);
    let upper = format!("/v1/reports/{}", REPORT.to_uppercase());
    for sent in [
        // bodies that are no search, change, note or selection
        bad(&post, "/v1/reports/search", Some("")),
        bad(
            &post,
            "/v1/reports/search",
            Some(r#"{"filter":{"unseen":true}}"#),
        ),
        bad(
            &post,
            "/v1/reports/search",
            Some(&format!(r#"{{"{canary}":1}}"#)),
        ),
        bad(&post, "/v1/reports/search", Some("{} {}")),
        bad(
            &post,
            "/v1/reports/bulk",
            Some(r#"{"reports":["x"],"bulk":{"do":"trash"}}"#),
        ),
        bad(
            &post,
            "/v1/reports/bulk",
            Some(&format!(
                r#"{{"reports":["{REPORT}"],"bulk":{{"do":"purge"}}}}"#
            )),
        ),
        bad(
            &Method::PATCH,
            &format!("/v1/reports/{REPORT}"),
            Some(r#"{"change":{"text":"x"}}"#),
        ),
        bad(
            &post,
            &format!("/v1/reports/{REPORT}/notes"),
            Some(r#"{"text":"n","author":"x"}"#),
        ),
        // a body where none belongs, a query anywhere
        bad(&get, "/v1/reports/stats", Some("{}")),
        bad(&get, "/v1/reports/stats?q=x", None),
        bad(&post, "/v1/reports/search?q=x", Some("{}")),
        bad(&post, &format!("/v1/reports/{REPORT}/trash"), Some("{}")),
        bad(
            &Method::DELETE,
            &format!("/v1/reports/{REPORT}"),
            Some("{}"),
        ),
        bad(&get, &format!("/v1/reports/{REPORT}/client?x=1"), None),
        // an id that is none
        bad(&get, "/v1/reports/nothing", None),
        bad(&get, &upper, None),
        bad(
            &Method::DELETE,
            &format!("/v1/reports/{REPORT}/notes/1"),
            None,
        ),
        bad(&get, "/v1/reports/0/client", None),
    ] {
        let what = format!("{} {}", sent.method(), sent.uri());
        let heard = send(&app, sent).await;
        assert_eq!(heard, refused, "{what}");
        assert!(
            stand
                .taken()
                .iter()
                .all(|asked| asked.starts_with("access")),
            "{what}"
        );
    }
    // a body's type is named once, and is JSON
    for kinds in [&["text/plain"][..], &[JSON, JSON], &[]] {
        let mut sent = bad(&post, "/v1/reports/search", Some("{}"));
        sent.headers_mut().remove(header::CONTENT_TYPE);
        for kind in kinds {
            sent.headers_mut()
                .append(header::CONTENT_TYPE, HeaderValue::from_static(kind));
        }
        assert_eq!(send(&app, sent).await, refused, "{kinds:?}");
    }
}

/// What the rules refuse, each route's: a token's bad request, a gateway's
/// refusal as it always was.
#[tokio::test]
async fn what_the_rules_refuse_is_a_token_s_bad_request_and_a_gateway_s_refusal() {
    let stand = Arc::new(Stand::default());
    let app = app(&stand);
    let refused = (
        StatusCode::BAD_REQUEST,
        "report request rejected\n".to_owned(),
    );
    stand.reject.store(true, Ordering::SeqCst);
    for (method, path, _, body) in routes() {
        let sent = built(
            request(&method, &path, Some(&report_bearer())),
            body.as_deref(),
        );
        assert_eq!(send(&app, sent).await, refused, "{method} {path}");
    }
    let gzip = |kind: &'static str, report: &str, bearer: &str| {
        request(
            &Method::PUT,
            &format!("/v1/reports/{report}/client"),
            Some(bearer),
        )
        .header(header::CONTENT_TYPE, kind)
        .header(header::CONTENT_LENGTH, "8")
        .body(Body::from(vec![0x1f; 8]))
        .unwrap()
    };
    for (bearer, channel, status) in [
        (report_bearer(), "token", StatusCode::BAD_REQUEST),
        (
            format!("Bearer {}", gateway().hex().as_str()),
            "gateway",
            StatusCode::UNAUTHORIZED,
        ),
    ] {
        let sent = built(
            request(&Method::POST, "/v1/reports", Some(&bearer)),
            Some(&intake(channel)),
        );
        assert_eq!(send(&app, sent).await.0, status);
        for (kind, report) in [
            ("application/gzip", REPORT),
            ("application/gzip", "nothing"),
            (JSON, REPORT),
        ] {
            let sent = gzip(kind, report, &bearer);
            assert_eq!(send(&app, sent).await.0, status, "{kind} {report}");
        }
        let mut sent = built(
            request(&Method::POST, "/v1/reports", Some(&bearer)),
            Some(&intake(channel)),
        );
        sent.headers_mut()
            .insert(header::CONTENT_TYPE, HeaderValue::from_static("text/plain"));
        assert_eq!(send(&app, sent).await.0, status);
    }
}

/// What is not there is not there, whoever asks, and nothing is asked for it.
#[tokio::test]
async fn another_path_or_method_is_no_route() {
    let stand = Arc::new(Stand::default());
    let app = app(&stand);
    let (post, get) = (Method::POST, Method::GET);
    let bad = |method: &Method, path: &str| {
        request(method, path, Some(&report_bearer()))
            .header(header::CONTENT_LENGTH, "0")
            .body(Body::empty())
            .unwrap()
    };
    for (method, path, status) in [
        (
            &post,
            format!("/v1/reports/{REPORT}/purge"),
            StatusCode::NOT_FOUND,
        ),
        (
            &get,
            format!("/v1/reports/{REPORT}/screenshot"),
            StatusCode::NOT_FOUND,
        ),
        (
            &get,
            format!("/v1/reports/{REPORT}/notes/{NOTE}/x"),
            StatusCode::NOT_FOUND,
        ),
        (
            &get,
            "/v1/reports".to_owned(),
            StatusCode::METHOD_NOT_ALLOWED,
        ),
        (
            &Method::DELETE,
            "/v1/reports/search".to_owned(),
            StatusCode::METHOD_NOT_ALLOWED,
        ),
        (
            &Method::PUT,
            format!("/v1/reports/{REPORT}"),
            StatusCode::METHOD_NOT_ALLOWED,
        ),
    ] {
        assert_eq!(
            send(&app, bad(method, &path)).await.0,
            status,
            "{method} {path}"
        );
    }
    stand.unasked();
}

#[tokio::test]
async fn a_gateway_and_a_token_file_alike() {
    let stand = Arc::new(Stand::default());
    let app = app(&stand);
    for (bearer, channel, who) in [
        (report_bearer(), "token", "filing token"),
        (
            format!("Bearer {}", gateway().hex().as_str()),
            "gateway",
            "filing gateway",
        ),
    ] {
        let body = intake(channel);
        let sent = built(
            request(&Method::POST, "/v1/reports", Some(&bearer)),
            Some(&body),
        );
        assert_eq!(
            send(&app, sent).await,
            (
                StatusCode::CREATED,
                format!(r#"{{"report_id":"{REPORT}"}}"#)
            )
        );
        assert_eq!(
            stand.taken(),
            [who.to_owned(), format!("file {}", body.len())]
        );
        let sent = request(
            &Method::PUT,
            &format!("/v1/reports/{REPORT}/record"),
            Some(&bearer),
        )
        .header(header::CONTENT_TYPE, "application/gzip")
        .header(header::CONTENT_LENGTH, "8")
        .body(Body::from(vec![0x1f; 8]))
        .unwrap();
        assert_eq!(
            send(&app, sent).await,
            (StatusCode::NO_CONTENT, String::new())
        );
        assert_eq!(stand.taken(), [who, "announced", "store 8"]);
        // another length than announced is not read
        let polls = Arc::new(AtomicUsize::new(0));
        let sent = request(
            &Method::PUT,
            &format!("/v1/reports/{REPORT}/record"),
            Some(&bearer),
        )
        .header(header::CONTENT_TYPE, "application/gzip")
        .header(header::CONTENT_LENGTH, "9")
        .body(Body::new(Watched(Arc::clone(&polls))))
        .unwrap();
        assert!(send(&app, sent).await.0.is_client_error());
        assert_eq!(polls.load(Ordering::SeqCst), 0);
        stand.taken();
        // a body past the metadata's bound is refused by its length alone
        let sent = request(&Method::POST, "/v1/reports", Some(&bearer))
            .header(header::CONTENT_TYPE, JSON)
            .header(header::CONTENT_LENGTH, (MAX_INTAKE_BYTES + 1).to_string())
            .body(Body::new(Watched(Arc::clone(&polls))))
            .unwrap();
        assert_eq!(send(&app, sent).await.0, StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(polls.load(Ordering::SeqCst), 0);
        assert_eq!(stand.taken(), [who]);
    }
}

/// Thirty refused report tokens from an address in an hour are the last the
/// store is asked about; other addresses, an unknown one and the address's
/// gateways are asked about as before.
#[tokio::test]
async fn refused_tokens_are_counted_by_their_address_and_gateways_are_not() {
    let stand = Arc::new(Stand::default());
    let app = app(&stand);
    let from = |address: &str, bearer: &str| {
        built(
            request(&Method::GET, "/v1/reports/stats", Some(bearer))
                .header("x-forwarded-for", address),
            None,
        )
    };
    let unknown = format!("Bearer brt_{}", "0".repeat(64));
    for _ in 0..crate::addresses::REPORT_REFUSALS {
        let heard = send(&app, from("198.51.100.7", &unknown)).await;
        assert_eq!(heard.0, StatusCode::UNAUTHORIZED);
    }
    stand.taken();
    // the store is asked no more, about a good token as little
    for bearer in [&unknown, &report_bearer()] {
        let heard = send(&app, from("198.51.100.7", bearer)).await;
        assert_eq!(
            heard,
            (
                StatusCode::TOO_MANY_REQUESTS,
                "account request rejected\n".into()
            )
        );
    }
    stand.unasked();
    // what files by a token is counted with them
    let filing = |address: &str, bearer: &str| {
        built(
            request(&Method::POST, "/v1/reports", Some(bearer)).header("x-forwarded-for", address),
            Some(&intake("gateway")),
        )
    };
    let heard = send(&app, filing("198.51.100.7", &report_bearer())).await;
    assert_eq!(heard.0, StatusCode::TOO_MANY_REQUESTS);
    // the address's gateway files on, and another address's token is served
    let gateways = format!("Bearer {}", gateway().hex().as_str());
    let heard = send(&app, filing("198.51.100.7", &gateways)).await;
    assert_eq!(heard.0, StatusCode::CREATED);
    let heard = send(&app, from("198.51.100.8", &report_bearer())).await;
    assert_eq!(heard.0, StatusCode::OK);
    // a gateway's refused credentials count for nothing
    let wrong = format!("Bearer {}", "0".repeat(64));
    for _ in 0..2 * crate::addresses::REPORT_REFUSALS {
        let heard = send(&app, filing("198.51.100.9", &wrong)).await;
        assert_eq!(heard.0, StatusCode::UNAUTHORIZED);
    }
    let heard = send(&app, from("198.51.100.9", &report_bearer())).await;
    assert_eq!(heard.0, StatusCode::OK);
    // what the rules refuse is no refused token
    stand.reject.store(true, Ordering::SeqCst);
    for _ in 0..2 * crate::addresses::REPORT_REFUSALS {
        let heard = send(&app, from("198.51.100.10", &report_bearer())).await;
        assert_eq!(heard.0, StatusCode::BAD_REQUEST);
    }
}

/// A token's calls run one at a time, and a second is told so at once; the
/// slot is free again when the first is answered or goes away.
#[tokio::test]
async fn a_token_has_one_call_at_once() {
    let stand = Arc::new(Stand::default());
    let app = app(&stand);
    let ask = || {
        built(
            request(&Method::GET, "/v1/reports/stats", Some(&report_bearer())),
            None,
        )
    };
    stand.wait.store(true, Ordering::SeqCst);
    let first = tokio::spawn({
        let (app, sent) = (app.clone(), ask());
        async move { send(&app, sent).await.0 }
    });
    stand.inside.notified().await;
    assert_eq!(send(&app, ask()).await.0, StatusCode::TOO_MANY_REQUESTS);
    stand.go.notify_one();
    assert_eq!(first.await.unwrap(), StatusCode::OK);
    // one whose connection goes away gives it back too
    let gone = tokio::spawn({
        let (app, sent) = (app.clone(), ask());
        async move { send(&app, sent).await.0 }
    });
    stand.inside.notified().await;
    gone.abort();
    assert!(gone.await.is_err());
    stand.wait.store(false, Ordering::SeqCst);
    assert_eq!(send(&app, ask()).await.0, StatusCode::OK);
}

/// A download's slot is held until its part is sent, and is free again when
/// the part is, or when its reader goes away.
#[tokio::test]
async fn a_download_holds_its_slot_until_it_is_sent() {
    let stand = Arc::new(Stand::default());
    let app = app(&stand);
    let ask = || {
        built(
            request(
                &Method::GET,
                &format!("/v1/reports/{REPORT}/client"),
                Some(&report_bearer()),
            ),
            None,
        )
    };
    let first = app.clone().oneshot(ask()).await.unwrap();
    assert_eq!(first.status(), StatusCode::OK);
    assert_eq!(send(&app, ask()).await.0, StatusCode::TOO_MANY_REQUESTS);
    // the token's other requests are served meanwhile
    let stats = built(
        request(&Method::GET, "/v1/reports/stats", Some(&report_bearer())),
        None,
    );
    assert_eq!(send(&app, stats).await.0, StatusCode::OK);
    let bytes = to_bytes(first.into_body(), 1_048_576).await.unwrap();
    assert_eq!(bytes.len(), 3 * CHUNK + 1);
    let second = app.clone().oneshot(ask()).await.unwrap();
    assert_eq!(second.status(), StatusCode::OK);
    drop(second);
    assert_eq!(
        app.clone().oneshot(ask()).await.unwrap().status(),
        StatusCode::OK
    );
}

/// A body that does not say how long it is.
struct Unmeasured(Option<Bytes>);

impl hyper::body::Body for Unmeasured {
    type Data = Bytes;
    type Error = std::io::Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
    ) -> Poll<Option<Result<hyper::body::Frame<Bytes>, Self::Error>>> {
        Poll::Ready(
            self.0
                .take()
                .map(|bytes| Ok(hyper::body::Frame::data(bytes))),
        )
    }
}

/// A body withheld by a good token holds its address's place and no more
/// than its time; it is never read past its bound.
#[tokio::test]
async fn a_good_token_s_body_keeps_the_boundary_s_bounds() {
    let stand = Arc::new(Stand::default());
    let app = app(&stand);
    let polls = Arc::new(AtomicUsize::new(0));
    let sent = request(&Method::POST, "/v1/reports/search", Some(&report_bearer()))
        .header(header::CONTENT_TYPE, JSON)
        .header(
            header::CONTENT_LENGTH,
            (report_api::MAX_BODY_BYTES + 1).to_string(),
        )
        .body(Body::new(Watched(Arc::clone(&polls))))
        .unwrap();
    assert_eq!(send(&app, sent).await.0, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(polls.load(Ordering::SeqCst), 0);
    // of unknown length and too long
    let long = Body::new(Unmeasured(Some(
        vec![b' '; report_api::MAX_BODY_BYTES + 1].into(),
    )));
    let sent = request(&Method::POST, "/v1/reports/search", Some(&report_bearer()))
        .header(header::CONTENT_TYPE, JSON)
        .body(long)
        .unwrap();
    assert_eq!(send(&app, sent).await.0, StatusCode::PAYLOAD_TOO_LARGE);
    let sent = request(&Method::POST, "/v1/reports/search", Some(&report_bearer()))
        .header(header::CONTENT_TYPE, JSON)
        .header(header::CONTENT_LENGTH, "8")
        .body(Body::new(Watched(Arc::clone(&polls))))
        .unwrap();
    let started = std::time::Instant::now();
    assert_eq!(send(&app, sent).await.0, StatusCode::REQUEST_TIMEOUT);
    assert!(started.elapsed() >= Duration::from_secs(5));
    assert!(polls.load(Ordering::SeqCst) > 0);
    assert!(stand.taken().iter().all(|asked| asked == "access read"));
}

/// No answer holds a credential, whatever it answers.
#[tokio::test]
async fn no_answer_holds_a_credential() {
    let stand = Arc::new(Stand::default());
    let app = app(&stand);
    let secret = token().hex();
    for reject in [false, true] {
        stand.reject.store(reject, Ordering::SeqCst);
        for refuse in [false, true] {
            stand.refuse.store(refuse, Ordering::SeqCst);
            for (method, path, _, body) in routes() {
                let sent = built(
                    request(&method, &path, Some(&report_bearer())),
                    body.as_deref(),
                );
                let response = app.clone().oneshot(sent).await.unwrap();
                for value in response.headers().values() {
                    assert!(!value.to_str().unwrap_or("").contains(secret.as_str()));
                }
                assert!(!said(response).await.1.contains(secret.as_str()));
            }
        }
    }
}

#[tokio::test]
async fn a_part_s_body_that_is_withheld_times_out_at_its_pace_and_others_are_refused() {
    let pace = Pace {
        window: Duration::from_millis(40),
        bytes: 1_000,
    };
    // withheld: 408 at the first window, long before an upload's minute
    let started = std::time::Instant::now();
    let withheld = Body::new(Watched(Arc::default()));
    let Err(stopped) = part_body(withheld, 2_000, pace, Stopped::Failed).await else {
        panic!("a withheld body was read");
    };
    assert!(started.elapsed() < Duration::from_secs(2));
    assert_eq!(
        said(stopped.into_response()).await,
        (StatusCode::REQUEST_TIMEOUT, "request timed out\n".into())
    );
    // longer than announced: the fixed refusal, a gateway's as before
    for (refused, status, text) in [
        (
            Stopped::Failed as fn(Error) -> Stopped,
            StatusCode::UNAUTHORIZED,
            REJECTED,
        ),
        (
            Stopped::Request,
            StatusCode::BAD_REQUEST,
            REPORT_ROUTE_REJECTED,
        ),
    ] {
        let long = Body::from(vec![0x1f; 2_001]);
        let Err(stopped) = part_body(long, 2_000, pace, refused).await else {
            panic!("a body past its announced length was read");
        };
        assert_eq!(said(stopped.into_response()).await, (status, text.into()));
    }
    // exactly as announced
    let whole = part_body(Body::from(vec![0x1f; 2_000]), 2_000, pace, Stopped::Failed).await;
    assert!(whole.is_ok_and(|bytes| bytes.len() == 2_000));
    // the pace asked of a real upload: 16 KiB in every five seconds
    assert_eq!(
        (UPLOAD_PACE.window, UPLOAD_PACE.bytes),
        (Duration::from_secs(5), 16_384)
    );
}

#[test]
fn an_upload_is_unfinished_unless_its_part_is_stored_or_the_store_failed() {
    for refused in [Stopped::Failed as fn(Error) -> Stopped, Stopped::Request] {
        // stored, stored meanwhile by the other upload, the store down
        for answer in [
            Ok(()),
            Err(refused(Error::Conflict)),
            Err(refused(Error::Unavailable)),
        ] {
            assert!(!unfinished(&answer));
        }
        // too slow, bytes other than announced or a connection that broke
        for answer in [
            Err(Stopped::Stalled),
            Err(refused(Error::Rejected)),
            Err(refused(Error::Busy)),
        ] {
            assert!(unfinished(&answer));
        }
    }
}

#[test]
fn a_report_route_s_credential_is_one_and_of_one_spelling() {
    let of = |bearers: &[&str]| {
        let mut headers = HeaderMap::new();
        for value in bearers {
            headers.append(header::AUTHORIZATION, value.parse().unwrap());
        }
        bearer(&headers)
    };
    let hex = "0123456789abcdef".repeat(4);
    let (gateways, reports) = (format!("Bearer {hex}"), format!("Bearer brt_{hex}"));
    assert!(matches!(of(&[&gateways]), Ok(ReportBearer::Gateway(t)) if t.hex().as_str() == hex));
    assert!(matches!(of(&[&reports]), Ok(ReportBearer::Report(t)) if t.hex().as_str() == hex));
    for bearers in [
        &[][..],
        &[gateways.as_str(), gateways.as_str()],
        &[gateways.as_str(), reports.as_str()],
        &[""],
        &["Bearer "],
        &[hex.as_str()],
        &["Bearer a.b.c"],
        &[&gateways.replace("Bearer", "bearer")],
        &[&gateways.replace("Bearer", "Basic")],
        &[&gateways.replace("Bearer ", "Bearer  ")],
        &[&gateways.to_uppercase()],
        &[&gateways[..gateways.len() - 1]],
        &[&format!("{gateways}0")],
        &[&format!("{gateways},{gateways}")],
        &[&format!("{reports},{gateways}")],
    ] {
        assert!(of(bearers).is_err(), "{bearers:?}");
    }
    // Octets that are no text are a refusal, not a request without a bearer.
    let mut headers = HeaderMap::new();
    headers.insert(
        header::AUTHORIZATION,
        HeaderValue::from_bytes(b"Bearer \xff").unwrap(),
    );
    assert!(bearer(&headers).is_err());
}
