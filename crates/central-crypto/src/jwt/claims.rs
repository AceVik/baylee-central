//! What each token says. Every type refuses unknown and duplicate fields, and none
//! implements `Debug`, so that no claim set reaches a log by accident.
use super::{Kind, MAX_SIDS};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// The registered claims every token carries.
pub struct Registered<'a> {
    /// Issuer.
    pub iss: &'a str,
    /// Audience: exactly one origin.
    pub aud: &'a str,
    /// Issued at, Unix seconds.
    pub iat: i64,
    /// Not before, Unix seconds.
    pub nbf: i64,
    /// Expiry, Unix seconds.
    pub exp: i64,
}

/// A claim set of one [`Kind`].
pub trait Claims: Serialize + DeserializeOwned {
    /// The kind whose `typ` these claims travel under.
    const KIND: Kind;
    /// The registered claims.
    fn registered(&self) -> Registered<'_>;
    /// The token's own unique identifier.
    fn jti(&self) -> &str;
    /// Every claim is inside its range.
    fn well_formed(&self) -> bool;
}

fn lower_hex(text: &str, length: usize) -> bool {
    text.len() == length
        && text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// A hyphenated lowercase UUID.
fn uuid_text(text: &str) -> bool {
    text.len() == 36
        && text.bytes().enumerate().all(|(index, b)| {
            if matches!(index, 8 | 13 | 18 | 23) {
                b == b'-'
            } else {
                b.is_ascii_digit() || (b'a'..=b'f').contains(&b)
            }
        })
}

/// A session identifier as a gateway sees it: 32 lowercase hex digits.
#[must_use]
pub fn valid_sid(text: &str) -> bool {
    lower_hex(text, 32)
}

/// What a gateway token admits to.
#[derive(Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Purpose {
    /// Play at this gateway.
    Admit,
    /// Administer this gateway; Central checked current membership at issue.
    Admin,
}

/// The gateway's terms an account accepted last.
#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TermsClaim {
    /// The published version.
    pub version: String,
    /// 64 lowercase hex digits: the digest of the version's texts.
    pub digest: String,
}

/// The client's session at Central. `sid` is the session's random capability, of which
/// Central keeps only a digest: the signing key alone forges no session. It is secret,
/// so these claims are wiped when dropped.
#[derive(Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
#[serde(deny_unknown_fields)]
pub struct CentralClaims {
    /// Central's origin.
    pub iss: String,
    /// Central's origin again: no one else reads this token.
    pub aud: String,
    /// The account's UUID.
    pub sub: String,
    /// The session's capability, 64 lowercase hex digits.
    pub sid: String,
    /// 32 lowercase hex digits, unique.
    pub jti: String,
    /// Issued at.
    pub iat: i64,
    /// Not before.
    pub nbf: i64,
    /// The session's cap; the session's row decides whether it is still live.
    pub exp: i64,
}

impl Claims for CentralClaims {
    const KIND: Kind = Kind::Central;
    fn registered(&self) -> Registered<'_> {
        Registered {
            iss: &self.iss,
            aud: &self.aud,
            iat: self.iat,
            nbf: self.nbf,
            exp: self.exp,
        }
    }
    fn jti(&self) -> &str {
        &self.jti
    }
    fn well_formed(&self) -> bool {
        uuid_text(&self.sub) && lower_hex(&self.sid, 64) && lower_hex(&self.jti, 32)
    }
}

/// A client's admission at the one gateway named by `aud`.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GatewayClaims {
    /// Central's origin.
    pub iss: String,
    /// The gateway's origin.
    pub aud: String,
    /// The account's UUID.
    pub sub: String,
    /// The session as this gateway knows it; no other gateway sees this value.
    pub sid: String,
    /// 32 lowercase hex digits, unique.
    pub jti: String,
    /// Issued at.
    pub iat: i64,
    /// Not before.
    pub nbf: i64,
    /// Expiry.
    pub exp: i64,
    /// What the token admits to.
    pub purpose: Purpose,
    /// The account's display name, while it has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The account's tag, while it has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag: Option<u32>,
    /// The official gateway's old account id, for a closed move only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legacy_id: Option<String>,
    /// This gateway's terms the account accepted last.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terms: Option<TermsClaim>,
}

impl GatewayClaims {
    /// The account's 16 bytes.
    #[must_use]
    pub fn account(&self) -> Option<[u8; 16]> {
        let digits = self.sub.replace('-', "");
        let mut bytes = [0; 16];
        for (index, byte) in bytes.iter_mut().enumerate() {
            *byte = u8::from_str_radix(digits.get(index * 2..index * 2 + 2)?, 16).ok()?;
        }
        (digits.len() == 32).then_some(bytes)
    }
}

impl Claims for GatewayClaims {
    const KIND: Kind = Kind::Gateway;
    fn registered(&self) -> Registered<'_> {
        Registered {
            iss: &self.iss,
            aud: &self.aud,
            iat: self.iat,
            nbf: self.nbf,
            exp: self.exp,
        }
    }
    fn jti(&self) -> &str {
        &self.jti
    }
    fn well_formed(&self) -> bool {
        uuid_text(&self.sub)
            && valid_sid(&self.sid)
            && lower_hex(&self.jti, 32)
            && self.tag != Some(0)
            && self
                .terms
                .as_ref()
                .is_none_or(|terms| !terms.version.is_empty() && lower_hex(&terms.digest, 64))
    }
}

/// Central's message to one gateway: the sessions numbered `from` to `seq` ended. It
/// names only `sid`s this gateway was given, and never a token.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RevocationClaims {
    /// Central's origin.
    pub iss: String,
    /// The gateway's canonical origin.
    pub aud: String,
    /// 32 lowercase hex digits, new for every send.
    pub jti: String,
    /// Issued at.
    pub iat: i64,
    /// Not before.
    pub nbf: i64,
    /// Expiry, a minute after `iat`.
    pub exp: i64,
    /// The first number covered, from 1.
    pub from: u64,
    /// The last number covered.
    pub seq: u64,
    /// One `sid` for each number from `from` to `seq`, in order.
    pub sids: Vec<String>,
}

impl Claims for RevocationClaims {
    const KIND: Kind = Kind::Revocation;
    fn registered(&self) -> Registered<'_> {
        Registered {
            iss: &self.iss,
            aud: &self.aud,
            iat: self.iat,
            nbf: self.nbf,
            exp: self.exp,
        }
    }
    fn jti(&self) -> &str {
        &self.jti
    }
    fn well_formed(&self) -> bool {
        lower_hex(&self.jti, 32)
            && self.from >= 1
            && self.seq >= self.from
            && !self.sids.is_empty()
            && self.sids.len() <= MAX_SIDS
            && u64::try_from(self.sids.len()).is_ok_and(|count| self.seq - self.from + 1 == count)
            && self.sids.iter().all(|sid| valid_sid(sid))
    }
}
