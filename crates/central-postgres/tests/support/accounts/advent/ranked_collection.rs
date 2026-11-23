//! Reach a rank on the authoritative rating row, then exercise the user's actual
//! mixed collection, including a later season and a second device's format.
use super::{A, Fixture, Player, Q, api};
use api::collection::{Item, Source};
use central_api::ranked::League;
use sea_orm::{ConnectionTrait, DbBackend, Statement, prelude::Uuid};

async fn items(f: &Fixture<'_>, owner: &Player) -> Vec<Item> {
    let A::Collection { items, next: None } =
        f.call(owner, Q::Collection { after: None }).await.unwrap()
    else {
        panic!("expected a single small synthetic collection page");
    };
    items
}

async fn standing(
    f: &Fixture<'_>,
    season: Uuid,
    format: &str,
    games: i32,
    league: i16,
    numeral: i16,
) {
    f.db.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "INSERT INTO central_identity.ranked_ratings(season_id,account_id,format,games,wins,league,division)
         VALUES($1,$2,$3,$4,$4,$5,$6) ON CONFLICT(season_id,format,account_id)
         DO UPDATE SET games=EXCLUDED.games,wins=EXCLUDED.wins,league=EXCLUDED.league,division=EXCLUDED.division",
        [season.into(),f.alice.account.into(),format.to_owned().into(),games.into(),league.into(),numeral.into()])).await.unwrap();
}

async fn unlocked(f: &Fixture<'_>, player: &Player) -> i64 {
    let row =
        f.db.query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT count(*) FROM central_identity.ranked_cosmetic_unlocks WHERE account_id=$1",
            [player.account.into()],
        ))
        .await
        .unwrap()
        .unwrap();
    row.try_get_by_index(0).unwrap()
}

async fn catalog_slot(f: &Fixture<'_>, kind: api::Kind) -> Uuid {
    let bytes = format!("rack-coverage-synthetic-{}", kind.as_str()).into_bytes();
    let row=f.db.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "WITH art AS (INSERT INTO central_identity.advent_art(source,kind,title_de,title_en,mime,size,width,height,digest,content)
         VALUES('ranked',$1,'Test','Fixture','image/webp',octet_length($2::bytea),4,2,sha256($2),$2) RETURNING id)
         INSERT INTO central_identity.ranked_cosmetic_catalog(league,division,kind,art_id)
         SELECT 5,3,$1,id FROM art RETURNING art_id", [kind.as_str().into(),bytes.into()])).await.unwrap().unwrap();
    row.try_get_by_index(0).unwrap()
}

async fn champion(f: &Fixture<'_>, season: Uuid, player: &Player, track: &str) {
    // This ledger row is ordinarily written by the daily Top 100 publisher.
    f.db.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "INSERT INTO central_identity.ranked_badge_peaks(account_id,season_id,track,format,league,division,place)
         VALUES($1,$2,$3,'standard',10,0,100)",[player.account.into(),season.into(),track.to_owned().into()])).await.unwrap();
}

pub(super) async fn run(f: &Fixture<'_>) -> Item {
    let old:Uuid=f.db.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "INSERT INTO central_identity.ranked_seasons(starts_at,ends_at) VALUES('1960-01-01','1960-01-02') RETURNING id"))
        .await.unwrap().unwrap().try_get_by_index(0).unwrap();
    let current:Uuid=f.db.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT id FROM central_identity.ranked_seasons WHERE starts_at<=now() AND (ends_at IS NULL OR ends_at>now())"))
        .await.unwrap().unwrap().try_get_by_index(0).unwrap();
    standing(f, old, "standard", 9, 0, 0).await;
    assert_eq!(unlocked(f, &f.alice).await, 0);
    standing(f, old, "standard", 10, 0, 0).await;
    assert_eq!(unlocked(f, &f.alice).await, 1);
    standing(f, old, "standard", 440, 5, 4).await;
    standing(f, old, "standard", 600, 5, 2).await;
    // A skipped Gold III and both of its items are available when imported,
    // even if the scoring transaction ran while the private shelves were empty.
    assert_eq!(unlocked(f, &f.alice).await, 16);
    let legacy = f.inventory(&f.alice).await;
    let sleeve = catalog_slot(f, api::Kind::Sleeve).await;
    let mat = catalog_slot(f, api::Kind::Playmat).await;
    let original = items(f, &f.alice).await;
    assert_eq!(original.len(), legacy.len() + 2);
    assert!(original.iter().any(|r| r.id == sleeve.into_bytes()));
    let reward = original
        .iter()
        .find(|r| r.id == mat.into_bytes())
        .unwrap()
        .clone();
    assert_eq!(
        reward.source,
        Source::Ranked {
            league: League::Gold,
            division: Some(3)
        }
    );
    assert_eq!(f.inventory(&f.alice).await, legacy);
    for (format, league, division) in [
        ("standard", 4, 1),
        ("highlander", 5, 2),
        ("commander", 5, 2),
    ] {
        standing(f, current, format, 700, league, division).await;
    }
    assert_eq!(items(f, &f.alice).await, original);
    champion(f, current, &f.bob, "deck").await;
    assert_eq!(unlocked(f, &f.bob).await, 0);
    let owned = Q::Asset {
        scope: api::AssetScope::Owned {
            item: mat.into_bytes(),
        },
        offset: 0,
        cached: Some(reward.asset.digest),
    };
    assert!(matches!(
        f.call(&f.alice, owned.clone()).await.unwrap(),
        A::Cached { .. }
    ));
    assert!(f.call(&f.bob, owned).await.is_err());
    champion(f, current, &f.alice, "player").await;
    assert_eq!(unlocked(f, &f.alice).await, 28);
    reward
}

pub(super) async fn remove_catalog(f: &Fixture<'_>) {
    // The independent restricted importer test starts from a vacant catalogue.
    f.db.execute_unprepared(
        "DELETE FROM central_identity.ranked_cosmetic_catalog WHERE league=5 AND division=3",
    )
    .await
    .unwrap();
}
