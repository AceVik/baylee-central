//! The mails Central sends and the client that hands them to the mailer
//! (`docs/email.md`): a code that confirms an address, a reset code, and a
//! notice to an address that was replaced or removed. Central speaks no
//! SMTP. A separate service on the same host, `baylee-mailer`, takes one
//! mail a request on loopback and delivers it; this crate reaches nothing
//! but that address.
//!
//! A mail holds an address and often a code, so nothing here prints one:
//! [`Mail`], [`Token`] and [`Client`] redact their Debug output, and
//! [`Outcome`] says what became of a mail without anything of it.
#![forbid(unsafe_code)]

mod client;
mod locale;
mod render;

pub use client::{Client, Token};
pub use locale::Lang;

use central_api::{email::MailCode, reset_code::ResetCode};
use zeroize::Zeroizing;

/// The longest sign-in name a mail names: the protocol's own bound.
const NAME_CHARS: usize = 24;

/// What became of a mail handed to the mailer. Nothing of the mail or of
/// the mailer's answer is in it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Outcome {
    /// The mailer took it and will deliver it; it is not delivered yet.
    Sent,
    /// A limit or a full queue refused it. Nothing was sent; later may work.
    Busy,
    /// The mailer refuses the address itself. Nothing was sent.
    Rejected,
    /// No answer, or one that says Central or the mailer is set up wrongly.
    /// Nothing is known to be sent.
    Unavailable,
}

/// Which of the mails a [`Mail`] is.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum What {
    /// The code that confirms an address.
    Confirm,
    /// A reset code.
    Reset,
    /// The notice to an address another one replaced.
    Replaced,
    /// The notice to an address that was removed.
    Removed,
}

/// What a mail carries besides its words.
enum Kind {
    Confirm {
        code: Zeroizing<String>,
        minutes: u32,
    },
    Reset {
        code: Zeroizing<String>,
        name: String,
        minutes: u32,
    },
    Replaced,
    Removed,
}

/// One mail to one address. Its words come from this crate; of what a
/// person typed it holds only the address it goes to and, in a reset mail,
/// the account's sign-in name. Debug says which mail it is and nothing more.
pub struct Mail {
    to: String,
    account: [u8; 16],
    lang: Lang,
    kind: Kind,
}

impl std::fmt::Debug for Mail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Mail({:?}, [REDACTED])", self.what())
    }
}

impl Mail {
    /// The mail that carries `code` to `to`, an address that waits to be
    /// confirmed for `account`. `lang` is the tag the request asked in;
    /// `minutes` is how long the code lasts, which the mail says and after
    /// which the mailer no longer sends it.
    #[must_use]
    pub fn confirm(to: &str, account: [u8; 16], lang: &str, code: &MailCode, minutes: u32) -> Self {
        Self::new(
            to,
            account,
            lang,
            Kind::Confirm {
                code: code.display(),
                minutes,
            },
        )
    }

