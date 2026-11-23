//! Private campaign authority. The native session supplies the sole acting account.
use super::{AccountStore, Error, Session, statement, uuid};
use central_api::advent as api;
use sea_orm::{ConnectionTrait, DatabaseTransaction, QueryResult, TransactionTrait, prelude::Uuid};

pub(super) mod art;
mod doors;
mod inventory;
mod looks;
pub(super) mod table;

fn unavailable(_: impl Sized) -> Error {
    Error::Unavailable
}
fn col<T: sea_orm::TryGetable>(row: &QueryResult, name: &str) -> Result<T, Error> {
    row.try_get("", name).map_err(unavailable)
}

impl AccountStore {
    /// Run a session-bound door or inventory operation under one database snapshot.
    ///
    /// # Errors
    /// Rejects unauthorized, unavailable or expired content and conflicting selections.
    pub async fn advent(
        &self,
        session: Session<'_>,
        req: api::Request,
    ) -> Result<api::Response, Error> {
        if !matches!(session, Session::Native(_)) {
            return Err(Error::Rejected);
        }
        let write = matches!(
            req,
            api::Request::Submit { .. }
                | api::Request::Claim { .. }
                | api::Request::Present { .. }
                | api::Request::Table { .. }
        );
        let db = if write {
            self.db.begin().await
        } else {
            self.db
                .begin_with_config(
                    Some(sea_orm::IsolationLevel::RepeatableRead),
                    Some(sea_orm::AccessMode::ReadOnly),
                )
                .await
        }
        .map_err(unavailable)?;
        let me = caller(&db, session, write).await?;
        let response = match req {
            api::Request::CurrentCalendar {} => doors::current(&db, me).await?,
            api::Request::Calendar { year } => doors::calendar(&db, me, year).await?,
            api::Request::Puzzle { year, door } => doors::puzzle(&db, year, door).await?,
            api::Request::Hint { year, door, number } => {
                doors::hint(&db, year, door, number).await?
            }
            api::Request::Submit { year, door, line } => {
                doors::submit(&db, me, year, door, &line).await?
            }
            api::Request::Claim { year, door } => inventory::claim(&db, me, year, door).await?,
            api::Request::Inventory { after } => inventory::list(&db, me, after.map(uuid)).await?,
            api::Request::Asset {
                scope,
                offset,
                cached,
            } => art::get(&db, me, scope, offset, cached).await?,
            api::Request::Present { match_id, look } => {
                looks::present(&db, me, uuid(match_id), look).await?
            }
            api::Request::TableLooks { match_id } => {
                looks::visible(&db, me, uuid(match_id)).await?
            }
            api::Request::Table { request } => api::Response::Table {
                response: table::handle(&db, me, session, request).await?,
            },
        };
        db.commit().await.map_err(unavailable)?;
        Ok(response)
    }
}

// Concurrent claims use the same account lock as look registration. NO KEY
// UPDATE remains compatible with the rating ledger's foreign-key checks.
async fn caller(
    db: &DatabaseTransaction,
    session: Session<'_>,
    write: bool,
) -> Result<Uuid, Error> {
    let locks = if write {
        "FOR NO KEY UPDATE OF a FOR SHARE OF s"
    } else {
        ""
    };
    let row=db.query_one_raw(statement(&format!("SELECT a.id FROM central_identity.accounts a
        JOIN central_identity.sessions s ON s.account_id=a.id AND a.credential_revision=s.credential_revision
        WHERE s.digest=$1 AND s.kind='native' AND s.expires_at>clock_timestamp() AND a.credential IS NOT NULL {locks}"),
        vec![session.digest().to_vec().into()])).await.map_err(unavailable)?.ok_or(Error::Rejected)?;
    col(&row, "id")
}
