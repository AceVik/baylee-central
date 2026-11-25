//! Report tokens (`docs/reports.md` §"Report tokens"): the credential an
//! administrator makes for a script, a build machine or an agent that files,
//! reads or triages bug and crash reports. It is no gateway's, no account's
//! and no administrator's, and works on the report routes of the native
//! listener alone.
//!
//! A report token is one [`Token`], shown once, stored as its SHA-256
//! digest, with the rights it keeps ([`ReportScopes`]), named, expiring and
//! revocable. Making one hands out a credential, so it needs a password
//! sign-in less than 30 minutes old, as minting a gateway's token does.
use crate::owner::{MAX_TOKEN_DAYS, token_name};
use crate::{Error, Token};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

/// Live (neither revoked nor expired) report tokens at once.
pub const LIVE_TOKENS: usize = 32;
/// Report tokens made in a day, live or not.
pub const TOKENS_PER_DAY: usize = 32;
/// Report tokens the administration lists: the live ones, and of those
/// revoked or expired in the last 30 days the newest.
pub const LISTED_TOKENS: usize = 128;
/// Requests a token makes in an hour besides filing.
pub const REQUESTS_PER_HOUR: u32 = 600;
/// Of them, downloads of a report's parts.
pub const DOWNLOADS_PER_HOUR: u32 = 120;
/// What a token's text form starts with, so a person (and a secret scanner)
/// can tell it from a gateway's and from any other secret: `brt_` and 64
/// lowercase hex digits.
pub const TOKEN_PREFIX: &str = "brt_";
const _: () = assert!(LIVE_TOKENS <= LISTED_TOKENS && DOWNLOADS_PER_HOUR <= REQUESTS_PER_HOUR);

/// One right of a report token.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportScope {
    /// File reports and their parts.
    File,
    /// Search and read reports and their parts.
    Read,
    /// Change reports, move them to the trash and back, and write notes.
    Triage,
    /// Delete reports for good, from the trash.
    Purge,
}

impl ReportScope {
    /// Every right, the least first.
    pub const ALL: [Self; 4] = [Self::File, Self::Read, Self::Triage, Self::Purge];

    /// The right this one needs beside it.
    #[must_use]
    pub const fn needs(self) -> Option<Self> {
        match self {
            Self::File | Self::Read => None,
            Self::Triage => Some(Self::Read),
            Self::Purge => Some(Self::Triage),
        }
    }

    /// Its word, as it travels and as its column is named.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Read => "read",
            Self::Triage => "triage",
            Self::Purge => "purge",
        }
    }

    const fn bit(self) -> u8 {
        match self {
            Self::File => 1,
            Self::Read => 2,
            Self::Triage => 4,
            Self::Purge => 8,
        }
    }
}

/// What a report token allows: a set of rights, travelling as their words,
/// each once. A token holds at least one, and each with what it needs
/// ([`Self::validate`]).
#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Vec<ReportScope>", into = "Vec<ReportScope>")]
pub struct ReportScopes(u8);

impl std::fmt::Debug for ReportScopes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_set().entries(self.rights()).finish()
    }
}

impl TryFrom<Vec<ReportScope>> for ReportScopes {
    type Error = Error;

    /// A set written with each right once, in any order.
    fn try_from(rights: Vec<ReportScope>) -> Result<Self, Error> {
        let scopes = Self::of(&rights);
        if scopes.rights().count() == rights.len() {
            Ok(scopes)
        } else {
            Err(Error::Rejected)
        }
    }
}

impl From<ReportScopes> for Vec<ReportScope> {
    fn from(scopes: ReportScopes) -> Self {
        scopes.rights().collect()
    }
}

impl ReportScopes {
    /// File and nothing else.
    pub const ONLY_POST: Self = Self::of(&[ReportScope::File]);
    /// Read and nothing else.
    pub const ONLY_READ: Self = Self::of(&[ReportScope::Read]);
    /// Read and triage.
    pub const READ_AND_EDIT: Self = Self::of(&[ReportScope::Read, ReportScope::Triage]);
    /// Every right.
    pub const EVERYTHING: Self = Self::of(&ReportScope::ALL);
    /// The presets the website offers, the least first.
    pub const PRESETS: [Self; 4] = [
        Self::ONLY_POST,
        Self::ONLY_READ,
        Self::READ_AND_EDIT,
        Self::EVERYTHING,
    ];

