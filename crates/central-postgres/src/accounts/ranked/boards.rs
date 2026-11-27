//! Only confirmed competition statistics and the public display identity leave
//! this module. Personal decks, private account names and sessions never do.

mod publication;
mod standings;

pub(super) use publication::{champion_place, deck_champion_place, publish};

use super::{column, statement, unavailable, uuid};
use central_api::{
    Error, profile,
    ranked::{
        self as api, Award, AwardKind, Id, League, ProfileTarget, Rank, Season, Standing, Track,
    },
};
use sea_orm::{ConnectionTrait, DatabaseTransaction, QueryResult, prelude::Uuid};

const PAGE: usize = 40;
const LEAGUES: [League; 11] = League::PLACED;
const SEASONS: &str =
    "SELECT s.id,COALESCE(s.name,to_char(s.starts_at AT TIME ZONE 'UTC','YYYY-MM-DD')) AS name,
    floor(extract(epoch FROM s.starts_at))::bigint AS starts,
    floor(extract(epoch FROM s.ends_at))::bigint AS ends
    FROM central_identity.ranked_seasons s WHERE s.starts_at <= transaction_timestamp()";

fn season(row: &QueryResult) -> Result<Season, Error> {
    Ok(Season {
        id: *column::<Uuid>(row, "id")?.as_bytes(),
        name: column(row, "name")?,
        starts: column(row, "starts")?,
        ends: column(row, "ends")?,
    })
}

pub(super) fn wire_rank(
    league: i16,
    division: i16,
    games: i32,
    place: Option<u8>,
) -> Result<Rank, Error> {
    let league = if games < 10 {
        League::Unranked
    } else if place.is_some() {
        League::Champion
    } else {
        *LEAGUES
            .get(usize::try_from(league).map_err(unavailable)?)
            .ok_or(Error::Unavailable)?
    };
    Ok(Rank {
        league,
        division: (division > 0 && (League::Wood..=League::Diamond).contains(&league))
            .then(|| u8::try_from(division).map_err(unavailable))
            .transpose()?,
        champion_place: if league == League::Champion {
            place
        } else {
            None
        },
    })
}

fn handle(value: &str) -> Result<(&str, i32), Error> {
    let (name, tag) = value.rsplit_once('#').ok_or(Error::Rejected)?;
    if !profile::valid_display_name(name)
        || !(4..=8).contains(&tag.len())
        || !tag.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err(Error::Rejected);
    }
    let parsed = u32::from_str_radix(tag, 16).map_err(|_| Error::Rejected)?;
    if !profile::valid_tag(parsed) || profile::tag_text(parsed).len() != tag.len() {
        return Err(Error::Rejected);
    }
    Ok((name, i32::try_from(parsed).map_err(|_| Error::Rejected)?))
}

async fn profile_identity(
    db: &DatabaseTransaction,
    target: &ProfileTarget,
) -> Result<QueryResult, Error> {
    let (filter, values) = match target {
        ProfileTarget::Account(id) => ("a.id=$1", vec![uuid(*id).into()]),
        ProfileTarget::Handle(value) => {
            let (name, tag) = handle(value)?;
            (
                "a.display_name=$1 AND a.tag=$2",
                vec![name.into(), tag.into()],
            )
        }
    };
    db.query_one_raw(statement(
        &format!(
            "SELECT a.id,a.display_name,a.tag FROM central_identity.accounts a WHERE {filter}
         AND ((a.display_name IS NOT NULL AND a.tag IS NOT NULL) OR
          EXISTS (SELECT 1 FROM central_identity.ranked_ratings r WHERE r.account_id=a.id))"
        ),
        values,
    ))
    .await
    .map_err(unavailable)?
    .ok_or(Error::Rejected)
}

async fn collection(
    db: &DatabaseTransaction,
    account: Uuid,
    season: Uuid,
) -> Result<Vec<Award>, Error> {
    // One copy per earned design over the career through the selected season;
    // visiting an older season must not reveal an achievement earned later.
    let rows = db
        .query_all_raw(statement(
            "SELECT DISTINCT ON (p.format,p.track,p.league,p.division,p.podium)
         p.season_id,p.format,p.track,p.league,p.division,p.place
         FROM central_identity.ranked_badge_peaks p JOIN central_identity.ranked_seasons s
         ON s.id=p.season_id WHERE p.account_id=$1 AND s.starts_at <=
         (SELECT starts_at FROM central_identity.ranked_seasons WHERE id=$2)
         ORDER BY p.format,p.track,p.league,p.division,p.podium,p.place NULLS LAST,s.starts_at",
            vec![account.into(), season.into()],
        ))
        .await
        .map_err(unavailable)?;
    rows.iter()
        .map(|r| {
            let place = column::<Option<i16>>(r, "place")?
                .map(u8::try_from)
                .transpose()
                .map_err(unavailable)?;
            let track = match column::<String>(r, "track")?.as_str() {
                "deck" => Track::Deck,
                "player" => Track::Player,
                _ => return Err(Error::Unavailable),
            };
            Ok(Award {
                kind: AwardKind::Peak,
                season: *column::<Uuid>(r, "season_id")?.as_bytes(),
                format: column::<String>(r, "format")?
                    .parse()
                    .map_err(unavailable)?,
                track,
                rank: wire_rank(column(r, "league")?, column(r, "division")?, 10, place)?,
            })
        })
        .collect()
}