    /// The mail that carries the reset `code` for the account with the
    /// sign-in name `name` to its confirmed address `to`. Of `name` only
    /// the characters a sign-in name is made of are kept.
    #[must_use]
    pub fn reset(
        to: &str,
        account: [u8; 16],
        lang: &str,
        name: &str,
        code: &ResetCode,
        minutes: u32,
    ) -> Self {
        let name = name
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || "_.-".contains(*c))
            .take(NAME_CHARS)
            .collect();
        Self::new(
            to,
            account,
            lang,
            Kind::Reset {
                code: code.display(),
                name,
                minutes,
            },
        )
    }

    /// The notice to `to`, which was the address of `account` until
    /// another address was confirmed in its place.
    #[must_use]
    pub fn replaced(to: &str, account: [u8; 16], lang: &str) -> Self {
        Self::new(to, account, lang, Kind::Replaced)
    }

    /// The notice to `to`, which was the address of `account` until it was
    /// removed.
    #[must_use]
    pub fn removed(to: &str, account: [u8; 16], lang: &str) -> Self {
        Self::new(to, account, lang, Kind::Removed)
    }

    fn new(to: &str, account: [u8; 16], lang: &str, kind: Kind) -> Self {
        Self {
            to: to.to_owned(),
            account,
            lang: Lang::from_tag(lang),
            kind,
        }
    }

    /// The address the mail goes to.
    #[must_use]
    pub fn to(&self) -> &str {
        &self.to
    }

    /// The account the mail is about, which the mailer's limit is counted
    /// for; no mail names it.
    #[must_use]
    pub const fn account(&self) -> [u8; 16] {
        self.account
    }

    /// The language it is written in.
    #[must_use]
    pub const fn lang(&self) -> Lang {
        self.lang
    }

    /// Which mail it is.
    #[must_use]
    pub const fn what(&self) -> What {
        match self.kind {
            Kind::Confirm { .. } => What::Confirm,
            Kind::Reset { .. } => What::Reset,
            Kind::Replaced => What::Replaced,
            Kind::Removed => What::Removed,
        }
    }

    /// The code it carries, as printed; a notice carries none.
    #[must_use]
    pub fn code(&self) -> Option<&str> {
        match &self.kind {
            Kind::Confirm { code, .. } | Kind::Reset { code, .. } => Some(code.as_str()),
            Kind::Replaced | Kind::Removed => None,
        }
    }

    /// The seconds after which the mailer no longer sends the mail: the
    /// life of its code. A notice has none and waits its turn.
    #[must_use]
    pub const fn ttl_secs(&self) -> Option<u32> {
        match self.kind {
            Kind::Confirm { minutes, .. } | Kind::Reset { minutes, .. } => {
                Some(minutes.saturating_mul(60))
            }
            Kind::Replaced | Kind::Removed => None,
        }
    }
}

/// Whatever sends Central's mail. The identity service knows only this; the
/// deployed one is [`Client`], and tests put their own in its place.
pub trait Mailer: Send + Sync {
    /// Hand `mail` over, once, and say what became of it. This blocks for
    /// as long as the mailer takes to answer, a few seconds at most, so it
    /// is called off the asynchronous threads.
    fn send(&self, mail: &Mail) -> Outcome;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mail_says_which_it_is_and_prints_nothing_else() {
        let code = MailCode::generate().unwrap();
        let reset = ResetCode::generate().unwrap();
        let account = [7; 16];
        let mails = [
            Mail::confirm("player@example.org", account, "de", &code, 15),
            Mail::reset(
                "player@example.org",
                account,
                "fr",
                "player.one",
                &reset,
                60,
            ),
            Mail::replaced("player@example.org", account, "pt-br"),
            Mail::removed("player@example.org", account, "klingon"),
        ];
        let what = [What::Confirm, What::Reset, What::Replaced, What::Removed];
        let ttl = [Some(900), Some(3600), None, None];
        let lang = [Lang::De, Lang::Fr, Lang::PtBr, Lang::En];
        for (index, mail) in mails.iter().enumerate() {
            assert_eq!(mail.what(), what[index]);
            assert_eq!(mail.ttl_secs(), ttl[index]);
            assert_eq!(mail.lang(), lang[index]);
            assert_eq!(mail.to(), "player@example.org");
            assert_eq!(mail.account(), account);
            let printed = format!("{mail:?}");
            for secret in [
                "player",
                "example",
                code.display().as_str(),
                reset.display().as_str(),
            ] {
                assert!(!printed.contains(secret), "{printed}");
            }
        }
        assert_eq!(mails[0].code(), Some(code.display().as_str()));
        assert_eq!(mails[1].code(), Some(reset.display().as_str()));
        assert!(mails[2].code().is_none() && mails[3].code().is_none());
    }

    #[test]
    fn a_reset_mail_keeps_only_what_a_sign_in_name_is_made_of() {
        let reset = ResetCode::generate().unwrap();
        let mail = Mail::reset(
            "player@example.org",
            [7; 16],
            "en",
            "pla<b>yer\r\n one&\u{202e}x\u{e9}_.-0123456789abcdefghij",
            &reset,
            60,
        );
        let Kind::Reset { name, .. } = &mail.kind else {
            panic!("a reset mail");
        };
        assert_eq!(name, "plabyeronex_.-0123456789");
        assert_eq!(name.len(), NAME_CHARS);
    }
}