    /// Exactly `rights`.
    #[must_use]
    pub const fn of(rights: &[ReportScope]) -> Self {
        let mut bits = 0;
        let mut at = 0;
        while at < rights.len() {
            bits |= rights[at].bit();
            at += 1;
        }
        Self(bits)
    }

    /// The rights among them, the least first.
    pub fn rights(self) -> impl Iterator<Item = ReportScope> {
        ReportScope::ALL
            .into_iter()
            .filter(move |right| self.allows(*right))
    }

    /// Whether `right` is among them.
    #[must_use]
    pub const fn allows(self, right: ReportScope) -> bool {
        self.0 & right.bit() != 0
    }

    /// `right` given, with what it needs; or taken, with what needs it: the
    /// rights of a form stay ones a token may hold.
    #[must_use]
    pub fn set(self, right: ReportScope, held: bool) -> Self {
        if held {
            let with = Self(self.0 | right.bit());
            return right.needs().map_or(with, |needed| with.set(needed, true));
        }
        let without = Self(self.0 & !right.bit());
        ReportScope::ALL
            .into_iter()
            .filter(|other| other.needs() == Some(right))
            .fold(without, |left, other| left.set(other, false))
    }

    /// The rights, unless none is set or one's need is missing.
    ///
    /// # Errors
    /// `Rejected` for a token that would allow nothing, or a right without
    /// the one it needs.
    pub fn validate(self) -> Result<Self, Error> {
        let whole = self
            .rights()
            .all(|right| right.needs().is_none_or(|needed| self.allows(needed)));
        if self != Self::default() && whole {
            Ok(self)
        } else {
            Err(Error::Rejected)
        }
    }
}

/// Check what a new token is asked for: its name ([`token_name`]), its
/// rights and its lifetime in days.
///
/// # Errors
/// `Rejected` for a malformed name, rights a token cannot hold or a lifetime
/// out of 1 to [`MAX_TOKEN_DAYS`].
pub fn validate_new(name: &str, scopes: ReportScopes, days: u16) -> Result<(), Error> {
    token_name(name)?;
    scopes.validate()?;
    if days == 0 || days > MAX_TOKEN_DAYS {
        return Err(Error::Rejected);
    }
    Ok(())
}

/// A report token's text form, as the website shows it and a script sends
/// it: [`TOKEN_PREFIX`] and the token's 64 hex digits, in a zeroizing
/// buffer. Never log it.
#[must_use]
pub fn token_text(token: &Token) -> Zeroizing<String> {
    let mut out = Zeroizing::new(String::with_capacity(TOKEN_PREFIX.len() + 64));
    out.push_str(TOKEN_PREFIX);
    out.push_str(&token.hex());
    out
}

/// Read a report token's text form, exactly as [`token_text`] writes it;
/// surrounding whitespace from a paste is the one allowance.
///
/// # Errors
/// `Rejected` for anything else: another prefix, case, length or character.
pub fn parse_token_text(text: &str) -> Result<Token, Error> {
    Token::from_hex(
        text.trim()
            .strip_prefix(TOKEN_PREFIX)
            .ok_or(Error::Rejected)?,
    )
}

/// What the report routes are presented with, told apart by its form alone,
/// so that each kind is looked for in its own table and nowhere else.
pub enum ReportBearer {
    /// `Bearer <64 hex>`: a gateway's credential or token, or the bridge's.
    Gateway(Token),
    /// `Bearer brt_<64 hex>`: a report token.
    Report(Token),
}

impl std::fmt::Debug for ReportBearer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Gateway(_) => "ReportBearer::Gateway([REDACTED])",
            Self::Report(_) => "ReportBearer::Report([REDACTED])",
        })
    }
}

impl ReportBearer {
    /// Read an `Authorization` header: the canonical spelling of either
    /// kind, no whitespace and no alternate encoding.
    ///
    /// # Errors
    /// `Rejected` for anything else.
    pub fn from_header(value: &str) -> Result<Self, Error> {
        let text = value.strip_prefix("Bearer ").ok_or(Error::Rejected)?;
        match text.strip_prefix(TOKEN_PREFIX) {
            Some(hex) => Ok(Self::Report(Token::from_hex(hex)?)),
            None => Ok(Self::Gateway(Token::from_hex(text)?)),
        }
    }
}