async fn seasons(db: &DatabaseTransaction, before: Option<Id>) -> Result<api::Response, Error> {
    let rows = db
        .query_all_raw(statement(
            &format!(
                "{SEASONS} AND ($1::uuid IS NULL OR
        s.starts_at < (SELECT starts_at FROM central_identity.ranked_seasons WHERE id=$1))
        ORDER BY s.starts_at DESC LIMIT {}",
                PAGE + 1
            ),
            vec![before.map(uuid).into()],
        ))
        .await
        .map_err(unavailable)?;
    let items: Vec<_> = rows
        .iter()
        .take(PAGE)
        .map(season)
        .collect::<Result<_, _>>()?;
    let next = (rows.len() > PAGE).then(|| items[PAGE - 1].id);
    Ok(api::Response::Seasons { items, next })
}

pub(super) async fn read(
    db: &DatabaseTransaction,
    request: &api::Request,
    owner: Option<Uuid>,
) -> Result<api::Response, Error> {
    let selected = match request {
        api::Request::Seasons { before } => return seasons(db, *before).await,
        api::Request::Profile { season, .. }
        | api::Request::Board { season, .. }
        | api::Request::OwnProfile { season } => season,
        _ => return Err(Error::Rejected),
    };
    let row = db
        .query_one_raw(statement(
            &format!(
                "{SEASONS} AND (s.id=$1 OR ($1::uuid IS NULL
        AND (s.ends_at IS NULL OR s.ends_at>transaction_timestamp())))"
            ),
            vec![selected.map(uuid).into()],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Rejected)?;
    let season = season(&row)?;
    let season_id = uuid(season.id);
    let finalized = publication::visible(db, season_id).await?;
    match request {
        api::Request::Board {
            format,
            track,
            league,
            division,
            after,
            players,
            ..
        } => {
            let page = standings::Page {
                format: *format,
                track: *track,
                league: *league,
                division: *division,
                after: *after,
                players: *players,
            };
            let (entries, next) = standings::board(db, season_id, page).await?;
            Ok(api::Response::Board {
                season,
                entries,
                next,
            })
        }
        api::Request::Profile { .. } | api::Request::OwnProfile { .. } => {
            let identity = if let api::Request::Profile { target, .. } = request {
                profile_identity(db, target).await?
            } else {
                db.query_one_raw(statement(
                    "SELECT id,display_name,tag FROM central_identity.accounts WHERE id=$1",
                    vec![owner.ok_or(Error::Rejected)?.into()],
                ))
                .await
                .map_err(unavailable)?
                .ok_or(Error::Rejected)?
            };
            let id: Uuid = column(&identity, "id")?;
            let (name, tag) = super::super::profiles::columns(&identity)?;
            let (formats, awards) = season_profile(db, season_id, &season, id, finalized).await?;
            Ok(api::Response::Profile {
                season,
                account: *id.as_bytes(),
                name,
                tag,
                formats,
                awards,
                ai: crate::accounts::ai::badge(db, id).await?,
            })
        }
        _ => Err(Error::Rejected),
    }
}

async fn season_profile(
    db: &DatabaseTransaction,
    season_id: Uuid,
    season: &Season,
    account: Uuid,
    finalized: bool,
) -> Result<(Vec<Standing>, Vec<Award>), Error> {
    let formats = standings::profile(db, season_id, account).await?;
    let mut awards = collection(db, account, season_id).await?;
    if finalized {
        awards.extend(
            formats
                .iter()
                .filter(|s| s.games > 0 && s.rank.league != League::Unranked)
                .map(|s| Award {
                    kind: AwardKind::SeasonFinish,
                    format: s.format,
                    track: s.track,
                    season: season.id,
                    rank: s.rank,
                }),
        );
    }
    Ok((formats, awards))
}

