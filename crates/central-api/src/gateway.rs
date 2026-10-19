//! Public gateway metadata and strict origin policy. Validation performs no network I/O.
use crate::{Endpoint, Error};
use serde::{Deserialize, Serialize};
use url::Url;

/// Public, bounded metadata. None of these fields grants ownership or administration.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GatewayMetadata {
    /// Human-readable gateway name, at most 80 UTF-8 bytes without control characters.
    pub name: String,
    /// Operator-reported software version, at most 64 ASCII bytes.
    pub version: String,
    /// Canonical website URL on the gateway's approved origin.
    pub website: String,
}

/// One approved public directory entry. Online means a recent authenticated heartbeat.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GatewayListing {
    /// Server-generated immutable gateway `UUIDv7`.
    pub id: [u8; 16],
    /// Canonical approved API origin.
    pub origin: String,
    /// Public operator metadata, never account data.
    pub metadata: GatewayMetadata,
    /// A heartbeat was accepted within the last 90 seconds; not an availability guarantee.
    pub online: bool,
}

/// Validate a canonical live HTTPS or explicitly selected loopback development origin.
///
/// # Errors
/// Rejects credentials, paths, aliases, queries, fragments and noncanonical spellings.
pub fn validate_origin(value: &str, development: bool) -> Result<(), Error> {
    if value.len() > 256 {
        return Err(Error::Rejected);
    }
    if development {
        return (Endpoint::development(value)?.origin() == value)
            .then_some(())
            .ok_or(Error::Rejected);
    }
    let url = Url::parse(value).map_err(|_| Error::Rejected)?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
        || url.origin().ascii_serialization() != value
    {
        return Err(Error::Rejected);
    }
    Ok(())
}

impl GatewayMetadata {
    /// Validate metadata without dereferencing the website or accepting another origin.
    ///
    /// # Errors
    /// Rejects unsafe, oversized or misleadingly noncanonical values.
    pub fn validate(&self, origin: &str) -> Result<(), Error> {
        let website = Url::parse(&self.website).map_err(|_| Error::Rejected)?;
        if self.name.is_empty()
            || self.name.len() > 80
            || self.name.trim() != self.name
            || self.name.chars().any(char::is_control)
            || self.version.is_empty()
            || self.version.len() > 64
            || !self
                .version
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b".-_+".contains(&b))
            || self.website.len() > 512
            || website.origin().ascii_serialization() != origin
            || !website.username().is_empty()
            || website.password().is_some()
            || website.query().is_some()
            || website.fragment().is_some()
            || website.as_str() != self.website
        {
            return Err(Error::Rejected);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn origins_and_public_metadata_cannot_redirect_or_smuggle_credentials() {
        for origin in [
            "http://gateway.invalid",
            "https://u@gateway.invalid",
            "https://gateway.invalid/",
            "https://GATEWAY.invalid",
            "https://gateway.invalid/path",
        ] {
            assert!(validate_origin(origin, false).is_err());
        }
        validate_origin("https://gateway.invalid", false).unwrap();
        validate_origin("http://127.0.0.1:9876", true).unwrap();
        assert!(validate_origin("https://gateway.invalid", true).is_err());
        let mut metadata = GatewayMetadata {
            name: "Synthetic".into(),
            version: "1.2.3-test".into(),
            website: "https://gateway.invalid/status".into(),
        };
        metadata.validate("https://gateway.invalid").unwrap();
        for website in [
            "https://foreign.invalid/",
            "https://u@gateway.invalid/",
            "https://gateway.invalid/?next=elsewhere",
        ] {
            metadata.website = website.into();
            assert!(metadata.validate("https://gateway.invalid").is_err());
        }
    }
}
