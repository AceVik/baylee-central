use super::{A, Fixture, Player, Q, Token, api};
use api::table::{self, HostRequest as H, HostResponse as R};
use central_api::{gateway::GatewayMetadata, ranked::SeatProof, wire::GatewayPurpose};
use central_postgres::RankedHostCredential::Registered;
use sea_orm::{ConnectionTrait, DbBackend, Statement, prelude::Uuid};

mod resume;
mod retention;

const HOME: &str = "https://advent-table.example";
const OTHER: &str = "https://advent-other.example";

struct Gateway {
    id: Uuid,
    credential: Token,
}
async fn register(f: &Fixture<'_>, owner: &Player, origin: &str) -> Gateway {
    let credential = Token::generate().unwrap();
    let pulse = Token::generate().unwrap();
    let meta = GatewayMetadata {
        name: "Fixture table".into(),
        version: "1".into(),
        website: format!("{origin}/"),
    };
    let id = f
        .store
        .register_gateway(&owner.session, origin, &meta, &credential, &pulse, false)
        .await
        .unwrap();
    let id = Uuid::from_bytes(id);
    f.db.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "INSERT INTO central_identity.gateway_approvals(gateway_id,enabled,ranked_verified) VALUES($1,true,true)",[id.into()])).await.unwrap();
    Gateway { id, credential }
}

async fn ranked(
    f: &Fixture<'_>,
    home: &Gateway,
    sleeve: &api::Reward,
    mat: &api::Reward,
) -> api::Id {
    let row=f.db.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "INSERT INTO central_identity.ranked_matches(season_id,format,account_a,account_b,commitment_a,commitment_b,host_id,ranked_epoch,origin)
         SELECT s.id,'standard',$1,$2,sha256('test-a'::bytea),sha256('test-b'::bytea),$3,h.ranked_epoch,$4
         FROM central_identity.ranked_seasons s,central_identity.gateway_approvals h WHERE h.gateway_id=$3
         AND s.starts_at<=now() AND (s.ends_at IS NULL OR s.ends_at>now()) RETURNING id",
        [f.alice.account.into(),f.bob.account.into(),home.id.into(),HOME.into()])).await.unwrap().unwrap();
    let id = row.try_get::<Uuid>("", "id").unwrap();
    let match_id = id.into_bytes();
    let equipment = api::Equipment {
        sleeve: Some(sleeve.id),
        playmat: Some(mat.id),
    };
    let present = |look| Q::Present { match_id, look };
    let show = Q::TableLooks { match_id };
    assert!(f.call(&f.observer, show.clone()).await.is_err());
    assert!(f.call(&f.bob, present(equipment)).await.is_err());
    let wrong = api::Equipment {
        sleeve: Some(mat.id),
        playmat: None,
    };
    assert!(f.call(&f.alice, present(wrong)).await.is_err());
    for _ in 0..2 {
        assert!(matches!(
            f.call(&f.alice, present(equipment)).await.unwrap(),
            A::Presented
        ));
    }
    assert!(
        f.call(&f.alice, present(api::Equipment::default()))
            .await
            .is_err()
    );
    assert!(f.call(&f.bob, show.clone()).await.is_err());
    let pool = String::from_utf8(
        central_api::encode(&[central_api::ranked::PoolEntry {
            card: [7; 32],
            copies: 60,
        }])
        .unwrap(),
    )
    .unwrap();
    f.db.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "UPDATE central_identity.ranked_matches SET state='playing',game_id=uuidv7(),claimed_a=true,claimed_b=true,
         start_fingerprint=sha256('synthetic start'::bytea),deck_a=$2::jsonb,deck_b=$2::jsonb WHERE id=$1",
        [id.into(),pool.into()])).await.unwrap();
    assert!(
        matches!(f.call(&f.bob,show.clone()).await.unwrap(),A::TableLooks{seats} if seats[0].sleeve==Some(sleeve.asset)
        && seats[0].playmat==Some(mat.asset) && seats[1]==api::SeatLooks::default())
    );
    let asset = |seat| Q::Asset {
        scope: api::AssetScope::Equipped {
            match_id,
            seat,
            kind: api::Kind::Playmat,
        },
        offset: 0,
        cached: Some(mat.asset.digest),
    };
    assert!(matches!(f.call(&f.bob,asset(0)).await.unwrap(),A::Cached{asset} if asset==mat.asset));
    for seat in [1, 2] {
        assert!(f.call(&f.bob, asset(seat)).await.is_err());
    }
    assert!(f.call(&f.observer, asset(0)).await.is_err());
    f.db.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "UPDATE central_identity.ranked_matches SET state='void',finished_at=clock_timestamp(),deck_a=NULL,deck_b=NULL WHERE id=$1",[id.into()])).await.unwrap();
    assert!(f.call(&f.bob, asset(0)).await.is_err());
    assert!(f.call(&f.alice, show).await.is_err());
    assert!(matches!(
        f.call(
            &f.alice,
            Q::Asset {
                scope: api::AssetScope::Owned { item: mat.id },
                offset: 0,
                cached: Some(mat.asset.digest)
            }
        )
        .await
        .unwrap(),
        A::Cached { .. }
    ));
    match_id
}

