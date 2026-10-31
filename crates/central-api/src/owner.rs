//! What an account does with its own sessions and gateways on the website
//! (`docs/web-accounts.md` §"The account area", `docs/gateway-registry.md`
//! §"Gateway tokens"): on the browser listener only, with the cookie's
//! session and Central's terms accepted. Minting a token is a change that
//! hands out a credential, so it needs a password sign-in less than 30
//! minutes old, as an administrator's change does.
//!
//! A gateway token is the credential a self-hosted gateway pastes into its
//! admin console: one [`Token`], shown once, stored as its SHA-256 digest,
//! scoped ([`GatewayScopes`]), named, expiring and revocable. It
//! authenticates exactly as its gateway, for what its scopes allow, and only
//! while the gateway is approved.
use crate::gateway::{GatewayMetadata, GatewayState};
use crate::{Error, Token};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

/// Live (neither revoked nor expired) tokens one gateway may hold at once.
pub const TOKENS_PER_GATEWAY: usize = 8;
/// The longest a token lives, in days; the database holds it to 366 too.
pub const MAX_TOKEN_DAYS: u16 = 366;
/// The longest token name, in bytes.
pub const MAX_TOKEN_NAME: usize = 64;
/// What a token's text form starts with, so a person (and a secret scanner)
/// can tell it from any other secret: `bgt_` and 64 lowercase hex digits.
pub const TOKEN_PREFIX: &str = "bgt_";

/// What a gateway token lets its gateway do. At least one is set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GatewayScopes {
    /// Sign players in through Central: redeem their tickets and check their
    /// leases, profiles, admin rights and terms.
    pub sso: bool,
    /// Beat for the public gateway directory: its name, version, website and
    /// whether it is online.
    pub listing: bool,
}

impl GatewayScopes {
    /// The scopes, unless none is set.
    ///
    /// # Errors
    /// `Rejected` for a token that would allow nothing.
    pub const fn validate(self) -> Result<Self, Error> {
        if self.sso || self.listing {
            Ok(self)
        } else {
            Err(Error::Rejected)
        }
    }
}

/// Check a token's name: 1 to [`MAX_TOKEN_NAME`] bytes of text without
/// control characters or surrounding space. It is the owner's own label,
/// shown to them and to administrators as text, never as markup.
///
/// # Errors
/// `Rejected` for an empty, overlong, padded or control-bearing name.
pub fn token_name(name: &str) -> Result<&str, Error> {
    if name.is_empty()
        || name.len() > MAX_TOKEN_NAME
        || name.trim() != name
        || name.chars().any(char::is_control)
    {
        return Err(Error::Rejected);
    }
    Ok(name)
}

/// A token's text form, as the website shows it and a gateway's console
/// takes it: [`TOKEN_PREFIX`] and the token's 64 hex digits, in a zeroizing
/// buffer. Never log it.
#[must_use]
pub fn token_text(token: &Token) -> Zeroizing<String> {
    let mut out = Zeroizing::new(String::with_capacity(TOKEN_PREFIX.len() + 64));
    out.push_str(TOKEN_PREFIX);
    out.push_str(&token.hex());
    out
}

/// Read a token's text form, exactly as [`token_text`] writes it; surrounding
/// whitespace from a paste is the one allowance.
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

/// Which listener a session belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionKind {
    /// The client's sign-in, on the native API.
    Native,
    /// The website's sign-in, the `__Host-central` cookie.
    Browser,
}

/// One live session of the account. Central knows no device: an account has
/// at most one session of each kind, and that is all there is to list.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionInfo {
    /// The listener it belongs to.
    pub kind: SessionKind,
    /// It is the session that asked.
    pub current: bool,
    /// Its password sign-in, in Unix seconds; renewal never moves it.
    pub signed_in_at: i64,
    /// When it ends unless renewed, in Unix seconds.
    pub expires_at: i64,
    /// The gateways it is signed in to now (the audiences of its live
    /// leases), at most [`TOKENS_PER_GATEWAY`] times four, sorted.
    pub gateways: Vec<String>,
}

