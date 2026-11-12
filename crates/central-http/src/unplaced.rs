//! Requests that reach the deployed account listener without the proxy's
//! address (`docs/operations.md` §"Sign-in guesses and address budgets").
//!
//! The listener binds loopback, and the proxy in front of it appends the
//! client it saw to `X-Forwarded-For`. A request without that hop is
//! counted by no address: the budgets everyone shares and each account's
//! own still hold, an address's do not. That is right for a local run and
//! wrong on the deployed listener, where it means a proxy that stopped
//! sending the header or a process on the host that talks to the listener
//! directly. Such requests are served as before; they are counted, and a
//! fixed line says so: the first one, then at most one an hour. The line
//! holds the count and nothing of any request. There is no address to
//! write: its absence is what is counted.
#![forbid(unsafe_code)]

use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

/// The least time between two lines.
const EVERY: Duration = Duration::from_secs(3_600);

/// How many such requests arrived since the start, and when the last
/// line was written.
#[derive(Default)]
pub(crate) struct Unplaced(Mutex<(u64, Option<Instant>)>);

impl Unplaced {
    /// Count one such request at `now`; the line to write, if one is due.
    pub(crate) fn seen(&self, now: Instant) -> Option<String> {
        // A poisoned count is not worth a refusal: the request is served.
        let mut state = self.0.lock().ok()?;
        state.0 = state.0.saturating_add(1);
        let due = state
            .1
            .is_none_or(|last| now.saturating_duration_since(last) >= EVERY);
        if !due {
            return None;
        }
        state.1 = Some(now);
        Some(format!(
            "central: {} request(s) since start reached the account listener without the \
             proxy's address; they are counted by no address (docs/operations.md, \
             \"The proxy must send the client's address\")",
            state.0
        ))
    }

    /// Count one such request and write the line when one is due.
    pub(crate) fn note(&self) {
        if let Some(line) = self.seen(Instant::now()) {
            eprintln!("{line}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_is_reported_and_then_one_an_hour_with_the_count() {
        let start = Instant::now();
        let unplaced = Unplaced::default();
        let first = unplaced.seen(start).expect("the first is reported");
        assert!(first.starts_with("central: 1 request(s) since start"));
        for second in 1..3_600 {
            assert_eq!(unplaced.seen(start + Duration::from_secs(second)), None);
        }
        let next = unplaced
            .seen(start + EVERY)
            .expect("an hour later, the next");
        assert!(next.starts_with("central: 3601 request(s) since start"));
        assert_eq!(unplaced.seen(start + EVERY + Duration::from_secs(1)), None);
        // The line is fixed but for the count: nothing of a request is in it.
        assert_eq!(
            first.replace("central: 1 ", ""),
            next.replace("central: 3601 ", "")
        );
    }
}
