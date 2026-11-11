use super::boundary::{Pace, REJECTED, UPLOAD_PACE, paced, timed_out};
use axum::{
    Router,
    body::{Body, Bytes},
    extract::{Path, Request, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{post, put},
};
use central_api::{Error, Token, reports::Part};
use central_identity::IdentityService;
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
        .with_state(Native {
            identity,
            uploads: Arc::new(Semaphore::new(UPLOADS)),
        })
}

/// Fixed public vocabulary; no parser/database/crypto diagnostics reach the peer.
fn failure(error: Error) -> Response {
    let status = match error {
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

/// A part's body, at most `length` bytes, as long as it keeps `pace`.
async fn part_body(body: Body, length: usize, pace: Pace) -> Result<Bytes, Stopped> {
    paced(body, length, pace)
        .await
        .map_err(|status| match status {
            StatusCode::REQUEST_TIMEOUT => Stopped::Stalled,
            _ => Stopped::Failed(Error::Rejected),
        })
}

async fn call(State(native): State<Native>, headers: HeaderMap, body: Bytes) -> Response {
    match process(&native.identity, &headers, &body).await {
        Ok(bytes) => ([(header::CONTENT_TYPE, "application/json")], bytes).into_response(),
        Err(error) => failure(error),
    }
}

/// Exactly one `Content-Type`, this one, and at most one `Authorization`, as a bearer.
fn credential(headers: &HeaderMap, content_type: &str) -> Result<Option<Token>, Error> {
    if headers.get_all(header::CONTENT_TYPE).iter().count() != 1
        || headers
            .get(header::CONTENT_TYPE)
            .and_then(|h| h.to_str().ok())
            != Some(content_type)
        || headers.get_all(header::AUTHORIZATION).iter().count() > 1
    {
        return Err(Error::Rejected);
    }
    headers
        .get(header::AUTHORIZATION)
        .map(|h| Token::from_bearer(h.to_str().map_err(|_| Error::Rejected)?))
        .transpose()
}

async fn process(
    identity: &IdentityService,
    headers: &HeaderMap,
    body: &[u8],
) -> Result<Vec<u8>, Error> {
    let token = credential(headers, "application/json")?;
    let request = central_api::decode(body)?;
    let response = identity.handle(request, token.as_ref()).await?;
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
/// slots, and only then is the body read, exactly that long and at
/// [`UPLOAD_PACE`]: a body that is withheld gives its slot back at the
/// pace's first empty window, not at the upload's whole time.
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
            return Err(Stopped::Failed(Error::Rejected));
        }
        let _slot = native
            .uploads
            .clone()
            .try_acquire_owned()
            .map_err(|_| Error::Busy)?;
        let length = usize::try_from(upload.bytes).map_err(|_| Error::Rejected)?;
        let gzip = part_body(body, length, UPLOAD_PACE).await?;
        Ok(native.identity.file_report_part(&upload, &gzip).await?)
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
}
