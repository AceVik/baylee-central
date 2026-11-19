use super::*;
use central_api::presence::Activity;
use tokio::sync::Notify;

const A: [u8; 16] = [1; 16];
const B: [u8; 16] = [2; 16];
const C: [u8; 16] = [3; 16];

fn digest(seed: u8) -> [u8; 32] {
    [seed; 32]
}

/// A socket of `account` (tag = its first byte), its friends read as `friends`
/// if it is the account's first.
fn socket(live: &Arc<Live>, account: [u8; 16], session: u8, friends: &[[u8; 16]]) -> Listener {
    let (listener, reading) = live
        .admit(
            account,
            Some(u32::from(account[0])),
            Choice::Online,
            digest(session),
        )
        .unwrap();
    if let Some(reading) = reading {
        live.friends_read(reading, friends.to_vec());
    }
    // what admission raised is not what the tests look at
    let _ = listener.take();
    listener
}

#[test]
fn news_reaches_exactly_the_accounts_it_names() {
    let live = Arc::new(Live::default());
    let a = [socket(&live, A, 1, &[]), socket(&live, A, 1, &[])];
    let b = socket(&live, B, 2, &[]);
    live.raise(&A, news::CHAT);
    assert!(a.iter().all(|a| a.take() == news::CHAT));
    assert_eq!(b.take(), 0, "B hears nothing of A's delivery");
    live.raise(&B, news::SOCIAL);
    assert_eq!(b.take(), news::SOCIAL);
    assert!(a.iter().all(|a| a.take() == 0));
    // an account without a socket is told nothing, and nothing breaks
    live.raise(&C, news::CHAT | news::SOCIAL);
    assert!(a.iter().all(|a| a.take() == 0));
    assert_eq!(b.take(), 0);
}

#[test]
fn news_faster_than_a_socket_writes_is_merged_not_queued() {
    let live = Arc::new(Live::default());
    let a = socket(&live, A, 1, &[]);
    for _ in 0..100_000 {
        live.raise(&A, news::CHAT);
    }
    live.raise(&A, news::SOCIAL);
    assert_eq!(a.take(), news::CHAT | news::SOCIAL);
    assert_eq!(a.take(), 0, "taken once, nothing left behind");
    // The doorbell keeps a ring that came while nobody waited, once.
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap();
    live.raise(&A, news::CHAT);
    live.raise(&A, news::CHAT);
    runtime.block_on(async {
        tokio::time::timeout(Duration::from_secs(1), a.rung())
            .await
            .expect("a ring raised before the wait is kept");
        assert!(
            tokio::time::timeout(Duration::from_millis(50), a.rung())
                .await
                .is_err(),
            "and kept once"
        );
    });
    assert_eq!(a.take(), news::CHAT);
}

#[test]
fn presence_is_friends_with_a_socket_that_said_ready() {
    let live = Arc::new(Live::default());
    let a = socket(&live, A, 1, &[B]);
    let c = socket(&live, C, 3, &[]);
    let presence = |listener: &Listener| live.presence(&listener.account().unwrap()).online;
    // B admitted but not ready: not online, nobody woken
    let b1 = socket(&live, B, 2, &[A]);
    assert_eq!(presence(&a), Vec::<u32>::new());
    assert_eq!(a.take(), 0);
    b1.ready();
    assert_eq!(a.take(), news::PRESENCE, "B's friend A is told");
    assert_eq!(c.take(), 0, "C, no friend of B's, is not woken");
    assert_eq!(presence(&a), vec![2]);
    assert_eq!(presence(&c), Vec::<u32>::new());
    // a second socket of B changes nothing for anybody
    let b2 = socket(&live, B, 2, &[A]);
    b2.ready();
    assert_eq!(a.take(), 0);
    drop(b1);
    assert_eq!(a.take(), 0, "B is still online");
    assert_eq!(presence(&a), vec![2]);
    drop(b2);
    assert_eq!(a.take(), news::PRESENCE, "B went offline");
    assert_eq!(presence(&a), Vec::<u32>::new());
    assert_eq!(c.take(), 0);
    // C going online is nothing to A, who is no friend of C's
    c.ready();
    assert_eq!(a.take(), 0);
    assert_eq!(presence(&a), Vec::<u32>::new());
    // A itself going online wakes nobody who does not count A a friend
    a.ready();
    assert_eq!(c.take(), 0);
    assert_eq!(live.online(), 2);
    drop((a, c));
    assert_eq!(live.online(), 0);
    assert!(live.state().accounts.is_empty() && live.state().sockets.is_empty());
}

