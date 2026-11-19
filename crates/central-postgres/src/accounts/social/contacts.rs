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

/// Whose own view of the pair a change altered: the caller's (`me`), the
/// other account's (`other`), both or neither. A block, and lifting one, is the
/// blocker's alone: the blocked account sees no list of who blocked it, only a
/// contact that ended, if there was one. `friendship`: a friendship began or
/// ended, the one change that alters what either side's presence shows. A
/// request, or a block of somebody who was no friend, is none, so it reads
/// nobody's friends and cannot be used to sound out whether the other side is
/// online.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Changed {
    pub(super) me: bool,
    pub(super) other: bool,
    pub(super) friendship: bool,
}

impl Changed {
    const BOTH: Self = Self {
        me: true,
        other: true,
        friendship: false,
    };
    const NEITHER: Self = Self {
        me: false,
        other: false,
        friendship: false,
    };

    fn or(self, more: Self) -> Self {
        Self {
            me: self.me || more.me,
            other: self.other || more.other,
            friendship: self.friendship || more.friendship,
        }
    }
}

pub(super) async fn change(
    db: &impl ConnectionTrait,
    me: Uuid,
    other: Uuid,
    action: ContactAction,
) -> Result<Changed, Error> {
    let (low, high) = if me < other { (me, other) } else { (other, me) };
    Ok(match action {
        ContactAction::Remove => remove(db, low, high).await?,
        ContactAction::Unblock => {
            let lifted = db
                .execute_raw(statement(
                    "DELETE FROM central_identity.social_blocks WHERE owner = $1 AND blocked = $2",
                    vec![me.into(), other.into()],
                ))
                .await
                .map_err(unavailable)?
                .rows_affected()
                > 0;
            Changed {
                me: lifted,
                ..Changed::NEITHER
            }
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
            let blocked = Changed {
                me: !already,
                ..Changed::NEITHER
            };
            blocked.or(remove(db, low, high).await?)
        }
        ContactAction::Request | ContactAction::Accept => {
            let blocked = db.query_one_raw(statement(
                "SELECT owner FROM central_identity.social_blocks WHERE (owner = $1 AND blocked = $2) OR (owner = $2 AND blocked = $1) LIMIT 1",
                vec![me.into(), other.into()],
            )).await.map_err(unavailable)?.is_some();
            if blocked {
                return Err(Error::Rejected);
            }
            request(db, me, other, low, high, action).await?
        }
    })
}

async fn request(
    db: &impl ConnectionTrait,
    me: Uuid,
    other: Uuid,
    low: Uuid,
    high: Uuid,
    action: ContactAction,
) -> Result<Changed, Error> {
    let old = db.query_one_raw(statement("SELECT requested_by, accepted FROM central_identity.social_contacts WHERE low = $1 AND high = $2", vec![low.into(), high.into()])).await.map_err(unavailable)?;
    if let Some(old) = old {
        if old.try_get::<bool>("", "accepted").map_err(unavailable)? {
            return Ok(Changed::NEITHER);
        }
        let from: Uuid = old.try_get("", "requested_by").map_err(unavailable)?;
        if from == me {
            return if action == ContactAction::Request {
                Ok(Changed::NEITHER)
            } else {
                Err(Error::Rejected)
            };
        }
        room(db, me, MAX_CONTACTS, FRIENDS).await?;
        room(db, other, MAX_CONTACTS, FRIENDS).await?;
        db.execute_raw(statement("UPDATE central_identity.social_contacts SET accepted = true WHERE low = $1 AND high = $2", vec![low.into(), high.into()])).await.map_err(unavailable)?;
        // The other's request accepted: a friendship begins.
        return Ok(Changed {
            friendship: true,
            ..Changed::BOTH
        });
    }
    if action == ContactAction::Accept {
        return Err(Error::Rejected);
    }
    room(db, me, MAX_CONTACTS, FRIENDS).await?;
    room(db, other, MAX_CONTACTS, FRIENDS).await?;
    room(db, me, MAX_OUTGOING, "SELECT count(*) FROM central_identity.social_contacts WHERE NOT accepted AND requested_by = $1").await?;
    room(db, other, MAX_INCOMING, "SELECT count(*) FROM central_identity.social_contacts WHERE NOT accepted AND requested_by <> $1 AND (low = $1 OR high = $1)").await?;
    limits::charge(db, me, limits::Budget::Request).await?;
    db.execute_raw(statement("INSERT INTO central_identity.social_contacts (low, high, requested_by) VALUES ($1, $2, $3)", vec![low.into(), high.into(), me.into()])).await.map_err(unavailable)?;
    // A request: both see it, and no friendship begins yet.
    Ok(Changed::BOTH)
}

/// The pair's contact ended, whatever it was (a request either way or a
/// friendship): both saw it, and a friendship ended if it was one. Nothing to
/// remove changed nothing.
async fn remove(db: &impl ConnectionTrait, low: Uuid, high: Uuid) -> Result<Changed, Error> {
    let removed = db
        .query_one_raw(statement(
            "DELETE FROM central_identity.social_contacts WHERE low = $1 AND high = $2
             RETURNING accepted",
            vec![low.into(), high.into()],
        ))
        .await
        .map_err(unavailable)?;
    let Some(row) = removed else {
        return Ok(Changed::NEITHER);
    };
    Ok(Changed {
        friendship: row.try_get::<bool>("", "accepted").map_err(unavailable)?,
        ..Changed::BOTH
    })
}
