//! Global challenges: account locking, immutable publications and atomic result accounting.

use super::{AccountStore, Error, Session, statement, uuid};
use central_api::challenges::{self as api, Attempt, AttemptState, Challenge, Request, Response};
use sea_orm::{
    AccessMode, ConnectionTrait, DatabaseTransaction, IsolationLevel, QueryResult,
    TransactionTrait, prelude::Uuid,
};

mod finish;
mod host;
mod owned;
mod personal;
mod read;
mod tower;

fn unavailable(_: impl Sized) -> Error {
    Error::Unavailable
}

async fn now(db: &impl ConnectionTrait) -> Result<i64, Error> {
    db.query_one_raw(statement(
        "SELECT floor(extract(epoch FROM pg_catalog.clock_timestamp()))::bigint AS now",
        vec![],
    ))
    .await
    .map_err(unavailable)?
    .ok_or(Error::Unavailable)?
    .try_get("", "now")
    .map_err(unavailable)
}

/// A write locks the account before its session, matching account lifecycle operations.
async fn caller(
    db: &DatabaseTransaction,
    session: Session<'_>,
    write: bool,
) -> Result<Uuid, Error> {
    let find = |lock: bool| {
        statement(
            &format!(
                "SELECT a.id FROM central_identity.accounts a JOIN central_identity.sessions s
         ON s.account_id=a.id AND s.credential_revision=a.credential_revision
         WHERE s.digest=$1 AND s.kind=$2 AND s.expires_at>pg_catalog.clock_timestamp()
         AND a.credential IS NOT NULL{}",
                if lock { " FOR SHARE OF s" } else { "" }
            ),
            vec![session.digest().to_vec().into(), session.kind().into()],
        )
    };
    let row = db
        .query_one_raw(find(false))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Rejected)?;
    let account = row.try_get("", "id").map_err(unavailable)?;
    if write {
        lock_account(db, account).await?;
        let checked = db
            .query_one_raw(find(true))
            .await
            .map_err(unavailable)?
            .ok_or(Error::Rejected)?;
        if checked.try_get::<Uuid>("", "id").map_err(unavailable)? != account {
            return Err(Error::Rejected);
        }
    }
    Ok(account)
}

async fn lock_account(db: &impl ConnectionTrait, account: Uuid) -> Result<(), Error> {
    db.query_one_raw(statement(
        "SELECT id FROM central_identity.accounts WHERE id=$1 FOR UPDATE",
        vec![account.into()],
    ))
    .await
    .map_err(unavailable)?
    .ok_or(Error::Rejected)?;
    Ok(())
}

fn challenge(row: &QueryResult) -> Result<Challenge, Error> {
    let id: Uuid = row.try_get("", "id").map_err(unavailable)?;
    let description: String = row.try_get("", "description").map_err(unavailable)?;
    let digest: Vec<u8> = row.try_get("", "digest").map_err(unavailable)?;
    Ok(Challenge {
        id: *id.as_bytes(),
        description: central_api::decode(description.as_bytes()).map_err(unavailable)?,
        digest: digest.try_into().map_err(unavailable)?,
        withdrawn: row.try_get("", "withdrawn").map_err(unavailable)?,
    })
}

async fn task(db: &impl ConnectionTrait, id: [u8; 16], private: bool) -> Result<Challenge, Error> {
    let row = db.query_one_raw(statement(
            &format!("SELECT id,description::text,digest,withdrawn FROM central_identity.challenges
         WHERE id=$1 AND ($2 OR (scope=0 AND starts_at<=floor(extract(epoch FROM pg_catalog.clock_timestamp())))){}",
             if private { " FOR SHARE" } else { "" }),
        vec![uuid(id).into(), private.into()],
    )).await.map_err(unavailable)?.ok_or(Error::Rejected)?;
    challenge(&row)
}

fn state(row: &QueryResult) -> Result<AttemptState, Error> {
    let value: String = row.try_get("", "state").map_err(unavailable)?;
    match value.as_str() {
        "reserved" => Ok(AttemptState::Reserved),
        "playing" => Ok(AttemptState::Playing),
        "won" => Ok(AttemptState::Won),
        "lost" => Ok(AttemptState::Lost),
        "abandoned" => Ok(AttemptState::Abandoned),
        "void" => Ok(AttemptState::Void),
        _ => Err(Error::Unavailable),
    }
}

