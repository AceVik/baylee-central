//! Accumulated game time, measured only between consecutive healthy authority
//! observations. All times are milliseconds from the database's own clock.
use central_api::ranked::replica::clock::{ClockEvent, Reason};
use serde::{Deserialize, Serialize};

pub(super) const DECISION_MS: i64 = 180_000;
pub(super) const AWAY_MS: i64 = 60_000;
// The native seat and primary publish at least once each five seconds. A gap
// across three consecutive observations is an outage, never billable catch-up.
const HEARTBEAT_MS: i64 = 15_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum Due {
    Concede(ClockEvent),
    Void,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct State {
    generation: i64,
    certified: bool,
    started: bool,
    waiting_ms: i64,
    // Live executors without primary socket observations can neither name a
    // disconnected player nor keep the private table open indefinitely.
    missing_since: Option<i64>,
    sample: Option<i64>,
    host_until: i64,
    attached: u8,
    seat_until: [i64; 2],
    question: [Option<u64>; 2],
    thinking_ms: [i64; 2],
    question_away_ms: [i64; 2],
    away_ms: [i64; 2],
    both_away_ms: i64,
    visible_present: [bool; 2],
    charged_through: Option<i64>,
    awaiting_pulses: Option<(Due, i64)>,
}

impl State {
    /// Ordinary observers may share a recent row version. A near-boundary
    /// tick still evaluates deadlines before taking this optimization.
    pub(super) fn recent_observation(&self, generation: i64, now: i64) -> bool {
        self.generation == generation
            && self.started
            && self.host_until > now
            && self.awaiting_pulses.is_none()
            && self
                .sample
                .is_some_and(|sample| (0..250).contains(&now.saturating_sub(sample)))
            && self.visible_present == [self.present(0, now), self.present(1, now)]
    }

    pub(super) fn generation(&mut self, generation: i64) {
        if self.generation != generation {
            self.suspend();
            self.generation = generation;
        }
    }

    pub(super) fn suspend(&mut self) {
        self.sample = None;
        self.host_until = 0;
        self.attached = 0;
        self.seat_until = [0; 2];
        self.awaiting_pulses = None;
        self.charged_through = None;
        self.missing_since = None;
    }

    /// Return time observed after the most recent bilateral proof of life.
    /// A disappearing replica cannot spend either player's remaining time.
    pub(super) fn refund_unwitnessed(&mut self, confirmed: i64) {
        self.awaiting_pulses = None;
        let Some(last) = self.charged_through.take() else {
            return;
        };
        let refund = last.saturating_sub(confirmed).max(0);
        for counter in self
            .thinking_ms
            .iter_mut()
            .chain(&mut self.question_away_ms)
            .chain(&mut self.away_ms)
            .chain([&mut self.waiting_ms, &mut self.both_away_ms])
        {
            *counter = counter.saturating_sub(refund).max(0);
        }
    }

    /// Both assigned executors must have pulsed at or after the expiration
    /// was observed. A heartbeat from the surviving host cannot order a loss
    /// while its peer is down, even inside the peer's remaining lease.
    pub(super) fn witnessed(&mut self, due: Option<Due>, now: i64, confirmed: i64) -> Option<Due> {
        if self.awaiting_pulses.is_none() {
            self.awaiting_pulses = due.map(|candidate| (candidate, now));
        }
        self.awaiting_pulses
            .filter(|(_, since)| confirmed >= *since)
            .map(|(candidate, _)| candidate)
    }

    pub(super) fn actionable(&self) -> bool {
        self.started && self.sample.is_some() && self.awaiting_pulses.is_none()
    }

    pub(super) fn certify(&mut self, question: [Option<u64>; 2]) {
        for (seat, token) in question.iter().enumerate() {
            if *token != self.question[seat] {
                self.thinking_ms[seat] = 0;
                self.question_away_ms[seat] = 0;
            }
        }
        self.question = question;
        self.certified = true;
    }

    pub(super) fn seat(&mut self, now: i64, seat: usize, connected: bool) {
        self.seat_until[seat] = if connected {
            now.saturating_add(HEARTBEAT_MS)
        } else {
            0
        };
    }

    pub(super) fn host(&mut self, now: i64, attached: u8) {
        self.host_until = now.saturating_add(HEARTBEAT_MS);
        self.attached = attached;
    }

    fn present(&self, seat: usize, at: i64) -> bool {
        self.attached & (1 << seat) != 0 || self.seat_until[seat] > at
    }

    fn admit(&mut self, now: i64) {
        if self.attached == 3 && self.seat_until.iter().all(|until| *until > now) {
            self.started = true;
        }
    }

    /// Continue without another report only inside the bounded healthy interval.
    /// Split at native lease expirations; even the last millisecond of a valid
    /// lease belongs to the connected player.
    pub(super) fn tick(&mut self, now: i64, healthy: bool) -> Option<Due> {
        if !healthy || !self.certified {
            self.sample = None;
            self.missing_since = None;
            return None;
        }
        if self.host_until <= now {
            self.sample = None;
            let since = *self.missing_since.get_or_insert(now);
            let limit = if self.started { 300_000 } else { AWAY_MS };
            return (now.saturating_sub(since) >= limit).then_some(Due::Void);
        }
        self.missing_since = None;
        let previous = self.sample.replace(now).unwrap_or(now);
        if now < previous {
            self.sample = Some(previous);
            return None;
        }
        if now - previous > HEARTBEAT_MS {
            return None;
        }
        let mut cuts = self.seat_until.map(|until| until.clamp(previous, now));
        cuts.sort_unstable();
        let mut start = previous;
        for end in [cuts[0], cuts[1], now] {
            if end > start {
                self.accrue(start, end - start);
                start = end;
            }
        }
        self.charged_through = Some(now);
        self.admit(now);
        self.visible_present = [self.present(0, now), self.present(1, now)];
        self.due(now)
    }

    pub(super) fn view(&self) -> Option<central_api::ranked::replica::clock::ClockView> {
        if !self.started {
            return None;
        }
        let seconds = |ms: i64| u16::try_from((ms.max(0) + 999) / 1000).unwrap_or(u16::MAX);
        let decision_secs = std::array::from_fn(|seat| {
            (self.visible_present[seat] && self.question[seat].is_some())
                .then(|| seconds(DECISION_MS - self.thinking_ms[seat]))
        });
        let reconnect_secs = std::array::from_fn(|seat| {
            (!self.visible_present[seat]).then(|| {
                let elapsed = if self.visible_present == [false; 2] {
                    self.both_away_ms
                } else {
                    self.away_ms[seat].max(self.question_away_ms[seat])
                };
                seconds(AWAY_MS - elapsed)
            })
        });
        Some(central_api::ranked::replica::clock::ClockView {
            decision_secs,
            reconnect_secs,
            running: self.sample.is_some(),
        })
    }

    fn accrue(&mut self, start: i64, elapsed: i64) {
        if !self.started {
            self.waiting_ms = self.waiting_ms.saturating_add(elapsed);
            return;
        }
        let mut both_away = true;
        for seat in 0..2 {
            if self.present(seat, start) {
                self.away_ms[seat] = 0;
                both_away = false;
                if self.question[seat].is_some() {
                    self.thinking_ms[seat] = self.thinking_ms[seat].saturating_add(elapsed);
                }
            } else {
                self.away_ms[seat] = self.away_ms[seat].saturating_add(elapsed);
                if self.question[seat].is_some() {
                    self.question_away_ms[seat] =
                        self.question_away_ms[seat].saturating_add(elapsed);
                }
            }
        }
        self.both_away_ms = if both_away {
            self.both_away_ms.saturating_add(elapsed)
        } else {
            0
        };
    }

    fn due(&self, now: i64) -> Option<Due> {
        if !self.started {
            return (self.waiting_ms >= AWAY_MS).then_some(Due::Void);
        }
        if !self.present(0, now) && !self.present(1, now) {
            return (self.both_away_ms >= AWAY_MS).then_some(Due::Void);
        }
        let mut earliest = None;
        for seat in [0_u8, 1] {
            let index = usize::from(seat);
            let Some(token) = self.question[index] else {
                continue;
            };
            let (decision, away) = if self.present(index, now) {
                (self.thinking_ms[index] - DECISION_MS, -AWAY_MS)
            } else {
                (
                    -DECISION_MS,
                    self.away_ms[index].max(self.question_away_ms[index]) - AWAY_MS,
                )
            };
            let excess = decision.max(away);
            if excess < 0 {
                continue;
            }
            let reason = if decision >= away {
                Reason::Decision
            } else {
                Reason::Disconnected
            };
            let candidate = Due::Concede(ClockEvent::new(seat, token, reason)?);
            earliest = match earliest {
                Some((old, _)) if old == excess => Some((old, Due::Void)),
                Some((old, _)) if old > excess => earliest,
                _ => Some((excess, candidate)),
            };
        }
        earliest.map(|(_, ruling)| ruling)
    }
}
