use super::super::{ADDRESS_DIALS, ADDRESS_SOCKETS, Client, Sockets, Upgrades};
use super::*;
use std::time::Duration;

/// The address a runner shares with a person.
fn home() -> Client {
    Client::from(std::net::IpAddr::from([198, 51, 100, 7]))
}

fn account(n: u8) -> [u8; 16] {
    [n; 16]
}

fn seat(account_n: u8, game: u8, seat: u8) -> RunnerSeat {
    RunnerSeat::new(account(account_n), [game; 16], seat)
}

/// A socket admitted from `client` as the listener admits every one.
fn admitted(upgrades: &Upgrades, client: Client) -> Option<Seat> {
    upgrades.admitted(client)
}

/// The sockets `client` still counts.
fn counted(sockets: &Sockets, client: Client) -> u8 {
    sockets.upgrades.counted(client)
}

/// What a person on `client` can still open: places, then dials.
fn room(sockets: &Sockets, client: Client) -> (u8, u32) {
    let mut places = Vec::new();
    while let Some(place) = sockets.upgrades.place(Some(client)) {
        places.push(place);
    }
    let places = u8::try_from(places.len()).unwrap();
    let mut dials = 0;
    while sockets.upgrades.dial(client).is_some() {
        dials += 1;
    }
    (places, dials)
}

/// A runner's proven relays leave its address: a person there keeps every
/// socket and every dial they would have taken. Before their tickets they
/// counted against it as anyone's do.
#[test]
fn a_runner_s_proven_relays_do_not_count_against_its_address() {
    let sockets = Sockets::new();
    let person = admitted(&sockets.upgrades, home()).unwrap();
    let mut relays: Vec<Seat> = (0..16)
        .map(|_| admitted(&sockets.upgrades, home()).unwrap())
        .collect();
    // Before the tickets: seventeen sockets and seventeen dials of the address.
    assert_eq!(counted(&sockets, home()), 17);
    let mut replaced = Vec::new();
    for (game, relay) in (1..).zip(relays.iter_mut()) {
        replaced.push(relay.runner(seat(1, game, 0)).expect("a runner's relay"));
        assert_eq!(relay.address(), None);
    }
    assert_eq!(counted(&sockets, home()), 1, "the person's socket alone");
    // The person on that address has every place and dial but the one of
    // her own socket.
    assert_eq!(
        room(&sockets, home()),
        (ADDRESS_SOCKETS - 1, ADDRESS_DIALS - 1)
    );
    // The listener still counts every socket: each is a descriptor.
    assert_eq!(sockets.upgrades.0.held.lock().unwrap().total, 17);
    drop((person, relays, replaced));
}

/// A socket that proves nothing, or proves a person's ticket, keeps the
/// budget it was admitted under: its dial stays spent, its place goes only
/// when it does.
#[test]
fn a_socket_that_is_no_runner_s_gains_nothing() {
    let sockets = Sockets::new();
    // A person's relay: its ticket names no AI account, so nothing is asked.
    let relay = admitted(&sockets.upgrades, home()).unwrap();
    // A socket whose ticket never came, or was no ticket.
    let unproven = admitted(&sockets.upgrades, home()).unwrap();
    drop(unproven);
    assert_eq!(counted(&sockets, home()), 1);
    assert_eq!(
        room(&sockets, home()),
        (ADDRESS_SOCKETS - 1, ADDRESS_DIALS - 2),
        "both dials stay spent"
    );
    drop(relay);
    assert_eq!(counted(&sockets, home()), 0);
}

/// One AI account holds at most `MAX_GAMES` relays, all runners together at
/// most `MAX_GLOBAL_SEATS`, half of the relay slots. A relay refused there
/// still gives its address back the dial, and its place when it closes.
#[test]
fn the_account_s_and_all_runners_ceilings_hold() {
    assert_eq!((RUNNER_SOCKETS, RUNNER_RELAYS), (16, 32));
    let sockets = Sockets::new();
    let mut held = Vec::new();
    let open = |n: u8, game: u8| {
        let mut relay = admitted(&sockets.upgrades, home()).unwrap();
        let taken = relay.runner(seat(n, game, 0));
        (relay, taken)
    };
    for game in 0..16 {
        let (relay, taken) = open(1, game);
        held.push((relay, taken.unwrap()));
    }
    let (refused, taken) = open(1, 16);
    assert!(taken.is_none(), "the account's seventeenth relay");
    assert_eq!(refused.address(), Some(home()), "counted until it closes");
    drop(refused);
    for game in 0..16 {
        let (relay, taken) = open(2, game);
        held.push((relay, taken.unwrap()));
    }
    let (refused, taken) = open(3, 0);
    assert!(taken.is_none(), "the runners' thirty-third relay");
    drop(refused);
    // One relay closes: another account's next one fits.
    drop(held.pop());
    let (relay, taken) = open(3, 0);
    assert!(taken.is_some());
    // Every refused one gave its dial and its place back.
    assert_eq!(counted(&sockets, home()), 0);
    assert_eq!(room(&sockets, home()), (ADDRESS_SOCKETS, ADDRESS_DIALS));
    drop((relay, taken, held));
    let held = sockets.upgrades.0.held.lock().unwrap();
    assert_eq!((held.total, held.runners.held.len()), (0, 0));
}

