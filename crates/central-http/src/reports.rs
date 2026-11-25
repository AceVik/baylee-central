//! The native listener's report routes (`docs/reports.md` §"Intake" and
//! §"Report tokens"): a gateway or a report token files reports and their
//! parts, and a report token reads and triages them under its rights.
//!
//! **The credential first.** The right a request needs follows from its
//! route alone, so it is asked for before a byte of the body is read; the
//! boundary leaves these bodies to this module ([`Unread`]). Until then every
//! refusal is the native route's fixed one, whatever its cause; a report
//! token that passed it is told a request the rules refuse as `400`. A small
//! body of a refused request is thrown away as it comes ([`discard`]), so
//! that its sender reads the answer.
//!
//! **Which credential.** `Bearer brt_…` is a report token and nothing else,
//! `Bearer <64 hex>` a gateway's or the bridge's and nothing else
//! ([`ReportBearer`]); only filing takes both.
use super::{
    addresses::Budgets,
    boundary::{Pace, Refusal, UPLOAD_PACE, Unread, paced, timed_out},
    browser::hex_id,
    in_flight::Client,
    native::{REPORT_ROUTE_REJECTED, failure, origin, single_headers},
    slots::{CALLS, CALLS_PER_TOKEN, DOWNLOADS, DOWNLOADS_PER_TOKEN, Slot, Slots},
    unplaced::Unplaced,
    uploads::Uploads,
};
use axum::{
    Router,
    body::{Body, Bytes},
    extract::{Path, Request, State},
    http::{Extensions, HeaderMap, HeaderName, HeaderValue, StatusCode, header, request::Parts},
    response::{IntoResponse, Response},
    routing::{get, patch, post, put},
};
use central_api::{
    Error, Token, report_api,
    report_tokens::{ReportBearer, ReportScope},
    reports::{MAX_INTAKE_BYTES, Part, RecordOrigin},
};
use central_identity::{IdentityService, Place, ReportCall, ReportReply};
use http_body_util::{BodyExt, Limited};
use hyper::body::Body as _;
use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
    time::{Duration, Instant},
};

/// What answers the report routes: the identity service, or a test's
/// stand-in. Each method that takes a credential is asked before the body.
pub(crate) trait Reports: Send + Sync + 'static {
    /// What files, its credential known to be good.
    type Filing: Send + Sync;

    /// Begin a filing or a part's upload: refused for the credential alone.
    fn filing(
        &self,
        bearer: &ReportBearer,
        place: Option<Place>,
    ) -> impl Future<Output = Result<Self::Filing, Error>> + Send;

    /// What files, by its id.
    fn filer(filing: &Self::Filing) -> [u8; 16];

    /// File the metadata of a report.
    fn file(
        &self,
        filing: &Self::Filing,
        body: &[u8],
    ) -> impl Future<Output = Result<[u8; 16], Error>> + Send;

    /// How many bytes a report's part was announced with.
    fn announced(
        &self,
        filing: &Self::Filing,
        report: [u8; 16],
        part: Part,
    ) -> impl Future<Output = Result<u32, Error>> + Send;

    /// Store a part's bytes.
    fn store(
        &self,
        filing: &Self::Filing,
        report: [u8; 16],
        part: Part,
        gzip: &[u8],
    ) -> impl Future<Output = Result<(), Error>> + Send;

    /// Begin a report token's request, refused for the token alone; its id.
    fn access(
        &self,
        token: &Token,
        right: ReportScope,
        download: bool,
        place: Option<Place>,
    ) -> impl Future<Output = Result<[u8; 16], Error>> + Send;

    /// Answer a report token's call.
    fn call(
        &self,
        id: [u8; 16],
        token: &Token,
        call: ReportCall,
    ) -> impl Future<Output = Result<ReportReply, Error>> + Send;

    /// A report's part as stored, for a report token.
    fn download(
        &self,
        token: &Token,
        report: [u8; 16],
        part: Part,
    ) -> impl Future<Output = Result<(Vec<u8>, Option<RecordOrigin>), Error>> + Send;
}

