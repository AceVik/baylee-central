//! Report tokens over the real listeners (`docs/reports.md` §"Report
//! tokens"): an administrator's website session makes them, scripts file,
//! read and triage with them on the native listener, each within its rights,
//! and a revocation ends one at once. A token is nothing on any other route,
//! and nothing else is a report token.
use super::{
    Http,
    browser::Browser,
    pace,
    password::{OLD, browser_sign_in, close, context, moved},
    reports::part,
};
use central_api::{
    Error, PROTOCOL_HEADER, PROTOCOL_VERSION, Token,
    admin::{AdminAnswer, AdminRequest},
    report_api::{self, By, Detail, Id},
    report_tokens::{ReportScopes, token_text},
    reports::{Announced, Channel, GatewaySays, ReportIntake, ReportKind, ReportStatus},
    wire::{Request, Response},
};
use central_client::Transport;
use central_postgres::appoint_admin;
use sea_orm::DatabaseConnection;
use std::{
    io::{Read, Write},
    net::TcpStream,
    time::{Duration, Instant},
};
use tokio::runtime::Runtime;

/// A request to the report routes, as a script sends it.
struct Ask<'a> {
    line: &'a str,
    bearer: &'a str,
    /// A body and its type.
    body: Option<(&'a str, &'a [u8])>,
    /// Further head lines.
    head: &'a str,
}

/// An answer: its status, head and body.
struct Said {
    status: u16,
    head: String,
    body: Vec<u8>,
}

impl Said {
    fn read<T: serde::de::DeserializeOwned>(&self) -> T {
        central_api::decode(&self.body).unwrap()
    }
}

fn send(http: &Http, ask: &Ask<'_>) -> Said {
    pace();
    let mut stream = TcpStream::connect_timeout(&http.address, Duration::from_secs(3)).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    let body = match ask.body {
        Some((kind, bytes)) => {
            format!(
                "Content-Type: {kind}\r\nContent-Length: {}\r\n",
                bytes.len()
            )
        }
        None => String::new(),
    };
    write!(
        stream,
        "{} HTTP/1.1\r\nHost: {}\r\n{PROTOCOL_HEADER}: {PROTOCOL_VERSION}\r\n{}{body}Authorization: {}\r\n\r\n",
        ask.line, http.address, ask.head, ask.bearer
    )
    .unwrap();
    if let Some((_, bytes)) = ask.body {
        // A request refused before its body is read may close on what was sent.
        let _ = stream.write_all(bytes);
    }
    let mut answer = Vec::new();
    stream.take(200_000).read_to_end(&mut answer).unwrap();
    let at = answer.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
    let head = String::from_utf8(answer[..at].to_vec()).unwrap();
    Said {
        status: head.split_whitespace().nth(1).unwrap().parse().unwrap(),
        head,
        body: answer[at + 4..].to_vec(),
    }
}

const JSON: &str = "application/json";

/// A script with a token, and what it hears.
struct Script<'a> {
    http: &'a Http,
    bearer: String,
}

impl Script<'_> {
    fn get(&self, path: &str) -> Said {
        self.bare(&format!("GET {path}"))
    }

    /// A request without a body.
    fn bare(&self, line: &str) -> Said {
        send(
            self.http,
            &Ask {
                line,
                bearer: &self.bearer,
                body: None,
                head: "",
            },
        )
    }

    fn tell(&self, line: &str, body: &str) -> Said {
        send(
            self.http,
            &Ask {
                line,
                bearer: &self.bearer,
                body: Some((JSON, body.as_bytes())),
                head: "",
            },
        )
    }

    fn put(&self, path: &str, gzip: &[u8]) -> Said {
        send(
            self.http,
            &Ask {
                line: &format!("PUT {path}"),
                bearer: &self.bearer,
                body: Some(("application/gzip", gzip)),
                head: "",
            },
        )
    }

    /// File a report that announces `client`; its id, as the routes spell it.
    fn file(&self, client: &Announced) -> String {
        let said = self.tell("POST /v1/reports", &intake(client));
        assert_eq!(said.status, 201);
        central_api::reports::hex(&said.read::<report_api::Filed>().report_id.0)
    }
}

fn intake(client: &Announced) -> String {
    let report = ReportIntake {
        gateway: GatewaySays::default(),
        channel: Channel::Token,
        reporter: "5bdc11".into(),
        kind: ReportKind::Bug,
        text: "From a script".into(),
        game_id: None,
        build: None,
        crash: None,
        client: Some(client.clone()),
        record: None,
    };
    String::from_utf8(central_api::encode(&report).unwrap()).unwrap()
}

