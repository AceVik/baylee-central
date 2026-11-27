//! History, pagination and the scarce global title are public data contracts.
use super::{AccountStore, DatabaseConnection};
use central_api::ranked::{
    AwardKind, Format, League, ProfileTarget, Request, Response, Row, Track,
};
use sea_orm::{ConnectionTrait, DbBackend, Statement, prelude::Uuid};
use std::collections::BTreeSet;

async fn fixture(db: &DatabaseConnection) {
    db.execute_unprepared(
        "INSERT INTO central_identity.ranked_seasons(starts_at,ends_at,name) VALUES
           ('2000-01-01 UTC','2001-01-01 UTC','Board era one'),
           ('2001-01-01 UTC','2002-01-01 UTC','Board era two');
         INSERT INTO central_identity.accounts(legacy_id,username_key,display_name,display_name_at,tag)
         SELECT 'boardfixture'||i,'boardfixture'||i,'Boarder',now(),600000000+i FROM generate_series(1,109)i;
         INSERT INTO central_identity.ranked_ratings
          (season_id,format,account_id,elo,games,wins,league,division,first_at,last_at)
         SELECT s.id,'standard',a.id,2400,2800,2800,9,0,'2000-01-01 UTC','2000-12-31 UTC'
         FROM central_identity.accounts a CROSS JOIN central_identity.ranked_seasons s
         WHERE a.tag BETWEEN 600000001 AND 600000106 AND s.name='Board era one';
         INSERT INTO central_identity.ranked_ratings
          (season_id,format,account_id,elo,games,wins,league,division,first_at,last_at)
         SELECT s.id,'standard',a.id,2550,3100,3100,9,0,'2000-01-01 UTC','2001-12-30 UTC'
         FROM central_identity.accounts a CROSS JOIN central_identity.ranked_seasons s
         WHERE a.tag BETWEEN 600000001 AND 600000106 AND s.name='Board era two';
         INSERT INTO central_identity.ranked_ratings
          (season_id,format,account_id,elo,games,wins,league,division,first_at,last_at)
         SELECT s.id,'commander',a.id,2700,3100,3100,9,0,'2000-01-01 UTC','2001-12-30 UTC'
         FROM central_identity.accounts a CROSS JOIN central_identity.ranked_seasons s
         WHERE a.tag=600000106 AND s.name='Board era two';
         INSERT INTO central_identity.ranked_deck_versions
          (account_id,format,version,cards,copies,elo,initial_pool,first_season)
         SELECT a.id,'highlander',decode(repeat('a5',32),'hex'),'{}',100,2800,false,s.id
         FROM central_identity.accounts a CROSS JOIN central_identity.ranked_seasons s
         WHERE a.tag IN (600000106,600000107) AND s.name='Board era two';
         INSERT INTO central_identity.ranked_deck_ratings
          (season_id,account_id,format,version,elo,games,season_games,wins,league,first_at,last_at)
         SELECT v.first_season,a.id,v.format,v.version,
          CASE WHEN a.tag=600000106 THEN 2900 ELSE 2800 END,3100,3100,3100,9,'2000-01-01 UTC','2001-12-30 UTC'
         FROM central_identity.ranked_deck_versions v JOIN central_identity.accounts a ON v.account_id=a.id
         WHERE a.tag IN (600000106,600000107);
         INSERT INTO central_identity.ranked_deck_versions
          (account_id,format,version,cards,copies,elo,initial_pool,first_season)
         SELECT a.id,'standard',sha256(int4send(i)),'{}',60,1700,false,s.id
         FROM central_identity.accounts a CROSS JOIN central_identity.ranked_seasons s
         CROSS JOIN generate_series(1,43) i WHERE a.tag=600000001 AND s.name='Board era one';
         INSERT INTO central_identity.ranked_deck_ratings
          (season_id,account_id,format,version,elo,games,season_games,wins,league,division,first_at,last_at)
         SELECT v.first_season,v.account_id,v.format,v.version,1700,500,500,500,5,4,'2000-01-01 UTC','2000-12-30 UTC'
         FROM central_identity.ranked_deck_versions v JOIN central_identity.accounts a ON v.account_id=a.id
         WHERE a.tag=600000001 AND v.format='standard';
         INSERT INTO central_identity.ranked_ratings
          (season_id,format,account_id,elo,games,wins,league,division,first_at,last_at)
         SELECT s.id,'standard',a.id,CASE WHEN a.tag=600000108 THEN 2900 ELSE 1500 END,
          CASE WHEN a.tag=600000108 THEN 3100 ELSE 9 END,
          CASE WHEN a.tag=600000108 THEN 3100 ELSE 9 END,
          CASE WHEN a.tag=600000108 THEN 9 ELSE 0 END,0,'2000-01-01 UTC','2001-12-01 UTC'
         FROM central_identity.accounts a CROSS JOIN central_identity.ranked_seasons s
         WHERE a.tag IN (600000108,600000109) AND s.name='Board era two';"
    ).await.unwrap();
}

