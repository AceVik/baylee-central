use super::{
    addresses::Budgets,
    boundary::{Origin, Pace, REJECTED, UPLOAD_PACE, paced, timed_out},
    in_flight::Client,
    unplaced::Unplaced,
    uploads::Uploads,
};
use axum::{
    Router,
    body::{Body, Bytes},
    extract::{Path, Request, State},
    http::{Extensions, HeaderMap, HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use central_api::{
    Error, PROTOCOL_HEADER, PROTOCOL_VERSION, Token, UPDATE_REQUIRED_BODY, UPDATE_REQUIRED_STATUS,
    reports::Part,
};
use central_identity::{IdentityService, Place, Presented};
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

#[derive(Clone)]
struct Native {
    identity: Arc<IdentityService>,
    /// The report parts being uploaded, and each gateway's unfinished ones
    /// (`uploads.rs`).
    uploads: Uploads,
    /// What each client address may ask in its window (`addresses.rs`).
    budgets: Arc<Mutex<Budgets>>,
    /// On the deployed listener, the requests that came without the
    /// proxy's address (`unplaced.rs`); none where that is as it should
    /// be, on a development listener.
    unplaced: Option<Arc<Unplaced>>,
}

/// The account listener's routes. `deployed` is the listener behind the
/// public name's proxy: there a request without the proxy's address is
/// counted and reported, never refused (`unplaced.rs`).
pub(crate) fn router(identity: Arc<IdentityService>, deployed: bool) -> Router {
    Router::new()
        .route("/v1/native", post(call))
        // A gateway's reports (`docs/reports.md`): the metadata, then each
        // announced part as raw gzip.
        .route("/v1/reports", post(file))
        .route("/v1/reports/{report}/{part}", put(file_part))
        // Above this line every route speaks the protocol; the key set below is public.
        .layer(middleware::from_fn(protocol))
        .route("/.well-known/jwks.json", get(jwks))
        .with_state(Native {
            identity,
            uploads: Uploads::default(),
            budgets: Arc::default(),
            unplaced: deployed.then(Arc::default),
        })
}

impl Native {
    /// The client address the boundary attached, if it knows one. It
    /// shares out budgets and grants nothing (`boundary.rs`).
    fn origin(&self, extensions: &Extensions) -> Option<Client> {
        let from = extensions.get::<Origin>().and_then(|origin| origin.0);
        if from.is_none()
            && let Some(unplaced) = &self.unplaced
        {
            unplaced.note();
        }
        from
    }
}

/// The answer to a build too old to be served: one fixed refusal a client can read. A
/// build that cannot read it still sees a status it does not take for a refused password.
pub(crate) fn update_required() -> Response {
    let status = StatusCode::from_u16(UPDATE_REQUIRED_STATUS).unwrap_or(StatusCode::BAD_REQUEST);
    (
        status,
        [
            (header::CONTENT_TYPE, "application/json"),
            // RFC 9110 asks a 426 to name what it requires.
            (header::UPGRADE, "baylee-central/2"),
        ],
        UPDATE_REQUIRED_BODY,
    )
        .into_response()
}

/// Refuse any other protocol before the request is decoded: before its credential, its budget
/// or the database. The boundary has read a small body by then, within its own bounds
/// (`boundary.rs`); a report part's body is not read at all. An old client hears this at its
/// first request, not as a failure to decode the answer to a sign-in that has already happened.
async fn protocol(request: Request, next: Next) -> Response {
    let mut values = request.headers().get_all(PROTOCOL_HEADER).iter();
    if values.next().map(HeaderValue::as_bytes) == Some(PROTOCOL_VERSION.as_bytes())
        && values.next().is_none()
    {
        return next.run(request).await;
    }
    update_required()
}

/// `GET /.well-known/jwks.json`: the keys tokens verify under. Public and made once.
async fn jwks(State(native): State<Native>) -> Response {
    match native.identity.jwks() {
        Ok(document) => (
            [
                (header::CONTENT_TYPE, "application/jwk-set+json"),
                (header::CACHE_CONTROL, "public, max-age=300"),
            ],
            document.to_vec(),
        )
            .into_response(),
        Err(error) => failure(error),
    }
}

/// Fixed public vocabulary; no parser/database/crypto diagnostics reach the peer.
fn failure(error: Error) -> Response {
    let status = match error {
        Error::UpdateRequired => return update_required(),
        Error::Rejected => StatusCode::UNAUTHORIZED,
        Error::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
        Error::Conflict => StatusCode::CONFLICT,
        Error::Busy => StatusCode::TOO_MANY_REQUESTS,
        Error::TermsRequired => StatusCode::FORBIDDEN,
    };
    (status, REJECTED).into_response()
}

/// Why a report part's upload stopped.
enum Stopped {
    /// The service or a check refused it.
    Failed(Error),
    /// Its body did not keep [`UPLOAD_PACE`] (408).
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
            Self::Failed(error) => failure(error),
            Self::Stalled => timed_out(),
        }
    }
}