/// A newer relay of the same account, match and seat takes the older one's
/// place, and the older one is told to close: a full budget does not refuse
/// a runner whose old relays hang half-open. Another seat of that match is
/// another relay.
#[tokio::test]
async fn a_newer_relay_of_a_seat_replaces_the_older_one() {
    let sockets = Sockets::new();
    let mut full = Vec::new();
    for game in 0..16 {
        let mut relay = admitted(&sockets.upgrades, home()).unwrap();
        let taken = relay.runner(seat(1, game, 1)).unwrap();
        full.push((relay, taken));
    }
    let (old, old_replaced) = full.remove(3);
    let mut new = admitted(&sockets.upgrades, home()).unwrap();
    let new_replaced = new.runner(seat(1, 3, 1)).expect("in the older one's place");
    tokio::time::timeout(Duration::from_secs(1), old_replaced.wait())
        .await
        .expect("the older one is told");
    // The seat's other chair is another relay, and the account is full.
    let mut other = admitted(&sockets.upgrades, home()).unwrap();
    assert!(other.runner(seat(1, 3, 0)).is_none());
    // The older one ends: the newer one keeps its place.
    drop(old);
    {
        let held = sockets.upgrades.0.held.lock().unwrap();
        assert_eq!(held.runners.of(account(1)), 16);
    }
    let mut late = admitted(&sockets.upgrades, home()).unwrap();
    assert!(late.runner(seat(1, 99, 0)).is_none(), "still full");
    // The newer one was not told anything.
    assert!(
        tokio::time::timeout(Duration::from_millis(50), new_replaced.wait())
            .await
            .is_err()
    );
    drop(new);
    let mut next = admitted(&sockets.upgrades, home()).unwrap();
    assert!(next.runner(seat(1, 99, 0)).is_some(), "the place came back");
    drop((full, other, late, next));
}

/// An AI account's proven dials: two whole redials a minute, replacing
/// ones too. Another account's are its own, and the address's are given
/// back whatever the runner's say.
#[test]
fn an_account_dials_two_redials_a_minute() {
    assert_eq!(RUNNER_DIALS, 32);
    let sockets = Sockets::new();
    for _ in 0..RUNNER_DIALS {
        let mut relay = admitted(&sockets.upgrades, home()).unwrap();
        assert!(relay.runner(seat(1, 0, 0)).is_some());
    }
    let mut relay = admitted(&sockets.upgrades, home()).unwrap();
    assert!(relay.runner(seat(1, 0, 0)).is_none());
    drop(relay);
    let mut relay = admitted(&sockets.upgrades, home()).unwrap();
    assert!(relay.runner(seat(2, 0, 0)).is_some());
    drop(relay);
    assert_eq!(room(&sockets, home()), (ADDRESS_SOCKETS, ADDRESS_DIALS));
}

/// Whatever ends a relay gives back what it holds; a socket proves once.
#[test]
fn a_closed_relay_gives_everything_back() {
    let sockets = Sockets::new();
    let mut relay = admitted(&sockets.upgrades, home()).unwrap();
    let taken = relay.runner(seat(1, 0, 0)).unwrap();
    assert!(relay.runner(seat(1, 1, 0)).is_none(), "only once");
    drop((relay, taken));
    // A relay from an address the boundary did not know.
    let mut unknown = sockets.upgrades.place(None).unwrap();
    assert!(unknown.runner(seat(1, 0, 0)).is_some());
    drop(unknown);
    let held = sockets.upgrades.0.held.lock().unwrap();
    assert_eq!(
        (held.total, held.by_client.len(), held.runners.held.len()),
        (0, 0, 0)
    );
}

/// A dial is given back only into the window that spent it.
#[test]
fn a_dial_is_given_back_only_in_its_own_minute() {
    let mut book = crate::browser::Book::default();
    let start = std::time::Instant::now();
    let minute = Duration::from_secs(60);
    assert!(book.charge(home(), start, 1, minute));
    assert!(!book.charge(home(), start, 1, minute));
    book.refund(home(), start, minute);
    assert!(book.charge(home(), start, 1, minute));
    // The next minute's window: a refund of the last one gives nothing.
    let later = start + minute;
    assert!(book.charge(home(), later, 1, minute));
    book.refund(home(), start, minute);
    assert!(!book.charge(home(), later, 1, minute));
}