/// The administrator's website session.
struct Desk<'a> {
    browser: &'a Browser,
    cookie: Token,
}

impl Desk<'_> {
    fn ask(&self, request: AdminRequest) -> Result<Response, Error> {
        self.browser
            .answer(&Request::Admin { request }, Some(&self.cookie))
    }

    /// Make a token: its id and the header a script sends.
    fn make(&self, name: &str, scopes: ReportScopes) -> ([u8; 16], String) {
        let Response::Admin {
            answer: AdminAnswer::ReportTokenCreated { id, token, .. },
        } = self
            .ask(AdminRequest::CreateReportToken {
                name: name.into(),
                scopes,
                days: 30,
            })
            .unwrap()
        else {
            panic!("a token");
        };
        (id, format!("Bearer {}", token_text(&token).as_str()))
    }

    fn revoke(&self, token: [u8; 16]) {
        let answer = self.ask(AdminRequest::RevokeReportToken { token }).unwrap();
        assert!(matches!(answer, Response::Done));
    }
}

pub(crate) fn run(
    http: &Http,
    browser: &Browser,
    bridge: &Token,
    runtime: &Runtime,
    operator: &DatabaseConnection,
) {
    let browser = Browser {
        address: browser.address,
        client: "198.51.100.43",
    };
    let (identity, session) = moved(http, bridge, "report-tokens-web");
    close(http, bridge, "report-tokens-web", identity);
    drop(session);
    let desk = Desk {
        cookie: browser_sign_in(&browser, identity, context(http, identity), OLD),
        browser: &browser,
    };
    let create = || AdminRequest::CreateReportToken {
        name: "Nightly build".into(),
        scopes: ReportScopes::EVERYTHING,
        days: 30,
    };
    assert_eq!(
        desk.ask(create()).err(),
        Some(Error::Rejected),
        "no administrator"
    );
    runtime
        .block_on(appoint_admin(operator, "report-tokens-web"))
        .unwrap();
    // the native listener makes none, whoever asks
    let request = Request::Admin { request: create() };
    for token in [&desk.cookie, bridge] {
        assert!(http.exchange(&request, Some(token.into())).is_err());
    }
    let [post, read, edit, all] = [
        ("Only post", ReportScopes::ONLY_POST),
        ("Only read", ReportScopes::ONLY_READ),
        ("Read and edit", ReportScopes::READ_AND_EDIT),
        ("Everything", ReportScopes::EVERYTHING),
    ]
    .map(|(name, scopes)| {
        let (id, bearer) = desk.make(name, scopes);
        (id, Script { http, bearer })
    });
    listed(&desk, &[&post.1, &read.1, &edit.1, &all.1]);
    let (gzip, announced) = part(runtime, operator);
    let report = post.1.file(&announced);
    let path = format!("/v1/reports/{report}");
    let client = format!("{path}/client");
    // the part is its filer's to send
    assert_eq!(all.1.put(&client, &gzip).status, 400);
    assert_eq!(post.1.put(&client, &gzip).status, 204);
    assert_eq!(post.1.put(&client, &gzip).status, 409);
    levels(&path, [&post.1, &read.1, &edit.1, &all.1]);
    reading(&read.1, &report, &gzip, post.0);
    triage(&edit.1, &all.1, &path, edit.0);
    selections(&post.1, &edit.1, &all.1, &announced);
    elsewhere(http, bridge, &all.1);
    refusals(http);
    // revoked: its very next request
    for (id, script) in [post, read, edit, all] {
        desk.revoke(id);
        assert_eq!(script.get("/v1/reports/stats").status, 401);
        assert_eq!(
            script.tell("POST /v1/reports", &intake(&announced)).status,
            401
        );
    }
}

/// The administration lists what a token may do and never the token.
fn listed(desk: &Desk<'_>, scripts: &[&Script<'_>]) {
    let Response::Admin { answer } = desk.ask(AdminRequest::ReportTokens).unwrap() else {
        panic!("the tokens");
    };
    let AdminAnswer::ReportTokens { tokens } = &answer else {
        panic!("the tokens");
    };
    assert_eq!(
        tokens.iter().rev().map(|t| t.scopes).collect::<Vec<_>>(),
        ReportScopes::PRESETS
    );
    let text = String::from_utf8(central_api::encode(&answer).unwrap()).unwrap();
    for script in scripts {
        let secret = script.bearer.strip_prefix("Bearer brt_").unwrap();
        assert!(!text.contains(secret));
    }
}

