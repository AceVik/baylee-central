use crate::Error;
use sha2::{Digest, Sha256};
use std::net::IpAddr;
use url::Url;

/// Official central service. A gateway cannot override this credential destination.
pub const LIVE_ORIGIN: &str = "https://central.baylee.app";
/// Sole live gateway permitted to receive the one-time legacy migration password,
/// and the official gateway's origin before it moved: migrated account
/// documents name it (`legacy.gateway`), so it never changes.
pub const LEGACY_ORIGIN: &str = "https://baylee.acevik.de";
/// The official gateway's canonical origin, where its bridge answers now.
pub const OFFICIAL_ORIGIN: &str = "https://gateway.baylee.app";
/// Further origins of the official gateway: installed clients ask Central for
/// its tickets at the address they know, and players who accepted its terms
/// there keep that acceptance. Compiled, as the official origin is: no
/// configuration can name another gateway the bridge's authority.
pub const OFFICIAL_ALIASES: &[&str] = &[LEGACY_ORIGIN];

/// Whether `origin` is one of the live official gateway's compiled origins
/// ([`OFFICIAL_ORIGIN`] or one of [`OFFICIAL_ALIASES`]), which no registered
/// gateway may hold.
#[must_use]
pub fn is_official_origin(origin: &str) -> bool {
    origin == OFFICIAL_ORIGIN || OFFICIAL_ALIASES.contains(&origin)
}

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

    /// The live official gateway's origins, canonical first; none in development,
    /// where the launcher names them.
    #[must_use]
    pub fn official_origins(&self) -> Vec<String> {
        if !self.live {
            return Vec::new();
        }
        std::iter::once(OFFICIAL_ORIGIN)
            .chain(OFFICIAL_ALIASES.iter().copied())
            .map(String::from)
            .collect()
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

    #[test]
    fn a_development_endpoint_is_one_canonical_loopback_origin_whatever_was_typed() {
        for (typed, origin) in [
            ("http://127.0.0.1:9000", "http://127.0.0.1:9000"),
            ("http://127.0.0.1:9000/", "http://127.0.0.1:9000"),
            ("HTTP://127.0.0.1:9000", "http://127.0.0.1:9000"),
            ("http://127.1:9000", "http://127.0.0.1:9000"),
            ("http://0x7f000001:9000", "http://127.0.0.1:9000"),
            ("http://127.8.9.10:1", "http://127.8.9.10:1"),
            ("http://[0:0:0:0:0:0:0:1]:65535", "http://[::1]:65535"),
        ] {
            let endpoint = Endpoint::development(typed).unwrap();
            assert_eq!(endpoint.origin(), origin, "{typed}");
            assert_eq!(endpoint.native_url(), format!("{origin}/v1/native"));
            // A typed spelling is an endpoint; only the canonical one is a stored origin.
            assert_eq!(
                crate::gateway::validate_origin(typed, true).is_ok(),
                typed == origin,
                "{typed}"
            );
        }
        for refused in [
            "",
            "127.0.0.1:9000",
            "//127.0.0.1:9000",
            "https://127.0.0.1:9000",
            "ws://127.0.0.1:9000",
            "http://127.0.0.1:80",
            "http://127.0.0.1:65536",
            "http://127.0.0.1.evil.example:9000",
            "http://127.0.0.1@evil.example:9000",
            "http://evil.example#@127.0.0.1:9000",
            "http://:secret@127.0.0.1:9000",
            "http://0.0.0.0:9000",
            "http://[::]:9000",
            "http://[::ffff:127.0.0.1]:9000",
            "http://169.254.169.254:9000",
            "http://2130706434.evil:9000",
            "http://128.0.0.1:9000",
            "http://127.0.0.1:9000?",
            "http://127.0.0.1:9000/..//v1",
        ] {
            assert!(Endpoint::development(refused).is_err(), "{refused}");
        }
    }

    #[test]
    fn only_the_live_endpoint_knows_official_gateways_and_only_by_their_exact_origin() {
        let live = Endpoint::live().official_origins();
        assert_eq!(live, [OFFICIAL_ORIGIN, LEGACY_ORIGIN]);
        for origin in &live {
            assert!(is_official_origin(origin), "{origin}");
            assert!(crate::gateway::validate_origin(origin, false).is_ok());
            for other in [
                format!("{origin}/"),
                format!("{origin}:443"),
                format!("{origin}."),
                format!("{origin}.evil.example"),
                origin.replace("https://", "http://"),
                origin.replace("https://", "https://user@"),
                origin.to_uppercase(),
                format!(" {origin}"),
                origin.replace("https://", ""),
            ] {
                assert!(!is_official_origin(&other), "{other}");
            }
        }
        // Central itself is no gateway.
        assert!(!is_official_origin(LIVE_ORIGIN));
        assert!(!is_official_origin(""));
        let development = Endpoint::development("http://127.0.0.1:9000").unwrap();
        assert_eq!(development.official_origins(), [""; 0]);
    }

    #[test]
    fn every_endpoint_is_an_issuer_of_its_own() {
        let development = |value: &str| Endpoint::development(value).unwrap();
        let base = development("http://127.0.0.1:9000");
        assert_eq!(base.issuer(), development("http://127.1:9000/").issuer());
        let mut seen = std::collections::BTreeSet::from([base.issuer()]);
        for other in [
            Endpoint::live(),
            development("http://127.0.0.1:9001"),
            development("http://127.0.0.2:9000"),
            development("http://[::1]:9000"),
        ] {
            assert!(seen.insert(other.issuer()), "{other:?}");
        }
    }
}
