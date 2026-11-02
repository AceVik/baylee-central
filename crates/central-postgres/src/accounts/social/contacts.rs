use super::{Error, limits, statement, unavailable};
use central_api::social::{ContactAction, MAX_BLOCKS, MAX_CONTACTS, MAX_INCOMING, MAX_OUTGOING};
use sea_orm::{ConnectionTrait, prelude::Uuid};

async fn count(db: &impl ConnectionTrait, sql: &str, me: Uuid) -> Result<usize, Error> {
    let row = db
        .query_one_raw(statement(sql, vec![me.into()]))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Unavailable)?;
    let count: i64 = row.try_get("", "count").map_err(unavailable)?;
    usize::try_from(count).map_err(unavailable)
}

async fn room(db: &impl ConnectionTrait, me: Uuid, cap: usize, sql: &str) -> Result<(), Error> {
    if count(db, sql, me).await? < cap {
        Ok(())
    } else {
        Err(Error::Busy)
    }
}

const FRIENDS: &str = "SELECT count(*) FROM central_identity.social_contacts WHERE accepted AND (low = $1 OR high = $1)";

pub(super) async fn change(
    db: &impl ConnectionTrait,
    me: Uuid,
    other: Uuid,
    action: ContactAction,
) -> Result<(), Error> {
    let (low, high) = if me < other { (me, other) } else { (other, me) };
    match action {
        ContactAction::Remove => {
            remove(db, low, high).await?;
        }
        ContactAction::Unblock => {
            db.execute_raw(statement(
                "DELETE FROM central_identity.social_blocks WHERE owner = $1 AND blocked = $2",
                vec![me.into(), other.into()],
            ))
            .await
            .map_err(unavailable)?;
        }
        ContactAction::Block => {
            let already = db.query_one_raw(statement("SELECT owner FROM central_identity.social_blocks WHERE owner = $1 AND blocked = $2", vec![me.into(), other.into()])).await.map_err(unavailable)?.is_some();
            if !already {
                room(
                    db,
                    me,
                    MAX_BLOCKS,
                    "SELECT count(*) FROM central_identity.social_blocks WHERE owner = $1",
                )
                .await?;
                db.execute_raw(statement(
                    "INSERT INTO central_identity.social_blocks (owner, blocked) VALUES ($1, $2)",
                    vec![me.into(), other.into()],
                ))
                .await
                .map_err(unavailable)?;
            }
            remove(db, low, high).await?;
        }
        ContactAction::Request | ContactAction::Accept => {
            let blocked = db.query_one_raw(statement(
                "SELECT owner FROM central_identity.social_blocks WHERE (owner = $1 AND blocked = $2) OR (owner = $2 AND blocked = $1) LIMIT 1",
                vec![me.into(), other.into()],
            )).await.map_err(unavailable)?.is_some();
            if blocked {
                return Err(Error::Rejected);
            }
            request(db, me, other, low, high, action).await?;
        }
    }
    Ok(())
}

async fn request(
    db: &impl ConnectionTrait,
    me: Uuid,
    other: Uuid,
    low: Uuid,
    high: Uuid,
    action: ContactAction,
) -> Result<(), Error> {
    let old = db.query_one_raw(statement("SELECT requested_by, accepted FROM central_identity.social_contacts WHERE low = $1 AND high = $2", vec![low.into(), high.into()])).await.map_err(unavailable)?;
    if let Some(old) = old {
        if old.try_get::<bool>("", "accepted").map_err(unavailable)? {
            return Ok(());
        }
        let from: Uuid = old.try_get("", "requested_by").map_err(unavailable)?;
        if from == me {
            return if action == ContactAction::Request {
                Ok(())
            } else {
                Err(Error::Rejected)
            };
        }
        room(db, me, MAX_CONTACTS, FRIENDS).await?;
        room(db, other, MAX_CONTACTS, FRIENDS).await?;
        db.execute_raw(statement("UPDATE central_identity.social_contacts SET accepted = true WHERE low = $1 AND high = $2", vec![low.into(), high.into()])).await.map_err(unavailable)?;
    } else {
        if action == ContactAction::Accept {
            return Err(Error::Rejected);
        }
        room(db, me, MAX_CONTACTS, FRIENDS).await?;
        room(db, other, MAX_CONTACTS, FRIENDS).await?;
        room(db, me, MAX_OUTGOING, "SELECT count(*) FROM central_identity.social_contacts WHERE NOT accepted AND requested_by = $1").await?;
        room(db, other, MAX_INCOMING, "SELECT count(*) FROM central_identity.social_contacts WHERE NOT accepted AND requested_by <> $1 AND (low = $1 OR high = $1)").await?;
        limits::charge(db, me, limits::Budget::Request).await?;
        db.execute_raw(statement("INSERT INTO central_identity.social_contacts (low, high, requested_by) VALUES ($1, $2, $3)", vec![low.into(), high.into(), me.into()])).await.map_err(unavailable)?;
    }
    Ok(())
}

async fn remove(db: &impl ConnectionTrait, low: Uuid, high: Uuid) -> Result<(), Error> {
    db.execute_raw(statement(
        "DELETE FROM central_identity.social_contacts WHERE low = $1 AND high = $2",
        vec![low.into(), high.into()],
    ))
    .await
    .map_err(unavailable)?;
    Ok(())
}