impl Reports for IdentityService {
    type Filing = central_identity::Filing;

    fn filing(
        &self,
        bearer: &ReportBearer,
        place: Option<Place>,
    ) -> impl Future<Output = Result<Self::Filing, Error>> + Send {
        self.report_filing(bearer, place)
    }

    fn file(
        &self,
        filing: &Self::Filing,
        body: &[u8],
    ) -> impl Future<Output = Result<[u8; 16], Error>> + Send {
        self.file_report(filing, body)
    }

    fn filer(filing: &Self::Filing) -> [u8; 16] {
        filing.filer()
    }

    fn announced(
        &self,
        filing: &Self::Filing,
        report: [u8; 16],
        part: Part,
    ) -> impl Future<Output = Result<u32, Error>> + Send {
        self.report_upload(filing, report, part)
    }

    fn store(
        &self,
        filing: &Self::Filing,
        report: [u8; 16],
        part: Part,
        gzip: &[u8],
    ) -> impl Future<Output = Result<(), Error>> + Send {
        self.file_report_part(filing, report, part, gzip)
    }

    fn access(
        &self,
        token: &Token,
        right: ReportScope,
        download: bool,
        place: Option<Place>,
    ) -> impl Future<Output = Result<[u8; 16], Error>> + Send {
        self.report_access(token, right, download, place)
    }

    fn call(
        &self,
        id: [u8; 16],
        token: &Token,
        call: ReportCall,
    ) -> impl Future<Output = Result<ReportReply, Error>> + Send {
        self.report_call(id, token, call)
    }

    fn download(
        &self,
        token: &Token,
        report: [u8; 16],
        part: Part,
    ) -> impl Future<Output = Result<(Vec<u8>, Option<RecordOrigin>), Error>> + Send {
        self.report_download(token, report, part)
    }
}

struct Desk<R> {
    reports: Arc<R>,
    /// The report parts being uploaded, and each filer's unfinished ones.
    uploads: Uploads,
    /// Report tokens' calls that ask the store, and their downloads.
    calls: Slots,
    downloads: Slots,
    /// What each client address may ask in its window (`addresses.rs`).
    budgets: Arc<Mutex<Budgets>>,
    /// As the native routes' (`native.rs`).
    unplaced: Option<Arc<Unplaced>>,
}

/// The report routes, sharing the native listener's budgets.
pub(crate) fn router<R: Reports, S: Clone + Send + Sync + 'static>(
    reports: Arc<R>,
    budgets: Arc<Mutex<Budgets>>,
    unplaced: Option<Arc<Unplaced>>,
) -> Router<S> {
    Router::new()
        .route("/v1/reports", post(file::<R>))
        .route("/v1/reports/search", post(search::<R>))
        .route("/v1/reports/facets", post(facets::<R>))
        .route("/v1/reports/crashes", post(crashes::<R>))
        .route("/v1/reports/stats", get(stats::<R>))
        .route("/v1/reports/admins", get(admins::<R>))
        .route("/v1/reports/bulk", post(bulk::<R>))
        .route("/v1/reports/purge", post(purge_all::<R>))
        .route(
            "/v1/reports/{report}",
            get(one::<R>).merge(patch(change::<R>).delete(purge::<R>)),
        )
        .route(
            "/v1/reports/{report}/{what}",
            put(file_part::<R>)
                .merge(get(download::<R>))
                .merge(post(act::<R>)),
        )
        .route(
            "/v1/reports/{report}/notes/{note}",
            axum::routing::delete(remove_note::<R>),
        )
        .with_state(Arc::new(Desk {
            reports,
            uploads: Uploads::default(),
            calls: Slots::new(CALLS, CALLS_PER_TOKEN),
            downloads: Slots::new(DOWNLOADS, DOWNLOADS_PER_TOKEN),
            budgets,
            unplaced,
        }))
}