fn begin(game: api::Id, origin: &str, look: api::Equipment) -> Q {
    Q::Table {
        request: table::Request::Begin {
            origin: origin.into(),
            game,
            seat: 1,
            look,
        },
    }
}

async fn admission(f: &Fixture<'_>, player: &Player, origin: &str) -> SeatProof {
    let fresh =
        f.db.query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT uuidv7() AS id",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<Uuid>("", "id")
        .unwrap()
        .into_bytes();
    let g = f
        .store
        .gateway_grant(
            &player.session,
            &[origin],
            true,
            GatewayPurpose::Admit,
            fresh,
        )
        .await
        .unwrap();
    SeatProof::from_bytes(g.sid)
}

async fn casual(
    f: &Fixture<'_>,
    game: api::Id,
    home: &Gateway,
    other: &Gateway,
    mat: &api::Reward,
) {
    let look = api::Equipment {
        sleeve: None,
        playmat: Some(mat.id),
    };
    assert!(f.call(&f.alice, begin(game, HOME, look)).await.is_err());
    let alice = admission(f, &f.alice, HOME).await;
    let bob = admission(f, &f.bob, HOME).await;
    assert!(f.call(&f.bob, begin(game, HOME, look)).await.is_err());
    assert!(f.call(&f.alice, begin(game, OTHER, look)).await.is_err());
    let A::Table {
        response: table::Response::Grant { proof },
    } = f.call(&f.alice, begin(game, HOME, look)).await.unwrap()
    else {
        panic!("scoped table invitation")
    };
    let access = |credential, req| f.store.advent_table_host(Registered(credential), req);
    let confirm = |table, seat, admission| H::Confirm {
        game: table,
        seat,
        proof: proof.clone(),
        admission,
    };
    let art = |seat, kind, cached| H::Art {
        game,
        seat,
        proof: proof.clone(),
        kind,
        offset: 0,
        cached,
    };
    let cached = || art(1, api::Kind::Playmat, Some(mat.asset.digest));
    assert!(access(&home.credential, cached()).await.is_err());
    assert!(
        access(&other.credential, confirm(game, 1, alice.clone()))
            .await
            .is_err()
    );
    for invalid in [
        confirm(game, 0, alice.clone()),
        confirm(game, 1, bob),
        confirm(home.id.into_bytes(), 1, alice.clone()),
    ] {
        assert!(access(&home.credential, invalid).await.is_err());
    }
    let valid = confirm(game, 1, alice);
    for _ in 0..2 {
        assert!(
            matches!(access(&home.credential,valid.clone()).await.unwrap(),R::Confirmed{worn,valid_for}
            if worn.event_sleeve.is_none() && worn.event_playmat==Some(mat.asset) && (1..=600).contains(&valid_for))
        );
    }
    f.db.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "UPDATE central_identity.advent_table_looks SET expires_at=clock_timestamp()+interval '90 seconds' WHERE proof=$1",
        [proof.digest().to_vec().into()])).await.unwrap();
    assert!(
        matches!(access(&home.credential,valid.clone()).await.unwrap(), R::Confirmed{valid_for,..} if (1..=90).contains(&valid_for))
    );
    assert!(
        matches!(access(&home.credential,cached()).await.unwrap(),R::Cached{asset} if asset==mat.asset)
    );
    assert!(access(&other.credential, cached()).await.is_err());
    assert!(
        access(&home.credential, art(0, api::Kind::Playmat, None))
            .await
            .is_err()
    );
    assert!(
        access(&home.credential, art(1, api::Kind::Sleeve, None))
            .await
            .is_err()
    );
    f.db.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "UPDATE central_identity.advent_table_looks SET expires_at=clock_timestamp()-interval '1 second' WHERE proof=$1",[proof.digest().to_vec().into()])).await.unwrap();
    assert!(access(&home.credential, cached()).await.is_err());
    assert!(access(&home.credential, valid).await.is_err());
}
pub(super) async fn run(
    f: &Fixture<'_>,
    sleeve: api::Reward,
    mat: api::Reward,
    keys: &super::ServerKeys,
) {
    let home = register(f, &f.alice, HOME).await;
    let other = register(f, &f.bob, OTHER).await;
    let game = ranked(f, &home, &sleeve, &mat).await;
    casual(f, game, &home, &other, &mat).await;
    retention::run(f, &home, &f.alice, &mat).await;
    resume::run(f, game, &home, &other, &sleeve, &mat, keys).await;
}
