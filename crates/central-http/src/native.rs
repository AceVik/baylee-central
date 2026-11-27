use super::{
    addresses::Budgets,
    boundary::{Origin, REJECTED},
    in_flight::Client,
    unplaced::Unplaced,
};
use axum::{
    Router,
    body::Bytes,
    extract::{Request, State},
    http::{Extensions, HeaderMap, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use central_api::{
    Error, PROTOCOL_HEADER, PROTOCOL_VERSION, PROTOCOL_VERSIONS, UPDATE_REQUIRED_BODY,
    UPDATE_REQUIRED_STATUS, ranked, wire,
};
use central_identity::{IdentityService, KofiAnswer, Place, Presented};
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

#[derive(Clone)]
struct Native {
    relay: crate::relay::Relay,
    identity: Arc<IdentityService>,
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
    let budgets: Arc<Mutex<Budgets>> = Arc::default();
    let unplaced: Option<Arc<Unplaced>> = deployed.then(Arc::default);
    // Reports (`docs/reports.md`, `reports.rs`): a gateway's, and what a
    // report token files, reads and triages.
    let reports = super::reports::router(
        Arc::clone(&identity),
        Arc::clone(&budgets),
        unplaced.clone(),
    );
    Router::new()
        .route("/v1/native", post(call))
        .merge(reports)
        // Above this line every route speaks the protocol; the key set below is public.
        .layer(middleware::from_fn(protocol))
        .route("/.well-known/jwks.json", get(jwks))
        // The push socket (`ws.rs`): the client's dial sends its bearer and the
        // handshake, no protocol header, and the socket's events are its own
        // fixed vocabulary.
        .route("/v1/ws", get(socket))
        .route("/v1/ranked/socket", get(ranked_socket))
        // Ko-fi's webhook (`docs/operations.md` §"Donations"): Ko-fi speaks no
        // protocol of ours, so it stands below the protocol's layer too.
        .route(KOFI_PATH, post(kofi))
        .with_state(Native {
            relay: crate::relay::Relay::new(!deployed),
            identity,
            budgets,
            unplaced,
        })
}

/// The client address the boundary attached, if it knows one; `unplaced` is
/// told where it should have. It shares out budgets and grants nothing
/// (`boundary.rs`).
pub(crate) fn origin(extensions: &Extensions, unplaced: Option<&Unplaced>) -> Option<Client> {
    let from = extensions.get::<Origin>().and_then(|origin| origin.0);
    if from.is_none()
        && let Some(unplaced) = unplaced
    {
        unplaced.note();
    }
    from
}

impl Native {
    fn origin(&self, extensions: &Extensions) -> Option<Client> {
        origin(extensions, self.unplaced.as_deref())
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
/// (`boundary.rs`); a report route's body is not read at all. An old client hears this at its
/// first request, not as a failure to decode the answer to a sign-in that has already happened.
pub(crate) async fn protocol(request: Request, next: Next) -> Response {
    if spoken(request.headers()).is_some() {
        return next.run(request).await;
    }
    update_required()
}

/// The protocol a request names: exactly one header, one of
/// [`PROTOCOL_VERSIONS`]. A caller of 2 is served without what 3 added.
pub(crate) fn spoken(headers: &HeaderMap) -> Option<&'static str> {
    let mut values = headers.get_all(PROTOCOL_HEADER).iter();
    let first = values.next()?;
    if values.next().is_some() {
        return None;
    }
    PROTOCOL_VERSIONS
        .into_iter()
        .find(|version| first.as_bytes() == version.as_bytes())
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

/// `GET /v1/ws`: a push socket for a signed-in native client (`ws.rs`).
async fn socket(State(native): State<Native>, request: Request) -> Response {
    let from = native.origin(request.extensions());
    super::ws::dial(&native.identity, from, request).await
}

async fn ranked_socket(State(native): State<Native>, request: Request) -> Response {
    let from = native.origin(request.extensions());
    super::ranked_socket::dial(native.relay, native.identity, from, request)
}

/// Where Ko-fi delivers: the one route that takes a platform's word for a
/// payment. No secret is in it; the token is in the body.
pub(crate) const KOFI_PATH: &str = "/v1/kofi";

/// `POST /v1/kofi`: one delivery from Ko-fi. Not found while Ko-fi is not
/// configured. The address's budget is spent first, then the service reads the
/// token before anything else of the body, and every refusal, whatever its
/// reason, is the same plain `400`; a delivery taken, also one that came
/// before, is `200`. The body is never logged, kept or echoed.
async fn kofi(
    State(native): State<Native>,
    extensions: Extensions,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if !native.identity.kofi_listening() {
        return (StatusCode::NOT_FOUND, "not found\n").into_response();
    }
    if let Some(client) = native.origin(&extensions) {
        // The guard is gone before anything is awaited.
        let charged = native
            .budgets
            .lock()
            .is_ok_and(|mut budgets| budgets.charge_kofi(client, Instant::now()));
        if !charged {
            return (StatusCode::TOO_MANY_REQUESTS, REJECTED).into_response();
        }
    }
    let mut types = headers.get_all(header::CONTENT_TYPE).iter();
    let content_type = match (types.next(), types.next()) {
        (Some(one), None) => Some(one.as_bytes()),
        _ => None,
    };
    match native.identity.kofi(content_type, &body).await {
        KofiAnswer::Taken => (StatusCode::OK, "ok\n").into_response(),
        KofiAnswer::Refused => (StatusCode::BAD_REQUEST, REJECTED).into_response(),
        KofiAnswer::Unavailable => (StatusCode::SERVICE_UNAVAILABLE, REJECTED).into_response(),
    }
}

/// Fixed public vocabulary; no parser/database/crypto diagnostics reach the peer.
pub(crate) fn failure(error: Error) -> Response {
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

/// What a report token whose credential is known to be good is told of a
/// request the rules refuse (`reports.rs`): a fixed text like the other.
pub(crate) const REPORT_ROUTE_REJECTED: &str = "report request rejected\n";

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
pub(crate) fn single_headers(headers: &HeaderMap, content_type: &str) -> Result<(), Error> {
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

/// An AI runner's exchange from `from` (`docs/ai-players.md` §2). Past
/// [`crate::addresses::RUNNER_REFUSALS`] refused ones in the hour the store is
/// not asked again from that address, and each refusal is counted for it; a
/// `Busy` answer is no refusal.
async fn exchange(
    native: &Native,
    request: wire::Request,
    presented: Option<&Presented>,
    from: Option<Client>,
) -> Result<wire::Response, Error> {
    if let Some(client) = from {
        // The guard is gone before anything is awaited.
        let may = native
            .budgets
            .lock()
            .map_err(|_| Error::Unavailable)?
            .may_exchange(client, Instant::now());
        if !may {
            return Err(Error::Busy);
        }
    }
    let said = native
        .identity
        .handle_from(request, presented, from.map(Place::from))
        .await;
    if matches!(said, Err(Error::Rejected))
        && let Some(client) = from
        && let Ok(mut budgets) = native.budgets.lock()
    {
        budgets.exchange_refused(client, Instant::now());
    }
    said
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
    let place = from.map(Place::from);
    let mut response = match request {
        wire::Request::Ranked {
            request:
                ranked::Request::Replicate {
                    request: ranked::replica::SeatRequest::Relay { fence, call },
                },
        } => {
            let authorization = headers
                .get(header::AUTHORIZATION)
                .and_then(|header| header.to_str().ok())
                .ok_or(Error::Rejected)?;
            native
                .relay
                .forward(
                    &native.identity,
                    presented.as_ref(),
                    authorization,
                    place,
                    fence,
                    call,
                )
                .await?
        }
        wire::Request::RankedToken { .. }
        | wire::Request::Ranked {
            request:
                ranked::Request::Replicate {
                    request: ranked::replica::SeatRequest::RelayRoute { .. },
                },
        } => return Err(Error::Rejected),
        request @ wire::Request::AiRunnerSession { .. } => {
            exchange(native, request, presented.as_ref(), from).await?
        }
        request => {
            native
                .identity
                .handle_from(request, presented.as_ref(), place)
                .await?
        }
    };
    let mut accepts = headers
        .get_all(central_api::advent::framing::CAPABILITY_HEADER)
        .iter();
    if !matches!((accepts.next(), accepts.next()), (Some(value), None) if value.as_bytes() == b"1")
    {
        match &mut response {
            wire::Response::Advent { response } => response.omit_framing(),
            wire::Response::AdventHost { response } => response.omit_framing(),
            _ => {}
        }
    }
    if spoken(headers) != Some(PROTOCOL_VERSION) {
        response.omit_ai();
    }
    central_api::encode(&response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{Body, to_bytes};
    use tower::ServiceExt;

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
        // 3 is this build's; 2 is still served, without what 3 added.
        assert_eq!(PROTOCOL_VERSION, "3");
        for version in PROTOCOL_VERSIONS {
            let response = call(&[version]).await;
            assert_eq!(response.status(), StatusCode::OK, "{version}");
            let body = to_bytes(response.into_body(), 64).await.unwrap();
            assert_eq!(&body[..], b"reached");
        }
    }

    #[tokio::test]
    async fn any_other_protocol_is_told_to_update_and_reaches_nothing() {
        for values in [
            &[][..],
            &["1"],
            &["4"],
            &["2 "],
            &["03"],
            &[""],
            &["2", "2"],
            &["3", "3"],
            &["2", "3"],
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
    fn a_request_names_its_one_type_and_at_most_one_bearer() {
        let of = |types: &[&str], bearers: &[&str]| {
            let mut headers = HeaderMap::new();
            for value in types {
                headers.append(header::CONTENT_TYPE, value.parse().unwrap());
            }
            for value in bearers {
                headers.append(header::AUTHORIZATION, value.parse().unwrap());
            }
            single_headers(&headers, "application/json")
        };
        let json = ["application/json"];
        assert!(of(&json, &[]).is_ok());
        assert!(of(&json, &["Bearer x"]).is_ok());
        assert!(of(&json, &["Bearer x", "Bearer x"]).is_err());
        for types in [
            &[][..],
            &["application/json", "application/json"],
            &["application/json", "text/plain"],
            &["text/plain"],
            &["application/JSON"],
            &["application/json; charset=utf-8"],
            &["application/json, text/plain"],
            &[""],
        ] {
            assert!(of(types, &[]).is_err(), "{types:?}");
        }
    }
}
