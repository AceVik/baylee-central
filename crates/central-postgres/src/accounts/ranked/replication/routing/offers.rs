//! Allocate verified capacity under the same locks for ordinary games, mirrored
//! games and standby replacements. The operator of the official host is NULL.
use super::{Error, Host, column, statement, unavailable};
use sea_orm::{ConnectionTrait, DatabaseTransaction, QueryResult, prelude::Uuid};

const OFFERS: &str = "SELECT h.host_id,g.origin,g.owner_id,h.ranked_epoch,h.compatibility,
   load.games::float8 / NULLIF(h.capacity,0) AS pressure,load.games,h.seen_at
 FROM central_identity.ranked_hosts h JOIN central_identity.gateways g ON g.id=h.host_id
 JOIN central_identity.gateway_approvals p ON p.gateway_id=g.id
 CROSS JOIN LATERAL (SELECT (
    (SELECT count(*) FROM central_identity.ranked_matches m WHERE m.host_id=h.host_id
        AND m.ranked_epoch=h.ranked_epoch AND m.state IN ('reserved','playing')
        AND m.lease_until>clock_timestamp() AND (m.created_at>=h.continuity_since OR EXISTS
         (SELECT 1 FROM central_identity.ranked_replicas r WHERE r.match_id=m.id))) +
    (SELECT count(*) FROM central_identity.ranked_replicas r
        JOIN central_identity.ranked_matches m ON m.id=r.match_id WHERE r.standby_id=h.host_id
        AND r.standby_epoch=h.ranked_epoch AND m.state IN ('reserved','playing')
        AND m.lease_until>clock_timestamp())) AS games) load
 WHERE h.capacity>load.games AND p.enabled AND p.ranked_verified AND p.ranked_epoch=h.ranked_epoch
   AND h.seen_at>clock_timestamp()-interval '90 seconds'";

const ORDER: &str = "pressure,games,seen_at DESC,host_id";

pub(in crate::accounts::ranked) struct Allocation {
    pub(in crate::accounts::ranked) primary: Host,
    pub(in crate::accounts::ranked) replica: Option<(Host, Vec<u8>)>,
}

fn host(row: &QueryResult) -> Result<Host, Error> {
    Ok(Host {
        id: column(row, "host_id")?,
        epoch: column(row, "ranked_epoch")?,
        origin: column(row, "origin")?,
    })
}

// All allocators lock capacities in host UUID order, then re-read committed
// reservations in a new statement. A busy slot can be retried on the next poll.
async fn lock(
    db: &DatabaseTransaction,
    first: Uuid,
    second: Option<Uuid>,
) -> Result<Vec<QueryResult>, Error> {
    let ids = || vec![first.into(), second.into()];
    let locked = db
        .query_all_raw(statement(
            "SELECT h.host_id FROM central_identity.ranked_hosts h
         JOIN central_identity.gateways g ON g.id=h.host_id
         JOIN central_identity.gateway_approvals p ON p.gateway_id=h.host_id
         WHERE h.host_id IN ($1,$2) ORDER BY h.host_id
         FOR UPDATE OF h SKIP LOCKED FOR SHARE OF p,g SKIP LOCKED",
            ids(),
        ))
        .await
        .map_err(unavailable)?;
    if locked.len() != 1 + usize::from(second.is_some()) {
        return Ok(Vec::new());
    }
    db.query_all_raw(statement(
        &format!(
            "{OFFERS} AND h.host_id IN ($1,$2)
         ORDER BY (h.host_id=$1) DESC"
        ),
        ids(),
    ))
    .await
    .map_err(unavailable)
}

pub(in crate::accounts::ranked) async fn allocate(
    db: &DatabaseTransaction,
    replicated: bool,
) -> Result<Option<Allocation>, Error> {
    let pick = if replicated {
        format!("WITH offers AS ({OFFERS} AND h.compatibility IS NOT NULL),
          owners AS (SELECT DISTINCT ON (compatibility,owner_id) * FROM offers
              ORDER BY compatibility,owner_id,{ORDER}),
          ranked AS (SELECT *,row_number() OVER(PARTITION BY compatibility ORDER BY {ORDER}) AS r FROM owners),
          build AS (SELECT compatibility FROM ranked WHERE r=2 ORDER BY {ORDER} LIMIT 1)
          SELECT host_id FROM (
             SELECT host_id,r FROM ranked JOIN build USING(compatibility) WHERE r<=2
             UNION ALL (SELECT host_id,1 FROM offers WHERE NOT EXISTS(SELECT 1 FROM build)
                         ORDER BY {ORDER} LIMIT 1)) chosen ORDER BY r")
    } else {
        format!("{OFFERS} ORDER BY {ORDER} LIMIT 1")
    };
    let selected = db
        .query_all_raw(statement(&pick, vec![]))
        .await
        .map_err(unavailable)?;
    let Some(first) = selected.first() else {
        return Ok(None);
    };
    let second = selected
        .get(1)
        .map(|r| column::<Uuid>(r, "host_id"))
        .transpose()?;
    let rows = lock(db, column(first, "host_id")?, second).await?;
    match rows.as_slice() {
        [primary]
            if second.is_none()
                && (!replicated
                    || column::<Option<Vec<u8>>>(primary, "compatibility")?.is_some()) =>
        {
            Ok(Some(Allocation {
                primary: host(primary)?,
                replica: None,
            }))
        }
        [primary, standby] if replicated => {
            let build: Option<Vec<u8>> = column(primary, "compatibility")?;
            let Some(build) = build else { return Ok(None) };
            if column::<Option<Vec<u8>>>(standby, "compatibility")?.as_ref() != Some(&build)
                || column::<Option<Uuid>>(primary, "owner_id")?
                    == column::<Option<Uuid>>(standby, "owner_id")?
            {
                return Ok(None);
            }
            Ok(Some(Allocation {
                primary: host(primary)?,
                replica: Some((host(standby)?, build)),
            }))
        }
        _ => Ok(None),
    }
}

pub(super) async fn replacement(
    db: &DatabaseTransaction,
    primary: Uuid,
    exclude: Option<Uuid>,
    build: &[u8],
) -> Result<Option<Host>, Error> {
    let pick = format!(
        "{OFFERS} AND h.compatibility=$1 AND h.host_id<>$2 AND h.host_id IS DISTINCT FROM $3
       AND h.seen_at>clock_timestamp()-interval '45 seconds' AND g.owner_id IS DISTINCT FROM
        (SELECT owner_id FROM central_identity.gateways WHERE id=$2) ORDER BY {ORDER} LIMIT 1"
    );
    let row = db
        .query_one_raw(statement(
            &pick,
            vec![build.to_vec().into(), primary.into(), exclude.into()],
        ))
        .await
        .map_err(unavailable)?;
    let Some(row) = row else { return Ok(None) };
    let locked = lock(db, column(&row, "host_id")?, None).await?;
    let Some(row) = locked.first() else {
        return Ok(None);
    };
    if column::<Option<Vec<u8>>>(row, "compatibility")?.as_deref() != Some(build) {
        return Ok(None);
    }
    let independent = db
        .query_one_raw(statement(
            "SELECT id FROM central_identity.gateways
        WHERE id=$1 AND owner_id IS DISTINCT FROM $2::uuid FOR SHARE SKIP LOCKED",
            vec![
                primary.into(),
                column::<Option<Uuid>>(row, "owner_id")?.into(),
            ],
        ))
        .await
        .map_err(unavailable)?;
    if independent.is_none() {
        return Ok(None);
    }
    host(row).map(Some)
}