/// One token of a gateway, as its owner and administrators see it: never the
/// token itself, which only its creation answer carries.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TokenInfo {
    /// The token's id.
    pub id: [u8; 16],
    /// Its owner's label.
    pub name: String,
    /// What it allows.
    pub scopes: GatewayScopes,
    /// When it was made, in Unix seconds.
    pub created_at: i64,
    /// When it stops working, in Unix seconds.
    pub expires_at: i64,
    /// When it was revoked, if it was.
    pub revoked_at: Option<i64>,
    /// When its gateway last used it (a heartbeat, a sign-in or a status
    /// check), to the minute, if ever.
    pub last_used_at: Option<i64>,
}

/// A gateway the account owns.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MyGateway {
    /// The gateway's id.
    pub id: [u8; 16],
    /// Its origin.
    pub origin: String,
    /// Its public metadata, as registered or last sent by its heartbeat.
    pub metadata: GatewayMetadata,
    /// Whether an administrator approved it.
    pub state: GatewayState,
    /// A heartbeat arrived in the last 90 seconds.
    pub online: bool,
    /// Its last heartbeat, in Unix seconds, if any.
    pub last_seen: Option<i64>,
    /// When it was registered, in Unix seconds.
    pub created_at: i64,
    /// Its tokens, newest first: live ones, and those revoked or expired in
    /// the last 30 days.
    pub tokens: Vec<TokenInfo>,
}

/// An account's request about its own sessions and gateways, tagged by
/// `action`.
#[derive(Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum OwnerRequest {
    /// The account's live sessions. Answered with [`OwnerAnswer::Sessions`].
    Sessions,
    /// End the account's client session (and with it its tickets and its
    /// sign-ins at gateways). The website's own session ends with `Logout`.
    /// `Done` also when there is none.
    EndClientSession,
    /// The gateways the account owns, with their tokens. Answered with
    /// [`OwnerAnswer::Gateways`].
    Gateways,
    /// Register a gateway at `origin` under `name`, awaiting an
    /// administrator's approval; it authenticates by tokens alone. Answered
    /// with [`OwnerAnswer::GatewayAdded`]. An origin already registered is a
    /// `Conflict`; an account owns at most eight gateways.
    AddGateway {
        /// Its canonical https origin.
        origin: String,
        /// Its public name until its first heartbeat sends one.
        name: String,
    },
    /// Mint a token for an owned gateway: a change, so it needs a password
    /// sign-in less than 30 minutes old. Answered with
    /// [`OwnerAnswer::TokenCreated`], the only answer that carries a token.
    CreateToken {
        /// The gateway's id.
        gateway: [u8; 16],
        /// The owner's label for it ([`token_name`]).
        name: String,
        /// What it allows.
        scopes: GatewayScopes,
        /// How many days it lives, 1 to [`MAX_TOKEN_DAYS`].
        days: u16,
    },
    /// Revoke a token of an owned gateway at once. `Done` again for one
    /// revoked already.
    RevokeToken {
        /// The token's id.
        token: [u8; 16],
    },
}

impl OwnerRequest {
    /// Whether the request hands out a credential and so needs a recent
    /// password sign-in. Every action is named, so a new one does not compile
    /// until it is decided here.
    #[must_use]
    pub const fn mints(&self) -> bool {
        match self {
            Self::CreateToken { .. } => true,
            Self::Sessions
            | Self::EndClientSession
            | Self::Gateways
            | Self::AddGateway { .. }
            | Self::RevokeToken { .. } => false,
        }
    }
}

