//! Slots report tokens hold (`docs/reports.md` §"Report tokens"): how many
//! of their requests run at once, all tokens together and each alone. A
//! slot is given back when it is dropped, however what held it ended. In
//! this process's memory, so a restart frees them all; the table holds
//! tokens that authenticated and hold a slot, no more than there are slots.
use central_api::Error;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, PoisonError},
};

/// A report token's calls that ask the store at once, all tokens together:
/// half of its four connections, so that the others' sign-ins and pages are
/// served however long a search takes.
pub(crate) const CALLS: usize = 2;
/// Of them one token's.
pub(crate) const CALLS_PER_TOKEN: usize = 1;
/// Downloads of a report's parts at once, all tokens together. A download
/// holds its part until it is sent: up to 64 MiB, and by this code's reading
/// and not by measurement up to three times that while the store answers,
/// so about 384 MiB with two of the largest.
pub(crate) const DOWNLOADS: usize = 2;
/// Of them one token's.
pub(crate) const DOWNLOADS_PER_TOKEN: usize = 1;

/// The slots held, and how many there are.
#[derive(Clone)]
pub(crate) struct Slots {
    held: Arc<Mutex<BTreeMap<[u8; 16], usize>>>,
    all: usize,
    each: usize,
}

impl Slots {
    /// `all` slots, of which one token holds `each` at most.
    pub(crate) fn new(all: usize, each: usize) -> Self {
        Self {
            held: Arc::default(),
            all,
            each,
        }
    }

    /// A slot for `token`.
    ///
    /// # Errors
    /// `Busy` while every slot is taken, or the token holds its own.
    pub(crate) fn take(&self, token: [u8; 16]) -> Result<Slot, Error> {
        let mut held = self.held.lock().map_err(|_| Error::Unavailable)?;
        let theirs = held.get(&token).copied().unwrap_or(0);
        if theirs >= self.each || held.values().sum::<usize>() >= self.all {
            return Err(Error::Busy);
        }
        held.insert(token, theirs + 1);
        Ok(Slot {
            slots: self.clone(),
            token,
        })
    }

    /// How many are held.
    #[cfg(test)]
    pub(crate) fn held(&self) -> usize {
        self.held.lock().unwrap().values().sum()
    }
}

/// One slot held.
pub(crate) struct Slot {
    slots: Slots,
    token: [u8; 16],
}

impl Drop for Slot {
    fn drop(&mut self) {
        // The counts are whole at every step, so a poisoned lock's are too.
        let mut held = self
            .slots
            .held
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let theirs = held.get(&self.token).copied().unwrap_or(0);
        if theirs > 1 {
            held.insert(self.token, theirs - 1);
        } else {
            held.remove(&self.token);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ONE: [u8; 16] = [1; 16];
    const TWO: [u8; 16] = [2; 16];
    const THREE: [u8; 16] = [3; 16];

    #[test]
    fn a_token_holds_its_own_and_all_together_theirs() {
        let slots = Slots::new(3, 2);
        let first = slots.take(ONE).unwrap();
        let _second = slots.take(ONE).unwrap();
        assert_eq!(slots.take(ONE).err(), Some(Error::Busy));
        let _third = slots.take(TWO).unwrap();
        // every slot is taken, whoever asks
        assert_eq!(slots.take(THREE).err(), Some(Error::Busy));
        assert_eq!(slots.held(), 3);
        drop(first);
        let _fourth = slots.take(THREE).unwrap();
        assert_eq!(slots.take(ONE).err(), Some(Error::Busy));
        // and nothing is kept of a token that holds none
        let slots = Slots::new(CALLS, CALLS_PER_TOKEN);
        drop(slots.take(ONE).unwrap());
        assert!(slots.held.lock().unwrap().is_empty());
        let _one = slots.take(ONE).unwrap();
        assert_eq!(slots.take(ONE).err(), Some(Error::Busy));
        let _two = slots.take(TWO).unwrap();
        assert_eq!(slots.take(THREE).err(), Some(Error::Busy));
    }
}
