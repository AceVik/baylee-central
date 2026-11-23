//! All calendar mutations traverse the deployed restricted store and independent native sessions.
use super::{AccountStore, DatabaseConnection, Error, ServerKeys, Token, enroll, login};
use central_api::advent::{self as api, Request as Q, Response as A};
use sea_orm::{ConnectionTrait, DbBackend, Statement, prelude::Uuid};

mod calendar;
#[path = "../advent_material.rs"]
mod material;
mod presentation;
mod ranked_collection;

const YEAR: u16 = 2105;
const DAY: i32 = 86_400;

struct Player {
    identity: central_api::wire::Identity,
    account: Uuid,
    session: Token,
}
struct Fixture<'a> {
    store: &'a AccountStore,
    db: &'a DatabaseConnection,
    alice: Player,
    bob: Player,
    observer: Player,
}

async fn player(store: &AccountStore, keys: &ServerKeys, name: &str) -> Player {
    let identity = enroll(store, keys, name, name).await;
    let session = login(store, keys, identity).await;
    store.confirm_restore(&session, 1).await.unwrap();
    store.close_legacy(name, identity).await.unwrap();
    Player {
        account: Uuid::from_bytes(identity.account),
        identity,
        session,
    }
}
impl Fixture<'_> {
    async fn call(&self, player: &Player, q: Q) -> Result<A, Error> {
        self.store.advent((&player.session).into(), q).await
    }
    async fn claim(&self, player: &Player, door: u8) -> api::Reward {
        let A::Reward { reward } = self
            .call(player, Q::Claim { year: YEAR, door })
            .await
            .unwrap()
        else {
            panic!("private reward")
        };
        reward
    }
    async fn inventory(&self, player: &Player) -> Vec<api::Reward> {
        let A::Inventory { items, next } = self
            .call(player, Q::Inventory { after: None })
            .await
            .unwrap()
        else {
            panic!("private inventory")
        };
        assert!(next.is_none());
        items
    }
    async fn grant(&self, player: &Player, door: u8, attackers: u8) {
        let response = self
            .call(
                player,
                Q::Submit {
                    year: YEAR,
                    door,
                    line: material::line(attackers),
                },
            )
            .await
            .unwrap();
        assert!(matches!(response, A::Submission { won: true }));
    }
    async fn asset(&self, day: u8) -> (api::Id, api::Digest) {
        let row=self.db.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT a.id,a.digest FROM central_identity.advent_art a JOIN central_identity.advent_doors d ON a.id=d.art_id
             WHERE d.year=$1 AND d.door=$2",[i16::try_from(YEAR).unwrap().into(),i16::from(day).into()])).await.unwrap().unwrap();
        (
            row.try_get::<Uuid>("", "id").unwrap().into_bytes(),
            row.try_get::<Vec<u8>>("", "digest")
                .unwrap()
                .try_into()
                .unwrap(),
        )
    }
    async fn elapsed(&self, seconds: i32) {
        material::elapsed(self.db, YEAR, seconds).await;
    }
}
pub(super) async fn run(store: &AccountStore, db: &DatabaseConnection, keys: &ServerKeys) {
    let fixture = Fixture {
        store,
        db,
        alice: player(store, keys, "adventnativeone").await,
        bob: player(store, keys, "adventnativetwo").await,
        observer: player(store, keys, "adventobserver").await,
    };
    assert!(matches!(
        fixture
            .call(&fixture.alice, Q::CurrentCalendar {})
            .await
            .unwrap(),
        A::NoCalendar {}
    ));
    material::provision(db, YEAR).await;
    let (sleeve, mat) = calendar::run(&fixture).await;
    presentation::run(&fixture, sleeve, mat, keys).await;
    for player in [fixture.alice, fixture.bob, fixture.observer] {
        db.execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "DELETE FROM central_identity.accounts WHERE id=$1",
            [player.account.into()],
        ))
        .await
        .unwrap();
    }
}
