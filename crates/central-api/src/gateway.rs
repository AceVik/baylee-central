//! Public gateway metadata and strict origin policy. Validation performs no network I/O.
use crate::{Endpoint, Error};
use serde::{Deserialize, Serialize};
use url::Url;

/// Public, bounded metadata. None of these fields grants ownership or administration.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GatewayMetadata {
    /// Human-readable gateway name, at most 80 UTF-8 bytes of [`shown_char`]s.
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

/// Where a registered gateway stands with Central's administrators. Only an
/// approved gateway is listed. A pending or approved one signs players in
/// with a gateway token (the token makes its owner answerable and can be
/// revoked); a credential pair still needs the approval. A disabled one does
/// neither.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GatewayState {
    /// Registered, not yet approved: not listed, signs players in by token.
    Pending,
    /// Approved and enabled.
    Approved,
    /// Approved once, or refused, and disabled now.
    Disabled,
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

/// Whether `c` may stand in a name another person reads: no control
/// character, and none of the invisible characters that change how the text
/// around them is drawn (direction marks, embeddings, overrides and
/// isolates, invisible separators and spaces, the byte order mark), which
/// could make a gateway's or a token's name read as something else. The
/// zero-width joiner and non-joiner stay: emoji sequences and some scripts
/// need them.
#[must_use]
pub const fn shown_char(c: char) -> bool {
    !c.is_control()
        && !matches!(
            c,
            '\u{061C}'
                | '\u{200B}'
                | '\u{200E}'
                | '\u{200F}'
                | '\u{202A}'..='\u{202E}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{2060}'..='\u{2064}'
                | '\u{2066}'..='\u{2069}'
                | '\u{FEFF}'
        )
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
            || !self.name.chars().all(shown_char)
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

    #[test]
    fn a_name_cannot_hide_how_it_reads() {
        let named = |name: &str| GatewayMetadata {
            name: name.into(),
            version: "1.2.3-test".into(),
            website: "https://gateway.invalid/".into(),
        };
        // a right-to-left override, an isolate, a mark, an invisible space,
        // a line separator, a byte order mark, a control character
        for name in [
            "Official\u{202E}lacif",
            "Table \u{2067}x\u{2069}",
            "Table\u{200F}",
            "Bay\u{200B}lee",
            "Two\u{2028}lines",
            "\u{FEFF}Table",
            "Tab\u{0007}le",
        ] {
            assert!(
                named(name).validate("https://gateway.invalid").is_err(),
                "{name:?}"
            );
            assert!(crate::owner::token_name(name).is_err(), "{name:?}");
        }
        // other scripts, accents and an emoji sequence with its joiners stay
        for name in [
            "Spieltisch Köln",
            "東京テーブル",
            "طاولة",
            "Family 👨\u{200D}👩\u{200D}👧",
        ] {
            named(name).validate("https://gateway.invalid").unwrap();
            crate::owner::token_name(name).unwrap();
        }
    }
}
