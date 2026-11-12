//! An account's optional e-mail address (`docs/email.md`): what a session
//! asks about its own address, and the request for a reset code by mail,
//! which needs no session. A code that came by mail is redeemed as an
//! administrator's is ([`crate::reset`], [`Session::redeem_reset`]).
use super::{Error, Request, Response, Session, Transport};
use central_api::email::{EmailRequest, EmailState, address, language};

impl Session {
    /// Ask about the account's e-mail address, or change it, with the
    /// native session: its status, an address to confirm, a new code, the
    /// code that confirms, giving up the address that waits, removing the
    /// address. The answer is the address as Central holds it afterwards.
    ///
    /// # Errors
    /// `Rejected` for another endpoint, an address or language tag of the
    /// wrong shape (nothing is asked then), an expired session, a wrong
    /// code and an unexpected answer; `Conflict` for the address the
    /// account has already; `Busy` when no mail could be handed over or
    /// the account asked too often; `TermsRequired` until Central's terms
    /// are accepted.
    pub fn email(
        &self,
        transport: &impl Transport,
        request: EmailRequest,
    ) -> Result<EmailState, Error> {
        self.same_endpoint(transport)?;
        match &request {
            EmailRequest::Set { address: to, lang } => {
                address(to)?;
                language(lang)?;
            }
            EmailRequest::Resend { lang } => language(lang)?,
            EmailRequest::Status
            | EmailRequest::Confirm { .. }
            | EmailRequest::Withdraw
            | EmailRequest::Remove => {}
        }
        match transport.exchange(&Request::Email { request }, Some(&self.token))? {
            Response::Email { state } => Ok(state),
            _ => Err(Error::Rejected),
        }
    }
}