/// The answer to an [`OwnerRequest`] that reads or creates; the others
/// answer `Done`.
#[derive(Serialize, Deserialize)]
#[serde(tag = "answer", rename_all = "snake_case", deny_unknown_fields)]
pub enum OwnerAnswer {
    /// The account's live sessions, the asking one first.
    Sessions {
        /// At most one of each kind.
        sessions: Vec<SessionInfo>,
    },
    /// The account's gateways, newest first.
    Gateways {
        /// At most eight.
        gateways: Vec<MyGateway>,
    },
    /// A gateway registered, awaiting approval.
    GatewayAdded {
        /// Its id.
        gateway: [u8; 16],
    },
    /// A token, shown to its owner once; Central keeps its digest.
    TokenCreated {
        /// The token's id.
        id: [u8; 16],
        /// The token: [`token_text`] is the form to show.
        token: Token,
        /// When it stops working, in Unix seconds.
        expires_at: i64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use zeroize::Zeroizing;

    #[test]
    fn a_token_reads_back_from_its_text_and_from_nothing_else() {
        let token = Token::generate().unwrap();
        let text = token_text(&token);
        assert_eq!(text.len(), 68);
        assert!(text.starts_with("bgt_"));
        assert_eq!(parse_token_text(&text).unwrap().digest(), token.digest());
        assert_eq!(
            parse_token_text(&format!("  {}\n", text.as_str()))
                .unwrap()
                .digest(),
            token.digest(),
            "a paste's surrounding whitespace"
        );
        let hex = token.hex();
        for bad in [
            hex.to_string(),
            format!("BGT_{}", hex.as_str()),
            format!("bgt-{}", hex.as_str()),
            format!("bgt_{}", hex.to_uppercase()),
            format!("bgt_{}", &hex[1..]),
            format!("bgt_{}0", hex.as_str()),
            format!("bgt_ {}", hex.as_str()),
            format!("Bearer {}", hex.as_str()),
            String::new(),
        ] {
            assert!(parse_token_text(&bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn the_text_form_matches_the_websites_vector() {
        // web/test/account/gateway-token.test.ts formats the same bytes
        let token = Token::from_secret_bytes(Zeroizing::new([0xab; 32]));
        assert_eq!(
            token_text(&token).as_str(),
            format!("bgt_{}", "ab".repeat(32))
        );
    }

    #[test]
    fn a_name_is_a_short_plain_label() {
        for good in [
            "Production",
            "Mein Server – Haupt",
            &"x".repeat(MAX_TOKEN_NAME),
        ] {
            assert_eq!(token_name(good), Ok(good));
        }
        for bad in [
            "",
            " padded",
            "padded ",
            "line\nbreak",
            "nul\0",
            &"x".repeat(MAX_TOKEN_NAME + 1),
        ] {
            assert_eq!(token_name(bad), Err(Error::Rejected), "{bad:?}");
        }
    }

    #[test]
    fn a_token_allows_something() {
        assert!(
            GatewayScopes {
                sso: false,
                listing: false
            }
            .validate()
            .is_err()
        );
        for (sso, listing) in [(true, false), (false, true), (true, true)] {
            assert!(GatewayScopes { sso, listing }.validate().is_ok());
        }
    }

    #[test]
    fn only_creating_a_token_mints() {
        assert!(
            OwnerRequest::CreateToken {
                gateway: [0; 16],
                name: "x".into(),
                scopes: GatewayScopes {
                    sso: true,
                    listing: true
                },
                days: 30,
            }
            .mints()
        );
        for request in [
            OwnerRequest::Sessions,
            OwnerRequest::EndClientSession,
            OwnerRequest::Gateways,
            OwnerRequest::AddGateway {
                origin: "https://gateway.invalid".into(),
                name: "x".into(),
            },
            OwnerRequest::RevokeToken { token: [0; 16] },
        ] {
            assert!(!request.mints());
        }
    }

    #[test]
    fn the_requests_and_answers_have_the_designed_shapes() {
        let request: OwnerRequest = crate::decode(
            br#"{"action":"create_token","gateway":[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],"name":"Main","scopes":{"sso":true,"listing":false},"days":90}"#,
        )
        .unwrap();
        assert!(matches!(
            request,
            OwnerRequest::CreateToken { days: 90, .. }
        ));
        for bad in [
            br#"{"action":"create_token","gateway":[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],"name":"Main","scopes":{"sso":true},"days":90}"#.as_slice(),
            br#"{"action":"create_token","gateway":[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],"name":"Main","scopes":{"sso":true,"listing":true,"admin":true},"days":90}"#,
            br#"{"action":"end_session","kind":"browser"}"#,
        ] {
            assert!(crate::decode::<OwnerRequest>(bad).is_err());
        }
        let answer = OwnerAnswer::Sessions {
            sessions: vec![SessionInfo {
                kind: SessionKind::Native,
                current: false,
                signed_in_at: 1_800_000_000,
                expires_at: 1_800_001_800,
                gateways: vec!["https://gateway.invalid".into()],
            }],
        };
        let text = String::from_utf8(crate::encode(&answer).unwrap()).unwrap();
        assert!(text.starts_with(r#"{"answer":"sessions","sessions":[{"kind":"native","#));
        assert!(crate::decode::<OwnerAnswer>(text.as_bytes()).is_ok());
    }
}