#[test]
fn an_ended_friendship_leaves_both_presences_when_read_again() {
    let live = Arc::new(Live::default());
    let a = socket(&live, A, 1, &[B]);
    let b = socket(&live, B, 2, &[A]);
    a.ready();
    b.ready();
    let _ = (a.take(), b.take());
    assert_eq!(live.presence(&A).online, vec![2]);
    // the friendship ended: both lists are read again
    let (for_a, for_b) = (live.reread(A).unwrap(), live.reread(B).unwrap());
    live.friends_read(for_a, vec![]);
    assert_eq!(live.presence(&A).online, Vec::<u32>::new());
    assert_eq!(a.take(), news::PRESENCE);
    assert_eq!(
        b.take(),
        news::PRESENCE,
        "B left A's list, so B looks again"
    );
    live.friends_read(for_b, vec![]);
    assert_eq!(live.presence(&B).online, Vec::<u32>::new());
    // nobody without a socket is read
    assert!(live.reread(C).is_none());
}

#[test]
fn an_older_reading_never_undoes_a_newer_one() {
    let live = Arc::new(Live::default());
    let a = socket(&live, A, 1, &[]);
    let b = socket(&live, B, 2, &[A]);
    b.ready();
    let older = live.reread(A).unwrap();
    let newer = live.reread(A).unwrap();
    live.friends_read(newer, vec![B]);
    live.friends_read(older, vec![]);
    assert_eq!(
        live.presence(&A).online,
        vec![2],
        "the newer reading stands"
    );
    // a failed reading only counts when it is the newest
    live.friends_unread(older);
    assert_eq!(a.take() & news::STALE, 0);
    let newest = live.reread(A).unwrap();
    live.friends_unread(newest);
    assert_eq!(a.take() & news::STALE, news::STALE);
    assert_eq!(b.take() & news::STALE, 0, "only A's sockets close");
}

#[test]
fn an_account_holds_four_sockets_and_a_session_dials_twelve_times_a_minute() {
    let live = Arc::new(Live::default());
    let mut held: Vec<Listener> = (0..ACCOUNT_SOCKETS)
        .map(|_| socket(&live, A, 1, &[]))
        .collect();
    assert!(matches!(
        live.admit(A, Some(1), Choice::Online, digest(1)),
        Err(Error::Busy)
    ));
    // another account is not held to A's
    let b = socket(&live, B, 2, &[]);
    held.pop();
    assert!(live.admit(A, Some(1), Choice::Online, digest(1)).is_ok());
    drop(b);
    let now = Instant::now();
    for _ in 0..SESSION_DIALS {
        live.dial(digest(1), now).unwrap();
    }
    assert_eq!(live.dial(digest(1), now), Err(Error::Busy));
    assert_eq!(live.dial(digest(2), now), Ok(()), "per session");
    assert_eq!(live.dial(digest(1), now + MINUTE), Ok(()), "a new minute");
}

#[test]
fn a_full_dial_table_takes_a_newcomer_in_the_oldest_s_place() {
    let live = Live::default();
    let now = Instant::now();
    let session = |seed: usize| {
        let mut session = [0; 32];
        session[..8].copy_from_slice(&u64::try_from(seed).unwrap().to_be_bytes());
        session
    };
    // the oldest first, then the rest a moment later
    live.dial(session(0), now).unwrap();
    for seed in 1..DIALERS {
        live.dial(session(seed), now + Duration::from_millis(1))
            .unwrap();
    }
    // Full of live minutes: a newcomer is let in, never refused for the table,
    // and the oldest entry gives way (its count starts again).
    let later = now + Duration::from_millis(2);
    assert_eq!(live.dial([0xff; 32], later), Ok(()));
    assert_eq!(live.state().dials.len(), DIALERS);
    assert!(!live.state().dials.contains_key(&session(0)));
    // a session it holds keeps its count
    for _ in 1..SESSION_DIALS {
        live.dial(session(1), later).unwrap();
    }
    assert_eq!(live.dial(session(1), later), Err(Error::Busy));
    // once the minute has passed, a newcomer finds the passed minutes forgotten
    assert_eq!(
        live.dial([0xfe; 32], now + MINUTE + Duration::from_millis(2)),
        Ok(())
    );
    assert_eq!(live.state().dials.len(), 1);
}

