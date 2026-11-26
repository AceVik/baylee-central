//! The AI runner's exchange (`docs/ai-players.md` §2): a runner token,
//! presented in the body on the native listener with no `Authorization`
//! header, for a native session of one AI account it lists, answered with
//! the same signed bearer a sign-in yields. The administration's side, the
//! making, listing and revoking of tokens, is in `admin.rs`.
use super::{BrowserReply, Error, IdentityService, Response, Surface, Token};
use std::time::Instant;

impl IdentityService {
    /// Trade `token` for a session of `account`. The account's hourly
    /// budget is read before the store and spent after a success only, so a
    /// stranger's made-up tokens cannot use up an AI's. The sessions the
    /// exchange ended are told to their push sockets, as a sign-out's are.
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
            .runner_fits(account, Instant::now())?;
        let session = Token::generate()?;
        let minted = self
            .store
            .ai_runner_session(token, account, &session)
            .await?;
        if let Ok(mut limits) = self.limits.lock() {
            limits.runner_exchanged(account, Instant::now());
        }
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
}
