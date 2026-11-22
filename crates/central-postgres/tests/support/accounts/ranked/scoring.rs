//! Real host settlements on independently edited versions and ten-game placements.
use central_api::{
    Token,
    ranked::{
        self as api, DeckResult, HostRequest, League, Outcome, PoolEntry, PreviewSource, Rank,
    },
};
use central_postgres::AccountStore;
use sea_orm::{
    ConnectionTrait, DatabaseConnection, DbBackend, QueryResult, Statement, TryGetable,
    prelude::Uuid,
};

fn field<T: TryGetable>(row: &QueryResult, name: &str) -> T {
    row.try_get("", name).unwrap()
}

fn entries(replaced: u16) -> Vec<PoolEntry> {
    let mut entries = Vec::new();
    if replaced < 100 {
        entries.push(PoolEntry {
            card: [1; 32],
            copies: 100 - replaced,
        });
    }
    if replaced > 0 {
        entries.push(PoolEntry {
            card: [2; 32],
            copies: replaced,
        });
    }
    entries
}

fn pool(replaced: u16) -> String {
    String::from_utf8(central_api::encode(&entries(replaced)).unwrap()).unwrap()
}

struct Table<'a> {
    db: &'a DatabaseConnection,
    store: &'a AccountStore,
    accounts: [Uuid; 2],
    owners: [&'a Token; 2],
    host: Uuid,
    credential: &'a Token,
    origin: &'a str,
}

impl Table<'_> {
    async fn preview(&self, seat: usize, replaced: u16) -> api::DeckPreview {
        let result = self
            .store
            .ranked(
                Some(self.owners[seat].into()),
                api::Request::PreviewDeck {
                    format: api::Format::Commander,
                    pool: entries(replaced),
                },
            )
            .await
            .unwrap();
        let api::Response::DeckPreview { preview } = result else {
            panic!("deck preview expected")
        };
        preview
    }

    async fn before_play(&self) {
        let request = api::Request::PreviewDeck {
            format: api::Format::Commander,
            pool: entries(0),
        };
        assert_eq!(
            self.store.ranked(None, request).await.unwrap_err(),
            central_api::Error::Rejected
        );
        for changed in [0, 50, 100] {
            let fresh = self.preview(0, changed).await;
            assert_eq!(
                (fresh.source, fresh.elo, fresh.placement_games),
                (PreviewSource::New, 1500, 0)
            );
        }
        let row = self
            .db
            .query_one_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT COUNT(*) AS count FROM central_identity.ranked_deck_versions
             WHERE account_id=$1 AND format='commander'",
                vec![self.accounts[0].into()],
            ))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(field::<i64>(&row, "count"), 0);
        let repeated = vec![
            PoolEntry {
                card: [1; 32],
                copies: 1
            };
            2
        ];
        let bad = api::Request::PreviewDeck {
            format: api::Format::Commander,
            pool: repeated,
        };
        assert_eq!(
            self.store
                .ranked(Some(self.owners[0].into()), bad)
                .await
                .unwrap_err(),
            central_api::Error::Rejected
        );
    }
    async fn draw(&self, edits: [u16; 2]) -> Uuid {
        let fixture=self.db.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "INSERT INTO central_identity.ranked_matches(season_id,format,account_a,account_b,
             commitment_a,commitment_b,claimed_a,claimed_b,deck_a,deck_b,host_id,ranked_epoch,origin,state,game_id,start_fingerprint)
             SELECT s.id,'commander',$1,$2,decode(repeat('00',32),'hex'),decode(repeat('01',32),'hex'),
             true,true,$3::jsonb,$4::jsonb,$5,p.ranked_epoch,$6,'playing',pg_catalog.uuidv7(),decode(repeat('02',32),'hex')
             FROM central_identity.ranked_seasons s, central_identity.gateway_approvals p
             WHERE p.gateway_id=$5 AND s.starts_at<=now() AND (s.ends_at IS NULL OR s.ends_at>now())
             RETURNING id,game_id",
             vec![self.accounts[0].into(),self.accounts[1].into(),pool(edits[0]).into(),pool(edits[1]).into(),
             self.host.into(),self.origin.into()],
        )).await.unwrap().unwrap();
        let (id, game) = (
            field::<Uuid>(&fixture, "id"),
            field::<Uuid>(&fixture, "game_id"),
        );
        self.store
            .ranked_host(
                central_postgres::RankedHostCredential::Registered(self.credential),
                HostRequest::Finish {
                    match_id: *id.as_bytes(),
                    game: *game.as_bytes(),
                    outcome: Outcome::Draw,
                },
            )
            .await
            .unwrap();
        id
    }

    async fn receipt(&self, match_id: Uuid) -> (Rank, Rank, DeckResult) {
        let row = self
            .db
            .query_one_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT before_rank::text,after_rank::text,deck_result::text FROM central_identity.ranked_receipts
             WHERE match_id=$1 AND account_id=$2",
                vec![match_id.into(), self.accounts[0].into()],
            ))
            .await
            .unwrap()
            .unwrap();
        let decode = |name| central_api::decode(field::<String>(&row, name).as_bytes()).unwrap();
        (
            decode("before_rank"),
            decode("after_rank"),
            central_api::decode(field::<String>(&row, "deck_result").as_bytes()).unwrap(),
        )
    }

    async fn latest(&self, seat: usize) -> (i32, i32) {
        let row=self.db.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT elo,games FROM central_identity.ranked_deck_ratings WHERE account_id=$1 AND format='commander'
             ORDER BY last_at DESC, version LIMIT 1",vec![self.accounts[seat].into()],
        )).await.unwrap().unwrap();
        (field(&row, "elo"), field(&row, "games"))
    }
}