async fn board(
    store: &AccountStore,
    id: [u8; 16],
    format: Format,
    track: Track,
    league: League,
) -> Vec<Row> {
    let mut after = None;
    let mut seen = Vec::new();
    loop {
        let result = store
            .ranked(
                None,
                Request::Board {
                    season: Some(id),
                    format,
                    track,
                    league,
                    division: None,
                    after,
                    players: central_api::ranked::ai::BoardPlayers::Everyone,
                },
            )
            .await
            .unwrap();
        let Response::Board { entries, next, .. } = result else {
            panic!("expected board")
        };
        assert!(entries.len() <= 40);
        seen.extend(entries);
        if next.is_none() {
            return seen;
        }
        assert_ne!(next, after);
        assert!(seen.len() <= 200);
        after = next;
    }
}

pub(super) async fn run(store: &AccountStore, db: &DatabaseConnection) {
    fixture(db).await;
    let Response::Seasons { items, .. } = store
        .ranked(None, Request::Seasons { before: None })
        .await
        .unwrap()
    else {
        panic!("expected seasons")
    };
    let season = items.iter().find(|s| s.name == "Board era two").unwrap().id;
    let earlier = items.iter().find(|s| s.name == "Board era one").unwrap().id;
    let tied = board(store, earlier, Format::Standard, Track::Deck, League::Gold).await;
    assert_eq!(tied.len(), 43);
    assert!(
        tied.windows(2)
            .all(|w| w[0].standing.deck < w[1].standing.deck)
    );
    champion_board(store, db, season).await;
    archive_profile(store, season).await;
    db.execute_unprepared(
        "DELETE FROM central_identity.accounts WHERE legacy_id LIKE 'boardfixture%'
            AND tag BETWEEN 600000001 AND 600000109;
         DELETE FROM central_identity.ranked_publications WHERE season_id IN (
            SELECT id FROM central_identity.ranked_seasons WHERE name IN ('Board era one','Board era two'));
         DELETE FROM central_identity.ranked_seasons WHERE name IN ('Board era one','Board era two')",
    )
    .await
    .unwrap();
}

