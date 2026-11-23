use super::policy::{Due, State};
use central_api::ranked::replica::clock::{ClockEvent, Reason};

struct Game {
    clock: State,
    now: i64,
    native: u8,
    attached: u8,
}

impl Game {
    fn new(question: [Option<u64>; 2]) -> Self {
        let mut game = Self {
            clock: State::default(),
            now: 2_000_000,
            native: 3,
            attached: 3,
        };
        game.clock.generation(1);
        game.clock.certify(question);
        game.report();
        assert_eq!(game.clock.tick(game.now, true), None);
        game
    }
    fn report(&mut self) {
        self.clock.host(self.now, self.attached);
        for seat in 0..2 {
            self.clock
                .seat(self.now, seat, self.native & (1 << seat) != 0);
        }
    }
    fn pass(&mut self, mut ms: i64) -> Option<Due> {
        while ms > 0 {
            let step = ms.min(5_000);
            self.now += step;
            if let Some(due) = self.clock.tick(self.now, true) {
                return Some(due);
            }
            self.report();
            self.clock.tick(self.now, true);
            ms -= step;
        }
        None
    }
    fn presence(&mut self, native: u8, attached: u8) {
        self.native = native;
        self.attached = attached;
        self.report();
        self.clock.tick(self.now, true);
    }
}

fn loss(seat: u8, token: u64, reason: Reason) -> Due {
    Due::Concede(ClockEvent::new(seat, token, reason).unwrap())
}

#[test]
fn other_seat_progress_and_repeated_renders_preserve_the_current_question() {
    let mut game = Game::new([Some(7), Some(7)]);
    assert_eq!(game.pass(89_999), None);
    game.clock.certify([Some(7), Some(8)]);
    assert_eq!(game.pass(89_999), None);
    game.clock.certify([Some(7), Some(8)]);
    assert_eq!(game.pass(1), None);
    assert_eq!(game.pass(1), Some(loss(0, 7, Reason::Decision)));
}

#[test]
fn a_new_decision_starts_fresh_and_simultaneous_deadlines_are_neutral() {
    let mut game = Game::new([Some(2), None]);
    assert_eq!(game.pass(179_999), None);
    game.clock.certify([Some(3), Some(4)]);
    assert_eq!(game.pass(179_999), None);
    assert_eq!(game.pass(1), Some(Due::Void));
}

#[test]
fn an_outage_and_fenced_recovery_preserve_the_unspent_decision_time() {
    let mut game = Game::new([None, Some(55)]);
    assert_eq!(game.pass(140_000), None);
    game.clock.suspend();
    game.now += 123_456_789;
    assert_eq!(game.clock.tick(game.now, false), None);
    game.clock.generation(2);
    game.report();
    game.clock.certify([None, Some(55)]);
    assert_eq!(game.clock.tick(game.now, true), None);
    assert_eq!(game.pass(39_999), None);
    assert_eq!(game.pass(1), Some(loss(1, 55, Reason::Decision)));
}

#[test]
fn disconnected_time_accumulates_for_the_same_question_without_shortening_a_fresh_grace() {
    let mut game = Game::new([Some(6), None]);
    assert_eq!(game.pass(179_000), None);
    game.presence(2, 2);
    assert_eq!(game.pass(45_000), None);
    game.presence(3, 3);
    assert_eq!(game.pass(450), None);
    game.presence(2, 2);
    assert_eq!(game.pass(14_999), None);
    assert_eq!(game.pass(1), Some(loss(0, 6, Reason::Disconnected)));
}

#[test]
fn a_single_party_cannot_accelerate_the_other_players_concession() {
    for (native, attached) in [(3, 2), (2, 3)] {
        let mut game = Game::new([Some(9), None]);
        game.presence(native, attached);
        assert_eq!(game.pass(179_999), None);
        assert_eq!(game.pass(1), Some(loss(0, 9, Reason::Decision)));
    }
}

