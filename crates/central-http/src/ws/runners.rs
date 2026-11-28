//! The AI runners' own budget for their game relays (`docs/ai-players.md`,
//! "More AI seats at once"). A runner's relays all come from one address, a
//! home line it shares with people, so they must not count against that
//! address once they proved whose they are.
//!
//! Before its ticket, a relay is admitted as anyone's socket is: its
//! address's dial and place, the listener's place, a relay slot
//! (`ranked_socket.rs`). Its ticket is Central's: it says an AI account only
//! when Central found, as it issued it, that the session was minted by a live
//! runner token (`IdentityService::relay_runner`). Then [`Seat::runner`]
//! gives the address back its dial and its place, whether the runners'
//! budget takes the socket or not, and counts it there instead:
//!
//! - [`RUNNER_SOCKETS`] relays an AI account, [`RUNNER_RELAYS`] all runners
//!   together, half of the listener's relay slots, so the other half stays
//!   with people;
//! - [`RUNNER_DIALS`] proven dials a minute an AI account;
//! - one relay a seat: a newer one of the same account, match and seat takes
//!   the older one's place, and the older one closes, as the host's newer
//!   socket for a chair replaces its older one. So a runner whose line
//!   changed its address is not refused while its old relays hang
//!   half-open.
//!
//! Past its budget the socket closes; its place at the listener goes when it
//! does.
use super::{PoisonError, Seat};
use central_api::ranked::ai::{MAX_GAMES, MAX_GLOBAL_SEATS};
use std::collections::BTreeMap;
use tokio::sync::oneshot;

/// Relays one AI account's runner holds at once: one for each match it may
/// play (`MAX_GAMES`).
pub(crate) const RUNNER_SOCKETS: usize = MAX_GAMES as usize;
/// Relays all runners together hold at once: one for each AI seat the
/// settings allow (`MAX_GLOBAL_SEATS`).
pub(crate) const RUNNER_RELAYS: usize = MAX_GLOBAL_SEATS as usize;
/// Proven relay dials a minute of one AI account: two whole redials of its
/// relays.
pub(crate) const RUNNER_DIALS: u32 = 2 * MAX_GAMES as u32;

/// A runner's relay: the AI account, the ranked match and the seat it plays.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct RunnerSeat {
    account: [u8; 16],
    game: [u8; 16],
    seat: u8,
}

impl RunnerSeat {
    pub(crate) const fn new(account: [u8; 16], game: [u8; 16], seat: u8) -> Self {
        Self {
            account,
            game,
            seat,
        }
    }

    /// Every relay of `account`, in the table's order.
    const fn first(account: [u8; 16]) -> Self {
        Self::new(account, [0; 16], 0)
    }

    const fn last(account: [u8; 16]) -> Self {
        Self::new(account, [u8::MAX; 16], u8::MAX)
    }
}

/// The runners' relays, by seat: the serial of the socket that holds it and
/// the switch that closes that socket once a newer one takes its place.
#[derive(Default)]
pub(super) struct Runners {
    held: BTreeMap<RunnerSeat, (u64, oneshot::Sender<()>)>,
    serial: u64,
}

/// A seat taken: its serial, the new socket's switch, and the older
/// socket's when it took that one's place.
struct Entered {
    serial: u64,
    replaced: oneshot::Receiver<()>,
    older: Option<oneshot::Sender<()>>,
}

impl Runners {
    /// The relays `account` holds.
    fn of(&self, account: [u8; 16]) -> usize {
        self.held
            .range(RunnerSeat::first(account)..=RunnerSeat::last(account))
            .count()
    }

    /// Take `seat` for a new socket: in another's place when one holds it,
    /// else only while the account and all runners are below their
    /// relays.
    fn enter(&mut self, seat: RunnerSeat) -> Option<Entered> {
        if !self.held.contains_key(&seat)
            && (self.held.len() >= RUNNER_RELAYS || self.of(seat.account) >= RUNNER_SOCKETS)
        {
            return None;
        }
        self.serial += 1;
        let (switch, replaced) = oneshot::channel();
        let older = self
            .held
            .insert(seat, (self.serial, switch))
            .map(|(_, older)| older);
        Some(Entered {
            serial: self.serial,
            replaced,
            older,
        })
    }

    /// The socket that took `seat` by `serial` ends: the seat is free,
    /// unless a newer socket took it meanwhile.
    pub(super) fn release(&mut self, seat: RunnerSeat, serial: u64) {
        if self
            .held
            .get(&seat)
            .is_some_and(|(holder, _)| *holder == serial)
        {
            self.held.remove(&seat);
        }
    }
}

/// When a relay's place among the runners' was taken by a newer socket of
/// its seat; never for any other socket.
pub(crate) struct Replaced(Option<oneshot::Receiver<()>>);

impl Replaced {
    /// For a socket that is no runner's.
    pub(crate) const fn never() -> Self {
        Self(None)
    }

    /// Wait until a newer socket took this one's place. Cancel-safe.
    pub(crate) async fn wait(self) {
        match self.0 {
            Some(replaced) => {
                // A sender dropped without a word is a place given back,
                // which happens only as this socket ends.
                let _ = replaced.await;
            }
            None => std::future::pending().await,
        }
    }
}

impl Seat {
    /// This socket proved it is the relay of `seat`, a runner's, by the
    /// ticket Central issued for the runner's session. Its address gets
    /// back the dial and the place it spent, and the socket is counted in
    /// the runners' budget instead, or refused there (`None`), and then
    /// closes. Only once a socket.
    pub(crate) fn runner(&mut self, seat: RunnerSeat) -> Option<Replaced> {
        if self.runner.is_some() {
            return None;
        }
        let now = std::time::Instant::now();
        // The address pays for no socket that proved it is a runner's,
        // whether the runners' budget then takes it or not.
        if let (Some(client), Some(dialed)) = (self.client, self.dialed.take()) {
            self.shared
                .dials
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .refund(client, dialed, super::DIAL_WINDOW);
        }
        if !self
            .shared
            .runner_dials
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .charge(seat.account, now, RUNNER_DIALS, super::DIAL_WINDOW)
        {
            return None;
        }
        let mut held = self
            .shared
            .held
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let entered = held.runners.enter(seat)?;
        if let Some(client) = self.client.take() {
            held.leave(client);
        }
        drop(held);
        self.runner = Some((seat, entered.serial));
        if let Some(older) = entered.older {
            let _ = older.send(());
        }
        Some(Replaced(Some(entered.replaced)))
    }

    /// The address this socket counts against, if it still does.
    #[cfg(test)]
    pub(crate) const fn address(&self) -> Option<super::Client> {
        self.client
    }
}

#[cfg(test)]
mod tests;