#[test]
fn a_sign_out_revokes_exactly_its_session_s_sockets() {
    let live = Arc::new(Live::default());
    let a = socket(&live, A, 1, &[]);
    let b = socket(&live, B, 2, &[]);
    live.ended(&digest(1));
    assert_eq!(a.take(), news::REVOKED);
    assert_eq!(b.take(), 0);
}

#[test]
fn the_periodic_check_revokes_what_it_asked_about_and_found_ended() {
    let live = Arc::new(Live::default());
    let a = socket(&live, A, 1, &[]);
    let b = socket(&live, B, 2, &[]);
    let asked = live.digests();
    assert_eq!(asked, BTreeSet::from([digest(1), digest(2)]));
    // a socket that opened after the question is not judged by its answer
    let c = socket(&live, C, 3, &[]);
    live.checked(&asked, &BTreeSet::from([digest(2)]));
    assert_eq!(a.take(), news::REVOKED);
    assert_eq!(b.take(), 0);
    assert_eq!(c.take(), 0);
    // a check that failed too often closes everybody, without `revoked`
    live.stale();
    for listener in [&a, &b, &c] {
        assert_eq!(listener.take(), news::STALE);
    }
}

#[test]
fn a_deleted_account_leaves_its_friends_and_a_new_tag_reaches_them() {
    let live = Arc::new(Live::default());
    let a = socket(&live, A, 1, &[B]);
    let b = socket(&live, B, 2, &[A]);
    let c = socket(&live, C, 3, &[]);
    a.ready();
    b.ready();
    c.ready();
    let _ = (a.take(), b.take(), c.take());
    live.retagged(&A, 0x1234);
    assert_eq!(a.take(), news::SOCIAL);
    assert_eq!(b.take(), news::SOCIAL | news::PRESENCE);
    assert_eq!(c.take(), 0);
    assert_eq!(live.presence(&B).online, vec![0x1234]);
    live.deleted(&A);
    assert_eq!(a.take(), news::CHECK, "its own sockets check their session");
    assert_eq!(b.take(), news::SOCIAL | news::PRESENCE);
    assert_eq!(c.take(), 0);
    assert_eq!(
        live.presence(&B).online,
        Vec::<u32>::new(),
        "forgotten at once"
    );
}

#[test]
fn a_listener_shows_nothing_of_its_session_or_account() {
    let live = Arc::new(Live::default());
    let a = socket(&live, A, 0xab, &[]);
    let shown = format!("{a:?}");
    assert_eq!(shown, "Listener");
}

#[test]
fn presence_reports_at_most_an_account_s_contacts() {
    let live = Arc::new(Live::default());
    let friends: Vec<[u8; 16]> = (0..=MAX_CONTACTS)
        .map(|index| {
            let mut account = [0; 16];
            account[..8].copy_from_slice(&(u64::try_from(index).unwrap() + 10).to_be_bytes());
            account
        })
        .collect();
    let a = socket(&live, A, 1, &friends);
    // More friends online than an account may have, each its own account.
    let mut online = Vec::new();
    for (index, friend) in friends.iter().enumerate() {
        let (listener, reading) = live
            .admit(
                *friend,
                Some(u32::try_from(index).unwrap() + 0x1000),
                Choice::Online,
                digest(9),
            )
            .unwrap();
        if let Some(reading) = reading {
            live.friends_read(reading, vec![A]);
        }
        listener.ready();
        online.push(listener);
    }
    let shown = live.presence(&a.account().unwrap()).online;
    assert_eq!(shown.len(), MAX_CONTACTS);
    assert!(shown.windows(2).all(|pair| pair[0] < pair[1]));
}

