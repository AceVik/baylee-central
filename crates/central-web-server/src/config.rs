//! The server's settings: a JSON document on standard input (deployed), or
//! development defaults (`--dev`). Nothing in it is a secret: the server has
//! no database, no key and no session.

use std::net::SocketAddr;
use std::path::PathBuf;

use serde::Deserialize;

/// What the server is told at start.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Where to listen: a loopback address, behind Caddy.
    pub listen: SocketAddr,
    /// The site's public origin, `https://baylee.app`; canonical addresses
    /// and the sitemap name it.
    pub origin: String,
    /// The built assets (`cargo xtask site`): `manifest.json` and the files.
    pub site: PathBuf,
    /// Central's browser listener on loopback, for the gateway directory.
    #[serde(default)]
    pub central: Option<SocketAddr>,
    /// Ask GitHub for the newest release, the CI state and the card status.
    #[serde(default = "yes")]
    pub github: bool,
    /// Data to use when GitHub or Central do not answer (and in tests in
    /// place of them): `release.json`, `cards.json`, `gateways.json`,
    /// `repo.json` in the shapes of `central_web::data`.
    #[serde(default)]
    pub fallback: Option<PathBuf>,
    /// A search console's verification value, if the owner sets one.
    #[serde(default)]
    pub verification: Option<String>,
}

fn yes() -> bool {
    true
}

/// A setting that cannot be used.
#[derive(Debug, PartialEq, Eq)]
pub struct Invalid(pub &'static str);

impl Config {
    /// Checks what serde cannot: a loopback listener, a plain origin, a
    /// loopback Central, a verification value of plain characters.
    ///
    /// # Errors
    /// The first setting that is not acceptable.
    pub fn validate(&self) -> Result<(), Invalid> {
        if !self.listen.ip().is_loopback() {
            return Err(Invalid("listen must be a loopback address"));
        }
        if self
            .central
            .is_some_and(|central| !central.ip().is_loopback())
        {
            return Err(Invalid("central must be a loopback address"));
        }
        let plain_origin = |origin: &str| {
            let rest = origin
                .strip_prefix("https://")
                .or_else(|| origin.strip_prefix("http://127.0.0.1:"));
            rest.is_some_and(|rest| {
                !rest.is_empty()
                    && rest
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-')
            })
        };
        if !plain_origin(&self.origin) {
            return Err(Invalid(
                "origin must be https://host or http://127.0.0.1:port",
            ));
        }
        if self.verification.as_deref().is_some_and(|value| {
            value.is_empty()
                || value.len() > 128
                || !value
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        }) {
            return Err(Invalid(
                "verification must be 1 to 128 letters, digits, - or _",
            ));
        }
        Ok(())
    }

    /// The development settings: `127.0.0.1:28792`, its own origin, the
    /// assets `cargo xtask site` wrote, no GitHub, the given fallback data.
    #[must_use]
    pub fn development(site: PathBuf, fallback: Option<PathBuf>) -> Self {
        Config {
            listen: SocketAddr::from(([127, 0, 0, 1], 28792)),
            origin: "http://127.0.0.1:28792".into(),
            site,
            central: None,
            github: false,
            fallback,
            verification: None,
        }
    }

    /// The `Host` and `Origin` Central's browser listener expects of this
    /// site: `baylee.app` live, the loopback origin in development.
    #[must_use]
    pub fn central_site(&self) -> (String, String) {
        let host = self
            .origin
            .split_once("://")
            .map_or_else(|| self.origin.clone(), |(_, host)| host.to_owned());
        (host, self.origin.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> Config {
        Config::development(PathBuf::from("site"), None)
    }

    #[test]
    fn listens_on_loopback_only() {
        assert_eq!(config().validate(), Ok(()));
        let open = Config {
            listen: SocketAddr::from(([0, 0, 0, 0], 80)),
            ..config()
        };
        assert!(open.validate().is_err());
        let remote = Config {
            central: Some(SocketAddr::from(([10, 0, 0, 1], 80))),
            ..config()
        };
        assert!(remote.validate().is_err());
    }

    #[test]
    fn origins_and_verification() {
        assert!(
            Config {
                origin: "https://baylee.app".into(),
                ..config()
            }
            .validate()
            .is_ok()
        );
        assert!(
            Config {
                origin: "https://baylee.app/x".into(),
                ..config()
            }
            .validate()
            .is_err()
        );
        assert!(
            Config {
                origin: "http://example.com".into(),
                ..config()
            }
            .validate()
            .is_err()
        );
        assert!(
            Config {
                verification: Some("abc_DEF-1".into()),
                ..config()
            }
            .validate()
            .is_ok()
        );
        assert!(
            Config {
                verification: Some("a\"b".into()),
                ..config()
            }
            .validate()
            .is_err()
        );
        assert_eq!(
            config().central_site(),
            ("127.0.0.1:28792".into(), "http://127.0.0.1:28792".into())
        );
    }

    #[test]
    fn unknown_fields_are_refused() {
        let text =
            r#"{"listen":"127.0.0.1:1","origin":"https://baylee.app","site":"s","secret":"x"}"#;
        assert!(serde_json::from_str::<Config>(text).is_err());
    }
}
