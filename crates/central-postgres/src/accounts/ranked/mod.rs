//! Central's 1v1 reservation ledger. Queue admission and routing never accept a player identity
//! or a host location from a player. The verified host alone freezes the engine game and result.

use super::{AccountStore, Error, Session, statement, uuid};
use central_api::ranked as api;
use sea_orm::{ConnectionTrait, DatabaseTransaction, QueryResult, TransactionTrait, prelude::Uuid};

pub(super) mod boards;
mod host;
mod matchmaking;
mod queue;
mod rules;
mod scoring;

fn unavailable(_: impl Sized) -> Error {
    Error::Unavailable
}

fn column<T: sea_orm::TryGetable>(row: &QueryResult, name: &str) -> Result<T, Error> {
    row.try_get("", name).map_err(unavailable)
}

async fn actor(db: &DatabaseTransaction, session: Session<'_>) -> Result<Uuid, Error> {
    // Share both rows through commit, permitting concurrent FK checks for the other seat.
    // The session kind, credential revision and expiry all have to agree on this connection.
    let row = db
        .query_one_raw(statement(
            "SELECT a.id FROM central_identity.accounts a JOIN central_identity.sessions s
         ON s.account_id=a.id AND s.credential_revision=a.credential_revision
         WHERE s.digest=$1 AND s.kind=$2 AND s.expires_at>pg_catalog.clock_timestamp()
         AND a.credential IS NOT NULL FOR SHARE OF a,s",
            vec![session.digest().to_vec().into(), session.kind().into()],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Rejected)?;
    column(&row, "id")
}

impl AccountStore {
    /// Read a public competition view or update the authenticated account's single queue ticket.
    ///
    /// # Errors
    /// Rejects invalid requests or sessions, concurrent incompatible tickets and storage failures.
    pub async fn ranked(
        &self,
        session: Option<Session<'_>>,
        request: api::Request,
    ) -> Result<api::Response, Error> {
        let db = self.db.begin().await.map_err(unavailable)?;
        let response = match request {
            api::Request::Join { .. } | api::Request::Status | api::Request::Cancel { .. } => {
                let me = actor(&db, session.ok_or(Error::Rejected)?).await?;
                api::Response::Queue {
                    state: queue::handle(&db, me, &request).await?,
                }
            }
            _ => boards::read(&db, &request).await?,
        };
        db.commit().await.map_err(unavailable)?;
        Ok(response)
    }

    /// Operate an assigned game as its authenticated gateway. The identity service supplies
    /// the origin from the credential. Verification, epoch and the reservation are rechecked
    /// under lock in the same transaction as every host mutation.
    ///
    /// # Errors
    /// Rejects unverified, stale or foreign hosts, conflicting retries and database failures.
    pub async fn ranked_host(
        &self,
        gateway: Option<api::Id>,
        origin: &str,
        request: api::HostRequest,
    ) -> Result<api::HostResponse, Error> {
        let db = self.db.begin().await.map_err(unavailable)?;
        let response = host::handle(&db, gateway, origin, &request).await?;
        db.commit().await.map_err(unavailable)?;
        Ok(response)
    }
}