/// Each preset on each kind of route: what it may, and `401` for the rest.
fn levels(path: &str, [post, read, edit, all]: [&Script<'_>; 4]) {
    for (script, reads, edits, purges) in [
        (post, false, false, false),
        (read, true, false, false),
        (edit, true, true, false),
        (all, true, true, true),
    ] {
        let status = |may: bool, status: u16| if may { status } else { 401 };
        assert_eq!(script.get(path).status, status(reads, 200));
        assert_eq!(script.get("/v1/reports/stats").status, status(reads, 200));
        assert_eq!(script.get("/v1/reports/admins").status, status(edits, 200));
        for what in ["facets", "crashes", "search"] {
            let said = script.tell(&format!("POST /v1/reports/{what}"), "{}");
            assert_eq!(said.status, status(reads, 200), "{what}");
        }
        // a change that the rules refuse, so that nothing changes
        let said = script.tell(&format!("PATCH {path}"), r#"{"change":{}}"#);
        assert_eq!(said.status, status(edits, 400));
        // a purge of a report that is not in the trash
        assert_eq!(
            script.bare(&format!("DELETE {path}")).status,
            status(purges, 400)
        );
        let said = script.tell("POST /v1/reports/purge", r#"{"reports":[]}"#);
        assert_eq!(said.status, status(purges, 400));
    }
}

fn reading(read: &Script<'_>, report: &str, gzip: &[u8], filer: [u8; 16]) {
    let token = central_api::reports::hex(&filer);
    let said = read.tell(
        "POST /v1/reports/search",
        &format!(r#"{{"filter":{{"token":"{token}","channel":"token"}}}}"#),
    );
    let page: report_api::Page = said.read();
    assert_eq!(page.total, 1);
    let found = &page.reports[0];
    assert_eq!(central_api::reports::hex(&found.id.0), report);
    assert_eq!(
        (found.channel, found.token, found.source.as_str()),
        (Channel::Token, Some(Id(filer)), "Only post")
    );
    let said = read.tell(
        "POST /v1/reports/search",
        r#"{"filter":{"gateway":"Only post"}}"#,
    );
    assert_eq!(
        said.read::<report_api::Page>().total,
        0,
        "a token is no gateway"
    );
    let detail: Detail = read.get(&format!("/v1/reports/{report}")).read();
    assert_eq!(detail.text, "From a script");
    assert_eq!(
        detail.history[0].by,
        Some(By::Token {
            id: Id(filer),
            name: "Only post".into()
        })
    );
    let said = read.get(&format!("/v1/reports/{report}/client"));
    assert_eq!(said.status, 200);
    assert!(
        said.head
            .to_lowercase()
            .contains("content-type: application/gzip")
    );
    assert_eq!(said.body, gzip);
    // announced and never sent, and never announced
    assert_eq!(
        read.get(&format!("/v1/reports/{report}/record")).status,
        400
    );
    assert_eq!(
        read.get("/v1/reports/00000000000000000000000000000000")
            .status,
        400
    );
}

/// A report changed, written on, trashed and deleted for good; its history
/// names the token.
fn triage(edit: &Script<'_>, all: &Script<'_>, path: &str, token: [u8; 16]) {
    let change = r#"{"change":{"status":"triaged","labels":["ui"],"priority":"high"},"expected_revision":0}"#;
    let said = edit.tell(&format!("PATCH {path}"), change);
    assert_eq!(said.status, 200);
    let detail: Detail = said.read();
    assert_eq!(
        (detail.report.status, detail.report.revision),
        (ReportStatus::Triaged, 1)
    );
    assert_eq!(edit.tell(&format!("PATCH {path}"), change).status, 409);
    let said = edit.tell(&format!("POST {path}/notes"), r#"{"text":"Seen it"}"#);
    let detail: Detail = said.read();
    let by = By::Token {
        id: Id(token),
        name: "Read and edit".into(),
    };
    assert_eq!(detail.notes[0].by, by);
    assert!(
        detail
            .history
            .iter()
            .filter(|row| row.action != "report.filed")
            .all(|row| row.by.as_ref() == Some(&by))
    );
    let note = central_api::reports::hex(&detail.notes[0].id.0);
    let remove = format!("DELETE {path}/notes/{note}");
    assert_eq!(all.bare(&remove).status, 400, "another token's note");
    assert_eq!(edit.bare(&remove).status, 200);
    assert_eq!(edit.bare(&remove).status, 400, "gone");
    assert_eq!(edit.bare(&format!("POST {path}/trash")).status, 204);
    assert_eq!(edit.bare(&format!("POST {path}/restore")).status, 204);
    assert_eq!(
        all.bare(&format!("DELETE {path}")).status,
        400,
        "not in the trash"
    );
    assert_eq!(edit.bare(&format!("POST {path}/trash")).status, 204);
    assert_eq!(edit.bare(&format!("DELETE {path}")).status, 401);
    assert_eq!(all.bare(&format!("DELETE {path}")).status, 204);
    assert_eq!(all.get(path).status, 400, "gone for good");
}

fn selections(post: &Script<'_>, edit: &Script<'_>, all: &Script<'_>, client: &Announced) {
    let reports = [post.file(client), post.file(client)];
    let ids = format!(r#""reports":["{}","{}"]"#, reports[0], reports[1]);
    for (bulk, changed) in [
        (r#"{"do":"change","change":{"labels_add":["batch"]}}"#, 2),
        (r#"{"do":"trash"}"#, 2),
        (r#"{"do":"trash"}"#, 0),
    ] {
        let said = edit.tell(
            "POST /v1/reports/bulk",
            &format!(r#"{{{ids},"bulk":{bulk}}}"#),
        );
        assert_eq!(said.read::<report_api::Changed>().changed, changed);
    }
    let purge = format!("{{{ids}}}");
    assert_eq!(edit.tell("POST /v1/reports/purge", &purge).status, 401);
    let said = all.tell("POST /v1/reports/purge", &purge);
    assert_eq!(said.read::<report_api::Changed>().changed, 2);
}

/// A report token is nothing anywhere else, and nothing else is one; a page
/// and another protocol reach nothing.
fn elsewhere(http: &Http, bridge: &Token, all: &Script<'_>) {
    for request in [
        Request::GatewayStatus,
        Request::SessionStatus,
        Request::Terms,
        Request::Admin {
            request: AdminRequest::ReportTokens,
        },
    ] {
        let body = central_api::encode(&request).unwrap();
        let said = send(
            http,
            &Ask {
                line: "POST /v1/native",
                bearer: &all.bearer,
                body: Some((JSON, &body)),
                head: "",
            },
        );
        assert_eq!(said.status, 401);
    }
    let hex = all.bearer.replace("brt_", "");
    for bearer in [bridge.bearer().as_str(), hex.as_str()] {
        let said = send(
            http,
            &Ask {
                line: "GET /v1/reports/stats",
                bearer,
                body: None,
                head: "",
            },
        );
        assert_eq!(said.status, 401);
    }
    for (head, status) in [
        ("Origin: https://baylee.app\r\n", 403),
        ("Sec-Fetch-Site: same-origin\r\n", 403),
        ("Baylee-Central-Protocol: 1\r\n", 426),
    ] {
        let said = send(
            http,
            &Ask {
                line: "GET /v1/reports/stats",
                bearer: &all.bearer,
                body: None,
                head,
            },
        );
        assert_eq!(said.status, status, "{head}");
    }
}

/// A bad token is refused at once, its body unread; and thirty of them from
/// one address in an hour are the last the store is asked about.
fn refusals(http: &Http) {
    let unknown = format!("Bearer brt_{}", Token::generate().unwrap().hex().as_str());
    let started = Instant::now();
    pace();
    let mut stream = TcpStream::connect_timeout(&http.address, Duration::from_secs(3)).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    write!(
        stream,
        "POST /v1/reports/search HTTP/1.1\r\nHost: {}\r\n{PROTOCOL_HEADER}: {PROTOCOL_VERSION}\r\nContent-Type: {JSON}\r\nContent-Length: 60000\r\nAuthorization: {unknown}\r\n\r\n",
        http.address
    )
    .unwrap();
    let mut answer = String::new();
    stream.take(8_192).read_to_string(&mut answer).unwrap();
    assert!(answer.starts_with("HTTP/1.1 401"), "{answer}");
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "its body was waited for"
    );
    let status = |bearer: &str, hop: &str| {
        send(
            http,
            &Ask {
                line: "GET /v1/reports/stats",
                bearer,
                body: None,
                head: &format!("X-Forwarded-For: {hop}\r\n"),
            },
        )
        .status
    };
    for _ in 0..30 {
        assert_eq!(status(&unknown, "198.51.100.44"), 401);
    }
    assert_eq!(status(&unknown, "198.51.100.44"), 429);
    assert_eq!(status(&unknown, "198.51.100.45"), 401);
}
