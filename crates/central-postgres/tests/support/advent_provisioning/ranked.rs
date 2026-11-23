use super::{Art, DatabaseConnection, Kind, MatFraming, StoreError, art};
use central_postgres::advent_import::RankedCatalog;
use sea_orm::ConnectionTrait;

fn image(league: u8, division: u8, kind: Kind) -> Art {
    let mut image = art(0);
    image.kind = Some(kind);
    image.bytes = vec![0xe7, league, division, u8::from(kind == Kind::Sleeve)];
    if kind == Kind::Playmat && division == 0 {
        image.mat_framing = Some(MatFraming::default());
    }
    image
}

async fn full(db: &DatabaseConnection) -> Result<(), StoreError> {
    let mut publication = RankedCatalog::begin(db).await?;
    for league in 0..=10 {
        let divisions = match league {
            1..=4 => 1..=3,
            5..=7 => 1..=4,
            _ => 0..=0,
        };
        for division in divisions {
            publication
                .pair(
                    league,
                    division,
                    image(league, division, Kind::Sleeve),
                    image(league, division, Kind::Playmat),
                )
                .await?;
        }
    }
    publication.finish().await
}

pub(super) async fn run(owner: &DatabaseConnection, db: &DatabaseConnection) {
    assert!(RankedCatalog::begin(db).await.is_err());
    owner
        .execute_unprepared(include_str!(
            "../../../../../ops/ranked-cosmetics-importer-grants.sql"
        ))
        .await
        .unwrap();
    let count = "SELECT count(*) AS value FROM central_identity.ranked_cosmetic_catalog";
    let mut partial = RankedCatalog::begin(db).await.unwrap();
    partial
        .pair(0, 0, image(0, 0, Kind::Sleeve), image(0, 0, Kind::Playmat))
        .await
        .unwrap();
    assert!(partial.finish().await.is_err());
    assert_eq!(super::super::scalar(owner, count).await, 0);
    let (first, retry) = tokio::join!(full(db), full(db));
    first.unwrap();
    retry.unwrap();
    assert_eq!(super::super::scalar(owner, count).await, 56);
    for (division, framing) in [(0, None), (1, Some(MatFraming::default()))] {
        let league = u8::from(division == 1);
        let mut changed = image(league, division, Kind::Playmat);
        changed.mat_framing = framing;
        let mut transaction = RankedCatalog::begin(db).await.unwrap();
        assert!(
            transaction
                .pair(
                    league,
                    division,
                    image(league, division, Kind::Sleeve),
                    changed
                )
                .await
                .is_err()
        );
        assert!(transaction.finish().await.is_err());
    }
    let mut transaction = RankedCatalog::begin(db).await.unwrap();
    let mut changed = image(0, 0, Kind::Sleeve);
    changed.bytes.push(1);
    assert!(
        transaction
            .pair(0, 0, changed, image(0, 0, Kind::Playmat))
            .await
            .is_err()
    );
    assert!(transaction.finish().await.is_err());
    assert_eq!(
        super::super::scalar(
            owner,
            "SELECT count(*) AS value FROM central_identity.advent_art WHERE source='ranked'"
        )
        .await,
        56
    );
    owner.execute_unprepared("BEGIN;
        DELETE FROM central_identity.ranked_cosmetic_catalog c USING central_identity.advent_art a
          WHERE c.art_id=a.id AND a.source='ranked' AND a.size=4 AND a.title_en='Fixture art' AND get_byte(a.content,0)=231;
        DELETE FROM central_identity.advent_art WHERE source='ranked' AND size=4 AND title_en='Fixture art' AND get_byte(content,0)=231;
        REVOKE INSERT (source) ON central_identity.advent_art FROM central_advent_importer;
        REVOKE SELECT (league,division,kind,art_id), INSERT (league,division,kind,art_id)
          ON central_identity.ranked_cosmetic_catalog FROM central_advent_importer;
        COMMIT;").await.unwrap();
}
