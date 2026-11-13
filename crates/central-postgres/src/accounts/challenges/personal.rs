//! Account-bound projections and exports, with stable bounded pages and no secret columns.
use super::{Error, Response, api, state, statement, unavailable, uuid};
use sea_orm::{ConnectionTrait, QueryResult, prelude::Uuid};

fn get<T: sea_orm::TryGetable>(row: &QueryResult, name: &str) -> Result<T, Error> {
    row.try_get("", name).map_err(unavailable)
}

fn unsigned(row: &QueryResult, name: &str) -> Result<u32, Error> {
    u32::try_from(get::<i32>(row, name)?).map_err(unavailable)
}

pub(super) async fn progress(
    db: &impl ConnectionTrait,
    me: Uuid,
    task: [u8; 16],
) -> Result<api::TaskProgress, Error> {
    let row = db.query_one_raw(statement(
        "SELECT COALESCE(p.starts,0) AS starts,COALESCE(p.failures,0) AS failures,
          COALESCE(p.hint_level,0)::smallint AS hint_level,COALESCE(p.listed,false) AS listed,p.first_win,
          floor(extract(epoch FROM c.earned_at))::bigint AS completed_at,c.live_week,
          COALESCE(c.tactician,false) AS tactician,COALESCE(c.first_try,false) AS first_try,
          (SELECT min(turn) FROM central_identity.challenge_attempts WHERE account_id=$1 AND challenge_id=$2 AND state='won') AS best_turn
         FROM (SELECT 1) seed LEFT JOIN central_identity.challenge_participation p ON p.account_id=$1 AND p.challenge_id=$2
         LEFT JOIN central_identity.challenge_completions c ON c.account_id=$1 AND c.challenge_id=$2",
        vec![me.into(), uuid(task).into()],
    )).await.map_err(unavailable)?.ok_or(Error::Unavailable)?;
    Ok(api::TaskProgress {
        challenge: task,
        starts: unsigned(&row, "starts")?,
        failures: unsigned(&row, "failures")?,
        hint_level: u8::try_from(get::<i16>(&row, "hint_level")?).map_err(unavailable)?,
        listed: get(&row, "listed")?,
        first_win: get::<Option<Uuid>>(&row, "first_win")?.map(|id| *id.as_bytes()),
        completed_at: get(&row, "completed_at")?,
        live_week: get(&row, "live_week")?,
        tactician: get(&row, "tactician")?,
        first_try: get(&row, "first_try")?,
        best_turn: get::<Option<i32>>(&row, "best_turn")?
            .map(u32::try_from)
            .transpose()
            .map_err(unavailable)?,
    })
}

pub(super) async fn export(
    db: &impl ConnectionTrait,
    me: Uuid,
    table: api::ExportTable,
    after: Option<[u8; 16]>,
) -> Result<Response, Error> {
    let query = match table {
        api::ExportTable::Participation => "SELECT challenge_id AS cursor FROM central_identity.challenge_participation
            WHERE account_id=$1 AND ($2::uuid IS NULL OR challenge_id>$2) ORDER BY challenge_id LIMIT 31",
        api::ExportTable::Attempts => "SELECT id AS cursor,id,challenge_id,kind,request_id,game_id,state,number,failures,assisted,live,turn,milliseconds,
            floor(extract(epoch FROM created_at))::bigint AS created_seconds,
            floor(extract(epoch FROM started_at))::bigint AS started_seconds,
            floor(extract(epoch FROM finished_at))::bigint AS finished_seconds,
            floor(extract(epoch FROM received_at))::bigint AS received_seconds
            FROM central_identity.challenge_attempts WHERE account_id=$1 AND ($2::uuid IS NULL OR id>$2) ORDER BY id LIMIT 31",
    };
    let rows = db
        .query_all_raw(statement(query, vec![me.into(), after.map(uuid).into()]))
        .await
        .map_err(unavailable)?;
    let more = rows.len() > api::PAGE_SIZE;
    let mut cursor = None;
    let mut entries = Vec::with_capacity(rows.len().min(api::PAGE_SIZE));
    for row in rows.iter().take(api::PAGE_SIZE) {
        let id = *get::<Uuid>(row, "cursor")?.as_bytes();
        cursor = Some(id);
        entries.push(match table {
            api::ExportTable::Participation => api::ExportEntry::Participation {
                progress: progress(db, me, id).await?,
            },
            api::ExportTable::Attempts => api::ExportEntry::Attempt {
                attempt: exported_attempt(row)?,
            },
        });
    }
    Ok(Response::Export {
        entries,
        after: if more { cursor } else { None },
    })
}

fn exported_attempt(row: &QueryResult) -> Result<api::ExportedAttempt, Error> {
    let kind: String = get(row, "kind")?;
    Ok(api::ExportedAttempt {
        id: *get::<Uuid>(row, "id")?.as_bytes(),
        challenge: *get::<Uuid>(row, "challenge_id")?.as_bytes(),
        kind: match kind.as_str() {
            "daily" => api::AttemptKind::Daily,
            "weekly" => api::AttemptKind::Weekly,
            "tower" => api::AttemptKind::Tower,
            _ => return Err(Error::Unavailable),
        },
        request_id: *get::<Uuid>(row, "request_id")?.as_bytes(),
        game: get::<Option<Uuid>>(row, "game_id")?.map(|id| *id.as_bytes()),
        state: state(row)?,
        number: unsigned(row, "number")?,
        failures: unsigned(row, "failures")?,
        assisted: get(row, "assisted")?,
        live: get(row, "live")?,
        turn: get::<Option<i32>>(row, "turn")?
            .map(u32::try_from)
            .transpose()
            .map_err(unavailable)?,
        milliseconds: get::<Option<i64>>(row, "milliseconds")?
            .map(u64::try_from)
            .transpose()
            .map_err(unavailable)?,
        created_at: get(row, "created_seconds")?,
        started_at: get(row, "started_seconds")?,
        finished_at: get(row, "finished_seconds")?,
        received_at: get(row, "received_seconds")?,
    })
}