/// Why a report route's request stopped.
enum Stopped {
    /// Its credential, or a gateway's request: the native route's fixed
    /// answers, the same `401` for every refusal.
    Failed(Error),
    /// A report token's own request, its credential known to be good: what
    /// the rules refuse is a bad request.
    Request(Error),
    /// Its body was refused (the boundary's answer).
    Body(Refusal),
    /// Its part did not keep [`UPLOAD_PACE`] (408).
    Stalled,
}

impl From<Error> for Stopped {
    fn from(error: Error) -> Self {
        Self::Failed(error)
    }
}

impl IntoResponse for Stopped {
    fn into_response(self) -> Response {
        match self {
            Self::Request(Error::Rejected) => {
                (StatusCode::BAD_REQUEST, REPORT_ROUTE_REJECTED).into_response()
            }
            Self::Failed(error) | Self::Request(error) => failure(error),
            Self::Body(refusal) => refusal.into_response(),
            Self::Stalled => timed_out(),
        }
    }
}

/// Nothing is there: what the boundary says of every other path.
fn nowhere() -> Response {
    (StatusCode::NOT_FOUND, "not found\n").into_response()
}

/// The one credential of a report route.
fn bearer(headers: &HeaderMap) -> Result<ReportBearer, Error> {
    let mut values = headers.get_all(header::AUTHORIZATION).iter();
    match (values.next(), values.next()) {
        (Some(one), None) => ReportBearer::from_header(one.to_str().map_err(|_| Error::Rejected)?),
        _ => Err(Error::Rejected),
    }
}

/// A part's body, at most `length` bytes, as long as it keeps `pace`; a
/// longer or a broken one is `refused`.
async fn part_body(
    body: Body,
    length: usize,
    pace: Pace,
    refused: fn(Error) -> Stopped,
) -> Result<Bytes, Stopped> {
    paced(body, length, pace)
        .await
        .map_err(|status| match status {
            StatusCode::REQUEST_TIMEOUT => Stopped::Stalled,
            _ => refused(Error::Rejected),
        })
}

/// Whether an upload that held a slot and ended with `answer` is held
/// against its filer (`uploads.rs`): one whose part was not stored because
/// of what was sent or not sent. A part stored meanwhile by the filer's
/// other upload (`Conflict`, which a gateway takes as sent) and a store
/// that failed are nobody's unfinished upload.
fn unfinished(answer: &Result<(), Stopped>) -> bool {
    !matches!(
        answer,
        Ok(())
            | Err(Stopped::Failed(Error::Conflict | Error::Unavailable)
                | Stopped::Request(Error::Conflict | Error::Unavailable))
    )
}

impl<R: Reports> Desk<R> {
    fn origin(&self, extensions: &Extensions) -> Option<Client> {
        origin(extensions, self.unplaced.as_deref())
    }

    /// Whether the store may be asked about a report token from `from`.
    fn may_present(&self, from: Option<Client>) -> Result<(), Error> {
        let Some(client) = from else { return Ok(()) };
        let may = self
            .budgets
            .lock()
            .map_err(|_| Error::Unavailable)?
            .may_present(client, Instant::now());
        if may { Ok(()) } else { Err(Error::Busy) }
    }

    /// What the store said of a credential: a refused report token is
    /// counted for its address.
    fn heard<T>(
        &self,
        said: Result<T, Error>,
        report_token: bool,
        from: Option<Client>,
    ) -> Result<T, Error> {
        if report_token
            && matches!(said, Err(Error::Rejected))
            && let Some(client) = from
            && let Ok(mut budgets) = self.budgets.lock()
        {
            budgets.report_refused(client, Instant::now());
        }
        said
    }

    /// A report token's request, its credential known to be good for `right`
    /// and one of its calls at once: the token, its id and the slot, held
    /// until the store has answered.
    async fn begin(
        &self,
        head: &Parts,
        right: ReportScope,
        download: bool,
    ) -> Result<(Token, [u8; 16], Slot), Error> {
        let ReportBearer::Report(token) = bearer(&head.headers)? else {
            return Err(Error::Rejected);
        };
        let from = self.origin(&head.extensions);
        self.may_present(from)?;
        let place = from.map(Place::from);
        let said = self.reports.access(&token, right, download, place).await;
        let id = self.heard(said, true, from)?;
        let slot = self.calls.take(id)?;
        Ok((token, id, slot))
    }