/// Whether an upload that held a slot and ended with `answer` is held
/// against its gateway (`uploads.rs`): one whose part was not stored
/// because of what was sent or not sent. A part stored meanwhile by the
/// gateway's other upload (`Conflict`, which a gateway takes as sent) and
/// a store that failed are nobody's unfinished upload.
fn unfinished(answer: &Result<(), Stopped>) -> bool {
    !matches!(
        answer,
        Ok(()) | Err(Stopped::Failed(Error::Conflict | Error::Unavailable))
    )
}

/// A part's body, at most `length` bytes, as long as it keeps `pace`.
async fn part_body(body: Body, length: usize, pace: Pace) -> Result<Bytes, Stopped> {
    paced(body, length, pace)
        .await
        .map_err(|status| match status {
            StatusCode::REQUEST_TIMEOUT => Stopped::Stalled,
            _ => Stopped::Failed(Error::Rejected),
        })
}

async fn call(
    State(native): State<Native>,
    extensions: Extensions,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    match process(&native, native.origin(&extensions), &headers, &body).await {
        Ok(bytes) => ([(header::CONTENT_TYPE, "application/json")], bytes).into_response(),
        Err(error) => failure(error),
    }
}

/// Exactly one `Content-Type`, this one, and at most one `Authorization`.
fn single_headers(headers: &HeaderMap, content_type: &str) -> Result<(), Error> {
    if headers.get_all(header::CONTENT_TYPE).iter().count() != 1
        || headers
            .get(header::CONTENT_TYPE)
            .and_then(|h| h.to_str().ok())
            != Some(content_type)
        || headers.get_all(header::AUTHORIZATION).iter().count() > 1
    {
        return Err(Error::Rejected);
    }
    Ok(())
}

/// A gateway's credential, on the routes that take nothing else.
fn credential(headers: &HeaderMap, content_type: &str) -> Result<Option<Token>, Error> {
    single_headers(headers, content_type)?;
    headers
        .get(header::AUTHORIZATION)
        .map(|h| Token::from_bearer(h.to_str().map_err(|_| Error::Rejected)?))
        .transpose()
}

/// A credential or a signed session, on the route that takes both. A signed session's check
/// is counted against `place`'s share of the signature checks (`limits.rs`).
fn presented(
    identity: &IdentityService,
    headers: &HeaderMap,
    place: Option<Place>,
) -> Result<Option<Presented>, Error> {
    single_headers(headers, "application/json")?;
    headers
        .get(header::AUTHORIZATION)
        .map(|h| identity.present(h.to_str().map_err(|_| Error::Rejected)?, place))
        .transpose()
}

