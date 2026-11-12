//! An account's optional e-mail address (`docs/email.md`): naming one,
//! confirming it with a mailed code, removing it, and asking for a reset
//! code by mail. No account needs one: a sign-in name and a password are
//! enough, and nothing else in the protocol knows an address.
use crate::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

/// Longest address taken.
pub const MAX_ADDRESS: usize = 254;
/// Longest text read as a code; longer input is refused before decoding.
pub const MAX_MAIL_CODE_TEXT: usize = 32;
/// Crockford's base32 alphabet: no I, L, O or U.
const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
/// Symbols of a confirmation code: 8 of 5 bits are its 40 bits.
const SYMBOLS: usize = 8;

/// The characters RFC 5322 allows in an unquoted local part (`atext`).
fn atext(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b"!#$%&'*+/=?^_`{|}~-".contains(&byte)
}

/// Check an address by the rules of the service that sends the mail
/// (`baylee-mailer`, `API.md`), so that Central stores none it would refuse:
/// one plain address in ASCII, at most [`MAX_ADDRESS`] characters, a
/// dot-atom local part of at most 64, and a host name of at least two
/// labels. No display name, no quoted local part, no address literal.
///
/// # Errors
/// `Rejected` for anything else. The text is not echoed.
pub fn address(text: &str) -> Result<(), Error> {
    if text.is_empty() || text.len() > MAX_ADDRESS || !text.is_ascii() {
        return Err(Error::Rejected);
    }
    let (local, domain) = text.rsplit_once('@').ok_or(Error::Rejected)?;
    if local.is_empty()
        || local.len() > 64
        || local.starts_with('.')
        || local.ends_with('.')
        || local.contains("..")
        || !local.bytes().all(|byte| atext(byte) || byte == b'.')
    {
        return Err(Error::Rejected);
    }
    let mut labels = 0;
    let mut last = "";
    for label in domain.split('.') {
        if label.is_empty()
            || label.len() > 63
            || label.starts_with('-')
            || label.ends_with('-')
            || !label
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return Err(Error::Rejected);
        }
        labels += 1;
        last = label;
    }
    if labels < 2 || last.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(Error::Rejected);
    }
    Ok(())
}

/// Check the shape of a language tag a mail is asked in (`en`, `pt-br`,
/// `zh-hant`): 2 to 10 lower-case letters and hyphens, a letter first and
/// last. Central writes in English where it has no text for the tag.
///
/// # Errors
/// `Rejected` for any other text.
pub fn language(tag: &str) -> Result<(), Error> {
    let bytes = tag.as_bytes();
    let letter = |byte: Option<&u8>| byte.is_some_and(u8::is_ascii_lowercase);
    if !(2..=10).contains(&bytes.len())
        || !letter(bytes.first())
        || !letter(bytes.last())
        || !bytes
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || *byte == b'-')
    {
        return Err(Error::Rejected);
    }
    Ok(())
}

/// The code a confirmation mail carries: 40 random bits, shown as two
/// groups of four symbols (`XXXX-XXXX`). It is short because it lives a
/// quarter of an hour and Central takes few wrong ones for an address
/// (`docs/email.md`); it authorizes nothing but the address it was mailed
/// to. Neither Clone nor Display; Debug redacts it.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct MailCode([u8; 5]);

impl std::fmt::Debug for MailCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MailCode([REDACTED])")
    }
}

impl MailCode {
    /// A new code from operating-system randomness.
    ///
    /// # Errors
    /// Fails closed when operating-system randomness is unavailable.
    pub fn generate() -> Result<Self, Error> {
        let mut code = Self([0; 5]);
        getrandom::fill(&mut code.0).map_err(|_| Error::Unavailable)?;
        Ok(code)
    }