#[test]
fn a_reading_given_up_closes_the_sockets_and_the_next_one_reads_anew() {
    let live = Arc::new(Live::default());
    let a = socket(&live, A, 1, &[B]);
    let b = socket(&live, B, 2, &[A]);
    b.ready();
    let _ = a.take();
    assert_eq!(live.presence(&A).online, vec![2]);
    // A request's future is dropped while it reads A's friends again (its
    // time ran out, its client went away): the reading ends as unread.
    let reading = Pending::new(Arc::clone(&live), live.reread(A).unwrap());
    drop(reading);
    assert_eq!(a.take() & news::STALE, news::STALE, "A's sockets close");
    assert_eq!(b.take() & news::STALE, 0);
    assert_eq!(
        live.presence(&A).online,
        Vec::<u32>::new(),
        "nothing is shown of a list that may be wrong"
    );
    // a socket of A's that opens while the old one is still closing reads
    // the list anew instead of taking it as read
    let (again, reading) = live.admit(A, Some(1), Choice::Online, digest(1)).unwrap();
    let reading = Pending::new(Arc::clone(&live), reading.expect("a new reading"));
    reading.read(vec![B]);
    assert_eq!(live.presence(&A).online, vec![2]);
    let _ = again.take();
    drop((a, again));
    // a reading that was read is not undone by its guard
    let c = socket(&live, C, 3, &[]);
    let reading = Pending::new(Arc::clone(&live), live.reread(C).unwrap());
    reading.read(vec![]);
    assert_eq!(c.take() & news::STALE, 0);
}

#[test]
fn a_reading_from_before_a_reconnect_never_takes_a_later_one_s_place() {
    let live = Arc::new(Live::default());
    let b = socket(&live, B, 2, &[A]);
    b.ready();
    let a = socket(&live, A, 1, &[B]);
    let slow = live.reread(A).unwrap();
    // A's sockets close and open again: a new entry, new readings
    drop(a);
    let (again, first) = live.admit(A, Some(1), Choice::Online, digest(1)).unwrap();
    live.friends_read(first.unwrap(), vec![B]);
    let newer = live.reread(A).unwrap();
    live.friends_read(newer, vec![B]);
    assert_eq!(live.presence(&A).online, vec![2]);
    // The reading begun before the reconnect ends now, with what was true
    // then; numbered over all accounts, it is older than every later one.
    assert_ne!(slow, newer);
    live.friends_read(slow, vec![]);
    assert_eq!(
        live.presence(&A).online,
        vec![2],
        "the newer reading stands"
    );
    live.friends_unread(slow);
    assert_eq!(
        again.take() & news::STALE,
        0,
        "and its failure closes nothing"
    );
}

#[test]
fn a_new_tag_reaches_friends_of_an_account_without_a_socket() {
    let live = Arc::new(Live::default());
    // B counts A a friend; A holds no socket
    let b = socket(&live, B, 2, &[A]);
    live.retagged(&A, 0x4321);
    assert_eq!(b.take(), news::SOCIAL | news::PRESENCE);
}

#[test]
fn news_that_a_session_ended_is_counted_for_a_dial_in_flight() {
    let live = Arc::new(Live::default());
    let start = live.ends();
    // nobody holds a socket yet: such news reaches no socket, but is counted
    live.ended(&digest(1));
    assert_eq!(live.ends(), start + 1);
    live.raise(&A, news::CHECK);
    assert_eq!(live.ends(), start + 2);
    live.deleted(&B);
    assert_eq!(live.ends(), start + 3);
    // other news is not, a choice taken neither; and none of it is a choice
    live.raise(&A, news::CHAT | news::SOCIAL | news::PRESENCE);
    live.chose(&A, Choice::Offline);
    assert_eq!(live.ends(), start + 3);
    assert_eq!(live.settled(), 0);
}

/// A socket of `account` whose stored choice is `choice`.
fn choosing(live: &Arc<Live>, account: [u8; 16], choice: Choice, friends: &[[u8; 16]]) -> Listener {
    let (listener, reading) = live
        .admit(
            account,
            Some(u32::from(account[0])),
            choice,
            digest(account[0]),
        )
        .unwrap();
    if let Some(reading) = reading {
        live.friends_read(reading, friends.to_vec());
    }
    let _ = listener.take();
    listener
}

