//! Report parts being uploaded (`docs/reports.md` §"Intake"): how many at
//! once, how many of one gateway, and how many a gateway may leave
//! unfinished in an hour.
//!
//! An upload holds its part in memory, so only [`UPLOADS`] run at once. A
//! part that was never stored may be sent again, and two connections that
//! keep the upload's pace without ever finishing hold two slots for as
//! long as an upload may take. So one gateway holds at most
//! [`UPLOADS_PER_GATEWAY`] of the slots, and the others stay for other
//! gateways' parts.
//!
//! A slot is taken once the credential, the announcement and the declared
//! length are checked, before the body is read. An upload that ends
//! without its part stored (too slow, a connection that went away, bytes
//! other than announced) is counted for its gateway. Past
//! [`UNFINISHED_PER_HOUR`] in an hour the gateway's uploads are `Busy`
//! until that hour is over. A part that another upload stored meanwhile,
//! and a store that failed, are held against nobody.
//!
//! All of it is in this process's memory: a restart frees every slot (the
//! uploads are gone with it) and forgets the counts. The table holds
//! gateways that authenticated and are uploading or have left an upload
//! unfinished in the last hour, so it is no larger than the approved
//! gateways are many.
#![forbid(unsafe_code)]

use central_api::Error;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, PoisonError},
    time::{Duration, Instant},
};

/// Uploads at once. A part is held once while it is read and, by this
/// code's reading and not by measurement, up to three times while it is
/// stored (the bytes read, the statement's parameter, the driver's
/// message): about 192 MiB for a record of 64 MiB, so about 768 MiB when
/// four of the largest are stored in the same moment. Two gateways can
/// each use their two.
pub(crate) const UPLOADS: usize = 4;

/// Uploads of one gateway at once. The released gateway sends two parts at
/// once (`baylee-gateway`, `report/central.rs`); with one it would be
/// refused by its own other upload.
pub(crate) const UPLOADS_PER_GATEWAY: usize = 2;

/// Uploads a gateway may leave unfinished in an hour. The released gateway
/// sends a part at most five times and a report has two parts, so a report
/// whose every send fails leaves ten: this is three such reports in an
/// hour, and a gateway that was restarted in the middle of its uploads
/// leaves two. A record too large for its line to carry in a minute can
/// be sent five times, so six of those in an hour reach it
/// (`docs/reports.md` says what that costs). A gateway that keeps the
/// pace and never finishes holds its two slots for a quarter of the hour
/// with it (thirty uploads of a minute), where it held them all hour.
pub(crate) const UNFINISHED_PER_HOUR: u32 = 30;

const _: () = assert!(UPLOADS_PER_GATEWAY >= 2 && UPLOADS >= 2 * UPLOADS_PER_GATEWAY);

const HOUR: Duration = Duration::from_secs(3_600);

/// One gateway's uploads: how many run, and how many ended unfinished in
/// the hour that began at `since`.
#[derive(Default)]
struct Gateway {
    running: usize,
    since: Option<Instant>,
    unfinished: u32,
}

#[derive(Default)]
struct State {
    running: usize,
    gateways: BTreeMap<[u8; 16], Gateway>,
}

impl State {
    /// Forget the hours that are over, and the gateways nothing is known of.
    fn settle(&mut self, now: Instant) {
        self.gateways.retain(|_, gateway| {
            if gateway
                .since
                .is_some_and(|since| now.saturating_duration_since(since) >= HOUR)
            {
                gateway.since = None;
                gateway.unfinished = 0;
            }
            gateway.running > 0 || gateway.unfinished > 0
        });
    }
}

/// The uploads of report parts that run, and each gateway's unfinished ones.
#[derive(Clone, Default)]
pub(crate) struct Uploads(Arc<Mutex<State>>);

impl Uploads {
    /// A slot for an upload of `gateway` at `now`.
    ///
    /// # Errors
    /// `Busy` while every slot is taken, while the gateway holds its own,
    /// or once it left too many unfinished in the hour.
    pub(crate) fn begin(&self, gateway: [u8; 16], now: Instant) -> Result<Slot, Error> {
        let mut state = self.0.lock().map_err(|_| Error::Unavailable)?;
        state.settle(now);
        let (running, unfinished) = state
            .gateways
            .get(&gateway)
            .map_or((0, 0), |theirs| (theirs.running, theirs.unfinished));
        if state.running >= UPLOADS
            || running >= UPLOADS_PER_GATEWAY
            || unfinished >= UNFINISHED_PER_HOUR
        {
            return Err(Error::Busy);
        }
        state.running += 1;
        state.gateways.entry(gateway).or_default().running += 1;
        Ok(Slot {
            uploads: self.clone(),
            gateway,
            ended: false,
        })
    }

