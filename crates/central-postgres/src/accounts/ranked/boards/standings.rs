//! Keyset-paged boards and the highest published deck in each profile format.
//! The lateral lookback runs on the selected page, after the bounded sort.

use super::{
    super::{column, rules, statement, unavailable},
    LEAGUES, PAGE, wire_rank,
};
use central_api::{
    Error,
    ranked::{Cursor, Format, League, Record, Row, Standing, Track},
};
use sea_orm::{ConnectionTrait, DatabaseTransaction, QueryResult, Value, prelude::Uuid};

struct Source {
    table: &'static str,
    public_deck: &'static str,
    deck_join: &'static str,
    champion_version: &'static str,
    previous_version: &'static str,
}

impl Source {
    fn new(track: Track) -> Self {
        match track {
            Track::Player => Self {
                table: "central_identity.ranked_ratings",
                public_deck: "NULL::bytea",
                deck_join: "",
                champion_version: "",
                previous_version: "",
            },
            Track::Deck => Self {
                table: "central_identity.ranked_deck_ratings",
                public_deck: "v.public_id",
                deck_join: "JOIN central_identity.ranked_deck_versions v USING (account_id,format,version)",
                champion_version: "AND c.version=r.version",
                previous_version: "AND p.version=b.version",
            },
        }
    }

    fn rows(&self, track: Track, filter: &str) -> String {
        let Self {
            table,
            public_deck,
            deck_join,
            champion_version,
            ..
        } = self;
        let version = if track == Track::Deck {
            "r.version"
        } else {
            "NULL::bytea"
        };
        let inactive = format!(
            "r.league>=8 AND r.last_at <= LEAST(transaction_timestamp(),s.ends_at)
           - interval '24 hours' * {}",
            rules::ELITE_IDLE_DAYS + 1
        );
        format!(
            "SELECT r.season_id,r.format,r.account_id,r.elo,r.games,r.wins,r.losses,r.draws,
         {public_deck} AS deck,{version} AS version,c.place,
         CASE WHEN c.place IS NOT NULL THEN 10 WHEN r.games<10 THEN -1
              WHEN {inactive} THEN 7 ELSE r.league END::smallint AS public_league,
         CASE WHEN c.place IS NOT NULL OR r.games<10 THEN 0
              WHEN {inactive} THEN 1 ELSE r.division END::smallint AS public_division,
         s.starts_at AS season_starts
         FROM {table} r {deck_join} JOIN central_identity.ranked_seasons s ON s.id=$1
         LEFT JOIN central_identity.ranked_champions c ON c.season_id=s.id
           AND c.account_id=r.account_id AND c.{}_format=r.format {champion_version}
           AND c.season_id=r.season_id
         WHERE {filter}",
            track.as_str()
        )
    }