    /// A report token's request without a body.
    async fn ask(
        &self,
        request: Request,
        right: ReportScope,
        call: Option<ReportCall>,
    ) -> Result<ReportReply, Stopped> {
        let (head, body) = request.into_parts();
        let begun = async {
            let begun = self.begin(&head, right, false).await?;
            bodyless(&head)?;
            let call = call.ok_or(Stopped::Request(Error::Rejected))?;
            Ok((begun, call))
        };
        let ((token, id, _slot), call) = kept(begun.await, body).await?.0;
        self.reports
            .call(id, &token, call)
            .await
            .map_err(Stopped::Request)
    }

    /// A report token's request with a JSON body, read once its credential
    /// is known to be good.
    async fn tell<T: serde::de::DeserializeOwned>(
        &self,
        request: Request,
        right: ReportScope,
        call: impl FnOnce(T) -> Option<ReportCall>,
    ) -> Result<ReportReply, Stopped> {
        let (head, body) = request.into_parts();
        let begun = async {
            let begun = self.begin(&head, right, false).await?;
            let unqueried = head.uri.query().is_none();
            if !unqueried || single_headers(&head.headers, "application/json").is_err() {
                return Err(Stopped::Request(Error::Rejected));
            }
            Ok((begun, unread(&head)?))
        };
        let (((token, id, _slot), unread), body) = kept(begun.await, body).await?;
        let (bytes, _place) = unread
            .read(&head, body, report_api::MAX_BODY_BYTES)
            .await
            .map_err(Stopped::Body)?;
        let body = central_api::decode(&bytes).map_err(Stopped::Request)?;
        let call = call(body).ok_or(Stopped::Request(Error::Rejected))?;
        self.reports
            .call(id, &token, call)
            .await
            .map_err(Stopped::Request)
    }
}

/// What a request may go on with, and its body still unread; a refused
/// request's is thrown away before its refusal is said ([`discard`]).
async fn kept<T>(begun: Result<T, Stopped>, body: Body) -> Result<(T, Body), Stopped> {
    match begun {
        Ok(begun) => Ok((begun, body)),
        Err(stopped) => {
            discard(body, DISCARD_TIME).await;
            Err(stopped)
        }
    }
}

/// How long a refused request's body may take to be thrown away: the grace
/// every other route's small body has (`boundary.rs`).
const DISCARD_TIME: Duration = Duration::from_millis(250);
/// The longest body that is thrown away; a longer one, or one of unknown
/// length, is left where it is.
const DISCARD_BYTES: u64 = report_api::MAX_BODY_BYTES as u64;

/// Throw a refused request's small body away as it comes, for `time` at
/// most. A connection that closes on bytes which were sent and never read is
/// reset, and the reset takes the answer with it: the sender would see a
/// broken connection instead of its refusal. Nothing of it is kept or looked
/// at.
async fn discard(body: Body, time: Duration) {
    if body
        .size_hint()
        .upper()
        .is_none_or(|bytes| bytes > DISCARD_BYTES)
    {
        return;
    }
    let mut body = Limited::new(body, report_api::MAX_BODY_BYTES);
    let _ = tokio::time::timeout(time, async {
        while let Some(Ok(_)) = body.frame().await {}
    })
    .await;
}

/// The unread body's boundary, which every request here passed.
fn unread(head: &Parts) -> Result<Unread, Stopped> {
    head.extensions
        .get::<Unread>()
        .cloned()
        .ok_or(Stopped::Failed(Error::Unavailable))
}

/// A request that asks nothing with its body or its address's query.
fn bodyless(head: &Parts) -> Result<(), Stopped> {
    let empty = head
        .headers
        .get_all(header::CONTENT_LENGTH)
        .iter()
        .all(|length| length.as_bytes() == b"0")
        && !head.headers.contains_key(header::TRANSFER_ENCODING);
    if empty && head.uri.query().is_none() {
        Ok(())
    } else {
        Err(Stopped::Request(Error::Rejected))
    }
}

