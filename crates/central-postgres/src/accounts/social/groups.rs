//! Membership transitions and ciphertext delivery share a group lock. Then all
//! affected account locks are taken in UUID order, before checking live sessions.
use super::{
    Error, Session, by_tag, caller, limits, lock_accounts, person, statement, unavailable,
};
use central_api::groups::{Change, GroupRequest, GroupResponse};
use sea_orm::{ConnectionTrait, prelude::Uuid};

mod change;
mod snapshot;

pub(super) struct Membership {
    account: Uuid,
    accepted: bool,
    inviter: Uuid,
}

pub(super) struct Locked {
    id: Uuid,
    owner: Uuid,
    members: Vec<Membership>,
}

impl Locked {
    fn member(&self, account: Uuid) -> Option<&Membership> {
        self.members.iter().find(|member| member.account == account)
    }

    fn joined(&self, account: Uuid) -> bool {
        self.member(account).is_some_and(|member| member.accepted)
    }

    pub(super) fn accounts(&self) -> Vec<Uuid> {
        self.members
            .iter()
            .filter(|member| member.accepted)
            .map(|member| member.account)
            .collect()
    }
}

pub(super) fn id(text: &str) -> Result<Uuid, Error> {
    let id = Uuid::parse_str(text).map_err(|_| Error::Rejected)?;
    if id.get_version_num() != 7 || id.as_bytes()[8] & 0xc0 != 0x80 || id.to_string() != text {
        return Err(Error::Rejected);
    }
    Ok(id)
}

pub(super) async fn lock(db: &impl ConnectionTrait, id: Uuid, me: Uuid) -> Result<Locked, Error> {
    let row = db
        .query_one_raw(statement(
            "SELECT owner FROM central_identity.chat_groups WHERE id = $1 FOR UPDATE",
            vec![id.into()],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Rejected)?;
    let owner = row.try_get("", "owner").map_err(unavailable)?;
    let rows = db.query_all_raw(statement(
        "SELECT account_id, accepted, inviter FROM central_identity.chat_members WHERE group_id = $1 ORDER BY account_id", vec![id.into()],
    )).await.map_err(unavailable)?;
    let members = rows
        .iter()
        .map(|row| {
            Ok(Membership {
                account: row.try_get("", "account_id").map_err(unavailable)?,
                accepted: row.try_get("", "accepted").map_err(unavailable)?,
                inviter: row.try_get("", "inviter").map_err(unavailable)?,
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    let group = Locked { id, owner, members };
    if group.member(me).is_none() {
        return Err(Error::Rejected);
    }
    Ok(group)
}

pub(super) async fn recipients(
    db: &impl ConnectionTrait,
    text: &str,
    me: Uuid,
) -> Result<Vec<Uuid>, Error> {
    let group = lock(db, id(text)?, me).await?;
    if !group.joined(me) {
        return Err(Error::Rejected);
    }
    Ok(group.accounts())
}

pub(super) async fn dispatch(
    db: &impl ConnectionTrait,
    session: Session<'_>,
    me: Uuid,
    request: GroupRequest,
) -> Result<GroupResponse, Error> {
    match request {
        GroupRequest::List => Ok(GroupResponse::Groups {
            snapshot: snapshot::read(db, me).await?,
        }),
        GroupRequest::Create { name } => create(db, session, me, &name).await,
        GroupRequest::Change { id: text, change } => {
            let group = lock(db, id(&text)?, me).await?;
            let target = match &change {
                Change::Invite { tag }
                | Change::Withdraw { tag }
                | Change::Remove { tag }
                | Change::Transfer { tag } => Some(
                    by_tag(db, *tag)
                        .await?
                        .ok_or(Error::Rejected)?
                        .try_get::<Uuid>("", "id")
                        .map_err(unavailable)?,
                ),
                Change::Rename { .. } | Change::Join | Change::Delete => None,
            };
            let mut accounts: Vec<Uuid> =
                group.members.iter().map(|member| member.account).collect();
            accounts.extend(target);
            lock_accounts(db, accounts).await?;
            caller(db, session, true).await?;
            change::apply(db, &group, me, target, change).await?;
            Ok(GroupResponse::Done)
        }
    }
}

async fn room(db: &impl ConnectionTrait, account: Uuid) -> Result<(), Error> {
    let count: i64 = db
        .query_one_raw(statement(
            "SELECT count(*) FROM central_identity.chat_members WHERE account_id = $1",
            vec![account.into()],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Unavailable)?
        .try_get("", "count")
        .map_err(unavailable)?;
    if count >= 20 {
        Err(Error::Busy)
    } else {
        Ok(())
    }
}

async fn create(
    db: &impl ConnectionTrait,
    session: Session<'_>,
    me: Uuid,
    name: &str,
) -> Result<GroupResponse, Error> {
    if !central_api::groups::valid_name(name) {
        return Err(Error::Rejected);
    }
    lock_accounts(db, vec![me]).await?;
    caller(db, session, true).await?;
    room(db, me).await?;
    limits::charge(db, me, limits::Budget::Request).await?;
    let id: Uuid = db
        .query_one_raw(statement(
            "INSERT INTO central_identity.chat_groups (owner, name) VALUES ($1, $2) RETURNING id",
            vec![me.into(), name.into()],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Unavailable)?
        .try_get("", "id")
        .map_err(unavailable)?;
    db.query_one_raw(statement(
        "INSERT INTO central_identity.chat_members (group_id, account_id, accepted, inviter) VALUES ($1, $2, true, $2) RETURNING id", vec![id.into(), me.into()],
    )).await.map_err(unavailable)?.ok_or(Error::Unavailable)?;
    Ok(GroupResponse::Created { id: id.to_string() })
}
