//! Every dataset is account-scoped and bounded independently, including retry metadata.
use super::{api, id_valid, load, statement, unavailable, uuid};
use central_api::{Error, challenges::Period};
use sea_orm::{ConnectionTrait, QueryResult, TryGetable, prelude::Uuid};

pub(super) async fn page(
    db: &impl ConnectionTrait,
    me: Uuid,
    table: api::ExportTable,
    after: Option<[u8; 16]>,
    origin: &str,
) -> Result<api::Response, Error> {
    if let Some(id) = after {
        id_valid(id)?;
    }
    let sql = match table {
        api::ExportTable::Runs => {
            "SELECT r.id, r.request_id,
            floor(extract(epoch FROM r.created_at)*1000000)::bigint AS created,
            CASE WHEN s.run_id IS NULL THEN NULL ELSE jsonb_build_object(
                'floors',s.floors,'losses',s.losses,'turns',s.turns,'milliseconds',s.milliseconds)
            END::text AS ranked
            FROM central_identity.tower_runs r LEFT JOIN central_identity.tower_scores s
            ON s.run_id=r.id AND s.account_id=r.account_id
            WHERE r.account_id=$1 AND ($2::uuid IS NULL OR r.id>$2) ORDER BY r.id LIMIT 11"
        }
        api::ExportTable::Actions => {
            "SELECT request_id, run_id, request::text,
            floor(extract(epoch FROM created_at)*1000000)::bigint AS created
            FROM central_identity.tower_actions WHERE account_id=$1
            AND ($2::uuid IS NULL OR request_id>$2) ORDER BY request_id LIMIT 11"
        }
        api::ExportTable::Participation => {
            "SELECT p.season_id,p.listed,s.starts_at,s.ends_at,
            COALESCE((SELECT jsonb_agg(jsonb_build_object('floor',a.floor,'flawless',a.flawless)
                ORDER BY a.floor) FROM central_identity.tower_accomplishments a
                WHERE a.account_id=p.account_id AND a.season_id=p.season_id),'[]'::jsonb)::text AS earned
            FROM central_identity.tower_participation p JOIN central_identity.tower_seasons s
            ON s.id=p.season_id WHERE p.account_id=$1
            AND ($2::uuid IS NULL OR p.season_id>$2) ORDER BY p.season_id LIMIT 11"
        }
    };
    let mut rows = db
        .query_all_raw(statement(sql, vec![me.into(), after.map(uuid).into()]))
        .await
        .map_err(unavailable)?;
    let more = rows.len() > 10;
    if more {
        rows.pop();
    }
    let mut entries = Vec::with_capacity(rows.len());
    for row in rows {
        entries.push(match table {
            api::ExportTable::Runs => {
                let id: Uuid = field(&row, "id")?;
                let ranked: Option<String> = field(&row, "ranked")?;
                api::ExportEntry::Run {
                    run: Box::new(load(db, me, *id.as_bytes(), origin).await?),
                    request_id: *field::<Uuid>(&row, "request_id")?.as_bytes(),
                    created_at_micros: field(&row, "created")?,
                    ranked: ranked
                        .map(|value| central_api::decode(value.as_bytes()))
                        .transpose()
                        .map_err(unavailable)?,
                }
            }
            api::ExportTable::Actions => api::ExportEntry::Action {
                request_id: *field::<Uuid>(&row, "request_id")?.as_bytes(),
                run: *field::<Uuid>(&row, "run_id")?.as_bytes(),
                request: central_api::decode(field::<String>(&row, "request")?.as_bytes())
                    .map_err(unavailable)?,
                created_at_micros: field(&row, "created")?,
            },
            api::ExportTable::Participation => api::ExportEntry::Participation {
                season: *field::<Uuid>(&row, "season_id")?.as_bytes(),
                period: Period {
                    kind: central_api::challenges::Kind::Weekly,
                    starts_at: field(&row, "starts_at")?,
                    ends_at: field(&row, "ends_at")?,
                },
                listed: field(&row, "listed")?,
                accomplishments: central_api::decode(field::<String>(&row, "earned")?.as_bytes())
                    .map_err(unavailable)?,
            },
        });
    }
    let after = if more {
        entries.last().map(|entry| entry.identity().1)
    } else {
        None
    };
    Ok(api::Response::Export { entries, after })
}

fn field<T: TryGetable>(row: &QueryResult, name: &str) -> Result<T, Error> {
    row.try_get("", name).map_err(unavailable)
}
