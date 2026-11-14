//! Scoped identity for the separate Baylee Tower authority, never game state.
use crate::Token;
use serde::{Deserialize, Serialize};

/// Short-lived access to the operator-configured Tower origin.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Grant {
    /// Exact trusted destination; never selected by a gateway or client request.
    pub origin: String,
    /// Independent opaque bearer, not the account's Central session.
    pub token: Token,
    /// Absolute expiry in Unix seconds.
    pub expires_at: i64,
}

/// Current account identity, returned only after live-session introspection.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Identity {
    /// The Tower service must compare this with its configured origin.
    pub origin: String,
    /// Existing database-generated account `UUIDv7`.
    pub account: [u8; 16],
    /// Current display name; never a private login name.
    pub display_name: Option<String>,
    /// Current public discriminator, when a display name exists.
    pub tag: Option<u32>,
    /// This credential's absolute expiry; grants no permission to cache revocation.
    pub expires_at: i64,
}

#[cfg(test)]
mod tests {
    use crate::{Error, wire::Request};

    #[test]
    fn tower_access_requests_cannot_choose_an_audience_account_or_price() {
        for operation in ["tower_token", "tower_identity"] {
            let valid = format!(r#"{{"operation":"{operation}"}}"#);
            assert!(crate::decode::<Request>(valid.as_bytes()).is_ok());
            for field in [
                r#""origin":"https://attacker.invalid""#,
                r#""account":[]"#,
                r#""gold":100"#,
            ] {
                let input = format!(r#"{{"operation":"{operation}",{field}}}"#);
                assert!(matches!(
                    crate::decode::<Request>(input.as_bytes()),
                    Err(Error::Rejected)
                ));
            }
        }
    }
}