fn attempt(row: &QueryResult, host: &str) -> Result<Attempt, Error> {
    let id: Uuid = row.try_get("", "id").map_err(unavailable)?;
    let challenge: Uuid = row.try_get("", "challenge_id").map_err(unavailable)?;
    let game: Option<Uuid> = row.try_get("", "game_id").map_err(unavailable)?;
    let number: i32 = row.try_get("", "number").map_err(unavailable)?;
    let turn: Option<i32> = row.try_get("", "turn").map_err(unavailable)?;
    Ok(Attempt {
        id: *id.as_bytes(),
        challenge: *challenge.as_bytes(),
        state: state(row)?,
        number: u32::try_from(number).map_err(unavailable)?,
        host: host.to_owned(),
        game: game.map(|id| *id.as_bytes()),
        assisted: row.try_get("", "assisted").map_err(unavailable)?,
        live: row.try_get("", "live").map_err(unavailable)?,
        turn: turn.map(u32::try_from).transpose().map_err(unavailable)?,
    })
}

async fn attempt_row(
    db: &impl ConnectionTrait,
    id: [u8; 16],
    owner: Option<Uuid>,
) -> Result<QueryResult, Error> {
    db.query_one_raw(statement(
        "SELECT * FROM central_identity.challenge_attempts WHERE id=$1 AND ($2::uuid IS NULL OR account_id=$2)",
        vec![uuid(id).into(), owner.into()],
    )).await.map_err(unavailable)?.ok_or(Error::Rejected)
}

impl AccountStore {
    /// Read public challenge data or perform an operation owned by the authenticated account.
    /// `host` is the service's configured official origin, never a request field.
    ///
    /// # Errors
    /// Invalid/foreign sessions, future tasks, conflicting starts, bounds or database failure.
    pub async fn challenges(
        &self,
        session: Option<Session<'_>>,
        request: Request,
        host: &str,
    ) -> Result<Response, Error> {
        let public = matches!(
            request,
            Request::Catalogue { .. }
                | Request::Detail { .. }
                | Request::Tower {
                    request: central_api::tower::Request::Read { .. }
                }
        );
        let write = matches!(
            request,
            Request::Begin { .. }
                | Request::Abandon { .. }
                | Request::Hint { .. }
                | Request::Visibility { .. }
                | Request::Tower {
                    request: central_api::tower::Request::Begin { .. }
                        | central_api::tower::Request::Start { .. }
                        | central_api::tower::Request::Choose { .. }
                        | central_api::tower::Request::Abandon { .. }
                        | central_api::tower::Request::Visibility { .. }
                }
        );
        let db = if write {
            self.db.begin().await
        } else {
            self.db
                .begin_with_config(
                    Some(IsolationLevel::RepeatableRead),
                    Some(AccessMode::ReadOnly),
                )
                .await
        }
        .map_err(unavailable)?;
        let me = match session {
            Some(session) if public => match caller(&db, session, false).await {
                Ok(account) => Some(account),
                Err(Error::Rejected) => None,
                Err(error) => return Err(error),
            },
            Some(session) => Some(caller(&db, session, write).await?),
            None if public => None,
            None => return Err(Error::Rejected),
        };
        let response = match request {
            Request::Tower { request } => Response::Tower {
                response: Box::new(tower::dispatch(&db, me, request, host).await?),
            },
            Request::Catalogue { before } => read::catalogue(&db, before).await?,
            Request::Detail {
                challenge,
                board,
                offset,
            } => read::detail(&db, challenge, board, offset, me).await?,
            other => owned::dispatch(&db, me.ok_or(Error::Rejected)?, other, host).await?,
        };
        db.commit().await.map_err(unavailable)?;
        Ok(response)
    }

    /// Official bridge's challenge operations. The identity service must verify its bridge
    /// credential before calling this server-only entry; ordinary gateway auth is insufficient.
    ///
    /// # Errors
    /// Invalid publication, conflicting claims/results, bounds or database failure.
    pub async fn official_challenges(
        &self,
        request: api::HostRequest,
        origin: &str,
    ) -> Result<api::HostResponse, Error> {
        let db = self.db.begin().await.map_err(unavailable)?;
        let response = host::dispatch(&db, request, origin).await?;
        db.commit().await.map_err(unavailable)?;
        Ok(response)
    }
}