fn json<T: serde::Serialize>(status: StatusCode, value: &T) -> Response {
    match report_api::encode(value) {
        Ok(bytes) => (status, [(header::CONTENT_TYPE, "application/json")], bytes).into_response(),
        Err(error) => failure(error),
    }
}

/// A call's answer.
fn answer(said: Result<ReportReply, Stopped>) -> Response {
    match said {
        Ok(ReportReply::Page(page)) => json(StatusCode::OK, &page),
        Ok(ReportReply::Facets(facets)) => json(StatusCode::OK, &facets),
        Ok(ReportReply::Crashes(groups)) => json(StatusCode::OK, &groups),
        Ok(ReportReply::Stats(stats)) => json(StatusCode::OK, &stats),
        Ok(ReportReply::Report(report)) => json(StatusCode::OK, &report),
        Ok(ReportReply::Admins(admins)) => json(StatusCode::OK, &admins),
        Ok(ReportReply::Changed(changed)) => json(StatusCode::OK, &changed),
        Ok(ReportReply::Done) => StatusCode::NO_CONTENT.into_response(),
        Err(stopped) => stopped.into_response(),
    }
}

type Counter<R> = State<Arc<Desk<R>>>;

/// `POST /v1/reports/search`.
async fn search<R: Reports>(State(desk): Counter<R>, request: Request) -> Response {
    let call = |body| Some(ReportCall::Search(body));
    answer(desk.tell(request, ReportScope::Read, call).await)
}

/// `POST /v1/reports/facets`.
async fn facets<R: Reports>(State(desk): Counter<R>, request: Request) -> Response {
    let call = |body| Some(ReportCall::Facets(body));
    answer(desk.tell(request, ReportScope::Read, call).await)
}

/// `POST /v1/reports/crashes`.
async fn crashes<R: Reports>(State(desk): Counter<R>, request: Request) -> Response {
    let call = |body| Some(ReportCall::Crashes(body));
    answer(desk.tell(request, ReportScope::Read, call).await)
}

/// `GET /v1/reports/stats`.
async fn stats<R: Reports>(State(desk): Counter<R>, request: Request) -> Response {
    let call = Some(ReportCall::Stats);
    answer(desk.ask(request, ReportScope::Read, call).await)
}

/// `GET /v1/reports/admins`.
async fn admins<R: Reports>(State(desk): Counter<R>, request: Request) -> Response {
    let call = Some(ReportCall::Admins);
    answer(desk.ask(request, ReportScope::Triage, call).await)
}

/// `POST /v1/reports/bulk`.
async fn bulk<R: Reports>(State(desk): Counter<R>, request: Request) -> Response {
    let call = |body| Some(ReportCall::Bulk(body));
    answer(desk.tell(request, ReportScope::Triage, call).await)
}

/// `POST /v1/reports/purge`.
async fn purge_all<R: Reports>(State(desk): Counter<R>, request: Request) -> Response {
    let call = |body| Some(ReportCall::PurgeAll(body));
    answer(desk.tell(request, ReportScope::Purge, call).await)
}

/// `GET /v1/reports/{report}`.
async fn one<R: Reports>(
    State(desk): Counter<R>,
    Path(report): Path<String>,
    request: Request,
) -> Response {
    let call = hex_id(&report).map(ReportCall::Report);
    answer(desk.ask(request, ReportScope::Read, call).await)
}

/// `PATCH /v1/reports/{report}`.
async fn change<R: Reports>(
    State(desk): Counter<R>,
    Path(report): Path<String>,
    request: Request,
) -> Response {
    let call = |body| hex_id(&report).map(|report| ReportCall::Change(report, body));
    answer(desk.tell(request, ReportScope::Triage, call).await)
}

