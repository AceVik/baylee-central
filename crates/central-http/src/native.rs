use axum::{
    Router,
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::post,
};
use central_api::{Error, Token};
use central_identity::IdentityService;
use std::sync::Arc;

pub(crate) fn router(identity: Arc<IdentityService>) -> Router {
    Router::new()
        .route("/v1/native", post(call))
        .with_state(identity)
}

async fn call(
    State(identity): State<Arc<IdentityService>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    match process(&identity, &headers, &body).await {
        Ok(bytes) => ([(header::CONTENT_TYPE, "application/json")], bytes).into_response(),
        Err(error) => {
            let status = match error {
                Error::Rejected => StatusCode::UNAUTHORIZED,
                Error::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
                Error::Conflict => StatusCode::CONFLICT,
                Error::Busy => StatusCode::TOO_MANY_REQUESTS,
            };
            // Fixed public vocabulary; no parser/database/crypto diagnostics reach the peer.
            (status, "account request rejected\n").into_response()
        }
    }
}

async fn process(
    identity: &IdentityService,
    headers: &HeaderMap,
    body: &[u8],
) -> Result<Vec<u8>, Error> {
    if headers.get_all(header::CONTENT_TYPE).iter().count() != 1
        || headers
            .get(header::CONTENT_TYPE)
            .and_then(|h| h.to_str().ok())
            != Some("application/json")
        || headers.get_all(header::AUTHORIZATION).iter().count() > 1
    {
        return Err(Error::Rejected);
    }
    let token = headers
        .get(header::AUTHORIZATION)
        .map(|h| Token::from_bearer(h.to_str().map_err(|_| Error::Rejected)?))
        .transpose()?;
    let request = central_api::decode(body)?;
    let response = identity.handle(request, token.as_ref()).await?;
    central_api::encode(&response)
}