/// Ask Central to mail a reset code to the confirmed address of the
/// account with the sign-in name `username`, in the language `lang`. `Ok`
/// says the request was taken and nothing else: it is the same for a name
/// that is nobody's, an account without an address and one a code is
/// mailed for. No session is needed, and none is sent.
///
/// # Errors
/// `Rejected` for a language tag of the wrong shape (nothing is asked
/// then), where Central sends no mail, and for an unexpected answer;
/// `Busy` when this address or Central as a whole asked too often.
pub fn reset_mail(transport: &impl Transport, username: String, lang: String) -> Result<(), Error> {
    language(&lang)?;
    match transport.exchange(&Request::ResetMail { username, lang }, None)? {
        Response::Done => Ok(()),
        _ => Err(Error::Rejected),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use central_api::{Endpoint, Token, email::MailCode, wire::Identity};
    use central_crypto::vault::MasterKey;
    use std::cell::RefCell;

    /// A stand-in Central: one prepared answer for whatever is asked, and
    /// what was asked, with whether a credential came along.
    struct Answering {
        endpoint: Endpoint,
        answer: Result<Vec<u8>, Error>,
        asked: RefCell<Vec<(String, bool)>>,
    }

    impl Answering {
        fn with(answer: Result<&Response, Error>) -> Self {
            Self {
                endpoint: Endpoint::live(),
                answer: answer.map(|answer| central_api::encode(answer).unwrap()),
                asked: RefCell::default(),
            }
        }

        fn asked(&self) -> Vec<(String, bool)> {
            self.asked.borrow().clone()
        }
    }

    impl Transport for Answering {
        fn endpoint(&self) -> &Endpoint {
            &self.endpoint
        }
        fn exchange(&self, request: &Request, bearer: Option<&Token>) -> Result<Response, Error> {
            let text = String::from_utf8(central_api::encode(request)?).unwrap();
            self.asked.borrow_mut().push((text, bearer.is_some()));
            central_api::decode(self.answer.as_ref().map_err(|error| *error)?)
        }
    }

    fn session(endpoint: Endpoint) -> Session {
        Session::new(
            endpoint,
            Identity {
                account: [7; 16],
                epoch: 1,
            },
            Token::generate().unwrap(),
            MasterKey::generate().unwrap(),
        )
    }

    fn state() -> EmailState {
        EmailState {
            mail: true,
            address: Some("player@example.org".into()),
            confirmed_at: Some(1_794_000_000),
            pending: Some("other@example.org".into()),
            pending_until: Some(1_794_000_900),
        }
    }

    #[test]
    fn each_action_is_asked_with_the_session_and_answered_with_the_state() {
        let central = Answering::with(Ok(&Response::Email { state: state() }));
        let session = session(Endpoint::live());
        let code = MailCode::generate().unwrap();
        let typed = code.display().to_string();
        let asked = [
            (EmailRequest::Status, r#"{"action":"status"}"#.to_owned()),
            (
                EmailRequest::Set {
                    address: "other@example.org".into(),
                    lang: "pt-br".into(),
                },
                r#"{"action":"set","address":"other@example.org","lang":"pt-br"}"#.to_owned(),
            ),
            (
                EmailRequest::Resend { lang: "de".into() },
                r#"{"action":"resend","lang":"de"}"#.to_owned(),
            ),
            (
                EmailRequest::Confirm { code },
                format!(r#"{{"action":"confirm","code":"{typed}"}}"#),
            ),
            (
                EmailRequest::Withdraw,
                r#"{"action":"withdraw"}"#.to_owned(),
            ),
            (EmailRequest::Remove, r#"{"action":"remove"}"#.to_owned()),
        ];
        let mut expected = Vec::new();
        for (request, body) in asked {
            assert_eq!(session.email(&central, request), Ok(state()));
            expected.push((format!(r#"{{"operation":"email","request":{body}}}"#), true));
        }
        assert_eq!(central.asked(), expected);
    }

    #[test]
    fn a_malformed_address_or_language_is_refused_before_anything_is_asked() {
        let central = Answering::with(Ok(&Response::Email { state: state() }));
        let session = session(Endpoint::live());
        let set = |address: &str, lang: &str| EmailRequest::Set {
            address: address.into(),
            lang: lang.into(),
        };
        for request in [
            set("no address", "en"),
            set("player@example.org\r\nBcc: other@example.org", "en"),
            set("player@example.org", "EN"),
            set("player@example.org", "en/../x"),
            EmailRequest::Resend {
                lang: String::new(),
            },
        ] {
            assert_eq!(session.email(&central, request), Err(Error::Rejected));
        }
        assert_eq!(central.asked(), Vec::new());
    }

    #[test]
    fn another_endpoint_and_another_answer_are_refused_and_errors_passed_on() {
        // A session of the development endpoint asks nothing of the live one.
        let central = Answering::with(Ok(&Response::Email { state: state() }));
        let elsewhere = session(Endpoint::development("http://127.0.0.1:28780").unwrap());
        assert_eq!(
            elsewhere.email(&central, EmailRequest::Status),
            Err(Error::Rejected)
        );
        assert_eq!(central.asked(), Vec::new());
        // `Done` is no state.
        let central = Answering::with(Ok(&Response::Done));
        let session = session(Endpoint::live());
        assert_eq!(
            session.email(&central, EmailRequest::Withdraw),
            Err(Error::Rejected)
        );
        for error in [
            Error::Busy,
            Error::Conflict,
            Error::TermsRequired,
            Error::Unavailable,
        ] {
            let central = Answering::with(Err(error));
            assert_eq!(session.email(&central, EmailRequest::Status), Err(error));
        }
    }

    #[test]
    fn a_reset_by_mail_is_asked_without_a_session_and_says_only_that_it_was_taken() {
        let central = Answering::with(Ok(&Response::Done));
        assert_eq!(
            reset_mail(&central, "Player.One".into(), "de".into()),
            Ok(())
        );
        assert_eq!(
            central.asked(),
            vec![(
                r#"{"operation":"reset_mail","username":"Player.One","lang":"de"}"#.to_owned(),
                false
            )]
        );
        // A tag of the wrong shape is not sent; a name is Central's to judge.
        for lang in ["", "DE", "de_AT", "x"] {
            assert_eq!(
                reset_mail(&central, "player".into(), lang.into()),
                Err(Error::Rejected)
            );
        }
        assert_eq!(central.asked().len(), 1);
        // Any other answer is refused; a refusal is passed on as it is.
        let central = Answering::with(Ok(&Response::Email { state: state() }));
        assert_eq!(
            reset_mail(&central, "player".into(), "en".into()),
            Err(Error::Rejected)
        );
        for error in [Error::Busy, Error::Rejected, Error::Unavailable] {
            let central = Answering::with(Err(error));
            assert_eq!(
                reset_mail(&central, "player".into(), "en".into()),
                Err(error)
            );
        }
    }
}