    fn finish(&self, page: &str, order: &str) -> String {
        // A match reserved just before a season boundary can finish just after
        // it. Select the previous *season*, independently of the settlement time.
        let table = self.table;
        let same = self.previous_version;
        format!("WITH page AS MATERIALIZED ({page})
          SELECT b.*,a.display_name,a.tag,ai.kind AS ai_kind,ai.label AS ai_label,
            CASE WHEN b.season_id=$1 THEN b.games-COALESCE(old.games,0) ELSE 0 END AS season_games,
            CASE WHEN b.season_id=$1 THEN b.wins-COALESCE(old.wins,0) ELSE 0 END AS season_wins,
            CASE WHEN b.season_id=$1 THEN b.losses-COALESCE(old.losses,0) ELSE 0 END AS season_losses,
            CASE WHEN b.season_id=$1 THEN b.draws-COALESCE(old.draws,0) ELSE 0 END AS season_draws
          FROM page b JOIN central_identity.accounts a ON a.id=b.account_id
          LEFT JOIN central_identity.ai_players ai ON ai.account_id=b.account_id
          LEFT JOIN LATERAL (SELECT p.games,p.wins,p.losses,p.draws FROM {table} p
            JOIN central_identity.ranked_seasons previous ON previous.id=p.season_id
            WHERE p.account_id=b.account_id AND p.format=b.format {same}
            AND p.season_id<>b.season_id AND previous.starts_at<b.season_starts
            ORDER BY p.last_at DESC LIMIT 1) old ON b.season_id=$1
          ORDER BY {order}")
    }
}

fn count(row: &QueryResult, key: &str) -> Result<u32, Error> {
    u32::try_from(column::<i32>(row, key)?).map_err(unavailable)
}

fn standing(row: &QueryResult, track: Track) -> Result<Standing, Error> {
    let games = column::<i32>(row, "games")?;
    let place = column::<Option<i16>>(row, "place")?
        .map(u8::try_from)
        .transpose()
        .map_err(unavailable)?;
    let deck = column::<Option<Vec<u8>>>(row, "deck")?
        .map(<[u8; 32]>::try_from)
        .transpose()
        .map_err(unavailable)?;
    Ok(Standing {
        format: column::<String>(row, "format")?
            .parse()
            .map_err(unavailable)?,
        track,
        deck,
        elo: column(row, "elo")?,
        placement_games: u8::try_from(games.min(10)).map_err(unavailable)?,
        games: count(row, "season_games")?,
        wins: count(row, "season_wins")?,
        losses: count(row, "season_losses")?,
        draws: count(row, "season_draws")?,
        career: Record {
            games: count(row, "games")?,
            wins: count(row, "wins")?,
            losses: count(row, "losses")?,
            draws: count(row, "draws")?,
        },
        rank: wire_rank(
            column(row, "public_league")?,
            column(row, "public_division")?,
            games,
            place,
        )?,
    })
}

fn league_number(league: League) -> i16 {
    LEAGUES
        .iter()
        .zip(0_i16..)
        .find_map(|(value, i)| (*value == league).then_some(i))
        .unwrap_or(-1)
}

/// People, AI players or both (docs/ai-players.md section 6), as a predicate
/// on the ladder row `r`.
const fn players_filter(players: central_api::ranked::ai::BoardPlayers) -> &'static str {
    match players {
        central_api::ranked::ai::BoardPlayers::Everyone => "",
        central_api::ranked::ai::BoardPlayers::People => {
            " AND NOT EXISTS (SELECT 1 FROM central_identity.ai_players p WHERE p.account_id=r.account_id)"
        }
        central_api::ranked::ai::BoardPlayers::Ai => {
            " AND EXISTS (SELECT 1 FROM central_identity.ai_players p WHERE p.account_id=r.account_id)"
        }
    }
}

/// One board page's selection, as the request names it.
pub(super) struct Page {
    pub(super) format: Format,
    pub(super) track: Track,
    pub(super) league: League,
    pub(super) division: Option<u8>,
    pub(super) after: Option<Cursor>,
    pub(super) players: central_api::ranked::ai::BoardPlayers,
}

