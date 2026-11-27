//! AI runner tokens (`docs/ai-players.md` §2): the credential an
//! administrator makes for a machine that runs AI players. It is no
//! gateway's, no person's and no administrator's: it trades itself, on the
//! native listener and without an `Authorization` header, for an ordinary
//! native session of one of the AI accounts it lists
//! ([`crate::wire::Request::AiRunnerSession`]).
//!
//! A runner token is one [`Token`], shown once, stored as its SHA-256
//! digest, named, listing 1 to [`MAX_ACCOUNTS`] AI accounts, expiring and
//! revocable. Making one hands out a credential, so it needs a password
//! sign-in less than 30 minutes old; revoking one only takes exposure away
//! and needs none. Unlike a report token it does not end with its maker's
//! role.
use crate::owner::{MAX_TOKEN_DAYS, token_name};
use crate::{Error, Token};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

/// What a token's text form starts with, so a person (and a secret scanner)
/// can tell it from a gateway's, a report token's and any other secret:
/// `bat_` and 64 lowercase hex digits.
pub const TOKEN_PREFIX: &str = "bat_";
/// Runner sessions one AI account holds at once, of every token together;
/// an exchange past them ends the oldest.
pub const MAX_SESSIONS: usize = 4;
/// AI accounts one token lists.
pub const MAX_ACCOUNTS: usize = 16;
/// Live (neither revoked nor expired) runner tokens at once.
pub const LIVE_TOKENS: usize = 32;
/// Runner tokens made in a day, live or not.
pub const TOKENS_PER_DAY: usize = 32;
/// Runner tokens the administration lists: the live ones, and of those
/// revoked or expired in the last 30 days the newest.
pub const LISTED_TOKENS: usize = 64;
/// Successful exchanges an hour for one AI account, so that a thief cannot
/// churn the runner's sessions faster than that.
pub const EXCHANGES_PER_HOUR: u32 = 12;
/// Days an exchange is kept in the record of sign-ins.
pub const SIGNIN_DAYS: u16 = 90;
const _: () = assert!(LIVE_TOKENS <= LISTED_TOKENS && LIVE_TOKENS <= TOKENS_PER_DAY);

/// Check what a new token is asked for: its name ([`token_name`]), 1 to
/// [`MAX_ACCOUNTS`] accounts, each once, and its lifetime in days. Whether
/// each account is an AI player is the store's to say.
///
/// # Errors
/// `Rejected` for a malformed name, no account, too many or one twice, or
/// a lifetime out of 1 to [`MAX_TOKEN_DAYS`].
pub fn validate_new(name: &str, accounts: &[[u8; 16]], days: u16) -> Result<(), Error> {
    token_name(name)?;
    let mut distinct = accounts.to_vec();
    distinct.sort_unstable();
    distinct.dedup();
    if accounts.is_empty() || accounts.len() > MAX_ACCOUNTS || distinct.len() != accounts.len() {
        return Err(Error::Rejected);
    }
    if days == 0 || days > MAX_TOKEN_DAYS {
        return Err(Error::Rejected);
    }
    Ok(())
}

/// A runner token's text form, as the website shows it and a runner's
/// credential file holds it: [`TOKEN_PREFIX`] and the token's 64 hex
/// digits, in a zeroizing buffer. Never log it.
#[must_use]
pub fn token_text(token: &Token) -> Zeroizing<String> {
    let mut out = Zeroizing::new(String::with_capacity(TOKEN_PREFIX.len() + 64));
    out.push_str(TOKEN_PREFIX);
    out.push_str(&token.hex());
    out
}

/// Read a runner token's text form, exactly as [`token_text`] writes it;
/// surrounding whitespace from a paste or a file's last line is the one
/// allowance.
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

/// An account a runner token lists, as it is called today.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiRunnerAccount {
    /// The account's id.
    pub id: [u8; 16],
    /// Its display name, if it has one.
    pub display_name: Option<String>,
    /// Its tag, if it has one.
    pub tag: Option<u32>,
}