const ONLINE: Shown = Shown::ONLINE;

#[test]
fn an_account_that_chose_offline_is_never_seen_and_only_its_choice_changes_that() {
    // after a restart: an empty registry, the choice as the store has it
    let live = Arc::new(Live::default());
    let b = socket(&live, B, 2, &[A]);
    b.ready();
    let a = choosing(&live, A, Choice::Offline, &[B]);
    a.ready();
    assert_eq!(
        b.take(),
        0,
        "no frame, not even a wake-up, when it connects"
    );
    assert_eq!(live.presence(&B).online, Vec::<u32>::new());
    // its own sockets know what it chose
    assert_eq!(live.presence(&A).me, Choice::Offline);
    assert_eq!(live.presence(&A).online, vec![2], "and see its friends");
    // a second device (its dial read the store's choice), what it does, its
    // idling, a device going: nothing
    let second = choosing(&live, A, Choice::Offline, &[B]);
    second.ready();
    live.report(digest(1), Some(Activity::Playing), true, Instant::now());
    drop(second);
    assert_eq!(b.take(), 0);
    assert_eq!(live.presence(&B).online, Vec::<u32>::new());
    // the choice is the one change friends see: online, then offline again
    live.chose(&A, Choice::Online);
    assert_eq!(b.take(), news::PRESENCE);
    assert_eq!(live.presence(&B).online, vec![1]);
    assert_eq!(
        a.take(),
        news::PRESENCE,
        "its own sockets report the choice"
    );
    live.chose(&A, Choice::Offline);
    assert_eq!(b.take(), news::PRESENCE);
    assert_eq!(live.presence(&B).online, Vec::<u32>::new());
    // and going while offline is nothing either
    drop(a);
    assert_eq!(b.take(), 0);
}

#[test]
fn away_and_activity_are_shown_and_only_changes_reach_friends() {
    let live = Arc::new(Live::default());
    let b = socket(&live, B, 2, &[A]);
    b.ready();
    let a = choosing(&live, A, Choice::Online, &[B]);
    a.ready();
    let _ = b.take();
    let shown = |live: &Live| live.presence(&B).states;
    assert_eq!(shown(&live), vec![ONLINE]);
    live.report(digest(1), Some(Activity::Table), false, Instant::now());
    assert_eq!(b.take(), news::PRESENCE);
    assert_eq!(
        shown(&live),
        vec![Shown {
            away: false,
            activity: Some(Activity::Table)
        }]
    );
    // the same again changes nothing, and wakes nobody
    live.report(digest(1), Some(Activity::Table), false, Instant::now());
    assert_eq!(b.take(), 0);
    // idle, or a chosen afk, is away
    live.report(digest(1), Some(Activity::Playing), true, Instant::now());
    assert_eq!(
        shown(&live),
        vec![Shown {
            away: true,
            activity: Some(Activity::Playing)
        }]
    );
    live.report(digest(1), None, false, Instant::now());
    live.chose(&A, Choice::Afk);
    assert_eq!(
        shown(&live),
        vec![Shown {
            away: true,
            activity: None
        }]
    );
    // what A does is no news to A's own sockets
    let _ = b.take();
    assert_eq!(a.take() & news::PRESENCE, news::PRESENCE, "the choice is");
    live.report(digest(1), Some(Activity::Table), false, Instant::now());
    assert_eq!(a.take(), 0);
}

#[test]
fn a_report_without_a_socket_changes_nothing_and_keeps_nothing() {
    let live = Arc::new(Live::default());
    let b = socket(&live, B, 2, &[A]);
    b.ready();
    live.report(digest(1), Some(Activity::Playing), true, Instant::now());
    assert!(!live.state().accounts.contains_key(&A));
    assert!(
        live.state()
            .sockets
            .values()
            .all(|socket| socket.report.is_none())
    );
    assert_eq!(b.take(), 0);
    // a choice of an account without a socket keeps nothing either
    live.chose(&A, Choice::Offline);
    assert!(!live.state().accounts.contains_key(&A));
}