/// One native request from `from`: its bearer read first (a signed session is verified
/// under its own budget and its address's share of it, unless Central issued or verified
/// that token before, `limits.rs` and `verified.rs`), then its address's budget for the operation
/// (`addresses.rs`), then the service, which is told the address for its guesses at
/// passwords and its share of the requests without a session.
async fn process(
    native: &Native,
    from: Option<Client>,
    headers: &HeaderMap,
    body: &[u8],
) -> Result<Vec<u8>, Error> {
    let presented = presented(&native.identity, headers, from.map(Place::from))?;
    let request = central_api::decode(body)?;
    if let Some(client) = from {
        // The guard is gone before anything is awaited.
        let charged = native
            .budgets
            .lock()
            .map_err(|_| Error::Unavailable)?
            .charge(&request, client, Instant::now());
        if !charged {
            return Err(Error::Busy);
        }
    }
    let response = native
        .identity
        .handle_from(request, presented.as_ref(), from.map(Place::from))
        .await?;
    central_api::encode(&response)
}

/// `POST /v1/reports`: `201 {"report_id": "<32 hex>"}`.
async fn file(
    State(native): State<Native>,
    extensions: Extensions,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let filed = async {
        let token = credential(&headers, "application/json")?;
        let place = native.origin(&extensions).map(Place::from);
        native
            .identity
            .file_report(token.as_ref(), &body, place)
            .await
    };
    match filed.await {
        Ok(id) => (
            StatusCode::CREATED,
            [(header::CONTENT_TYPE, "application/json")],
            format!(r#"{{"report_id":"{}"}}"#, central_api::reports::hex(&id)),
        )
            .into_response(),
        Err(error) => failure(error),
    }
}

/// `PUT /v1/reports/{report}/{client|record}`: the gzip as the raw body;
/// `204`. The credential and the announcement come first, then the
/// declared length must be the announced one, then a slot (`uploads.rs`:
/// a few in all, two for one gateway, none for a gateway that left too
/// many uploads unfinished in the hour), and only then is the body read,
/// exactly that long and at [`UPLOAD_PACE`]: a body that is withheld gives
/// its slot back at the pace's first empty window, not at the upload's
/// whole time. An upload that is dropped while it holds a slot (its
/// connection went away, its whole time was over) gives the slot back
/// too, and is counted unfinished like one that stalled.
async fn file_part(
    State(native): State<Native>,
    Path((report, part)): Path<(String, String)>,
    request: Request,
) -> Response {
    let filed = async {
        let (head, body) = request.into_parts();
        let token = credential(&head.headers, "application/gzip")?;
        let (Some(report), Ok(part)) = (super::browser::hex_id(&report), Part::from_stored(&part))
        else {
            return Err(Stopped::Failed(Error::Rejected));
        };
        let place = native.origin(&head.extensions).map(Place::from);
        let upload = native
            .identity
            .report_upload(token.as_ref(), report, part, place)
            .await?;
        let declared = head
            .headers
            .get(header::CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok());
        if declared != Some(u64::from(upload.bytes)) {
            return Err(Stopped::Failed(Error::Rejected));
        }
        let length = usize::try_from(upload.bytes).map_err(|_| Error::Rejected)?;
        let slot = native.uploads.begin(upload.gateway(), Instant::now())?;
        let stored = match part_body(body, length, UPLOAD_PACE).await {
            Ok(gzip) => native
                .identity
                .file_report_part(&upload, &gzip)
                .await
                .map_err(Stopped::Failed),
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
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use std::time::Duration;
    use tower::ServiceExt;

    /// A body announced and never sent.
    struct Withheld;

    impl hyper::body::Body for Withheld {
        type Data = Bytes;
        type Error = std::io::Error;

        fn poll_frame(
            self: std::pin::Pin<&mut Self>,
            _: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Option<Result<hyper::body::Frame<Bytes>, Self::Error>>> {
            std::task::Poll::Pending
        }
    }

    async fn said(response: Response) -> (StatusCode, String) {
        let status = response.status();
        let text = to_bytes(response.into_body(), 1024).await.unwrap();
        (status, String::from_utf8(text.to_vec()).unwrap())
    }

    #[tokio::test]
    async fn a_part_s_body_that_is_withheld_times_out_at_its_pace_and_others_are_refused() {
        let pace = Pace {
            window: Duration::from_millis(40),
            bytes: 1_000,
        };
        // withheld: 408 at the first window, long before an upload's minute
        let started = std::time::Instant::now();
        let Err(stopped) = part_body(Body::new(Withheld), 2_000, pace).await else {
            panic!("a withheld body was read");
        };
        assert!(started.elapsed() < Duration::from_secs(2));
        assert_eq!(
            said(stopped.into_response()).await,
            (StatusCode::REQUEST_TIMEOUT, "request timed out\n".into())
        );
        // longer than announced: the fixed refusal, as before
        let Err(stopped) = part_body(Body::from(vec![0x1f; 2_001]), 2_000, pace).await else {
            panic!("a body past its announced length was read");
        };
        assert_eq!(
            said(stopped.into_response()).await,
            (StatusCode::UNAUTHORIZED, REJECTED.into())
        );
        // exactly as announced
        let whole = part_body(Body::from(vec![0x1f; 2_000]), 2_000, pace).await;
        assert!(whole.is_ok_and(|bytes| bytes.len() == 2_000));
        // the pace asked of a real upload: 16 KiB in every five seconds
        assert_eq!(
            (UPLOAD_PACE.window, UPLOAD_PACE.bytes),
            (Duration::from_secs(5), 16_384)
        );
    }

    #[test]
    fn an_upload_is_unfinished_unless_its_part_is_stored_or_the_store_failed() {
        // stored, stored meanwhile by the other upload, the store down
        for answer in [
            Ok(()),
            Err(Stopped::Failed(Error::Conflict)),
            Err(Stopped::Failed(Error::Unavailable)),
        ] {
            assert!(!unfinished(&answer));
        }
        // too slow, bytes other than announced or a connection that broke
        for answer in [
            Err(Stopped::Stalled),
            Err(Stopped::Failed(Error::Rejected)),
            Err(Stopped::Failed(Error::Busy)),
        ] {
            assert!(unfinished(&answer));
        }
    }

    /// The refusal in front of a route that must not be reached.
    fn guarded() -> Router {
        Router::new()
            .route("/v1/native", post(|| async { "reached" }))
            .layer(middleware::from_fn(protocol))
    }

    async fn call(values: &[&str]) -> Response {
        let mut request = axum::http::Request::post("/v1/native");
        for value in values {
            request = request.header(PROTOCOL_HEADER, *value);
        }
        guarded()
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn the_protocol_of_this_build_reaches_the_route() {
        let response = call(&[PROTOCOL_VERSION]).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 64).await.unwrap();
        assert_eq!(&body[..], b"reached");
    }

    #[tokio::test]
    async fn any_other_protocol_is_told_to_update_and_reaches_nothing() {
        for values in [
            &[][..],
            &["1"],
            &["3"],
            &["2 "],
            &["02"],
            &[""],
            &["2", "2"],
        ] {
            let response = call(values).await;
            assert_eq!(response.status().as_u16(), 426, "{values:?}");
            assert_eq!(
                response.headers().get(header::CONTENT_TYPE).unwrap(),
                "application/json"
            );
            assert!(response.headers().contains_key(header::UPGRADE));
            let body = to_bytes(response.into_body(), 64).await.unwrap();
            assert_eq!(&body[..], UPDATE_REQUIRED_BODY.as_bytes(), "{values:?}");
        }
    }

    #[test]
    fn the_refusal_is_the_contracts_form() {
        assert_eq!(
            UPDATE_REQUIRED_BODY,
            r#"{"refusal":{"code":"update_required"}}"#
        );
        assert_eq!(UPDATE_REQUIRED_STATUS, 426);
    }

    #[tokio::test]
    async fn an_update_required_error_answers_as_the_refusal_does() {
        let response = failure(Error::UpdateRequired);
        assert_eq!(response.status().as_u16(), 426);
        let body = to_bytes(response.into_body(), 64).await.unwrap();
        assert_eq!(&body[..], UPDATE_REQUIRED_BODY.as_bytes());
    }

    #[test]
    fn a_session_is_never_taken_where_only_a_credential_is() {
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());
        headers.insert(header::AUTHORIZATION, "Bearer a.b.c".parse().unwrap());
        assert!(matches!(
            credential(&headers, "application/json"),
            Err(Error::Rejected)
        ));
    }
}
