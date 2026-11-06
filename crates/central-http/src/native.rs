use axum::{
    Router,
    body::{Bytes, to_bytes},
    extract::{Path, Request, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use central_api::{
    Error, PROTOCOL_HEADER, PROTOCOL_VERSION, Token, UPDATE_REQUIRED_BODY, UPDATE_REQUIRED_STATUS,
    reports::Part,
};
use central_identity::{IdentityService, Presented};
use std::sync::Arc;
use tokio::sync::Semaphore;

/// Uploads of report parts at once: each holds at most its announced
/// length (64 MiB) in memory.
const UPLOADS: usize = 2;

#[derive(Clone)]
struct Native {
    identity: Arc<IdentityService>,
    uploads: Arc<Semaphore>,
}

pub(crate) fn router(identity: Arc<IdentityService>) -> Router {
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
            uploads: Arc::new(Semaphore::new(UPLOADS)),
        })
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

/// Refuse any other protocol before the request is read: before its credential, its budget
/// or the database. An old client hears this at its first request, not as a failure to
/// decode the answer to a sign-in that has already happened.
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
    (status, "account request rejected\n").into_response()
}

async fn call(State(native): State<Native>, headers: HeaderMap, body: Bytes) -> Response {
    match process(&native.identity, &headers, &body).await {
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

/// A credential or a signed session, on the route that takes both.
fn presented(identity: &IdentityService, headers: &HeaderMap) -> Result<Option<Presented>, Error> {
    single_headers(headers, "application/json")?;
    headers
        .get(header::AUTHORIZATION)
        .map(|h| identity.present(h.to_str().map_err(|_| Error::Rejected)?))
        .transpose()
}

async fn process(
    identity: &IdentityService,
    headers: &HeaderMap,
    body: &[u8],
) -> Result<Vec<u8>, Error> {
    let presented = presented(identity, headers)?;
    let request = central_api::decode(body)?;
    let response = identity.handle(request, presented.as_ref()).await?;
    central_api::encode(&response)
}

/// `POST /v1/reports`: `201 {"report_id": "<32 hex>"}`.
async fn file(State(native): State<Native>, headers: HeaderMap, body: Bytes) -> Response {
    let filed = async {
        let token = credential(&headers, "application/json")?;
        native.identity.file_report(token.as_ref(), &body).await
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
/// declared length must be the announced one, then one of [`UPLOADS`]
/// slots, and only then is the body read, exactly that long.
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
            return Err(Error::Rejected);
        };
        let upload = native
            .identity
            .report_upload(token.as_ref(), report, part)
            .await?;
        let declared = head
            .headers
            .get(header::CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok());
        if declared != Some(u64::from(upload.bytes)) {
            return Err(Error::Rejected);
        }
        let _slot = native
            .uploads
            .clone()
            .try_acquire_owned()
            .map_err(|_| Error::Busy)?;
        let length = usize::try_from(upload.bytes).map_err(|_| Error::Rejected)?;
        let gzip = to_bytes(body, length).await.map_err(|_| Error::Rejected)?;
        native.identity.file_report_part(&upload, &gzip).await
    };
    match filed.await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => failure(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
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
