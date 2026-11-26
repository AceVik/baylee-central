//! Central's 1v1 reservation ledger. Queue admission and routing never accept a player identity
//! or a host location from a player. The verified host alone freezes the engine game and result.

use super::{AccountStore, Error, Session, statement, uuid};
use central_api::ranked as api;
use sea_orm::{ConnectionTrait, DatabaseTransaction, QueryResult, TransactionTrait, prelude::Uuid};

mod ai;
pub(super) mod boards;
mod credentials;
mod export;
mod fill_in;
mod host;
mod matchmaking;
pub(super) mod privacy;
mod queue;
pub use queue::CLOSED_TICKET_HOURS;
pub(super) use queue::sweep as sweep_tickets;
pub(super) mod replication;
mod rules;
mod scoring;

/// Host identity evidence supplied by the authenticated service.
#[derive(Clone, Copy)]
pub enum RankedHostCredential<'a> {
    /// Configured official origin after verification of the service's bridge secret.
    Official(&'a str),
    /// Registered full gateway credential or explicitly ranked-scoped token.
    Registered(&'a central_api::Token),
}

fn unavailable(_: impl Sized) -> Error {
    Error::Unavailable
}

fn column<T: sea_orm::TryGetable>(row: &QueryResult, name: &str) -> Result<T, Error> {
    row.try_get("", name).map_err(unavailable)
}

async fn actor(db: &DatabaseTransaction, session: Session<'_>, write: bool) -> Result<Uuid, Error> {
    // Share both rows through commit, permitting concurrent FK checks for the other seat.
    // The session kind, credential revision and expiry all have to agree on this connection.
    let row = db
        .query_one_raw(statement(
            &format!(
                "SELECT a.id FROM central_identity.accounts a JOIN central_identity.sessions s
         ON s.account_id=a.id AND s.credential_revision=a.credential_revision
         WHERE s.digest=$1 AND s.kind=$2 AND s.expires_at>pg_catalog.clock_timestamp()
         AND a.credential IS NOT NULL AND a.restored AND (a.legacy_id IS NULL OR a.legacy_closed)
         {}",
                if write { "FOR SHARE OF a,s" } else { "" }
            ),
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
        if let api::Request::PreviewDeck { format, pool } = &request {
            let db = self
                .db
                .begin_with_config(
                    Some(sea_orm::IsolationLevel::RepeatableRead),
                    Some(sea_orm::AccessMode::ReadOnly),
                )
                .await
                .map_err(unavailable)?;
            let me = actor(&db, session.ok_or(Error::Rejected)?, false).await?;
            let preview = scoring::preview(&db, me, *format, pool).await?;
            db.commit().await.map_err(unavailable)?;
            return Ok(api::Response::DeckPreview { preview });
        }
        if let api::Request::Export { table, after } = request {
            // One snapshot for the page and the rows its parents own.
            let db = self
                .db
                .begin_with_config(
                    Some(sea_orm::IsolationLevel::RepeatableRead),
                    Some(sea_orm::AccessMode::ReadOnly),
                )
                .await
                .map_err(unavailable)?;
            let me = export::owner(&db, session.ok_or(Error::Rejected)?).await?;
            let response = export::page(&db, me, table, after).await?;
            db.commit().await.map_err(unavailable)?;
            return Ok(response);
        }
        let db = self.db.begin().await.map_err(unavailable)?;
        let response = match request {
            api::Request::Join { .. } | api::Request::Status | api::Request::Cancel { .. } => {
                let me = actor(&db, session.ok_or(Error::Rejected)?, true).await?;
                api::Response::Queue {
                    state: queue::handle(&db, me, &request, false, api::ai::AiOpponents::None)
                        .await?,
                }
            }
            api::Request::Ai { request } => {
                let Some(session @ Session::Native(_)) = session else {
                    return Err(Error::Rejected);
                };
                let me = actor(&db, session, true).await?;
                api::Response::Ai {
                    response: ai::handle(&db, me, session.digest(), &request).await?,
                }
            }
            api::Request::Replicate { request } => {
                let Some(session @ Session::Native(_)) = session else {
                    return Err(Error::Rejected);
                };
                let me = actor(&db, session, true).await?;
                api::Response::Replica {
                    response: replication::seat(&db, me, request).await?,
                }
            }
            api::Request::OwnProfile { .. } => {
                let me = actor(&db, session.ok_or(Error::Rejected)?, false).await?;
                boards::read(&db, &request, Some(me)).await?
            }
            _ => boards::read(&db, &request, None).await?,
        };
        db.commit().await.map_err(unavailable)?;
        Ok(response)
    }

    /// Operate an assigned game as the configured official bridge or a registered host.
    /// The transaction resolves the registered credential to its gateway and locks the
    /// verification grant, scope and reservation before accepting the operation.
    ///
    /// # Errors
    /// Rejects unverified, stale or foreign hosts, conflicting retries and database failures.
    pub async fn ranked_host(
        &self,
        credential: RankedHostCredential<'_>,
        request: api::HostRequest,
    ) -> Result<api::HostResponse, Error> {
        let db = self.db.begin().await.map_err(unavailable)?;
        let authority = credentials::authorize(&db, credential).await?;
        let response = host::handle(&db, &authority.host, &request).await?;
        authority.check_expiry(&db).await?;
        db.commit().await.map_err(unavailable)?;
        Ok(response)
    }
}
