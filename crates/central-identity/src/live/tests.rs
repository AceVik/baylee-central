use super::*;

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
        .admit(account, Some(u32::from(account[0])), digest(session))
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
    let presence = |listener: &Listener| live.presence(&listener.account().unwrap());
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
    assert_eq!(live.presence(&A), vec![2]);
    // the friendship ended: both lists are read again
    let (for_a, for_b) = (live.reread(A).unwrap(), live.reread(B).unwrap());
    live.friends_read(for_a, vec![]);
    assert_eq!(live.presence(&A), Vec::<u32>::new());
    assert_eq!(a.take(), news::PRESENCE);
    assert_eq!(
        b.take(),
        news::PRESENCE,
        "B left A's list, so B looks again"
    );
    live.friends_read(for_b, vec![]);
    assert_eq!(live.presence(&B), Vec::<u32>::new());
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
    assert_eq!(live.presence(&A), vec![2], "the newer reading stands");
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
        live.admit(A, Some(1), digest(1)),
        Err(Error::Busy)
    ));
    // another account is not held to A's
    let b = socket(&live, B, 2, &[]);
    held.pop();
    assert!(live.admit(A, Some(1), digest(1)).is_ok());
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
fn the_dial_table_refuses_newcomers_only_while_full_of_live_minutes() {
    let live = Live::default();
    let now = Instant::now();
    for seed in 0..DIALERS {
        let mut session = [0; 32];
        session[..8].copy_from_slice(&u64::try_from(seed).unwrap().to_be_bytes());
        live.dial(session, now).unwrap();
    }
    assert_eq!(live.dial([0xff; 32], now), Err(Error::Busy));
    // a session it holds still dials
    assert_eq!(live.dial([0; 32], now), Ok(()));
    // once the minute has passed, the table makes room
    assert_eq!(live.dial([0xff; 32], now + MINUTE), Ok(()));
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
    assert_eq!(live.presence(&B), vec![0x1234]);
    live.deleted(&A);
    assert_eq!(a.take(), news::CHECK, "its own sockets check their session");
    assert_eq!(b.take(), news::SOCIAL | news::PRESENCE);
    assert_eq!(c.take(), 0);
    assert_eq!(live.presence(&B), Vec::<u32>::new(), "forgotten at once");
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
                digest(9),
            )
            .unwrap();
        if let Some(reading) = reading {
            live.friends_read(reading, vec![A]);
        }
        listener.ready();
        online.push(listener);
    }
    let shown = live.presence(&a.account().unwrap());
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
    assert_eq!(live.presence(&A), vec![2]);
    // A request's future is dropped while it reads A's friends again (its
    // time ran out, its client went away): the reading ends as unread.
    let reading = Pending::new(&live, live.reread(A).unwrap());
    drop(reading);
    assert_eq!(a.take() & news::STALE, news::STALE, "A's sockets close");
    assert_eq!(b.take() & news::STALE, 0);
    assert_eq!(
        live.presence(&A),
        Vec::<u32>::new(),
        "nothing is shown of a list that may be wrong"
    );
    // a socket of A's that opens while the old one is still closing reads
    // the list anew instead of taking it as read
    let (again, reading) = live.admit(A, Some(1), digest(1)).unwrap();
    let reading = Pending::new(&live, reading.expect("a new reading"));
    reading.read(vec![B]);
    assert_eq!(live.presence(&A), vec![2]);
    let _ = again.take();
    drop((a, again));
    // a reading that was read is not undone by its guard
    let c = socket(&live, C, 3, &[]);
    let reading = Pending::new(&live, live.reread(C).unwrap());
    reading.read(vec![]);
    assert_eq!(c.take() & news::STALE, 0);
}