#[test]
fn a_report_is_charged_before_anything_is_looked_at() {
    let live = Arc::new(Live::default());
    let now = Instant::now();
    // a session without a socket spends its reports all the same
    for _ in 0..SESSION_REPORTS {
        live.report(digest(1), Some(Activity::Table), false, now);
    }
    let b = socket(&live, B, 2, &[A]);
    b.ready();
    let a = choosing(&live, A, Choice::Online, &[B]);
    a.ready();
    let _ = b.take();
    live.report(digest(1), Some(Activity::Playing), false, now);
    assert_eq!(
        live.presence(&B).states,
        vec![ONLINE],
        "the minute's are spent"
    );
    assert_eq!(b.take(), 0);
}

#[test]
fn a_device_s_report_goes_with_it() {
    let live = Arc::new(Live::default());
    let b = socket(&live, B, 2, &[A]);
    b.ready();
    let first = choosing(&live, A, Choice::Online, &[B]);
    first.ready();
    let (second, _) = live.admit(A, Some(1), Choice::Online, digest(77)).unwrap();
    second.ready();
    // the first device plays while idle, the second says nothing
    live.report(digest(1), Some(Activity::Playing), true, Instant::now());
    let _ = b.take();
    assert_eq!(
        live.presence(&B).states,
        vec![Shown {
            away: true,
            activity: Some(Activity::Playing)
        }]
    );
    // the first device goes: what it said goes with it
    drop(first);
    assert_eq!(b.take(), news::PRESENCE);
    assert_eq!(live.presence(&B).states, vec![ONLINE]);
    // and an older report of a device that stays stands again
    let (third, _) = live.admit(A, Some(1), Choice::Online, digest(78)).unwrap();
    third.ready();
    live.report(digest(77), Some(Activity::Table), false, Instant::now());
    live.report(digest(78), Some(Activity::Playing), false, Instant::now());
    drop(third);
    assert_eq!(
        live.presence(&B).states,
        vec![Shown {
            away: false,
            activity: Some(Activity::Table)
        }]
    );
    drop(second);
}

#[test]
fn a_session_s_reports_and_choices_are_budgeted() {
    let live = Arc::new(Live::default());
    let a = choosing(&live, A, Choice::Online, &[]);
    a.ready();
    let now = Instant::now();
    for _ in 0..SESSION_REPORTS {
        live.report(digest(1), None, false, now);
    }
    // past the budget a report is not heard
    let b = socket(&live, B, 2, &[A]);
    b.ready();
    let _ = b.take();
    live.report(digest(1), Some(Activity::Playing), false, now);
    assert_eq!(live.presence(&B).states, vec![ONLINE]);
    // a new minute
    live.report(digest(1), Some(Activity::Playing), false, now + MINUTE);
    assert_eq!(
        live.presence(&B).states,
        vec![Shown {
            away: false,
            activity: Some(Activity::Playing)
        }]
    );
    for _ in 0..SESSION_CHOICES {
        live.choose(digest(1), now).unwrap();
    }
    assert_eq!(live.choose(digest(1), now), Err(Error::Busy));
    assert_eq!(live.choose(digest(2), now), Ok(()), "per session");
}

#[test]
fn the_last_report_of_an_account_s_devices_stands() {
    let live = Arc::new(Live::default());
    let b = socket(&live, B, 2, &[A]);
    b.ready();
    let first = choosing(&live, A, Choice::Online, &[B]);
    first.ready();
    // a second device with a session of its own
    let (second, _) = live.admit(A, Some(1), Choice::Online, digest(77)).unwrap();
    second.ready();
    live.report(digest(1), Some(Activity::Table), false, Instant::now());
    live.report(digest(77), Some(Activity::Playing), false, Instant::now());
    assert_eq!(
        live.presence(&B).states,
        vec![Shown {
            away: false,
            activity: Some(Activity::Playing)
        }]
    );
    // the account's last socket gone, what it did is gone with it
    drop((first, second));
    assert!(!live.state().accounts.contains_key(&A));
    let again = choosing(&live, A, Choice::Online, &[B]);
    again.ready();
    assert_eq!(live.presence(&B).states, vec![ONLINE]);
}

/// A store that holds one account's choice, in the tests of the order in
/// which choices are taken.
type Stored = Arc<Mutex<Choice>>;