async fn champion_board(store: &AccountStore, db: &DatabaseConnection, season: [u8; 16]) {
    let standard = board(
        store,
        season,
        Format::Standard,
        Track::Player,
        League::Champion,
    )
    .await;
    assert_eq!(standard.len(), 98);
    assert!(
        standard
            .windows(2)
            .all(|w| w[0].standing.rank.champion_place < w[1].standing.rank.champion_place)
    );
    assert!(
        standard
            .iter()
            .all(|r| r.standing.games == 300 && r.standing.career.games == 3100)
    );
    let commander = board(
        store,
        season,
        Format::Commander,
        Track::Player,
        League::Champion,
    )
    .await;
    let decks = board(
        store,
        season,
        Format::Highlander,
        Track::Deck,
        League::Champion,
    )
    .await;
    assert_eq!(decks.len(), 2);
    assert_eq!(commander.len(), 1);
    assert_eq!(decks[0].account, commander[0].account);
    assert_eq!(decks[0].standing.rank.champion_place, Some(1));
    assert_eq!(decks[1].standing.rank.champion_place, Some(2));
    assert_ne!(decks[0].standing.deck, Some([0xa5; 32]));
    assert_ne!(decks[0].standing.deck, decks[1].standing.deck);
    let people: BTreeSet<_> = standard
        .iter()
        .chain(&commander)
        .chain(&decks)
        .map(|r| r.account)
        .collect();
    assert_eq!(people.len(), 100);
    // The publisher found 100 distinct people, including one deck-only title.
    // Exactly its 99 published player accounts own the Champion equipment.
    let players: BTreeSet<_> = standard
        .iter()
        .chain(&commander)
        .map(|r| r.account)
        .collect();
    assert_eq!(players.len(), 99);
    let grants: BTreeSet<_> = db
        .query_all_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT u.account_id FROM central_identity.ranked_cosmetic_unlocks u
         JOIN central_identity.accounts a ON a.id=u.account_id
         WHERE u.league=10 AND u.division=0 AND a.tag BETWEEN 600000001 AND 600000109",
        ))
        .await
        .unwrap()
        .iter()
        .map(|row| row.try_get_by_index::<Uuid>(0).unwrap().into_bytes())
        .collect();
    assert_eq!(grants, players);
}

async fn archive_profile(store: &AccountStore, season: [u8; 16]) {
    let inactive = board(
        store,
        season,
        Format::Standard,
        Track::Player,
        League::Diamond,
    )
    .await;
    assert_eq!(inactive.len(), 1);
    assert_eq!(inactive[0].standing.rank.division, Some(1));
    let placement = board(
        store,
        season,
        Format::Standard,
        Track::Player,
        League::Unranked,
    )
    .await;
    assert_eq!(placement.len(), 1);
    assert_eq!(placement[0].standing.placement_games, 9);
    let Response::Profile {
        formats, awards, ..
    } = store
        .ranked(
            None,
            Request::Profile {
                target: ProfileTarget::Handle(central_api::profile::handle("Boarder", 600_000_106)),
                season: Some(season),
            },
        )
        .await
        .unwrap()
    else {
        panic!("expected profile")
    };
    assert_eq!(
        formats
            .iter()
            .filter(|s| s.rank.champion_place == Some(1))
            .count(),
        2
    );
    assert!(
        awards
            .iter()
            .any(|a| a.kind == AwardKind::Peak && a.rank.champion_place == Some(1))
    );
    assert!(
        awards
            .iter()
            .any(|a| a.kind == AwardKind::SeasonFinish && a.track == Track::Deck)
    );
    let Response::Profile { awards, .. } = store
        .ranked(
            None,
            Request::Profile {
                target: ProfileTarget::Account(placement[0].account),
                season: Some(season),
            },
        )
        .await
        .unwrap()
    else {
        panic!("expected profile")
    };
    assert_eq!(awards, []);
}

pub(super) async fn own_placements(store: &AccountStore, session: &super::Token, owner: [u8; 16]) {
    let own = Request::OwnProfile { season: None };
    assert!(!own.is_public());
    assert_eq!(
        store.ranked(None, own.clone()).await.unwrap_err(),
        central_api::Error::Rejected
    );
    let public = Request::Profile {
        target: ProfileTarget::Account(owner),
        season: None,
    };
    assert_eq!(
        store.ranked(None, public).await.unwrap_err(),
        central_api::Error::Rejected
    );
    let Response::Profile {
        account,
        name,
        formats,
        awards,
        ..
    } = store.ranked(Some(session.into()), own).await.unwrap()
    else {
        panic!("expected private profile")
    };
    assert_eq!(owner, account);
    assert_eq!(name, None);
    assert_eq!(awards, []);
    assert_eq!(formats.len(), 3);
    for (standing, format) in formats.into_iter().zip(Format::ALL) {
        assert_eq!(standing.format, format);
        assert_eq!(standing.track, Track::Player);
        assert_eq!(standing.rank.league, League::Unranked);
        assert_eq!(standing.rank.division, None);
        assert_eq!(standing.placement_games, 0);
        assert_eq!(standing.career.games, 0);
    }
}