/// A report token as administrators see it: never the token itself, which
/// only its creation answer carries.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReportTokenInfo {
    /// The token's id.
    pub id: [u8; 16],
    /// Its name.
    pub name: String,
    /// What it allows.
    pub scopes: ReportScopes,
    /// The administrator who made it.
    pub created_by: [u8; 16],
    /// That account's display name today, if it has one.
    pub created_by_name: Option<String>,
    /// That account's tag, if it has one.
    pub created_by_tag: Option<u32>,
    /// Its maker is no administrator now, so it authenticates nothing.
    pub suspended: bool,
    /// When it was made, in Unix seconds.
    pub created_at: i64,
    /// When it stops working, in Unix seconds.
    pub expires_at: i64,
    /// When it was revoked, if it was.
    pub revoked_at: Option<i64>,
    /// The administrator who revoked it; none for the operator.
    pub revoked_by: Option<[u8; 16]>,
    /// That account's display name today, if it has one.
    pub revoked_by_name: Option<String>,
    /// That account's tag, if it has one.
    pub revoked_by_tag: Option<u32>,
    /// When it was last used, to the minute, if ever.
    pub last_used_at: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every combination of the four rights.
    fn combinations() -> impl Iterator<Item = ReportScopes> {
        (0_u8..16).map(ReportScopes)
    }

    #[test]
    fn a_token_holds_something_and_each_right_with_what_it_needs() {
        let valid: Vec<ReportScopes> = combinations()
            .filter(|scopes| scopes.validate().is_ok())
            .collect();
        // with or without file: nothing (then file alone), read, read and
        // triage, or all three
        assert_eq!(valid.len(), 7, "{valid:?}");
        for scopes in combinations() {
            let [file, read, triage, purge] = ReportScope::ALL.map(|right| scopes.allows(right));
            // the chain of needs, then something at all
            let chain = match (read, triage, purge) {
                (_, false, false) => true,
                (read, true, _) => read,
                (_, false, true) => false,
            };
            let sound = chain && (file || read || triage || purge);
            assert_eq!(scopes.validate().is_ok(), sound, "{scopes:?}");
        }
        for preset in ReportScopes::PRESETS {
            assert_eq!(preset.validate(), Ok(preset));
        }
    }

    #[test]
    fn the_presets_are_the_four_the_owner_named() {
        let held = |scopes: ReportScopes| ReportScope::ALL.map(|right| scopes.allows(right));
        assert_eq!(
            ReportScopes::PRESETS.map(held),
            [
                [true, false, false, false],
                [false, true, false, false],
                [false, true, true, false],
                [true, true, true, true],
            ]
        );
    }

    #[test]
    fn a_right_given_brings_its_needs_and_one_taken_takes_what_needs_it() {
        let none = ReportScopes::default();
        assert_eq!(
            none.set(ReportScope::Purge, true),
            ReportScopes::of(&[ReportScope::Read, ReportScope::Triage, ReportScope::Purge])
        );
        assert_eq!(
            none.set(ReportScope::Triage, true),
            ReportScopes::READ_AND_EDIT
        );
        assert_eq!(
            ReportScopes::EVERYTHING.set(ReportScope::Read, false),
            ReportScopes::ONLY_POST
        );
        // a form that only ever sets and takes never holds a broken set
        for scopes in combinations().filter(|scopes| scopes.validate().is_ok()) {
            for right in ReportScope::ALL {
                for held in [true, false] {
                    let next = scopes.set(right, held);
                    assert_eq!(next.allows(right), held);
                    assert!(next == none || next.validate().is_ok(), "{next:?}");
                }
            }
        }
    }

    #[test]
    fn a_new_token_is_asked_for_within_its_bounds() {
        let ask = |name: &str, days| validate_new(name, ReportScopes::ONLY_READ, days);
        assert!(ask("CI", 1).is_ok());
        assert!(ask("CI", MAX_TOKEN_DAYS).is_ok());
        for (name, days) in [
            ("CI", 0),
            ("CI", MAX_TOKEN_DAYS + 1),
            ("", 30),
            (" CI", 30),
            ("C\u{202e}I", 30),
            (&"x".repeat(65), 30),
        ] {
            assert_eq!(ask(name, days), Err(Error::Rejected), "{name:?} {days}");
        }
        assert_eq!(
            validate_new("CI", ReportScopes::default(), 30),
            Err(Error::Rejected)
        );
    }

    #[test]
    fn a_token_reads_back_from_its_text_and_from_nothing_else() {
        let token = Token::generate().unwrap();
        let text = token_text(&token);
        assert_eq!(text.len(), 68);
        assert!(text.starts_with("brt_"));
        assert_eq!(parse_token_text(&text).unwrap().digest(), token.digest());
        assert_eq!(
            parse_token_text(&format!(" {}\n", text.as_str()))
                .unwrap()
                .digest(),
            token.digest()
        );
        let hex = token.hex();
        for bad in [
            hex.to_string(),
            format!("bgt_{}", hex.as_str()),
            format!("BRT_{}", hex.as_str()),
            format!("brt_{}", hex.to_uppercase()),
            format!("brt_{}", &hex[1..]),
            format!("brt_{}0", hex.as_str()),
            format!("brt_ {}", hex.as_str()),
            String::new(),
        ] {
            assert!(parse_token_text(&bad).is_err(), "{bad:?}");
        }
        // a gateway's text form is no report token's, nor the other way
        assert!(crate::owner::parse_token_text(&text).is_err());
        assert!(parse_token_text(&crate::owner::token_text(&token)).is_err());
    }

    #[test]
    fn a_header_says_which_kind_it_carries_and_prints_neither() {
        let token = Token::generate().unwrap();
        let hex = token.hex();
        let report = format!("Bearer brt_{}", hex.as_str());
        let gateway = format!("Bearer {}", hex.as_str());
        let read = |value: &str| ReportBearer::from_header(value);
        assert!(
            matches!(read(&report), Ok(ReportBearer::Report(t)) if t.digest() == token.digest())
        );
        assert!(
            matches!(read(&gateway), Ok(ReportBearer::Gateway(t)) if t.digest() == token.digest())
        );
        for bad in [
            String::new(),
            hex.to_string(),
            format!("brt_{}", hex.as_str()),
            format!("bearer brt_{}", hex.as_str()),
            format!("Bearer  brt_{}", hex.as_str()),
            format!("Bearer bgt_{}", hex.as_str()),
            format!("Bearer brt_brt_{}", hex.as_str()),
            format!("Bearer brt_{} ", hex.as_str()),
            format!("Bearer brt_{}", hex.to_uppercase()),
            format!("Bearer brt_{}", &hex[1..]),
            format!("Bearer brt_{},brt_{}", hex.as_str(), hex.as_str()),
            "Bearer a.b.c".to_owned(),
        ] {
            assert!(read(&bad).is_err(), "{bad:?}");
        }
        for bearer in [read(&report).unwrap(), read(&gateway).unwrap()] {
            let printed = format!("{bearer:?}");
            assert!(printed.contains("REDACTED") && !printed.contains(hex.as_str()));
        }
    }

    #[test]
    fn the_rights_travel_whole_and_nothing_else_with_them() {
        let text = serde_json::to_string(&ReportScopes::READ_AND_EDIT).unwrap();
        assert_eq!(text, r#"["read","triage"]"#);
        for text in [text.as_str(), r#"["triage","read"]"#] {
            assert_eq!(
                serde_json::from_str::<ReportScopes>(text).unwrap(),
                ReportScopes::READ_AND_EDIT
            );
        }
        for scopes in combinations() {
            let text = serde_json::to_string(&scopes).unwrap();
            assert_eq!(serde_json::from_str::<ReportScopes>(&text).unwrap(), scopes);
        }
        for bad in [
            r#"["read","read"]"#,
            r#"["read","admin"]"#,
            r#"["READ"]"#,
            r#""read""#,
            r#"{"read":true}"#,
            "2",
        ] {
            assert!(serde_json::from_str::<ReportScopes>(bad).is_err(), "{bad}");
        }
        assert_eq!(
            ReportScope::ALL.map(ReportScope::as_str),
            ["file", "read", "triage", "purge"]
        );
    }
}