/// Active season metadata, standings across formats/tracks, and career awards
/// for `account` (returning `(None, vec![], vec![])` if no ranked season has started).
pub(in crate::accounts) async fn account_ranked_badges(
    db: &DatabaseTransaction,
    account: Uuid,
) -> Result<(Option<Season>, Vec<Standing>, Vec<Award>), Error> {
    let Some(row) = db
        .query_one_raw(statement(
            &format!(
                "{SEASONS} AND (s.ends_at IS NULL OR s.ends_at > transaction_timestamp())
                 ORDER BY s.starts_at DESC LIMIT 1"
            ),
            vec![],
        ))
        .await
        .map_err(unavailable)?
    else {
        return Ok((None, Vec::new(), Vec::new()));
    };
    let season = season(&row)?;
    let season_id = uuid(season.id);
    let finalized = publication::visible(db, season_id).await?;
    let (formats, awards) = season_profile(db, season_id, &season, account, finalized).await?;
    Ok((Some(season), formats, awards))
}

/// Current-season best `Track::Player` rank per tag (`games > 0`), reusing the
/// same season projection and inactive elite clamp as boards.
pub(in crate::accounts) async fn peer_best_ranks(
    db: &DatabaseTransaction,
    accounts: &[(Uuid, u32)],
) -> Result<Vec<central_api::social::TaggedRank>, Error> {
    if accounts.is_empty() {
        return Ok(Vec::new());
    }
    let Some(row) = db
        .query_one_raw(statement(
            &format!(
                "{SEASONS} AND (s.ends_at IS NULL OR s.ends_at > transaction_timestamp())
                 ORDER BY s.starts_at DESC LIMIT 1"
            ),
            vec![],
        ))
        .await
        .map_err(unavailable)?
    else {
        return Ok(Vec::new());
    };
    let season_id: Uuid = column(&row, "id")?;
    let ids: Vec<Uuid> = accounts.iter().map(|(id, _)| *id).collect();
    let idle = super::rules::ELITE_IDLE_DAYS + 1;
    let rows = db
        .query_all_raw(statement(
            &format!(
                "SELECT r.account_id, r.games, c.place,
                     CASE WHEN c.place IS NOT NULL THEN 10
                          WHEN r.games < 10 THEN -1
                          WHEN r.league >= 8 AND r.last_at <= LEAST(transaction_timestamp(), s.ends_at)
                              - interval '24 hours' * {idle} THEN 7
                          ELSE r.league END::smallint AS public_league,
                     CASE WHEN c.place IS NOT NULL OR r.games < 10 THEN 0
                          WHEN r.league >= 8 AND r.last_at <= LEAST(transaction_timestamp(), s.ends_at)
                              - interval '24 hours' * {idle} THEN 1
                          ELSE r.division END::smallint AS public_division
                 FROM central_identity.ranked_ratings r
                 JOIN central_identity.ranked_seasons s ON s.id = $1
                 LEFT JOIN central_identity.ranked_champions c
                     ON c.season_id = s.id AND c.account_id = r.account_id AND c.player_format = r.format
                 WHERE r.season_id = $1 AND r.account_id = ANY($2) AND r.games > 0"
            ),
            vec![season_id.into(), ids.into()],
        ))
        .await
        .map_err(unavailable)?;
    let mut by_account: std::collections::BTreeMap<Uuid, Rank> = std::collections::BTreeMap::new();
    for r in &rows {
        let account_id: Uuid = column(r, "account_id")?;
        let games: i32 = column(r, "games")?;
        let place = column::<Option<i16>>(r, "place")?
            .map(u8::try_from)
            .transpose()
            .map_err(unavailable)?;
        let rank = wire_rank(
            column(r, "public_league")?,
            column(r, "public_division")?,
            games,
            place,
        )?;
        by_account
            .entry(account_id)
            .and_modify(|current| {
                if central_api::ranked::rank_key(rank) > central_api::ranked::rank_key(*current) {
                    *current = rank;
                }
            })
            .or_insert(rank);
    }
    Ok(accounts
        .iter()
        .filter_map(|(id, tag)| {
            by_account
                .get(id)
                .map(|rank| central_api::social::TaggedRank {
                    tag: *tag,
                    rank: *rank,
                })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn handle_requires_full_public_name_and_unambiguous_hex_discriminator() {
        assert_eq!(handle("Alice#0007"), Ok(("Alice", 7)));
        assert_eq!(handle("Alice#ABCD"), Ok(("Alice", 0xabcd)));
        for invalid in [
            "Alice",
            "Alice#00",
            "Alice#00007",
            "Al#0007",
            "Alice#0000",
            "Alice#80000000",
            "Alice#+007",
        ] {
            assert!(handle(invalid).is_err(), "{invalid}");
        }
    }
    #[test]
    fn placement_and_podium_do_not_inherit_a_division() {
        assert_eq!(wire_rank(0, 0, 9, None).unwrap().league, League::Unranked);
        assert_eq!(wire_rank(0, 0, 10, None).unwrap().league, League::Paper);
        assert_eq!(
            wire_rank(9, 1, 3000, Some(2)).unwrap(),
            Rank {
                league: League::Champion,
                division: None,
                champion_place: Some(2)
            }
        );
    }
}
