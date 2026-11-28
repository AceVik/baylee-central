//! The AI runner's exchange (`docs/ai-players.md` §2): a runner token,
//! presented in the body on the native listener with no `Authorization`
//! header, for a native session of one AI account it lists, answered with
//! the same signed bearer a sign-in yields. The administration's side, the
//! making, listing and revoking of tokens, is in `admin.rs`.
use super::{BrowserReply, Error, IdentityService, Response, Surface, Token, limits::Limits};
use std::sync::Mutex;
use std::time::Instant;

/// An exchange the account's hourly budget counts while the store answers
/// it; it ends when this goes, also when the caller stops waiting, and
/// spends the hour only when it minted a session.
struct InFlight<'a> {
    limits: &'a Mutex<Limits>,
    account: [u8; 16],
    minted: bool,
}

impl Drop for InFlight<'_> {
    fn drop(&mut self) {
        if let Ok(mut limits) = self.limits.lock() {
            limits.runner_end(self.account, Instant::now(), self.minted);
        }
    }
}

impl IdentityService {
    /// Trade `token` for a session of `account`. The account's hourly
    /// budget counts the exchange from before the store is asked until it
    /// ends, and keeps it only when it minted a session, so exchanges sent
    /// at once cannot pass the budget together and a stranger's made-up
    /// tokens cannot use up an AI's. The sessions the exchange ended are
    /// told to their push sockets, as a sign-out's are.
    pub(super) async fn ai_runner_session(
        &self,
        token: &Token,
        account: [u8; 16],
        bearer: Option<&Token>,
        surface: Surface,
    ) -> Result<BrowserReply, Error> {
        // Neither a session nor a credential comes with it, and the website
        // never sends one.
        if surface != Surface::Native || bearer.is_some() {
            return Err(Error::Rejected);
        }
        self.limits
            .lock()
            .map_err(|_| Error::Unavailable)?
            .runner_begin(account, Instant::now())?;
        let mut flight = InFlight {
            limits: &self.limits,
            account,
            minted: false,
        };
        let session = Token::generate()?;
        let minted = self
            .store
            .ai_runner_session(token, account, &session)
            .await?;
        flight.minted = true;
        drop(flight);
        self.issued(&session);
        for digest in &minted.ended {
            self.live.ended(digest);
        }
        self.sessions_changed(&minted.identity.account);
        Ok(BrowserReply {
            response: Response::AiRunnerSession {
                token: self.central_token(&session, minted.identity.account)?,
                identity: minted.identity,
                expires_at: minted.expires_at,
            },
            session: None,
            refresh: None,
            clear_cookies: false,
        })
    }

    /// The AI account whose runner holds the native session `session`, or
    /// `None` for any other session, a person's among them: the store's word,
    /// read when Central's relay issues a socket ticket for the session and
    /// kept with it, so that a runner's relay sockets are counted in a budget
    /// of their own (`central-http`'s `ws.rs`), never by anything the client
    /// says.
    ///
    /// # Errors
    /// Storage failure.
    pub async fn relay_runner(&self, session: &Token) -> Result<Option<[u8; 16]>, Error> {
        self.store.relay_runner(session).await
    }
}
