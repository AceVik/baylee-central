//! Real PostgreSQL adversarial expedition accounting; synthetic packages, no engine claims.
use super::*;
use central_api::tower;

mod edges;
mod play;

async fn call(
    store: &AccountStore,
    token: Option<&Token>,
    request: tower::Request,
) -> Result<tower::Response, Error> {
    let Response::Tower { response } = public(store, token, Request::Tower { request }).await?
    else {
        panic!("tower reply");
    };
    Ok(*response)
}

async fn mutate(store: &AccountStore, token: &Token, request: tower::Request) -> tower::Run {
    let tower::Response::Run { run } = call(store, Some(token), request).await.unwrap() else {
        panic!("run reply");
    };
    *run
}

async fn read_run(store: &AccountStore, token: &Token, run: [u8; 16]) -> tower::Run {
    mutate(store, token, tower::Request::Run { run }).await
}

async fn publish_tower(
    store: &AccountStore,
    request: tower::HostRequest,
) -> Result<Vec<tower::Stock>, Error> {
    let HostResponse::Tower { stock } = host(store, HostRequest::Tower { request }).await? else {
        panic!("tower stock");
    };
    Ok(stock)
}

fn candidates(now: i64) -> (tower::Manifest, Vec<Publication>) {
    let nodes: Vec<_> = (0..tower::NODES)
        .map(|index| {
            let mut node = publication(Kind::Weekly, now);
            node.package.extend(index.to_le_bytes());
            node.digest = node.calculated_digest().unwrap();
            node
        })
        .collect();
    let theme = tower::Theme {
        period: Period::containing(Kind::Weekly, now).unwrap(),
        title: words("Synthetic expedition"),
        floors: std::array::from_fn(|i| words(&format!("Floor {}", i + 1))),
        upgrades: std::array::from_fn(|_| {
            std::array::from_fn(|_| tower::Upgrade {
                title: words("Synthetic improvement"),
                summary: words("Replace two cards."),
                copies: 2,
                remove: "Shock".into(),
                add: "Lightning Bolt".into(),
            })
        }),
    };
    (
        tower::Manifest {
            theme,
            nodes: nodes.iter().map(|node| node.digest).collect(),
        },
        nodes,
    )
}

async fn staging(store: &AccountStore, now: i64) -> [u8; 16] {
    let (manifest, nodes) = candidates(now);
    let stage = |index: usize| tower::HostRequest::Stage {
        manifest: Box::new(manifest.clone()),
        node: u8::try_from(index).unwrap(),
        publication: Box::new(nodes[index].clone()),
    };
    let stock = publish_tower(store, stage(0)).await.unwrap().remove(0);
    assert_eq!(stock.staged, [0]);
    assert!(!stock.published);
    let id = stock.id;
    assert!(matches!(
        call(
            store,
            None,
            tower::Request::Read {
                season: Some(id),
                board: tower::Board::Ascent,
                offset: 0,
            }
        )
        .await,
        Err(Error::Rejected)
    ));
    assert!(matches!(
        publish_tower(store, tower::HostRequest::Activate { season: id }).await,
        Err(Error::Conflict)
    ));
    let mut changed = manifest.clone();
    changed.theme.title = words("A conflicting expedition");
    assert!(matches!(
        publish_tower(
            store,
            tower::HostRequest::Stage {
                manifest: Box::new(changed),
                node: 0,
                publication: Box::new(nodes[0].clone())
            }
        )
        .await,
        Err(Error::Conflict)
    ));
    for index in 1..tower::NODES {
        publish_tower(store, stage(index)).await.unwrap();
    }
    let ready = publish_tower(store, tower::HostRequest::Activate { season: id })
        .await
        .unwrap()
        .remove(0);
    assert!(ready.published);
    assert_eq!(ready.staged.len(), tower::NODES);
    assert!(
        publish_tower(store, tower::HostRequest::Activate { season: id })
            .await
            .unwrap()[0]
            .published
    );
    assert_eq!(publish_tower(store, stage(0)).await.unwrap()[0].id, id);
    id
}

pub(super) async fn run(
    store: &AccountStore,
    operator: &DatabaseConnection,
    keys: &ServerKeys,
    now: i64,
) {
    let season = staging(store, now).await;
    let alice = enroll(store, keys, "tower-test-a", "tower-test-a").await;
    let bob = enroll(store, keys, "tower-test-b", "tower-test-b").await;
    let a = login(store, keys, alice).await;
    let b = login(store, keys, bob).await;
    let request = tower::Request::Begin {
        season,
        request_id: id(),
        listed: true,
    };
    let (first, retry) = tokio::join!(
        mutate(store, &a, request.clone()),
        mutate(store, &a, request)
    );
    assert_eq!(first.id, retry.id);
    assert_eq!(first.progress.lives, 3);
    assert!(matches!(
        call(store, Some(&b), tower::Request::Run { run: first.id }).await,
        Err(Error::Rejected)
    ));
    assert!(matches!(
        call(
            store,
            Some(&a),
            tower::Request::Begin {
                season,
                request_id: id(),
                listed: true
            }
        )
        .await,
        Err(Error::Conflict)
    ));
    // A guessed private node cannot open a separate hand through Daily/Weekly endpoints.
    for request in [
        Request::Begin {
            challenge: first.encounter.id,
            request_id: id(),
            listed: true,
        },
        Request::Hint {
            challenge: first.encounter.id,
            level: 1,
        },
        Request::Detail {
            challenge: first.encounter.id,
            board: Board::FirstWin,
            offset: 0,
        },
    ] {
        assert!(matches!(
            public(store, Some(&a), request).await,
            Err(Error::Rejected)
        ));
    }
    let completed = play::ascent(store, &a, &b, alice, bob, first).await;
    edges::run(store, operator, &a, &b, alice, season, completed).await;
}