pub(super) async fn run(
    store: &AccountStore,
    db: &DatabaseConnection,
    accounts: [([u8; 16], &Token); 2],
    host: [u8; 16],
    credential: &Token,
    origin: &str,
) {
    let table = Table {
        db,
        store,
        accounts: accounts.map(|(id, _)| Uuid::from_bytes(id)),
        owners: accounts.map(|(_, token)| token),
        host: Uuid::from_bytes(host),
        credential,
        origin,
    };
    table.before_play().await;
    for completed in 1..10 {
        let id = table.draw([0, 0]).await;
        let (_, player, deck) = table.receipt(id).await;
        assert_eq!(player.league, League::Unranked);
        assert_eq!(deck.after.league, League::Unranked);
        assert_eq!(deck.placement_games, completed);
        assert_eq!(table.preview(0, 0).await.placement_games, completed);
    }
    let tenth = table.draw([0, 0]).await;
    let (before, after, deck) = table.receipt(tenth).await;
    assert_eq!(before.league, League::Unranked);
    assert_ne!(after.league, League::Unranked);
    assert_eq!((deck.before, deck.after), (before, after));
    assert_eq!((deck.placement_games, deck.elo, deck.delta), (10, 1500, 0));
    let original = table.latest(0).await;
    assert_eq!(original, (1500, 10));
    let unchanged = table.preview(0, 0).await;
    assert_eq!(
        (unchanged.source, unchanged.elo, unchanged.rank),
        (PreviewSource::Recorded, deck.elo, deck.after)
    );
    let direct = table.preview(0, 50).await;
    assert_eq!(
        (
            direct.source,
            direct.elo,
            direct.edit_loss,
            direct.changed_copies,
            direct.compared_copies
        ),
        (PreviewSource::Inherited, 1095, 405, 50, 100)
    );

    // Completely different lists must never consume a second 1500 seed.
    let disjoint = table.draw([100, 100]).await;
    let (_, player, edited) = table.receipt(disjoint).await;
    assert_ne!(player.league, League::Unranked);
    assert_eq!(edited.after.league, League::Unranked);
    assert_eq!(edited.placement_games, 1);
    assert_eq!(table.latest(0).await, (150, 1));
    assert_eq!(table.latest(1).await, (150, 1));
    // One seat now takes intermediate steps and plays; neither seat touches
    // its fixed original. Both end with exactly half the same pool replaced.
    table.draw([100, 25]).await;
    assert_eq!(table.preview(0, 25).await.source, PreviewSource::Inherited);
    assert_eq!(table.preview(1, 25).await.source, PreviewSource::Recorded);
    table.draw([100, 35]).await;
    assert_eq!(table.preview(1, 50).await, direct);
    table.draw([50, 50]).await;
    assert_eq!(table.latest(0).await, (1095, 1));
    assert_eq!(table.latest(1).await, (1095, 1));
    // Main/side swaps arrive as the same registered multiset.
    table.draw([0, 0]).await;
    assert_eq!(table.latest(0).await, (original.0, original.1 + 1));
    assert_eq!(table.latest(1).await, (original.0, original.1 + 1));
    assert_eq!(table.receipt(tenth).await, (before, after, deck));

    rollover(&table).await;
}

async fn rollover(table: &Table<'_>) {
    // Exactly forty matches on the half rebuild qualify it at rollover.
    // The second pilot's closer 35% variant only played once in that season.
    for _ in 1..40 {
        table.draw([50, 50]).await;
    }
    assert_eq!(table.latest(0).await, (1095, 40));
    table
        .db
        .execute_unprepared(
            "WITH closed AS (
          UPDATE central_identity.ranked_seasons SET ends_at=statement_timestamp()
          WHERE ends_at IS NULL RETURNING ends_at)
         INSERT INTO central_identity.ranked_seasons(starts_at,name)
         SELECT ends_at,'Scoring era two' FROM closed",
        )
        .await
        .unwrap();
    for seat in 0..2 {
        let preview = table.preview(seat, 33).await;
        assert_eq!(
            (
                preview.source,
                preview.elo,
                preview.edit_loss,
                preview.changed_copies
            ),
            (PreviewSource::Inherited, 1041, 54, 17)
        );
    }
    let after_cut = table.draw([33, 33]).await;
    let (_, pilot, new_version) = table.receipt(after_cut).await;
    assert_ne!(pilot.league, League::Unranked);
    assert_eq!(new_version.placement_games, 1);
    assert_eq!(table.latest(0).await, (1041, 1));
    assert_eq!(table.latest(1).await, (1041, 1));
    let restored = table.draw([50, 50]).await;
    assert_eq!(table.receipt(restored).await.2.placement_games, 10);
    assert_eq!(table.latest(0).await, (1095, 41));
}