#[test]
fn both_away_voids_and_an_unasked_disconnect_waits_until_a_real_question() {
    let mut game = Game::new([None, Some(3)]);
    game.presence(2, 2);
    assert_eq!(game.pass(75_000), None);
    game.clock.certify([Some(5), None]);
    assert_eq!(
        game.clock.tick(game.now, true),
        Some(loss(0, 5, Reason::Disconnected))
    );
    let mut game = Game::new([Some(4), None]);
    game.presence(0, 0);
    assert_eq!(game.pass(59_999), None);
    assert_eq!(game.pass(1), Some(Due::Void));
}

#[test]
fn missing_admission_voids_without_a_winner_and_late_pulses_never_catch_up() {
    let mut game = Game::new([Some(1), None]);
    game.clock = State::default();
    game.presence(1, 1);
    game.clock.certify([Some(1), None]);
    game.clock.tick(game.now, true);
    assert_eq!(game.pass(59_999), None);
    assert_eq!(game.pass(1), Some(Due::Void));
    let mut game = Game::new([Some(1), None]);
    assert_eq!(game.pass(175_000), None);
    game.now += 90_000;
    assert_eq!(game.clock.tick(game.now, true), None);
    game.report();
    assert_eq!(game.clock.tick(game.now, true), None);
    assert_eq!(game.pass(4999), None);
    assert_eq!(game.pass(1), Some(loss(0, 1, Reason::Decision)));
}

#[test]
fn a_lease_alone_cannot_order_a_loss_and_recovery_refunds_the_unconfirmed_tail() {
    let mut game = Game::new([Some(31), None]);
    assert_eq!(game.pass(170_000), None);
    let last_both = game.now;
    let due = game.pass(10_000);
    assert_eq!(due, Some(loss(0, 31, Reason::Decision)));
    assert_eq!(game.clock.witnessed(due, game.now, last_both), None);
    assert!(!game.clock.actionable());
    assert_eq!(game.clock.witnessed(None, game.now, last_both), None);
    assert_eq!(game.clock.witnessed(due, game.now, game.now - 1), None);
    assert_eq!(game.clock.witnessed(due, game.now, game.now), due);
    game.clock.refund_unwitnessed(last_both);
    game.clock.suspend();
    let frozen = game.clock.view().unwrap();
    assert!(!frozen.running);
    assert_eq!(frozen.decision_secs, [Some(10), None]);
    game.now += 900_000;
    game.report();
    game.clock.tick(game.now, true);
    assert!(game.clock.actionable());
    assert_eq!(game.pass(9_999), None);
    assert_eq!(game.pass(1), due);
}

#[test]
fn absent_primary_socket_observations_cannot_keep_a_healthy_match_alive_forever() {
    let mut clock = State::default();
    clock.certify([Some(3), None]);
    assert_eq!(clock.tick(17, true), None);
    assert_eq!(clock.tick(60_016, true), None);
    assert_eq!(clock.tick(60_017, true), Some(Due::Void));
    let mut game = Game::new([Some(3), None]);
    game.now += 16_000;
    assert_eq!(game.clock.tick(game.now, true), None);
    assert!(!game.clock.view().unwrap().running);
    assert_eq!(game.clock.tick(game.now + 299_999, true), None);
    assert_eq!(game.clock.tick(game.now + 300_000, true), Some(Due::Void));
    assert_eq!(game.clock.tick(game.now + 300_001, false), None);
}

#[test]
fn native_heartbeat_expiration_splits_an_irregular_sample_at_the_exact_millisecond() {
    for cadence in [997, 4_321, 7_531, 12_011] {
        let mut game = Game::new([Some(17), None]);
        let origin = game.now;
        // The primary already saw a disconnect, but the account's last
        // positive observation remains valid for another fifteen seconds.
        game.clock.host(origin, 2);
        while game.now < origin + 74_999 {
            game.now = (game.now + cadence).min(origin + 74_999);
            assert_eq!(game.clock.tick(game.now, true), None);
            game.clock.host(game.now, 2);
        }
        assert_eq!(
            game.clock.tick(origin + 75_000, true),
            Some(loss(0, 17, Reason::Disconnected))
        );
    }
}