    /// Read a code as a person typed it: case-insensitive, with `O` read as
    /// `0` and `I` or `L` as `1`, hyphens and ASCII white space anywhere.
    ///
    /// # Errors
    /// Rejects longer input than [`MAX_MAIL_CODE_TEXT`], a wrong number of
    /// symbols and any symbol outside the alphabet.
    pub fn parse(text: &str) -> Result<Self, Error> {
        if text.len() > MAX_MAIL_CODE_TEXT {
            return Err(Error::Rejected);
        }
        let mut code = Self([0; 5]);
        let mut count = 0;
        for byte in text.bytes() {
            if byte == b'-' || byte.is_ascii_whitespace() {
                continue;
            }
            if count == SYMBOLS {
                return Err(Error::Rejected);
            }
            let symbol = match byte.to_ascii_uppercase() {
                b'O' => b'0',
                b'I' | b'L' => b'1',
                other => other,
            };
            let value = ALPHABET
                .iter()
                .position(|a| *a == symbol)
                .ok_or(Error::Rejected)?;
            for bit in 0..5 {
                if value >> (4 - bit) & 1 == 1 {
                    let position = count * 5 + bit;
                    code.0[position / 8] |= 0x80 >> (position % 8);
                }
            }
            count += 1;
        }
        if count != SYMBOLS {
            return Err(Error::Rejected);
        }
        Ok(code)
    }

    /// The printed form, two groups of four symbols.
    #[must_use]
    pub fn display(&self) -> Zeroizing<String> {
        let mut text = Zeroizing::new(String::with_capacity(SYMBOLS + 1));
        for index in 0..SYMBOLS {
            if index == 4 {
                text.push('-');
            }
            let mut value = 0;
            for bit in 0..5 {
                let position = index * 5 + bit;
                value = value << 1 | usize::from(self.0[position / 8] >> (7 - position % 8) & 1);
            }
            text.push(char::from(ALPHABET[value]));
        }
        text
    }

    /// What Central stores of a code: SHA-256 over a label, the account's id
    /// and the code's 40 bits, so that one account's digest says nothing of
    /// another's.
    #[must_use]
    pub fn digest(&self, account: &[u8; 16]) -> [u8; 32] {
        let mut hash = Sha256::new();
        hash.update(b"baylee-central mail code v1");
        hash.update(account);
        hash.update(self.0);
        hash.finalize().into()
    }
}

impl Serialize for MailCode {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.display())
    }
}

impl<'de> Deserialize<'de> for MailCode {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = Zeroizing::new(String::deserialize(deserializer)?);
        Self::parse(&text).map_err(|_| serde::de::Error::custom("invalid mail code"))
    }
}

/// What a signed-in account asks about its own address
/// ([`crate::wire::Request::Email`]). Every answer is the
/// [`EmailState`] after it. No Debug: a request holds an address or a code.
#[derive(Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum EmailRequest {
    /// The account's address and what waits for its code.
    Status,
    /// Name an address ([`address`]). Central mails a code to it and keeps
    /// it apart until [`Self::Confirm`]; a confirmed address stays in use
    /// until then. A change of who can reset the login, so it needs a
    /// password sign-in less than 30 minutes old. A new address replaces
    /// one that waited. `Conflict` when it is the account's address
    /// already. `Busy` when no mail could be handed over or the account
    /// asked too often: nothing was sent, and nothing new waits.
    Set {
        /// The address, as it will be written to.
        address: String,
        /// The language of the mail ([`language`]).
        lang: String,
    },
    /// Mail a new code to the address that waits; the earlier code ends.
    /// `Rejected` when none waits, `Busy` as for [`Self::Set`].
    Resend {
        /// The language of the mail ([`language`]).
        lang: String,
    },
    /// Confirm the address that waits with the code mailed to it. It
    /// becomes the account's address, and an address it replaces is told.
    /// A wrong code is `Rejected`; after a few of them the address that
    /// waited is given up and has to be named again.
    Confirm {
        /// The code as typed.
        code: MailCode,
    },
    /// Give up the address that waits; the account's address stays.
    Withdraw,
    /// Remove the account's address and one that waits. The removed
    /// address is told.
    Remove,
}

