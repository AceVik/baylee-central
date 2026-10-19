use crate::Error;
use sha2::{Digest, Sha256};
use std::net::IpAddr;
use url::Url;

/// Official central service. A gateway cannot override this credential destination.
pub const LIVE_ORIGIN: &str = "https://central.baylee.app";
/// Sole live gateway permitted to receive the one-time legacy migration password.
pub const LEGACY_ORIGIN: &str = "https://baylee.acevik.de";

/// Explicit environment selection; development never falls back to live services.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Endpoint {
    origin: String,
    live: bool,
}

impl Endpoint {
    /// The compiled, canonical production endpoint.
    #[must_use]
    pub fn live() -> Self {
        Self {
            origin: LIVE_ORIGIN.into(),
            live: true,
        }
    }

    /// An explicit numeric loopback development endpoint with a nonzero port.
    ///
    /// # Errors
    /// Rejects remote hosts, DNS aliases, credentials, paths, fragments and query parameters.
    pub fn development(value: &str) -> Result<Self, Error> {
        let url = Url::parse(value).map_err(|_| Error::Rejected)?;
        let host = url
            .host_str()
            .ok_or(Error::Rejected)?
            .trim_matches(['[', ']']);
        let ip: IpAddr = host.parse().map_err(|_| Error::Rejected)?;
        if url.scheme() != "http"
            || !ip.is_loopback()
            || url.port().is_none_or(|p| p == 0)
            || !url.username().is_empty()
            || url.password().is_some()
            || url.path() != "/"
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err(Error::Rejected);
        }
        Ok(Self {
            origin: url.origin().ascii_serialization(),
            live: false,
        })
    }

    /// Canonical service origin, without a trailing slash.
    #[must_use]
    pub fn origin(&self) -> &str {
        &self.origin
    }

    /// Native protocol endpoint. Transports must disable redirects and ambient proxy credentials.
    #[must_use]
    pub fn native_url(&self) -> String {
        format!("{}/v1/native", self.origin)
    }

    /// Whether an explicitly authorized live legacy migration is available.
    #[must_use]
    pub fn legacy_origin(&self) -> Option<&'static str> {
        self.live.then_some(LEGACY_ORIGIN)
    }

    /// Public deployment identifier bound into OPAQUE and ciphertext contexts.
    #[must_use]
    pub fn issuer(&self) -> [u8; 32] {
        Sha256::digest(format!("baylee-central/issuer/v1\0{}", self.origin).as_bytes()).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn development_cannot_redirect_credentials_to_live_or_remote_services() {
        for value in [
            LIVE_ORIGIN,
            LEGACY_ORIGIN,
            "http://localhost:9000",
            "http://127.0.0.1",
            "http://127.0.0.1:0",
            "http://127.0.0.1:9000/path",
            "http://u@127.0.0.1:9000",
            "http://127.0.0.1:9000/?next=live",
            "http://127.0.0.1:9000/#live",
            "http://10.0.0.1:9000",
        ] {
            assert!(Endpoint::development(value).is_err(), "{value}");
        }
        for value in ["http://127.0.0.1:9000", "http://[::1]:9000"] {
            let dev = Endpoint::development(value).unwrap();
            assert_eq!(dev.legacy_origin(), None);
            assert_ne!(dev.issuer(), Endpoint::live().issuer());
        }
        assert_eq!(Endpoint::live().legacy_origin(), Some(LEGACY_ORIGIN));
    }
}
