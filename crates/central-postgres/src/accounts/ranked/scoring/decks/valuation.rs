//! Both live registration and a read-only preview value a new list here.
use super::super::{api, boards, published, rules};
use super::{Career, DatabaseTransaction, Error, Pool, get, statement, unavailable};
use sea_orm::{ConnectionTrait, QueryResult, prelude::Uuid};

pub(super) struct Quote {
    pub(super) elo: i32,
    loss: i32,
    changed: u16,
    size: u16,
}

pub(super) async fn known(
    db: &DatabaseTransaction,
    account: Uuid,
    format: &str,
    pool: &Pool,
) -> Result<Option<QueryResult>, Error> {
    db.query_one_raw(statement(
        "SELECT * FROM central_identity.ranked_deck_ratings WHERE account_id=$1 AND format=$2 AND version=$3
         ORDER BY last_at DESC,season_id DESC LIMIT 1",vec![account.into(),format.into(),pool.version.clone().into()],
    )).await.map_err(unavailable)
}

pub(super) async fn inherit(
    db: &DatabaseTransaction,
    account: Uuid,
    format: &str,
    pool: &Pool,
    pending_cut: Option<Uuid>,
) -> Result<Option<Quote>, Error> {
    // Live settlement freezes the previous season before registration. A preview
    // sees the identical new reference set even before the owner's first game
    // of the new season, without promoting or updating any stored version.
    let row=db.query_one_raw(statement(
        "WITH cut AS MATERIALIZED (
         SELECT d.version,d.elo FROM central_identity.ranked_deck_ratings d
         WHERE d.season_id=$5 AND d.account_id=$1 AND d.format=$2 AND d.season_games>=40
         ), refs AS (
         SELECT v.cards,v.copies,v.version,LEAST(v.elo,v.anchor) AS base FROM central_identity.ranked_deck_versions v
         WHERE v.account_id=$1 AND v.format=$2 AND v.anchor IS NOT NULL AND NOT EXISTS
             (SELECT 1 FROM cut WHERE cut.version=v.version)
         UNION ALL SELECT v.cards,v.copies,v.version,LEAST(v.elo,cut.elo) FROM cut
         JOIN central_identity.ranked_deck_versions v ON v.account_id=$1 AND v.format=$2 AND v.version=cut.version
         ) SELECT refs.base,GREATEST(refs.copies,$4::integer) AS size,overlap.shared
         FROM refs CROSS JOIN LATERAL (SELECT COALESCE(SUM(LEAST(e.value::integer,
             COALESCE((refs.cards->>e.key)::integer,0))),0)::integer AS shared
             FROM jsonb_each_text($3::jsonb) e) overlap
         ORDER BY overlap.shared::numeric/GREATEST(refs.copies,$4::integer) DESC,refs.base,refs.version LIMIT 1",
        vec![account.into(),format.into(),pool.counts.as_str().into(),pool.copies.into(),pending_cut.into()],
    )).await.map_err(unavailable)?;
    row.map(|row| {
        let size = u16::try_from(get::<i32>(&row, "size")?).map_err(unavailable)?;
        let shared = u16::try_from(get::<i32>(&row, "shared")?).map_err(unavailable)?;
        let changed = size.checked_sub(shared).ok_or(Error::Unavailable)?;
        let base: i32 = get(&row, "base")?;
        let elo = rules::deck_start(base, f64::from(changed) / f64::from(size));
        Ok(Quote {
            elo,
            loss: base - elo,
            changed,
            size,
        })
    })
    .transpose()
}

pub(in super::super::super) async fn preview(
    db: &DatabaseTransaction,
    account: Uuid,
    format: api::Format,
    entries: &[api::PoolEntry],
) -> Result<api::DeckPreview, Error> {
    let pool = Pool::from_entries(entries)?;
    let scope=db.query_one_raw(statement(
        "SELECT s.id,transaction_timestamp() AS now,
         (SELECT r.season_id FROM central_identity.ranked_ratings r WHERE r.account_id=$1 AND r.format=$2
          ORDER BY r.last_at DESC,r.season_id DESC LIMIT 1) AS last_season
         FROM central_identity.ranked_seasons s WHERE s.starts_at<=transaction_timestamp() AND
         (s.ends_at IS NULL OR s.ends_at>transaction_timestamp())",
        vec![account.into(),format.as_str().into()],
    )).await.map_err(unavailable)?.ok_or(Error::Rejected)?;
    let season: Uuid = get(&scope, "id")?;
    let now = get(&scope, "now")?;
    let copies = u16::try_from(pool.copies).map_err(unavailable)?;
    if let Some(row) = known(db, account, format.as_str(), &pool).await? {
        let career = Career::read(&row)?;
        let elo = get(&row, "elo")?;
        let place =
            boards::deck_champion_place(db, season, format.as_str(), account, &pool.version)
                .await?;
        return Ok(api::DeckPreview {
            format,
            source: api::PreviewSource::Recorded,
            elo,
            edit_loss: 0,
            rank: published(&career, elo, now, place)?,
            placement_games: u8::try_from(career.games().min(10)).map_err(unavailable)?,
            changed_copies: 0,
            compared_copies: copies,
        });
    }
    let last: Option<Uuid> = get(&scope, "last_season")?;
    let quote = inherit(
        db,
        account,
        format.as_str(),
        &pool,
        last.filter(|id| *id != season),
    )
    .await?;
    let (source, elo, edit_loss, changed_copies, compared_copies) =
        quote.map_or((api::PreviewSource::New, 1500, 0, 0, copies), |q| {
            (
                api::PreviewSource::Inherited,
                q.elo,
                q.loss,
                q.changed,
                q.size,
            )
        });
    Ok(api::DeckPreview {
        format,
        source,
        elo,
        edit_loss,
        rank: published(&Career::fresh(now), elo, now, None)?,
        placement_games: 0,
        changed_copies,
        compared_copies,
    })
}