fn stored(stored: &Stored) -> Choice {
    *stored.lock().unwrap()
}

/// A write of `choice` for A as the store does it: committed as soon as it
/// runs, its answer then held until `answer` is notified.
fn write(
    store: &Stored,
    choice: Choice,
    answer: &Arc<Notify>,
) -> impl Future<Output = Result<[u8; 16], Error>> + Send + 'static {
    let store = Arc::clone(store);
    let answer = Arc::clone(answer);
    async move {
        *store.lock().unwrap() = choice;
        answer.notified().await;
        Ok(A)
    }
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap()
}

/// Let the runtime's other tasks run as far as they can.
async fn others() {
    for _ in 0..64 {
        tokio::task::yield_now().await;
    }
}

#[test]
fn a_choice_whose_request_went_away_is_still_taken() {
    let live = Arc::new(Live::default());
    let _a = choosing(&live, A, Choice::Online, &[]);
    let store: Stored = Arc::new(Mutex::new(Choice::Online));
    let answer = Arc::new(Notify::new());
    runtime().block_on(async {
        // the request is dropped while the store answers (its time is up,
        // its client gone); the store has the choice by then
        let request = live.settle(A, Choice::Offline, write(&store, Choice::Offline, &answer));
        assert!(
            tokio::time::timeout(Duration::from_millis(20), request)
                .await
                .is_err()
        );
        answer.notify_one();
        others().await;
    });
    assert_eq!(stored(&store), Choice::Offline);
    assert_eq!(
        live.presence(&A).me,
        Choice::Offline,
        "the registry takes what the store holds"
    );
}

#[test]
fn choices_at_once_are_taken_in_the_order_the_store_wrote_them() {
    let live = Arc::new(Live::default());
    let _a = choosing(&live, A, Choice::Online, &[]);
    let store: Stored = Arc::new(Mutex::new(Choice::Online));
    let (first, second) = (Arc::new(Notify::new()), Arc::new(Notify::new()));
    runtime().block_on(async {
        // the client chooses offline, and while the store answers it the
        // website chooses online; the website's answer comes first
        let client = tokio::spawn({
            let (live, write) = (Arc::clone(&live), write(&store, Choice::Offline, &first));
            async move { live.settle(A, Choice::Offline, write).await }
        });
        others().await;
        let website = tokio::spawn({
            let (live, write) = (Arc::clone(&live), write(&store, Choice::Online, &second));
            async move { live.settle(A, Choice::Online, write).await }
        });
        others().await;
        second.notify_one();
        others().await;
        first.notify_one();
        client.await.unwrap().unwrap();
        website.await.unwrap().unwrap();
    });
    assert_eq!(
        live.presence(&A).me,
        stored(&store),
        "the registry ends where the store does"
    );
}

#[test]
fn a_dial_waits_for_a_choice_under_way_and_admits_with_what_the_store_holds() {
    let live = Arc::new(Live::default());
    let b = socket(&live, B, 2, &[A]);
    b.ready();
    let store: Stored = Arc::new(Mutex::new(Choice::Online));
    let answer = Arc::new(Notify::new());
    // what B's presence showed when A's new socket said `ready`
    let seen: Arc<Mutex<Option<Vec<u32>>>> = Arc::new(Mutex::new(None));
    runtime().block_on(async {
        // a dial counts the settled choices and reads its status: online
        let since = live.settled();
        let read = stored(&store);
        // the account chooses offline; the store has it, its answer is slow
        let chosen = tokio::spawn({
            let (live, write) = (Arc::clone(&live), write(&store, Choice::Offline, &answer));
            async move { live.settle(A, Choice::Offline, write).await }
        });
        others().await;
        assert_eq!(stored(&store), Choice::Offline);
        // the dial admits; it reads again if a choice settled meanwhile
        let dial = tokio::spawn({
            let (live, store, seen) = (Arc::clone(&live), Arc::clone(&store), Arc::clone(&seen));
            async move {
                let again = async { Ok(stored(&store)) };
                let (listener, reading) = live
                    .admit_in_turn(A, Some(1), read, since, digest(1), again)
                    .await
                    .unwrap();
                if let Some(reading) = reading {
                    live.friends_read(reading, vec![B]);
                }
                listener.ready();
                *seen.lock().unwrap() = Some(live.presence(&B).online);
                listener
            }
        });
        others().await;
        answer.notify_one();
        chosen.await.unwrap().unwrap();
        let listener = dial.await.unwrap();
        assert_eq!(live.presence(&A).me, Choice::Offline);
        drop(listener);
    });
    assert_eq!(
        seen.lock().unwrap().clone(),
        Some(Vec::new()),
        "never shown after it chose offline"
    );
}

