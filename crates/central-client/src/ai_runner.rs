//! The AI runner's sign-in (`docs/ai-players.md` §2): a runner token traded
//! for a native session of one AI account it lists. The session is an
//! ordinary one for renewal, ranked requests, admission and the relay, but no
//! password opened it: it holds no master key, so the document, objects, chat
//! keys, the local vault key, a password change, a reset that keeps the vault
//! and the key check all refuse.
use super::{Error, Request, Response, Session, Token, Transport, context};

pub use central_api::ai_runner::{MAX_SESSIONS, TOKEN_PREFIX, parse_token_text, token_text};

/// Trade the runner token `token` for a session of the AI account `account`,
/// sent without a bearer. The answer must name that account; a session
/// answered for any other is ended at once.
///
/// # Errors
/// `Rejected` for every refusal of the token (unknown, revoked, expired, not
/// listing the account, an account that is no AI) and for a mismatched
/// answer; `Busy` past the address's or the account's budget; transport
/// failure.
pub fn ai_runner_session(
    transport: &impl Transport,
    token: &Token,
    account: [u8; 16],
) -> Result<Session, Error> {
    let request = Request::AiRunnerSession {
        token: Token::from_secret_bytes(token.to_secret_bytes()),
        account,
    };
    let Response::AiRunnerSession {
        token,
        identity,
        expires_at,
    } = transport.exchange(&request, None)?
    else {
        return Err(Error::Rejected);
    };
    if identity.account != account
        || expires_at <= 0
        || context(transport.endpoint(), identity).is_err()
    {
        let _ = transport.exchange(&Request::Logout, Some((&token).into()));
        return Err(Error::Rejected);
    }
    Ok(Session::runner(
        transport.endpoint().clone(),
        identity,
        token,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use central_api::{Bearer, Endpoint, SignedToken, wire::Identity};
    use std::cell::RefCell;

    const ACCOUNT: [u8; 16] = [0x01, 0x93, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 7];

    /// Answers the exchange with `answer` and records what was sent.
    struct Central {
        endpoint: Endpoint,
        answer: RefCell<Option<Response>>,
        sent: RefCell<Vec<(String, bool)>>,
    }

    impl Central {
        fn answering(answer: Response) -> Self {
            Self {
                endpoint: Endpoint::live(),
                answer: RefCell::new(Some(answer)),
                sent: RefCell::new(Vec::new()),
            }
        }
    }

    impl Transport for Central {
        fn endpoint(&self) -> &Endpoint {
            &self.endpoint
        }
        fn exchange(&self, req: &Request, bearer: Option<Bearer<'_>>) -> Result<Response, Error> {
            let text = String::from_utf8(central_api::encode(req).unwrap()).unwrap();
            let operation = text.split('"').nth(3).unwrap().to_owned();
            self.sent.borrow_mut().push((operation, bearer.is_some()));
            match req {
                Request::AiRunnerSession { .. } => {
                    self.answer.borrow_mut().take().ok_or(Error::Unavailable)
                }
                Request::Logout => Ok(Response::Done),
                _ => Err(Error::Rejected),
            }
        }
    }

    fn minted(account: [u8; 16], epoch: u64, expires_at: i64) -> Response {
        Response::AiRunnerSession {
            token: SignedToken::new(zeroize::Zeroizing::new("synthetic.runner.session".into()))
                .unwrap(),
            identity: Identity { account, epoch },
            expires_at,
        }
    }

    #[test]
    fn a_runner_session_is_the_account_s_and_has_no_master_key() {
        let central = Central::answering(minted(ACCOUNT, 1, 1_800_001_800));
        let token = Token::generate().unwrap();
        let mut session = ai_runner_session(&central, &token, ACCOUNT).unwrap();
        // sent once, without a bearer, and nothing else
        assert_eq!(
            *central.sent.borrow(),
            vec![("ai_runner_session".to_owned(), false)]
        );
        assert_eq!(session.identity().account, ACCOUNT);
        assert!(session.is_runner());
        // what needs the master key refuses, and asks Central nothing
        assert_eq!(session.chat_identity().err(), Some(Error::Rejected));
        assert_eq!(
            session.local_vault_key(&[0; 32]).err(),
            Some(Error::Rejected)
        );
        assert_eq!(session.refresh(&central), Err(Error::Rejected));
        assert_eq!(session.save(&central, b"{}"), Err(Error::Rejected));
        assert_eq!(session.confirm_restored(&central), Err(Error::Rejected));
        assert_eq!(
            session.read_object(&central, [1; 16], 1).err(),
            Some(Error::Rejected)
        );
        assert_eq!(
            session.write_object(&central, [1; 16], 1, b"x").err(),
            Some(Error::Rejected)
        );
        assert_eq!(
            session.change_password(&central, "synthetic old".into(), "synthetic new".into()),
            Err(Error::Rejected)
        );
        assert_eq!(session.ensure_key_check(&central), Err(Error::Rejected));
        assert_eq!(central.sent.borrow().len(), 1);
        // a runner restores nothing, so it asks for no lobby or Tower token
        assert!(
            session
                .gateway_token(
                    &central,
                    "https://play.example".into(),
                    central_api::wire::GatewayPurpose::Admit
                )
                .is_err()
        );
        assert!(session.tower_token(&central).is_err());
        assert_eq!(central.sent.borrow().len(), 1);
    }

    #[test]
    fn a_session_for_another_account_is_ended_and_refused() {
        let mut other = ACCOUNT;
        other[15] = 8;
        for answer in [
            minted(other, 1, 1_800_001_800),
            minted(ACCOUNT, 1, 0),
            minted(ACCOUNT, 0, 1_800_001_800),
        ] {
            let central = Central::answering(answer);
            let token = Token::generate().unwrap();
            assert_eq!(
                ai_runner_session(&central, &token, ACCOUNT).err(),
                Some(Error::Rejected)
            );
            // the minted session is signed out with its own bearer
            assert_eq!(
                *central.sent.borrow(),
                vec![
                    ("ai_runner_session".to_owned(), false),
                    ("logout".to_owned(), true)
                ]
            );
        }
        // another answer altogether, or a refusal, is passed on as refused
        let central = Central::answering(Response::Done);
        assert_eq!(
            ai_runner_session(&central, &Token::generate().unwrap(), ACCOUNT).err(),
            Some(Error::Rejected)
        );
        let central = Central {
            endpoint: Endpoint::live(),
            answer: RefCell::new(None),
            sent: RefCell::new(Vec::new()),
        };
        assert_eq!(
            ai_runner_session(&central, &Token::generate().unwrap(), ACCOUNT).err(),
            Some(Error::Unavailable)
        );
    }

    #[test]
    fn the_runner_reads_its_token_from_the_text_the_website_shows() {
        let token = Token::generate().unwrap();
        let text = token_text(&token);
        assert!(text.starts_with(TOKEN_PREFIX));
        assert_eq!(parse_token_text(&text).unwrap().digest(), token.digest());
        assert_eq!(MAX_SESSIONS, 4);
    }
}