/// `DELETE /v1/reports/{report}`: for good, from the trash.
async fn purge<R: Reports>(
    State(desk): Counter<R>,
    Path(report): Path<String>,
    request: Request,
) -> Response {
    let call = hex_id(&report).map(ReportCall::Purge);
    answer(desk.ask(request, ReportScope::Purge, call).await)
}

/// `POST /v1/reports/{report}/trash`, `…/restore` and `…/notes`.
async fn act<R: Reports>(
    State(desk): Counter<R>,
    Path((report, what)): Path<(String, String)>,
    request: Request,
) -> Response {
    let report = hex_id(&report);
    let right = ReportScope::Triage;
    match what.as_str() {
        "trash" => {
            let call = report.map(ReportCall::Trash);
            answer(desk.ask(request, right, call).await)
        }
        "restore" => {
            let call = report.map(ReportCall::Restore);
            answer(desk.ask(request, right, call).await)
        }
        "notes" => {
            let call = |body| report.map(|report| ReportCall::Note(report, body));
            answer(desk.tell(request, right, call).await)
        }
        _ => nowhere(),
    }
}

/// `DELETE /v1/reports/{report}/notes/{note}`: the token's own.
async fn remove_note<R: Reports>(
    State(desk): Counter<R>,
    Path((report, note)): Path<(String, String)>,
    request: Request,
) -> Response {
    let call = hex_id(&report)
        .zip(hex_id(&note))
        .map(|(report, note)| ReportCall::RemoveNote(report, note));
    answer(desk.ask(request, ReportScope::Triage, call).await)
}

/// How much of a part is handed on at once: the connection's buffer takes a
/// few of these, so a part's slot is held until nearly all of it is sent.
const CHUNK: usize = 65_536;

/// A part being sent, which holds its download's slot until it is.
struct Sending {
    left: Bytes,
    _slot: Slot,
}

impl hyper::body::Body for Sending {
    type Data = Bytes;
    type Error = std::convert::Infallible;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
    ) -> Poll<Option<Result<hyper::body::Frame<Bytes>, Self::Error>>> {
        if self.left.is_empty() {
            return Poll::Ready(None);
        }
        let next = self.left.len().min(CHUNK);
        let chunk = self.left.split_to(next);
        Poll::Ready(Some(Ok(hyper::body::Frame::data(chunk))))
    }

    fn is_end_stream(&self) -> bool {
        self.left.is_empty()
    }

    fn size_hint(&self) -> hyper::body::SizeHint {
        hyper::body::SizeHint::with_exact(self.left.len() as u64)
    }
}

/// `GET /v1/reports/{report}/{client|record}`: the gzip as stored, its
/// record's writer beside it. One of the token's downloads of its hour, and
/// the one it has at once.
async fn download<R: Reports>(
    State(desk): Counter<R>,
    Path((report, part)): Path<(String, String)>,
    request: Request,
) -> Response {
    let Ok(part) = Part::from_stored(&part) else {
        return nowhere();
    };
    let (head, body) = request.into_parts();
    let sent = async {
        let begun = async {
            let begun = desk.begin(&head, ReportScope::Read, true).await?;
            bodyless(&head)?;
            let report = hex_id(&report).ok_or(Stopped::Request(Error::Rejected))?;
            Ok((begun, report))
        };
        let ((token, id, _slot), report) = kept(begun.await, body).await?.0;
        let slot = desk.downloads.take(id)?;
        let (gzip, origin) = desk
            .reports
            .download(&token, report, part)
            .await
            .map_err(Stopped::Request)?;
        Ok::<_, Stopped>((gzip, origin, slot))
    };
    match sent.await {
        Ok((gzip, origin, slot)) => {
            let body = Body::new(Sending {
                left: Bytes::from(gzip),
                _slot: slot,
            });
            let mut response = ([(header::CONTENT_TYPE, "application/gzip")], body).into_response();
            if let Some(origin) = origin {
                response.headers_mut().insert(
                    HeaderName::from_static("x-record-origin"),
                    HeaderValue::from_static(origin.as_str()),
                );
            }
            response
        }
        Err(stopped) => stopped.into_response(),
    }
}

