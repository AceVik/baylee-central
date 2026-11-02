//! Central social state. Session ownership is checked at the database boundary.
use super::{AccountStore, Error, Session, statement};
use central_api::social::{Contact, Person, Relation, SocialRequest, SocialResponse};
use sea_orm::{ConnectionTrait, QueryResult, TransactionTrait, prelude::Uuid};

mod contacts;
mod limits;

fn unavailable(_: impl Sized) -> Error {
    Error::Unavailable
}

/// Rechecked after account locks on every mutation, so revocation cannot race a write.
async fn caller(
    db: &impl ConnectionTrait,
    session: Session<'_>,
    lock: bool,
) -> Result<Uuid, Error> {
    db.query_one_raw(statement(
        &format!(
            "SELECT a.id FROM central_identity.accounts a JOIN central_identity.sessions s
         ON s.account_id = a.id AND s.credential_revision = a.credential_revision
         WHERE s.digest = $1 AND s.kind = $2 AND s.expires_at > pg_catalog.clock_timestamp()
         AND a.credential IS NOT NULL AND a.display_name IS NOT NULL AND a.tag IS NOT NULL{}",
            if lock { " FOR SHARE OF s" } else { "" }
        ),
        vec![session.digest().to_vec().into(), session.kind().into()],
    ))
    .await
    .map_err(unavailable)?
    .ok_or(Error::Rejected)?
    .try_get("", "id")
    .map_err(unavailable)
}

fn person(row: &QueryResult) -> Result<Person, Error> {
    let tag: i32 = row.try_get("", "tag").map_err(unavailable)?;
    Ok(Person {
        tag: u32::try_from(tag).map_err(unavailable)?,
        display_name: row.try_get("", "display_name").map_err(unavailable)?,
    })
}

async fn by_tag(db: &impl ConnectionTrait, tag: u32) -> Result<Option<QueryResult>, Error> {
    let tag = i32::try_from(tag).map_err(|_| Error::Rejected)?;
    db.query_one_raw(statement(
        "SELECT id, tag, display_name FROM central_identity.accounts
         WHERE tag = $1 AND credential IS NOT NULL AND display_name IS NOT NULL",
        vec![tag.into()],
    ))
    .await
    .map_err(unavailable)
}

/// All social mutations lock affected accounts in ascending UUID order. This also
/// serializes quotas and account deletion without opposing-contact deadlocks.
async fn lock_accounts(db: &impl ConnectionTrait, ids: Vec<Uuid>) -> Result<(), Error> {
    db.query_all_raw(statement(
        "SELECT id FROM central_identity.accounts WHERE id = ANY($1) ORDER BY id FOR UPDATE",
        vec![ids.into()],
    ))
    .await
    .map_err(unavailable)?;
    Ok(())
}

impl AccountStore {
    /// Run an account-owned social operation on either authenticated surface.
    ///
    /// # Errors
    /// Rejects invalid sessions, self-actions and unauthorized transitions; bounds
    /// return `Busy`; database failure never substitutes empty contacts.
    pub async fn social(
        &self,
        session: Session<'_>,
        request: SocialRequest,
    ) -> Result<SocialResponse, Error> {
        let tx = self.db.begin().await.map_err(unavailable)?;
        let me = caller(&tx, session, false).await?;
        let response = match request {
            SocialRequest::Contacts => SocialResponse::Contacts {
                contacts: snapshot(&tx, me).await?,
            },
            SocialRequest::Lookup { tag } => {
                lock_accounts(&tx, vec![me]).await?;
                caller(&tx, session, true).await?;
                limits::charge(&tx, me, limits::Budget::Lookup).await?;
                let found = by_tag(&tx, tag).await?;
                SocialResponse::Person {
                    person: found.as_ref().map(person).transpose()?,
                }
            }
            SocialRequest::Contact { tag, change } => {
                let other = by_tag(&tx, tag).await?.ok_or(Error::Rejected)?;
                let other: Uuid = other.try_get("", "id").map_err(unavailable)?;
                if me == other {
                    return Err(Error::Rejected);
                }
                lock_accounts(&tx, vec![me, other]).await?;
                caller(&tx, session, true).await?;
                contacts::change(&tx, me, other, change).await?;
                SocialResponse::Done
            }
        };
        tx.commit().await.map_err(unavailable)?;
        Ok(response)
    }
}

async fn snapshot(db: &impl ConnectionTrait, me: Uuid) -> Result<Vec<Contact>, Error> {
    let rows = db.query_all_raw(statement(
        "SELECT a.tag, a.display_name, r.relation FROM (
            SELECT CASE WHEN low = $1 THEN high ELSE low END AS other,
                   CASE WHEN accepted THEN 'friend' WHEN requested_by = $1 THEN 'outgoing' ELSE 'incoming' END AS relation
            FROM central_identity.social_contacts WHERE low = $1 OR high = $1
            UNION ALL SELECT blocked AS other, 'blocked' FROM central_identity.social_blocks WHERE owner = $1
         ) r JOIN central_identity.accounts a ON a.id = r.other
         WHERE a.credential IS NOT NULL AND a.tag IS NOT NULL AND a.display_name IS NOT NULL
         ORDER BY a.tag LIMIT 1651", vec![me.into()],
    )).await.map_err(unavailable)?;
    if rows.len() > 1650 {
        return Err(Error::Unavailable);
    }
    rows.iter()
        .map(|row| {
            let relation = match row
                .try_get::<String>("", "relation")
                .map_err(unavailable)?
                .as_str()
            {
                "friend" => Relation::Friend,
                "incoming" => Relation::Incoming,
                "outgoing" => Relation::Outgoing,
                "blocked" => Relation::Blocked,
                _ => return Err(Error::Unavailable),
            };
            Ok(Contact {
                person: person(row)?,
                relation,
            })
        })
        .collect()
}