    /// An upload of `gateway` ended at `now`: its slot is free, and it is
    /// counted unless it `finished`.
    fn end(&self, gateway: [u8; 16], finished: bool, now: Instant) {
        // The counts are whole at every step, so a poisoned lock's are too:
        // a slot is given back whatever happened elsewhere.
        let mut state = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        state.settle(now);
        state.running = state.running.saturating_sub(1);
        if let Some(theirs) = state.gateways.get_mut(&gateway) {
            theirs.running = theirs.running.saturating_sub(1);
            if !finished {
                theirs.since.get_or_insert(now);
                theirs.unfinished = theirs.unfinished.saturating_add(1);
            }
        }
        state.settle(now);
    }
}

/// One running upload. Dropped without [`Slot::ended`] (the connection
/// went away, or the upload's whole time was over), it ends unfinished.
pub(crate) struct Slot {
    uploads: Uploads,
    gateway: [u8; 16],
    ended: bool,
}

impl Slot {
    /// The upload ended at `now`; `finished` unless it is to be counted
    /// for its gateway.
    pub(crate) fn ended(mut self, finished: bool, now: Instant) {
        self.ended = true;
        self.uploads.end(self.gateway, finished, now);
    }
}

impl Drop for Slot {
    fn drop(&mut self) {
        if !self.ended {
            self.uploads.end(self.gateway, false, Instant::now());
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
    fn one_gateway_holds_two_slots_and_another_is_still_served() {
        let now = Instant::now();
        let uploads = Uploads::default();
        let first = uploads.begin(ONE, now).unwrap();
        let _second = uploads.begin(ONE, now).unwrap();
        assert_eq!(uploads.begin(ONE, now).err(), Some(Error::Busy));
        // another gateway's two parts are taken beside them
        let third = uploads.begin(TWO, now).unwrap();
        let _fourth = uploads.begin(TWO, now).unwrap();
        // and then every slot is taken, whoever asks
        assert_eq!(uploads.begin(THREE, now).err(), Some(Error::Busy));
        // a slot that ended is free for whoever comes
        first.ended(true, now);
        let _fifth = uploads.begin(THREE, now).unwrap();
        assert_eq!(uploads.begin(ONE, now).err(), Some(Error::Busy));
        // and a gateway that holds one again takes a second, and no third
        third.ended(true, now);
        let _sixth = uploads.begin(ONE, now).unwrap();
        third_of(&uploads, ONE, now);
    }

    /// `gateway` holds its two: a third is refused.
    fn third_of(uploads: &Uploads, gateway: [u8; 16], now: Instant) {
        assert_eq!(uploads.begin(gateway, now).err(), Some(Error::Busy));
        assert_eq!(
            uploads.0.lock().unwrap().gateways[&gateway].running,
            UPLOADS_PER_GATEWAY
        );
    }

    #[test]
    fn a_gateway_s_unfinished_uploads_are_counted_for_an_hour() {
        let start = Instant::now();
        let uploads = Uploads::default();
        // The released gateway's worst: three reports, two parts each, every
        // part sent five times and never finished.
        for sent in 0..UNFINISHED_PER_HOUR {
            let now = start + Duration::from_secs(u64::from(sent));
            uploads.begin(ONE, now).unwrap().ended(false, now);
        }
        let later = start + Duration::from_secs(60);
        assert_eq!(uploads.begin(ONE, later).err(), Some(Error::Busy));
        // another gateway is not held to it
        uploads.begin(TWO, later).unwrap().ended(true, later);
        // the hour began at the first unfinished one, and ends with it
        let last = start + Duration::from_secs(3_599);
        assert_eq!(uploads.begin(ONE, last).err(), Some(Error::Busy));
        uploads.begin(ONE, start + HOUR).unwrap();
    }

    #[test]
    fn finished_uploads_are_not_counted_however_many() {
        let now = Instant::now();
        let uploads = Uploads::default();
        for _ in 0..10 * UNFINISHED_PER_HOUR {
            uploads.begin(ONE, now).unwrap().ended(true, now);
        }
        uploads.begin(ONE, now).unwrap();
        // nothing is kept of a gateway that has nothing running or unfinished
        uploads.begin(TWO, now).unwrap().ended(true, now);
        assert_eq!(uploads.0.lock().unwrap().gateways.len(), 1);
    }

    #[test]
    fn a_slot_that_is_dropped_is_free_again_and_counted_unfinished() {
        let now = Instant::now();
        let uploads = Uploads::default();
        for _ in 0..UNFINISHED_PER_HOUR {
            // the connection went away: nothing called `ended`
            drop(uploads.begin(ONE, now).unwrap());
        }
        {
            let state = uploads.0.lock().unwrap();
            assert_eq!(state.running, 0);
            assert_eq!(state.gateways[&ONE].unfinished, UNFINISHED_PER_HOUR);
        }
        assert_eq!(uploads.begin(ONE, Instant::now()).err(), Some(Error::Busy));
    }
}
