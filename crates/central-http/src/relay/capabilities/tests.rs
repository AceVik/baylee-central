use super::*;
use crate::relay::upstream;
use central_api::ranked::replica::{SetupBytes, relay::Reply};

fn scope() -> Scope {
    Scope {
        session: [1; 32],
        fence: Fence {
            match_id: [2; 16],
            generation: 3,
        },
        seat: 0,
        origin: "https://primary.invalid".into(),
    }
}
fn private(c: char) -> Secret {
    Secret::new(std::iter::repeat_n(c, 64).collect()).unwrap()
}
fn play(bearer: Secret) -> Call {
    Call::Play {
        bearer,
        deck: SetupBytes::new(b"{}".to_vec()).unwrap(),
    }
}
fn ticket(handle: Secret, game: Id) -> Call {
    Call::SocketTicket {
        seat_token: handle,
        game,
    }
}
fn response_contains(reply: &Reply, secret: &Secret) -> bool {
    let Reply::Http {
        status: 200,
        body: Some(body),
    } = reply
    else {
        panic!("expected projected HTTP success")
    };
    body.as_slice()
        .windows(secret.expose().len())
        .any(|word| word == secret.expose().as_bytes())
}

#[tokio::test]
async fn relay_admission_is_private_bound_and_reusable() {
    let vault = Vault::default();
    let home = scope();
    let bearer = private('a');
    let handle = vault.admission(&home, bearer.clone(), 120).unwrap();
    let reply = upstream::admitted(handle.clone(), 120).unwrap();
    assert!(response_contains(&reply, &handle));
    assert!(!response_contains(&reply, &bearer));
    assert!(vault.authorize(&home, None, play(bearer.clone())).is_err());
    let mut other_match = home.clone();
    other_match.fence.match_id = [6; 16];
    let mut later = home.clone();
    later.fence.generation += 1;
    for other in [
        other_match,
        later,
        Scope {
            seat: 1,
            ..home.clone()
        },
        Scope {
            session: [9; 32],
            ..home.clone()
        },
        Scope {
            origin: "https://different.invalid".into(),
            ..home.clone()
        },
    ] {
        assert!(vault.authorize(&other, None, play(handle.clone())).is_err());
    }
    let Call::Play {
        bearer: forwarded, ..
    } = vault.authorize(&home, None, play(handle.clone())).unwrap()
    else {
        panic!("wrong route")
    };
    assert_eq!(forwarded, bearer);
    let game = [7; 16];
    assert!(
        vault
            .authorize(&home, Some(game), ticket(handle.clone(), game))
            .is_err()
    );
    let resume = Call::Reclaim {
        bearer: handle.clone(),
        game,
    };
    assert!(vault.authorize(&home, None, resume.clone()).is_err());
    let Call::Reclaim {
        bearer: forwarded, ..
    } = vault.authorize(&home, Some(game), resume).unwrap()
    else {
        panic!("wrong route")
    };
    assert_eq!(forwarded, bearer);
    let next = Scope {
        fence: Fence {
            generation: 4,
            ..home.fence
        },
        ..home.clone()
    };
    vault.admission(&next, private('b'), 120).unwrap();
    assert!(vault.authorize(&home, None, play(handle)).is_err());
}

#[tokio::test]
async fn seat_ticket_is_private_game_bound_and_spent_only_by_its_owner() {
    let vault = Vault::default();
    let home = scope();
    let game = [3; 16];
    let seat = private('b');
    let handle = vault.seat(&home, game, seat.clone()).unwrap();
    let reply = upstream::seated(handle.clone(), game, home.seat).unwrap();
    assert!(response_contains(&reply, &handle));
    assert!(!response_contains(&reply, &seat));
    assert!(
        vault
            .authorize(&home, Some(game), ticket(seat.clone(), game))
            .is_err()
    );
    assert!(
        vault
            .authorize(&home, Some(game), play(handle.clone()))
            .is_err()
    );
    assert!(
        vault
            .authorize(&home, Some([8; 16]), ticket(handle.clone(), [8; 16]))
            .is_err()
    );
    let other = Scope {
        seat: 1,
        ..home.clone()
    };
    assert!(
        vault
            .authorize(&other, Some(game), ticket(handle.clone(), game))
            .is_err()
    );
    let Call::SocketTicket { seat_token, .. } = vault
        .authorize(&home, Some(game), ticket(handle.clone(), game))
        .unwrap()
    else {
        panic!("wrong route")
    };
    assert_eq!(seat_token, seat);
    assert!(
        vault
            .authorize(&home, Some(game), ticket(handle, game))
            .is_err()
    );
}

#[tokio::test]
async fn idle_expiry_clears_handles_and_socket_grants() {
    let vault = Vault::default();
    let scope = scope();
    vault.start().unwrap();
    let issued = vault
        .entries
        .lock()
        .unwrap()
        .mint(
            Instant::now(),
            &scope,
            Kind::Admission,
            private('c'),
            Duration::from_millis(10),
        )
        .unwrap();
    let grant = Grant {
        origin: scope.origin.clone(),
        game: [3; 16],
        seat: 0,
        development: false,
        upstream_ticket: zeroize::Zeroizing::new(private('a').expose().to_owned()),
        runner: None,
        fence: scope.fence,
        authorization: zeroize::Zeroizing::new("synthetic".into()),
    };
    let socket = vault.issue(grant, Duration::from_millis(10)).unwrap();
    tokio::time::timeout(Duration::from_secs(4), async {
        loop {
            {
                let entries = vault.entries.lock().unwrap();
                if entries.handles.is_empty() && entries.sockets.is_empty() {
                    break;
                }
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .unwrap();
    assert!(vault.authorize(&scope, None, play(issued)).is_err());
    assert!(vault.take(socket.expose().as_bytes()).is_err());
}

#[tokio::test]
async fn renewal_keeps_a_bounded_overlap() {
    let vault = Vault::default();
    let home = scope();
    let a = vault.admission(&home, private('a'), 10).unwrap();
    let b = vault.admission(&home, private('b'), 20).unwrap();
    let c = vault.admission(&home, private('c'), 30).unwrap();
    assert!(vault.authorize(&home, None, play(a)).is_err());
    assert!(vault.authorize(&home, None, play(b)).is_ok());
    assert!(vault.authorize(&home, None, play(c)).is_ok());
}