pub(super) async fn board(
    db: &DatabaseTransaction,
    season: Uuid,
    page: Page,
) -> Result<(Vec<Row>, Option<Cursor>), Error> {
    let Page {
        format,
        track,
        league,
        division,
        after,
        players,
    } = page;
    let n = league_number(league);
    let divisions = match n {
        1..=4 => 3,
        5..=7 => 4,
        _ => 0,
    };
    if division.is_some_and(|d| d == 0 || d > divisions)
        || after.is_some_and(|c| c.deck.is_some() != (track == Track::Deck))
    {
        return Err(Error::Rejected);
    }
    let src = Source::new(track);
    // Keep an indexable base predicate in addition to the projected league.
    let eligible = match (n, division) {
        (-1, _) => "r.league=0 AND r.games<10",
        (7, None | Some(1)) => "r.league BETWEEN 7 AND 9",
        (10, _) => "c.place IS NOT NULL",
        (_, Some(_)) => "r.league=$3 AND r.division=$4",
        _ => "r.league=$3",
    };
    let who = players_filter(players);
    let rows = src.rows(
        track,
        &format!("r.season_id=$1 AND r.format=$2 AND {eligible}{who}"),
    );
    let mut values: Vec<Value> = vec![
        season.into(),
        format.as_str().into(),
        n.into(),
        division.map(i16::from).into(),
    ];
    let order = if league == League::Champion {
        "place,account_id,deck"
    } else {
        "elo DESC,account_id,deck"
    };
    let cursor = if let Some(c) = after {
        values.push(Uuid::from_bytes(c.account).into());
        if league == League::Champion {
            format!(
                "AND place > (SELECT place FROM central_identity.ranked_champions
               WHERE season_id=$1 AND account_id=$5 AND {}_format=$2)",
                track.as_str()
            )
        } else {
            values.push(c.elo.into());
            if let Some(deck) = c.deck {
                values.push(deck.to_vec().into());
                "AND (elo < $6 OR (elo=$6 AND (account_id,deck)>($5,$7)))".into()
            } else {
                "AND (elo < $6 OR (elo=$6 AND account_id>$5))".into()
            }
        }
    } else {
        String::new()
    };
    let page = format!(
        "SELECT * FROM ({rows}) AS ladder WHERE public_league=$3
        AND ($4::smallint IS NULL OR public_division=$4) {cursor}
        ORDER BY {order} LIMIT {}",
        PAGE + 1
    );
    let sql = src.finish(&page, &order.replace("account_id", "b.account_id"));
    let rows = db
        .query_all_raw(statement(&sql, values))
        .await
        .map_err(unavailable)?;
    let entries: Vec<_> = rows
        .iter()
        .take(PAGE)
        .map(|r| {
            let (name, tag) = super::super::super::profiles::columns(r)?;
            Ok(Row {
                account: *column::<Uuid>(r, "account_id")?.as_bytes(),
                name,
                tag,
                standing: standing(r, track)?,
                ai: crate::accounts::ai::badge_of(r)?,
            })
        })
        .collect::<Result<_, Error>>()?;
    let next = (rows.len() > PAGE).then(|| {
        let row = &entries[PAGE - 1];
        Cursor {
            account: row.account,
            elo: row.standing.elo,
            deck: row.standing.deck,
        }
    });
    Ok((entries, next))
}

fn initial(format: Format) -> Standing {
    Standing {
        format,
        track: Track::Player,
        deck: None,
        elo: 1500,
        placement_games: 0,
        games: 0,
        wins: 0,
        losses: 0,
        draws: 0,
        career: Record {
            games: 0,
            wins: 0,
            losses: 0,
            draws: 0,
        },
        rank: central_api::ranked::Rank {
            league: League::Unranked,
            division: None,
            champion_place: None,
        },
    }
}

pub(super) async fn profile(
    db: &DatabaseTransaction,
    season: Uuid,
    account: Uuid,
) -> Result<Vec<Standing>, Error> {
    let mut list = Vec::with_capacity(6);
    for track in [Track::Player, Track::Deck] {
        let src = Source::new(track);
        // The last participated season carries a career into a new one, but
        // contributes zero games to this season until a result is confirmed.
        let rows = src.rows(
            track,
            "r.account_id=$2 AND (r.format,r.season_id) IN
            (SELECT format,season_id FROM recent)",
        );
        let table = src.table;
        let page = format!(
            "WITH recent AS MATERIALIZED (
                SELECT f.format,t.season_id FROM (VALUES ('standard'),('highlander'),('commander')) f(format)
                CROSS JOIN LATERAL (SELECT p.season_id FROM {table} p
                 JOIN central_identity.ranked_seasons past ON past.id=p.season_id
                 WHERE p.account_id=$2 AND p.format=f.format AND past.starts_at <=
                 (SELECT starts_at FROM central_identity.ranked_seasons WHERE id=$1)
                 ORDER BY p.last_at DESC,p.season_id DESC LIMIT 1) t
            ) SELECT DISTINCT ON (format) * FROM ({rows}) AS ladder
           ORDER BY format,public_league DESC,public_division,place NULLS LAST,elo DESC,deck"
        );
        let results = db
            .query_all_raw(statement(
                &src.finish(&page, "b.format"),
                vec![season.into(), account.into()],
            ))
            .await
            .map_err(unavailable)?;
        for r in &results {
            list.push(standing(r, track)?);
        }
    }
    for f in Format::ALL {
        if !list
            .iter()
            .any(|s| s.track == Track::Player && s.format == f)
        {
            list.push(initial(f));
        }
    }
    list.sort_by_key(|s| (s.format, s.track));
    Ok(list)
}
