//! Strict development configuration with bounded resource consumption.

use std::{fmt, net::SocketAddr, num::NonZeroUsize};

const PREFIX: &str = "BAYLEE_CENTRAL_";

/// Recognize the reserved namespace regardless of ASCII casing on every platform.
/// Parsing still requires canonical uppercase names, so casing mistakes fail closed.
#[must_use]
pub fn is_reserved_setting(key: &str) -> bool {
    key.get(..PREFIX.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(PREFIX))
}

/// Validated local-development settings. Production listeners are not supported.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Config {
    listen: SocketAddr,
    request_bytes: NonZeroUsize,
    concurrent_requests: NonZeroUsize,
}

/// A diagnostic without attacker-controlled configuration values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfigError {
    /// A setting with the reserved prefix is not recognized.
    UnknownSetting,
    /// A setting was supplied more than once.
    DuplicateSetting,
    /// A setting could not be parsed or exceeded its allowed bounds.
    InvalidValue,
    /// Only loopback listeners are supported in this implementation stage.
    NonLoopbackListener,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::UnknownSetting => "unrecognized central configuration setting",
            Self::DuplicateSetting => "duplicate central configuration setting",
            Self::InvalidValue => "invalid central configuration value",
            Self::NonLoopbackListener => "central development listener must be loopback",
        })
    }
}

impl std::error::Error for ConfigError {}

impl Default for Config {
    fn default() -> Self {
        Self {
            listen: SocketAddr::from(([127, 0, 0, 1], 28_780)),
            request_bytes: NonZeroUsize::new(65_536).expect("nonzero constant"),
            concurrent_requests: NonZeroUsize::new(32).expect("nonzero constant"),
        }
    }
}

impl Config {
    /// Parse settings without echoing keys or values in errors.
    ///
    /// Unknown unrelated environment variables are ignored. Unknown reserved
    /// settings and duplicate settings fail closed. No production escape flag.
    ///
    /// # Errors
    ///
    /// Returns a redacted error for unknown, duplicate, malformed or unsafe input.
    pub fn from_pairs<'a>(
        pairs: impl IntoIterator<Item = (&'a str, &'a str)>,
    ) -> Result<Self, ConfigError> {
        let mut result = Self::default();
        let mut seen = [false; 3];
        for (key, value) in pairs {
            if !is_reserved_setting(key) {
                continue;
            }
            let name = key
                .strip_prefix(PREFIX)
                .ok_or(ConfigError::UnknownSetting)?;
            let index = match name {
                "LISTEN" => 0,
                "REQUEST_BYTES" => 1,
                "CONCURRENT_REQUESTS" => 2,
                _ => return Err(ConfigError::UnknownSetting),
            };
            if std::mem::replace(&mut seen[index], true) {
                return Err(ConfigError::DuplicateSetting);
            }
            match index {
                0 => {
                    let address: SocketAddr =
                        value.parse().map_err(|_| ConfigError::InvalidValue)?;
                    if !address.ip().is_loopback() {
                        return Err(ConfigError::NonLoopbackListener);
                    }
                    result.listen = address;
                }
                1 => result.request_bytes = bounded_integer(value, 1_024, 1_048_576)?,
                _ => result.concurrent_requests = bounded_integer(value, 1, 128)?,
            }
        }
        Ok(result)
    }

    /// Local listen address; no listener is started by this crate.
    #[must_use]
    pub const fn listen(self) -> SocketAddr {
        self.listen
    }

    /// Maximum buffered request size; streaming endpoints need separate limits.
    #[must_use]
    pub const fn request_bytes(self) -> NonZeroUsize {
        self.request_bytes
    }

    /// Upper bound on concurrently admitted requests.
    #[must_use]
    pub const fn concurrent_requests(self) -> NonZeroUsize {
        self.concurrent_requests
    }
}

fn bounded_integer(value: &str, min: usize, max: usize) -> Result<NonZeroUsize, ConfigError> {
    if value.is_empty() || value.len() > 10 || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(ConfigError::InvalidValue);
    }
    let parsed = value
        .parse::<usize>()
        .map_err(|_| ConfigError::InvalidValue)?;
    if !(min..=max).contains(&parsed) {
        return Err(ConfigError::InvalidValue);
    }
    NonZeroUsize::new(parsed).ok_or(ConfigError::InvalidValue)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_local_and_bounded() {
        let config = Config::from_pairs([]).unwrap();
        assert!(config.listen().ip().is_loopback());
        assert_eq!(config.request_bytes().get(), 65_536);
        assert_eq!(config.concurrent_requests().get(), 32);
    }

    #[test]
    fn rejects_all_non_loopback_categories() {
        for address in [
            "0.0.0.0:80",
            "[::]:80",
            "10.0.0.1:80",
            "192.168.1.1:80",
            "8.8.8.8:80",
            "[fe80::1]:80",
            "[::ffff:127.0.0.1]:80",
            "localhost:80",
            "https://127.0.0.1",
            "127.0.0.1:65536",
        ] {
            assert!(Config::from_pairs([("BAYLEE_CENTRAL_LISTEN", address)]).is_err());
        }
        for address in ["127.0.0.1:0", "[::1]:28780"] {
            assert!(Config::from_pairs([("BAYLEE_CENTRAL_LISTEN", address)]).is_ok());
        }
    }

    #[test]
    fn exhaustive_concurrency_boundaries() {
        for count in 0..=512 {
            let text = count.to_string();
            assert_eq!(
                Config::from_pairs([("BAYLEE_CENTRAL_CONCURRENT_REQUESTS", text.as_str())]).is_ok(),
                (1..=128).contains(&count)
            );
        }
    }

    #[test]
    fn request_size_boundaries_and_malformed_numbers() {
        for (value, accepted) in [
            ("1023", false),
            ("1024", true),
            ("1048576", true),
            ("1048577", false),
            ("", false),
            ("+32", false),
            ("-1", false),
            (" 32", false),
            ("32\n", false),
            ("999999999999999999999999999", false),
        ] {
            assert_eq!(
                Config::from_pairs([("BAYLEE_CENTRAL_REQUEST_BYTES", value)]).is_ok(),
                accepted
            );
        }
    }

    #[test]
    fn unknown_and_duplicate_settings_fail_closed() {
        assert_eq!(
            Config::from_pairs([("BAYLEE_CENTRAL_ALLOW_UNSAFE", "true")]),
            Err(ConfigError::UnknownSetting)
        );
        assert_eq!(
            Config::from_pairs([
                ("BAYLEE_CENTRAL_LISTEN", "127.0.0.1:0"),
                ("BAYLEE_CENTRAL_LISTEN", "127.0.0.1:0"),
            ]),
            Err(ConfigError::DuplicateSetting)
        );
        assert!(Config::from_pairs([("UNRELATED_SETTING", "irrelevant")]).is_ok());
    }

    #[test]
    fn errors_never_echo_secret_or_control_characters() {
        let secret = "password-canary\r\nforged-log-line";
        let error = Config::from_pairs([("BAYLEE_CENTRAL_LISTEN", secret)]).unwrap_err();
        assert!(!format!("{error:?}: {error}").contains("canary"));
        assert!(!error.to_string().contains(['\r', '\n']));
    }

    #[test]
    fn noncanonical_reserved_names_cannot_silently_select_defaults() {
        for key in [
            "baylee_central_listen",
            "Baylee_Central_REQUEST_BYTES",
            "baylee_central_UNKNOWN",
        ] {
            assert_eq!(
                Config::from_pairs([(key, "invalid-canary")]),
                Err(ConfigError::UnknownSetting)
            );
        }
    }
}
