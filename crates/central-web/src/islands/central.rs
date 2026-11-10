//! Central's browser surface (`docs/web-accounts.md`): `POST /api/v1` on this
//! origin, the session in the `__Host-central` cookie only (the page never
//! sees it), `X-Baylee-Central: 1` so a foreign page needs a preflight it
//! never gets. Requests and answers are `central-api`'s own types, so the
//! browser speaks exactly what the native client speaks: every answer is
//! parsed strictly (`deny_unknown_fields`, exact byte lengths) and a
//! refusal is read by its status only, never by its text.

use central_api::wire::{Request, Response};

use super::net::{self, Credentials};

/// Why a request did not give an answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    /// 401: the request or the session was refused (also a wrong password).
    Rejected,
    /// 403: not allowed, or Central's terms are not accepted.
    Forbidden,
    /// 409: a conflict (a taken name, a code already kept).
    Conflict,
    /// 429: too many requests just now.
    Busy,
    /// 5xx: Central is unavailable.
    Unavailable,
    /// An answer of another shape than the request expects.
    Malformed,
    /// No answer at all: no network, a redirect.
    Network,
}

/// Sends `request` and reads Central's answer.
///
/// # Errors
/// The refusal, by its status.
pub async fn call(request: &Request) -> Result<Response, Failure> {
    let body = serde_json::to_string(request).map_err(|_| Failure::Malformed)?;
    let (status, text) = net::request(
        "POST",
        "/api/v1",
        Some(&body),
        &[("X-Baylee-Central", "1")],
        Credentials::SameOrigin,
    )
    .await
    .map_err(|_| Failure::Network)?;
    match status {
        200 => serde_json::from_str(&text).map_err(|_| Failure::Malformed),
        401 => Err(Failure::Rejected),
        403 => Err(Failure::Forbidden),
        409 => Err(Failure::Conflict),
        429 => Err(Failure::Busy),
        500..=599 => Err(Failure::Unavailable),
        _ => Err(Failure::Malformed),
    }
}

/// Whether `id` is a `UUIDv7`, as Central makes every account's.
#[must_use]
pub fn uuid_v7(id: &[u8; 16]) -> bool {
    id[6] >> 4 == 7 && id[8] >> 6 == 2
}

/// A sign-in name as Central keeps it: 3 to 24 of `a-z`, `0-9`, `_`, `.`,
/// `-`, starting with a letter or digit.
#[must_use]
pub fn valid_username(name: &str) -> bool {
    (3..=24).contains(&name.len())
        && name
            .bytes()
            .next()
            .is_some_and(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        && name.bytes().all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'.' | b'-')
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usernames() {
        assert!(valid_username("erika"));
        assert!(valid_username("e.r-k_9"));
        assert!(!valid_username("Erika"));
        assert!(!valid_username("er"));
        assert!(!valid_username("_erika"));
        assert!(!valid_username(&"e".repeat(25)));
    }

    #[test]
    fn requests_serialize_as_central_reads_them() {
        let json = serde_json::to_string(&Request::Discover {
            username: "erika".into(),
        })
        .expect("json");
        assert_eq!(json, r#"{"operation":"discover","username":"erika"}"#);
        let answer: Response = serde_json::from_str(r#"{"result":"done"}"#).expect("done");
        assert!(matches!(answer, Response::Done));
        let renewed = r#"{"result":"renewed","expires_at":1}"#;
        assert!(matches!(
            serde_json::from_str(renewed),
            Ok(Response::Renewed { expires_at: 1 })
        ));
        let extra = r#"{"result":"renewed","expires_at":1,"extra":1}"#;
        assert!(
            serde_json::from_str::<Response>(extra).is_err(),
            "unknown fields are refused"
        );
    }
}