impl<R: Reports> Desk<R> {
    /// What files, for the routes that take a gateway's credential, the
    /// bridge's or a report token's alike, and how their requests are
    /// refused once it is known to be good.
    async fn filer(&self, head: &Parts) -> Result<(R::Filing, fn(Error) -> Stopped), Stopped> {
        let bearer = bearer(&head.headers)?;
        let report_token = matches!(bearer, ReportBearer::Report(_));
        let from = self.origin(&head.extensions);
        if report_token {
            self.may_present(from)?;
        }
        let said = self.reports.filing(&bearer, from.map(Place::from)).await;
        let filing = self.heard(said, report_token, from)?;
        Ok((
            filing,
            if report_token {
                Stopped::Request
            } else {
                Stopped::Failed
            },
        ))
    }
}

/// `POST /v1/reports`: `201 {"report_id": "<32 hex>"}`, for a gateway's
/// credential, the bridge's, or a report token that may file.
async fn file<R: Reports>(State(desk): Counter<R>, request: Request) -> Response {
    let (head, body) = request.into_parts();
    let filed = async {
        let begun = async {
            let (filing, refused) = desk.filer(&head).await?;
            single_headers(&head.headers, "application/json").map_err(refused)?;
            Ok((filing, refused, unread(&head)?))
        };
        let ((filing, refused, unread), body) = kept(begun.await, body).await?;
        let (bytes, _place) = unread
            .read(&head, body, MAX_INTAKE_BYTES)
            .await
            .map_err(Stopped::Body)?;
        desk.reports.file(&filing, &bytes).await.map_err(refused)
    };
    match filed.await {
        Ok(id) => json(
            StatusCode::CREATED,
            &report_api::Filed {
                report_id: report_api::Id(id),
            },
        ),
        Err(stopped) => stopped.into_response(),
    }
}

/// `PUT /v1/reports/{report}/{client|record}`: the gzip as the raw body;
/// `204`. The credential and the announcement come first, then the
/// declared length must be the announced one, then a slot (`uploads.rs`:
/// a few in all, two for one filer, none for a filer that left too many
/// uploads unfinished in the hour), and only then is the body read,
/// exactly that long and at [`UPLOAD_PACE`]: a body that is withheld gives
/// its slot back at the pace's first empty window, not at the upload's
/// whole time. An upload that is dropped while it holds a slot (its
/// connection went away, its whole time was over) gives the slot back
/// too, and is counted unfinished like one that stalled.
async fn file_part<R: Reports>(
    State(desk): Counter<R>,
    Path((report, part)): Path<(String, String)>,
    request: Request,
) -> Response {
    let filed = async {
        let (head, body) = request.into_parts();
        let begun = async {
            let (filing, refused) = desk.filer(&head).await?;
            single_headers(&head.headers, "application/gzip").map_err(refused)?;
            let (Some(report), Ok(part)) = (hex_id(&report), Part::from_stored(&part)) else {
                return Err(refused(Error::Rejected));
            };
            let bytes = desk
                .reports
                .announced(&filing, report, part)
                .await
                .map_err(refused)?;
            let declared = head
                .headers
                .get(header::CONTENT_LENGTH)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok());
            if declared != Some(u64::from(bytes)) {
                return Err(refused(Error::Rejected));
            }
            let length = usize::try_from(bytes).map_err(|_| Error::Rejected)?;
            let slot = desk.uploads.begin(R::filer(&filing), Instant::now())?;
            Ok((filing, refused, report, part, length, slot))
        };
        let ((filing, refused, report, part, length, slot), body) = kept(begun.await, body).await?;
        let stored = match part_body(body, length, UPLOAD_PACE, refused).await {
            Ok(gzip) => desk
                .reports
                .store(&filing, report, part, &gzip)
                .await
                .map_err(refused),
            Err(stopped) => Err(stopped),
        };
        slot.ended(!unfinished(&stored), Instant::now());
        stored
    };
    match filed.await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(stopped) => stopped.into_response(),
    }
}

#[cfg(test)]
mod tests;
