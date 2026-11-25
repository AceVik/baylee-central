use super::{Change, ConnectionTrait, Error, Locked, Uuid, limits, room, statement, unavailable};
use std::collections::BTreeSet;

/// Whose view of the group a change altered, for the push socket: what each
/// member sees is the group snapshot (`snapshot.rs`). A joined member sees
/// the name, the members and the invited; an invitee only the group's name
/// and its own inviter. So the joined members see every change, an invitee
/// a rename, the group's end and the end of its own invitation, and nobody
/// outside the group anything.
struct Seen<'a> {
    group: &'a Locked,
    accounts: BTreeSet<Uuid>,
}

impl<'a> Seen<'a> {
    fn joined(group: &'a Locked) -> Self {
        Self {
            group,
            accounts: group.accounts().into_iter().collect(),
        }
    }

    /// The invitees too.
    fn everyone(group: &'a Locked) -> Self {
        Self {
            group,
            accounts: group.members.iter().map(|member| member.account).collect(),
        }
    }

    fn and(mut self, account: Uuid) -> Self {
        self.accounts.insert(account);
        self
    }

    /// The invitees `inviter` invited, whose invitation ends with its membership.
    fn invited_by(mut self, inviter: Uuid) -> Self {
        self.accounts.extend(
            self.group
                .members
                .iter()
                .filter(|member| !member.accepted && member.inviter == inviter)
                .map(|member| member.account),
        );
        self
    }
}

/// Apply `change`, and say whose view it altered: nobody's when it changed
/// nothing (an invitation that stands, a join already made).
pub(super) async fn apply(
    db: &impl ConnectionTrait,
    group: &Locked,
    me: Uuid,
    target: Option<Uuid>,
    change: Change,
) -> Result<BTreeSet<Uuid>, Error> {
    let is_owner = group.owner == me;
    let is_admin = group.is_admin(me);
    let seen = match change {
        Change::Rename { name } if is_admin => rename(db, group, &name).await?,
        Change::SetDescription { description } if is_admin => {
            set_description(db, group, &description).await?
        }
        Change::RemovePicture if is_admin => {
            if remove_picture(db, group.id).await? {
                Seen::everyone(group)
            } else {
                return Ok(BTreeSet::new());
            }
        }
        Change::SetAdmin { admin, .. } if is_admin => {
            let target = target.ok_or(Error::Rejected)?;
            if set_admin(db, group, target, admin).await? {
                Seen::joined(group)
            } else {
                return Ok(BTreeSet::new());
            }
        }
        Change::Invite { .. } if group.joined(me) => {
            let target = target.ok_or(Error::Rejected)?;
            if invite(db, group, me, target).await? {
                Seen::joined(group).and(target)
            } else {
                return Ok(BTreeSet::new());
            }
        }
        Change::Join => {
            if join(db, group, me).await? {
                Seen::joined(group).and(me)
            } else {
                return Ok(BTreeSet::new());
            }
        }
        Change::Withdraw { .. } => {
            let target = target.ok_or(Error::Rejected)?;
            let member = group.member(target).ok_or(Error::Rejected)?;
            if member.accepted || !(me == target || me == member.inviter || is_admin) {
                return Err(Error::Rejected);
            }
            remove(db, group.id, target).await?;
            Seen::joined(group).and(target)
        }
        Change::Remove { .. } => {
            let target = target.ok_or(Error::Rejected)?;
            let member = group.member(target).ok_or(Error::Rejected)?;
            let allowed = me == target || is_owner || (is_admin && !member.admin);
            if !member.accepted || target == group.owner || !allowed {
                return Err(Error::Rejected);
            }
            remove(db, group.id, target).await?;
            Seen::joined(group).invited_by(target)
        }
        Change::Transfer { .. } if is_owner => {
            let target = target.ok_or(Error::Rejected)?;
            transfer(db, group, target).await?;
            Seen::joined(group)
        }
        Change::Delete if is_owner => {
            // Only membership metadata is deleted. Retained encrypted history has no group FK.
            db.execute_raw(statement(
                "DELETE FROM central_identity.chat_groups WHERE id = $1",
                vec![group.id.into()],
            ))
            .await
            .map_err(unavailable)?;
            Seen::everyone(group)
        }
        _ => return Err(Error::Rejected),
    };
    Ok(seen.accounts)
}

async fn rename<'a>(
    db: &impl ConnectionTrait,
    group: &'a Locked,
    name: &str,
) -> Result<Seen<'a>, Error> {
    if !central_api::groups::valid_name(name) {
        return Err(Error::Rejected);
    }
    db.execute_raw(statement(
        "UPDATE central_identity.chat_groups SET name = $2 WHERE id = $1",
        vec![group.id.into(), name.into()],
    ))
    .await
    .map_err(unavailable)?;
    Ok(Seen::everyone(group))
}