/// An account's address as Central holds it. Times are Unix seconds.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmailState {
    /// This Central can send mail. Without it an address can be withdrawn
    /// or removed, but none is named, mailed a code or confirmed
    /// (`Rejected`), and [`crate::wire::Request::ResetMail`] is `Rejected`
    /// for every name.
    pub mail: bool,
    /// The confirmed address, if the account has one.
    pub address: Option<String>,
    /// When it was confirmed.
    pub confirmed_at: Option<i64>,
    /// An address that waits for its code.
    pub pending: Option<String>,
    /// When that code ends.
    pub pending_until: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinary_addresses_pass_and_the_mailers_refusals_are_refused() {
        for good in [
            "player@example.org",
            "first.last+tag@mail.example.co.uk",
            "o'brien@example.ie",
            "x@a-b.example",
            "UPPER@Example.ORG",
        ] {
            assert!(address(good).is_ok(), "{good}");
        }
        let long_local = format!("{}@example.org", "a".repeat(65));
        let too_long = format!("{}@{}.org", "a".repeat(60), "b".repeat(200));
        let long_label = format!("a@{}.org", "b".repeat(64));
        for bad in [
            "",
            "plain",
            "@example.org",
            "user@",
            "user@localhost",
            "user@[127.0.0.1]",
            "user@127.0.0.1",
            "\"quoted\"@example.org",
            "a b@example.org",
            ".lead@example.org",
            "trail.@example.org",
            "dou..ble@example.org",
            "user@-bad.example",
            "user@bad-.example",
            "user@exa_mple.org",
            "user@example..org",
            "user@example.org.",
            "üser@example.org",
            "user@exämple.org",
            "user@example.org\r\nBcc: victim@example.org",
            "Name <user@example.org>",
            "a@b@example.org",
            long_local.as_str(),
            too_long.as_str(),
            long_label.as_str(),
        ] {
            assert!(address(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn a_language_is_a_short_lower_case_tag() {
        for good in ["en", "de", "pt-br", "zh-hans", "zh-hant"] {
            assert!(language(good).is_ok(), "{good}");
        }
        for bad in [
            "",
            "e",
            "EN",
            "en_US",
            "-en",
            "en-",
            "en us",
            "abcdefghijk",
            "é",
        ] {
            assert!(language(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn codes_print_and_read_back() {
        // 0x00..=0x04 as 40 bits in groups of five: the first eight symbols
        // of the reset code's test vector.
        let code = MailCode([0, 1, 2, 3, 4]);
        let printed = code.display();
        assert_eq!(printed.as_str(), "000G-40R4");
        assert_eq!(MailCode::parse(&printed).unwrap().0, code.0);
        assert_eq!(MailCode::parse(" ooog 4or4 ").unwrap().0, code.0);
        for bad in ["", "000G-40R", "000G-40R4-0", "000G-40RU", "000G_40R4"] {
            assert!(MailCode::parse(bad).is_err(), "{bad}");
        }
        assert!(MailCode::parse(&format!("000G-40R4{}", " ".repeat(40))).is_err());
        assert_eq!(format!("{code:?}"), "MailCode([REDACTED])");
        // the digest is the account's own
        assert_ne!(code.digest(&[1; 16]), code.digest(&[2; 16]));
        assert_ne!(
            code.digest(&[1; 16]),
            MailCode([0, 1, 2, 3, 5]).digest(&[1; 16])
        );
    }

    #[test]
    fn a_code_travels_as_its_printed_form_and_differs_each_time() {
        let code = MailCode::generate().unwrap();
        let json = crate::encode(&code).unwrap();
        assert_eq!(
            String::from_utf8(json.clone()).unwrap(),
            format!("\"{}\"", code.display().as_str())
        );
        assert_eq!(crate::decode::<MailCode>(&json).unwrap().0, code.0);
        assert!(crate::decode::<MailCode>(b"\"0\"").is_err());
        assert_ne!(MailCode::generate().unwrap().0, code.0);
    }

    #[test]
    fn requests_are_closed() {
        let set = crate::decode::<EmailRequest>(
            br#"{"action":"set","address":"player@example.org","lang":"de"}"#,
        )
        .unwrap();
        assert!(matches!(set, EmailRequest::Set { .. }));
        assert!(matches!(
            crate::decode::<EmailRequest>(br#"{"action":"confirm","code":"000g-40r4"}"#).unwrap(),
            EmailRequest::Confirm { .. }
        ));
        // (a request without fields takes extra ones, as every such request
        // of the protocol does: serde reads only the tag of a unit variant)
        for bad in [
            br#"{"action":"set","address":"player@example.org"}"#.as_slice(),
            br#"{"action":"set","address":"player@example.org","lang":"de","to":"x@example.org"}"#,
            br#"{"action":"confirm","code":"000g"}"#,
            br#"{"action":"forward"}"#,
        ] {
            assert!(crate::decode::<EmailRequest>(bad).is_err());
        }
    }
}
