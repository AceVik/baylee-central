//! The one-time re-ranking after leagues and divisions came to follow the Elo
//! alone (28.11.2026): a dry run writes nothing, the run writes exactly what
//! the dry run named and grants the newly reached division's badge, a
//! record in placement is never touched, and a second run finds nothing.
use super::DatabaseConnection;
use sea_orm::{ConnectionTrait, DbBackend, Statement, prelude::Uuid};

/// A league and division as stored.
type Stored = (i16, i16);
/// A fixture's record: tag, Elo, games, stored rank, and the rank the rule
/// gives (None: the rule agrees, nothing to do).
type Record = (i32, i32, i32, Stored, Option<Stored>);
/// A change as the test compares it: account, track, before, after.
type Named = (Uuid, &'static str, Stored, Stored);

const RECORDS: [Record; 5] = [
    // more Elo but fewer games than the next: Bronze II under the old gates
    (600_000_201, 1719, 76, (3, 2), Some((5, 4))),
    (600_000_202, 1692, 107, (3, 1), Some((5, 4))),
    // a division held under the old 15-point buffer
    (600_000_203, 1515, 300, (3, 1), Some((3, 2))),
    // in placement: no public rank, whatever the Elo
    (600_000_204, 2600, 9, (0, 0), None),
    // already where the rule puts it
    (600_000_205, 1250, 50, (1, 2), None),
];

async fn fixture(db: &DatabaseConnection) {
    db.execute_unprepared(
        "INSERT INTO central_identity.ranked_seasons(starts_at,ends_at,name) VALUES
           ('1990-01-01 UTC','1991-01-01 UTC','Rerank era');
         INSERT INTO central_identity.accounts(legacy_id,username_key,display_name,display_name_at,tag)
         SELECT 'rerankfixture'||i,'rerankfixture'||i,'Reranked',now(),600000200+i FROM generate_series(1,5)i;",
    )
    .await
    .unwrap();
    for (tag, elo, games, (league, division), _) in RECORDS {
        db.execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO central_identity.ranked_ratings
              (season_id,format,account_id,elo,games,wins,league,division,first_at,last_at)
             SELECT s.id,'standard',a.id,$2,$3,$3,$4,$5,'1990-01-01 UTC','1990-01-09 UTC'
             FROM central_identity.accounts a CROSS JOIN central_identity.ranked_seasons s
             WHERE a.tag=$1 AND s.name='Rerank era'",
            vec![
                tag.into(),
                elo.into(),
                games.into(),
                league.into(),
                division.into(),
            ],
        ))
        .await
        .unwrap();
    }
    // one deck, stored as the old gates left it
    db.execute_unprepared(
        "INSERT INTO central_identity.ranked_deck_versions
          (account_id,format,version,cards,copies,elo,initial_pool,first_season)
         SELECT a.id,'standard',decode(repeat('c7',32),'hex'),'{}',60,1719,false,s.id
         FROM central_identity.accounts a CROSS JOIN central_identity.ranked_seasons s
         WHERE a.tag=600000201 AND s.name='Rerank era';
         INSERT INTO central_identity.ranked_deck_ratings
          (season_id,account_id,format,version,elo,games,season_games,wins,league,division,first_at,last_at)
         SELECT v.first_season,v.account_id,v.format,v.version,1719,76,76,76,3,2,'1990-01-01 UTC','1990-01-09 UTC'
         FROM central_identity.ranked_deck_versions v JOIN central_identity.accounts a ON v.account_id=a.id
         WHERE a.tag=600000201;",
    )
    .await
    .unwrap();
}

async fn account(db: &DatabaseConnection, tag: i32) -> Uuid {
    db.query_one_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT id FROM central_identity.accounts WHERE tag=$1 AND legacy_id LIKE 'rerankfixture%'",
        vec![tag.into()],
    ))
    .await
    .unwrap()
    .unwrap()
    .try_get_by_index(0)
    .unwrap()
}

/// League and division as stored, players' first, then the deck's.
async fn stored(db: &DatabaseConnection, account: Uuid) -> Vec<(i16, i16)> {
    db.query_all_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT league,division FROM central_identity.ranked_ratings WHERE account_id=$1
         UNION ALL
         SELECT league,division FROM central_identity.ranked_deck_ratings WHERE account_id=$1",
        vec![account.into()],
    ))
    .await
    .unwrap()
    .iter()
    .map(|row| {
        (
            row.try_get_by_index(0).unwrap(),
            row.try_get_by_index(1).unwrap(),
        )
    })
    .collect()
}

pub(super) async fn run(db: &DatabaseConnection) {
    fixture(db).await;
    let mut accounts = Vec::new();
    for (tag, ..) in RECORDS {
        accounts.push(account(db, tag).await);
    }
    let mine = |changes: Vec<central_postgres::Rerank>| -> Vec<central_postgres::Rerank> {
        changes
            .into_iter()
            .filter(|c| accounts.contains(&c.account))
            .collect()
    };

    // a dry run names every record the rule places elsewhere and writes nothing
    let dry = mine(central_postgres::rerank(db, false).await.unwrap());
    let expected: Vec<Named> = RECORDS
        .iter()
        .zip(&accounts)
        .filter_map(|(&(_, _, _, before, after), &id)| {
            after.map(|after| (id, "player", before, after))
        })
        .chain([(accounts[0], "deck", (3, 2), (5, 4))])
        .collect();
    let named: Vec<Named> = dry
        .iter()
        .map(|c| (c.account, c.track, c.before, c.after))
        .collect();
    assert_eq!(named.len(), expected.len(), "{named:?}");
    for change in &expected {
        assert!(named.contains(change), "{change:?} in {named:?}");
    }
    assert_eq!(dry.iter().filter(|c| c.promoted()).count(), 3);
    assert_eq!(
        stored(db, accounts[0]).await,
        [(3, 2), (3, 2)],
        "nothing written"
    );

    // the run writes exactly that, and the new division earns its badge
    let applied = mine(central_postgres::rerank(db, true).await.unwrap());
    assert_eq!(applied, dry);
    assert_eq!(stored(db, accounts[0]).await, [(5, 4), (5, 4)]);
    assert_eq!(stored(db, accounts[2]).await, [(3, 2)]);
    assert_eq!(
        stored(db, accounts[3]).await,
        [(0, 0)],
        "placement untouched"
    );
    let badge = db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT count(*)::bigint FROM central_identity.ranked_badge_peaks
             WHERE account_id=$1 AND league=5 AND division=4",
            vec![accounts[0].into()],
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get_by_index::<i64>(0)
        .unwrap();
    assert_eq!(badge, 2, "the player's and the deck's Gold IV");

    // a second run finds nothing more
    assert_eq!(mine(central_postgres::rerank(db, false).await.unwrap()), []);

    db.execute_unprepared(
        "DELETE FROM central_identity.accounts WHERE legacy_id LIKE 'rerankfixture%';
         DELETE FROM central_identity.ranked_seasons WHERE name='Rerank era';",
    )
    .await
    .unwrap();
}
