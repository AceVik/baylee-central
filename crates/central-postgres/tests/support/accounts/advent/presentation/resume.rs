//! A new ordinary admission must re-confirm the last worn outfit for this chair.
use super::super::{A, ServerKeys, login};
use super::{
    Fixture, Gateway, H, HOME, OTHER, Player, Q, R, Registered, SeatProof, admission, api, begin,
    table,
};
use central_postgres::AccountStore;

fn resume(origin: &str, game: api::Id, seat: u8) -> Q {
    Q::Table {
        request: table::Request::Resume {
            origin: origin.into(),
            game,
            seat,
        },
    }
}
fn grant(response: A) -> table::Proof {
    let A::Table {
        response: table::Response::Grant { proof },
    } = response
    else {
        panic!("fresh presentation grant")
    };
    proof
}
fn empty(response: &A) {
    assert!(matches!(
        response,
        A::Table {
            response: table::Response::NoLook
        }
    ));
}

async fn confirm(
    store: &AccountStore,
    node: &Gateway,
    game: api::Id,
    proof: table::Proof,
    admission: SeatProof,
) -> Result<R, central_api::Error> {
    store
        .advent_table_host(
            Registered(&node.credential),
            H::Confirm {
                game,
                seat: 1,
                proof,
                admission,
            },
        )
        .await
}
async fn art(
    store: &AccountStore,
    node: &Gateway,
    game: api::Id,
    proof: &table::Proof,
    digest: api::Digest,
) -> Result<R, central_api::Error> {
    store
        .advent_table_host(
            Registered(&node.credential),
            H::Art {
                game,
                seat: 1,
                proof: proof.clone(),
                kind: api::Kind::Playmat,
                offset: 0,
                cached: Some(digest),
            },
        )
        .await
}
async fn fresh(f: &Fixture<'_>, keys: &ServerKeys) -> Player {
    let old = &f.alice;
    let session = login(f.store, keys, old.identity).await;
    f.store.confirm_restore(&session, 1).await.unwrap();
    Player {
        account: old.account,
        identity: old.identity,
        session,
    }
}

pub(super) async fn run(
    f: &Fixture<'_>,
    game: api::Id,
    home: &Gateway,
    other: &Gateway,
    sleeve: &api::Reward,
    mat: &api::Reward,
    keys: &ServerKeys,
) {
    let original = api::Equipment {
        sleeve: None,
        playmat: Some(mat.id),
    };
    let transient = api::Equipment {
        sleeve: Some(sleeve.id),
        playmat: None,
    };
    let old = admission(f, &f.alice, HOME).await;
    let proof = grant(f.call(&f.alice, begin(game, HOME, original)).await.unwrap());
    confirm(f.store, home, game, proof.clone(), old.clone())
        .await
        .unwrap();
    assert!(
        art(f.store, home, game, &proof, mat.asset.digest)
            .await
            .is_ok()
    );
    // A candidate in the deck editor, even offered for another seat, has never been worn there.
    let candidate = Q::Table {
        request: table::Request::Begin {
            origin: HOME.into(),
            game,
            seat: 2,
            look: transient,
        },
    };
    f.call(&f.alice, candidate).await.unwrap();
    let player = fresh(f, keys).await;
    assert!(
        f.inventory(&player)
            .await
            .iter()
            .any(|owned| owned.id == mat.id && owned.asset == mat.asset)
    );
    assert!(
        art(f.store, home, game, &proof, mat.asset.digest)
            .await
            .is_err()
    );
    assert!(f.call(&f.alice, resume(HOME, game, 1)).await.is_err());
    let valid = admission(f, &player, HOME).await;
    admission(f, &player, OTHER).await;
    for r in [
        resume(OTHER, game, 1),
        resume(HOME, game, 2),
        resume(HOME, home.id.into_bytes(), 1),
    ] {
        empty(&f.call(&player, r).await.unwrap());
    }
    empty(&f.call(&f.bob, resume(HOME, game, 1)).await.unwrap());
    let grant = grant(f.call(&player, resume(HOME, game, 1)).await.unwrap());
    assert!(
        confirm(f.store, home, game, grant.clone(), old)
            .await
            .is_err()
    );
    assert!(
        art(f.store, home, game, &grant, mat.asset.digest)
            .await
            .is_err()
    );
    assert!(
        matches!(confirm(f.store,home,game,grant.clone(),valid).await.unwrap(),R::Confirmed{worn,..}
        if worn.event_playmat==Some(mat.asset) && worn.event_sleeve.is_none())
    );
    f.store
        .advent_table_host(Registered(&other.credential), H::Closed { game })
        .await
        .unwrap();
    assert!(
        art(f.store, home, game, &grant, mat.asset.digest)
            .await
            .is_ok()
    );
    for _ in 0..2 {
        assert!(matches!(
            f.store
                .advent_table_host(Registered(&home.credential), H::Closed { game })
                .await
                .unwrap(),
            R::Closed
        ));
    }
    assert!(
        art(f.store, home, game, &grant, mat.asset.digest)
            .await
            .is_err()
    );
    empty(&f.call(&player, resume(HOME, game, 1)).await.unwrap());
}