/// A runner token as administrators see it: never the token itself, which
/// only its creation answer carries.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiRunnerTokenInfo {
    /// The token's id.
    pub id: [u8; 16],
    /// Its name.
    pub name: String,
    /// The AI accounts it lists, by id; one that is no AI any more has left
    /// the list.
    pub accounts: Vec<AiRunnerAccount>,
    /// The administrator who made it, or none for the operator
    /// (`central-service --ai-players-stdin`).
    pub created_by: Option<[u8; 16]>,
    /// That account's display name today, if it has one.
    pub created_by_name: Option<String>,
    /// That account's tag, if it has one.
    pub created_by_tag: Option<u32>,
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
    /// When it was last exchanged for a session, if ever.
    pub last_used_at: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account(n: u8) -> [u8; 16] {
        [n; 16]
    }

    #[test]
    fn a_new_token_is_asked_for_within_its_bounds() {
        let one = [account(1)];
        let most: Vec<[u8; 16]> = (0..16).map(account).collect();
        assert!(validate_new("runner-1", &one, 1).is_ok());
        assert!(validate_new("runner-1", &most, MAX_TOKEN_DAYS).is_ok());
        let too_many: Vec<[u8; 16]> = (0..17).map(account).collect();
        for (name, accounts, days) in [
            ("runner-1", &one[..], 0),
            ("runner-1", &one[..], MAX_TOKEN_DAYS + 1),
            ("runner-1", &[][..], 90),
            ("runner-1", &too_many[..], 90),
            ("runner-1", &[account(1), account(2), account(1)][..], 90),
            ("", &one[..], 90),
            (" runner", &one[..], 90),
            ("run\u{202e}ner", &one[..], 90),
            (&"x".repeat(65), &one[..], 90),
        ] {
            assert_eq!(
                validate_new(name, accounts, days),
                Err(Error::Rejected),
                "{name:?} {} {days}",
                accounts.len()
            );
        }
    }

    #[test]
    fn a_token_reads_back_from_its_text_and_from_nothing_else() {
        let token = Token::generate().unwrap();
        let text = token_text(&token);
        assert_eq!(text.len(), 68);
        assert!(text.starts_with("bat_"));
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
            format!("brt_{}", hex.as_str()),
            format!("bgt_{}", hex.as_str()),
            format!("BAT_{}", hex.as_str()),
            format!("Bat_{}", hex.as_str()),
            format!("bat_{}", hex.to_uppercase()),
            format!("bat_{}", &hex[1..]),
            format!("bat_{}0", hex.as_str()),
            format!("bat_{}", &hex[..62]),
            format!("bat_ {}", hex.as_str()),
            format!("bat_bat_{}", hex.as_str()),
            format!("Bearer bat_{}", hex.as_str()),
            "bat_".to_owned(),
            String::new(),
        ] {
            assert!(parse_token_text(&bad).is_err(), "{bad:?}");
        }
        // a gateway's and a report token's text forms are no runner
        // token's, nor the other way round
        assert!(crate::owner::parse_token_text(&text).is_err());
        assert!(crate::report_tokens::parse_token_text(&text).is_err());
        assert!(parse_token_text(&crate::owner::token_text(&token)).is_err());
        assert!(parse_token_text(&crate::report_tokens::token_text(&token)).is_err());
    }

    #[test]
    fn a_listing_carries_no_secret_and_refuses_what_it_does_not_know() {
        let info = AiRunnerTokenInfo {
            id: [1; 16],
            name: "runner-1".into(),
            accounts: vec![AiRunnerAccount {
                id: [3; 16],
                display_name: Some("KI-Haus-Expert".into()),
                tag: Some(0x3fa2),
            }],
            created_by: Some([2; 16]),
            created_by_name: Some("Synthetic".into()),
            created_by_tag: None,
            created_at: 1_800_000_000,
            expires_at: 1_807_776_000,
            revoked_at: None,
            revoked_by: None,
            revoked_by_name: None,
            revoked_by_tag: None,
            last_used_at: Some(1_800_000_060),
        };
        let text = serde_json::to_string(&info).unwrap();
        assert!(!text.contains("digest") && !text.contains("token"));
        assert_eq!(
            serde_json::from_str::<AiRunnerTokenInfo>(&text).unwrap(),
            info
        );
        let more = text.replace(r#""name":"runner-1""#, r#""name":"runner-1","digest":"x""#);
        assert!(serde_json::from_str::<AiRunnerTokenInfo>(&more).is_err());
    }
}