#[test]
fn a_fresh_reading_replaces_a_standing_entry_s_older_choice() {
    let live = Arc::new(Live::default());
    let b = socket(&live, B, 2, &[A]);
    b.ready();
    // an entry made with a choice the store no longer holds (online)
    let first = choosing(&live, A, Choice::Online, &[B]);
    first.ready();
    let _ = (b.take(), first.take());
    assert_eq!(live.presence(&B).online, vec![1]);
    // a dial reads the store's (offline) in the account's turn
    let since = live.settled();
    let (second, _) = runtime()
        .block_on(
            live.admit_in_turn(A, Some(1), Choice::Offline, since, digest(77), async {
                Ok(Choice::Online)
            }),
        )
        .unwrap();
    assert_eq!(
        live.presence(&B).online,
        Vec::<u32>::new(),
        "its reading wins"
    );
    assert_eq!(b.take(), news::PRESENCE);
    assert_eq!(first.take(), news::PRESENCE, "the standing socket says it");
    assert_eq!(live.presence(&A).me, Choice::Offline);
    drop((first, second));
}

#[test]
fn an_offline_choice_whose_write_failed_is_taken_all_the_same() {
    let live = Arc::new(Live::default());
    let b = socket(&live, B, 2, &[A]);
    b.ready();
    let a = choosing(&live, A, Choice::Online, &[B]);
    a.ready();
    let runtime = runtime();
    // the store may have it or not: hidden is the side to err on
    let failed =
        runtime.block_on(live.settle(A, Choice::Offline, async { Err(Error::Unavailable) }));
    assert_eq!(failed, Err(Error::Unavailable));
    assert_eq!(live.presence(&B).online, Vec::<u32>::new());
    // any other failed choice is left to the next reading
    let failed =
        runtime.block_on(live.settle(A, Choice::Online, async { Err(Error::Unavailable) }));
    assert_eq!(failed, Err(Error::Unavailable));
    assert_eq!(live.presence(&A).me, Choice::Offline);
    // both count as settled, so a dial that read meanwhile reads again
    assert_eq!(live.settled(), 2);
    drop(a);
}

#[test]
fn one_account_s_turn_never_waits_for_another_s_and_a_reading_counts_nothing() {
    let live = Arc::new(Live::default());
    let store: Stored = Arc::new(Mutex::new(Choice::Online));
    let answer = Arc::new(Notify::new());
    runtime().block_on(async {
        // B's choice is under way and its answer slow
        let chosen = tokio::spawn({
            let (live, write) = (Arc::clone(&live), write(&store, Choice::Afk, &answer));
            async move { live.settle(B, Choice::Afk, write).await }
        });
        others().await;
        // A's dial is admitted at once
        let since = live.settled();
        let admitted = tokio::time::timeout(
            Duration::from_millis(200),
            live.admit_in_turn(A, Some(1), Choice::Online, since, digest(1), async {
                Ok(Choice::Online)
            }),
        )
        .await
        .expect("not held up by B's turn")
        .unwrap();
        answer.notify_one();
        chosen.await.unwrap().unwrap();
        // a dial that reads again counts nothing: no other dial reads again
        // for it
        let settled = live.settled();
        let (again, _) = live
            .admit_in_turn(A, Some(1), Choice::Offline, since, digest(9), async {
                Ok(Choice::Online)
            })
            .await
            .unwrap();
        assert_eq!(live.presence(&A).me, Choice::Online, "read again");
        assert_eq!(live.settled(), settled);
        drop((admitted, again));
    });
    // and no turn is kept once nobody holds or awaits it
    assert!(live.turns().is_empty());
}