async fn set_description<'a>(
    db: &impl ConnectionTrait,
    group: &'a Locked,
    description: &str,
) -> Result<Seen<'a>, Error> {
    if !central_api::groups::valid_description(description) {
        return Err(Error::Rejected);
    }
    db.execute_raw(statement(
        "UPDATE central_identity.chat_groups SET description = $2 WHERE id = $1",
        vec![group.id.into(), description.into()],
    ))
    .await
    .map_err(unavailable)?;
    Ok(Seen::everyone(group))
}

async fn remove_picture(db: &impl ConnectionTrait, group_id: Uuid) -> Result<bool, Error> {
    let removed = db
        .execute_raw(statement(
            "DELETE FROM central_identity.chat_group_pictures WHERE group_id = $1",
            vec![group_id.into()],
        ))
        .await
        .map_err(unavailable)?
        .rows_affected();
    Ok(removed > 0)
}

async fn set_admin(
    db: &impl ConnectionTrait,
    group: &Locked,
    target: Uuid,
    admin: bool,
) -> Result<bool, Error> {
    let member = group.member(target).ok_or(Error::Rejected)?;
    if !member.accepted || target == group.owner {
        return Err(Error::Rejected);
    }
    if member.admin == admin {
        return Ok(false);
    }
    db.execute_raw(statement(
        "UPDATE central_identity.chat_members SET admin = $3 WHERE group_id = $1 AND account_id = $2",
        vec![group.id.into(), target.into(), admin.into()],
    ))
    .await
    .map_err(unavailable)?;
    Ok(true)
}

async fn join(db: &impl ConnectionTrait, group: &Locked, me: Uuid) -> Result<bool, Error> {
    let member = group.member(me).ok_or(Error::Rejected)?;
    // Joined already: nothing changes.
    if member.accepted {
        return Ok(false);
    }
    let blocked = db
        .query_one_raw(statement(
            "SELECT 1 FROM central_identity.social_blocks
             WHERE (owner = $1 AND blocked = $2) OR (owner = $2 AND blocked = $1)",
            vec![me.into(), member.inviter.into()],
        ))
        .await
        .map_err(unavailable)?
        .is_some();
    if blocked {
        return Err(Error::Rejected);
    }
    // Once joined, membership must survive deletion of the original inviter.
    db.execute_raw(statement(
        "UPDATE central_identity.chat_members
         SET accepted = true, inviter = account_id, created_at = pg_catalog.clock_timestamp()
         WHERE group_id = $1 AND account_id = $2",
        vec![group.id.into(), me.into()],
    ))
    .await
    .map_err(unavailable)?;
    Ok(true)
}

async fn transfer(db: &impl ConnectionTrait, group: &Locked, target: Uuid) -> Result<(), Error> {
    if !group.joined(target) {
        return Err(Error::Rejected);
    }
    db.execute_raw(statement(
        "UPDATE central_identity.chat_groups SET owner = $2 WHERE id = $1",
        vec![group.id.into(), target.into()],
    ))
    .await
    .map_err(unavailable)?;
    db.execute_raw(statement(
        "UPDATE central_identity.chat_members SET admin = true WHERE group_id = $1 AND account_id = $2",
        vec![group.id.into(), target.into()],
    ))
    .await
    .map_err(unavailable)?;
    Ok(())
}

async fn invite(
    db: &impl ConnectionTrait,
    group: &Locked,
    me: Uuid,
    other: Uuid,
) -> Result<bool, Error> {
    if me == other {
        return Err(Error::Rejected);
    }
    let allowed = db
        .query_one_raw(statement(
            "SELECT 1 FROM central_identity.accounts a
             WHERE a.id = $2
               AND a.credential IS NOT NULL
               AND a.display_name IS NOT NULL
               AND a.tag IS NOT NULL
               AND NOT EXISTS (
                   SELECT 1 FROM central_identity.social_blocks b
                   WHERE (b.owner = $1 AND b.blocked = $2)
                      OR (b.owner = $2 AND b.blocked = $1)
               )",
            vec![me.into(), other.into()],
        ))
        .await
        .map_err(unavailable)?
        .is_some();
    if !allowed {
        return Err(Error::Rejected);
    }
    if group.member(other).is_some() {
        return Ok(false);
    }
    if group.members.len() >= central_api::groups::MAX_MEMBERS {
        return Err(Error::Busy);
    }
    room(db, other).await?;
    limits::charge(db, me, limits::Budget::Request).await?;
    db.query_one_raw(statement(
        "INSERT INTO central_identity.chat_members (group_id, account_id, inviter) VALUES ($1, $2, $3) RETURNING id",
        vec![group.id.into(), other.into(), me.into()],
    ))
    .await
    .map_err(unavailable)?
    .ok_or(Error::Unavailable)?;
    Ok(true)
}

async fn remove(db: &impl ConnectionTrait, group: Uuid, account: Uuid) -> Result<(), Error> {
    // Outstanding invitations made by a departing member cannot later grant access.
    db.execute_raw(statement(
        "DELETE FROM central_identity.chat_members WHERE group_id = $1 AND (account_id = $2 OR (NOT accepted AND inviter = $2))", vec![group.into(), account.into()],
    )).await.map_err(unavailable)?;
    Ok(())
}
