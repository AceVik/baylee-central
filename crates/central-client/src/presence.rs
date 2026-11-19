//! The account's status as its friends see it (`docs/push-socket.md`
//! §"Status"): what the person chose, what the client is doing, and how to
//! read a push socket's `presence` frame.
//!
//! The client tells Central when it sits down at a table or gets up, when a
//! game begins and ends (online or against the house AI), and when it has
//! been idle for ten minutes and when it is back. Watching a game and building
//! a deck are neither: online. Nothing else can be said.
use super::{Session, Transport};
use central_api::{
    Error,
    presence::{Activity, Choice, Shown},
    wire::{Request, Response},
};

impl Session {
    /// Choose how the account is shown to its friends; stored at Central and
    /// told to every socket of the account. Returns the stored choice.
    ///
    /// # Errors
    /// `TermsRequired` until Central's terms are accepted, `Busy` past ten
    /// changes a minute, `Rejected` for an ended session, transport failures.
    pub fn set_presence(
        &self,
        transport: &impl Transport,
        choice: Choice,
    ) -> Result<Choice, Error> {
        self.same_endpoint(transport)?;
        match transport.exchange(&Request::SetPresence { choice }, Some((&self.token).into()))? {
            Response::Presence { choice } => Ok(choice),
            _ => Err(Error::Rejected),
        }
    }

    /// Say what the client is doing now: playing, at a table, or neither,
    /// and whether it has been idle for ten minutes. Heard only while this
    /// session's push socket is open; send it again after each dial. Central
    /// answers alike whether it heard it or not.
    ///
    /// # Errors
    /// Transport failures and a refusal of the session's request.
    pub fn report_presence(
        &self,
        transport: &impl Transport,
        activity: Option<Activity>,
        idle: bool,
    ) -> Result<(), Error> {
        self.same_endpoint(transport)?;
        match transport.exchange(
            &Request::ReportPresence { activity, idle },
            Some((&self.token).into()),
        )? {
            Response::Done => Ok(()),
            _ => Err(Error::Rejected),
        }
    }
}

/// What a `presence` frame says: the friends shown online, each with its
/// state, and the account's own choice. `states` and `me` are absent from a
/// Central before the status; read as [`Shown::ONLINE`] and no choice.
#[must_use]
pub fn read_presence<'a>(
    online: &'a [String],
    states: Option<&str>,
    me: Option<&str>,
) -> (Vec<(&'a str, Shown)>, Option<Choice>) {
    (Shown::read(online, states), me.and_then(Choice::named))
}
