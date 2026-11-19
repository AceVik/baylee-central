//! What the native push socket (`GET /v1/ws`, `docs/push-socket.md`) asks of the
//! store: which of the sessions it serves still stand, and who an account's
//! friends are. Nothing here writes; presence itself is never stored.
use super::{AccountStore, Error, statement, uuid};
use central_api::social::MAX_CONTACTS;
use sea_orm::{ConnectionTrait, prelude::Uuid};
use std::collections::BTreeSet;

/// The most session digests one check names: more than the push sockets one
/// listener holds at once (`central-http`'s `SOCKETS`), so the periodic check
/// of all of them is one statement.
pub const LIVE_CHECK: usize = 1_024;

impl AccountStore {
    /// Of `digests`, the sessions that stand now: not ended or replaced, not
    /// expired, and of an account whose credential is the one they were issued
    /// under. Exactly what `SessionStatus` asks ([`Self::session_status`]),
    /// which the client's own check reads, so a socket is never told its
    /// session ended while that check would still find it. One statement
    /// however many are named; a digest that is nobody's is simply absent (a
    /// browser session is stored under a digest of its own domain).
    ///
    /// # Errors
    /// `Rejected` for more than [`LIVE_CHECK`] digests; storage failure, which
    /// never reads as an ended session.
    pub async fn live_sessions(&self, digests: &[[u8; 32]]) -> Result<BTreeSet<[u8; 32]>, Error> {
        if digests.is_empty() {
            return Ok(BTreeSet::new());
        }
        if digests.len() > LIVE_CHECK {
            return Err(Error::Rejected);
        }
        let named: Vec<Vec<u8>> = digests.iter().map(|digest| digest.to_vec()).collect();
        let rows = self
            .db
            .query_all_raw(statement(
                "SELECT s.digest FROM central_identity.accounts a
                 JOIN central_identity.sessions s
                 ON s.account_id = a.id AND s.credential_revision = a.credential_revision
                 WHERE s.digest = ANY($1) AND s.expires_at > pg_catalog.clock_timestamp()",
                vec![named.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        rows.iter()
            .map(|row| {
                let digest: Vec<u8> = row.try_get("", "digest").map_err(|_| Error::Unavailable)?;
                digest.try_into().map_err(|_| Error::Unavailable)
            })
            .collect()
    }

    /// The accounts `account` is friends with: the accepted contacts, either
    /// side having asked. A block removes the contact, so a blocked account is
    /// never among them. At most [`MAX_CONTACTS`], which every acceptance checks.
    ///
    /// # Errors
    /// Storage failure, or more rows than an account can hold, which never
    /// reads as no friends.
    pub async fn live_friends(&self, account: [u8; 16]) -> Result<Vec<[u8; 16]>, Error> {
        let rows = self
            .db
            .query_all_raw(statement(
                "SELECT CASE WHEN low = $1 THEN high ELSE low END AS other
                 FROM central_identity.social_contacts
                 WHERE accepted AND (low = $1 OR high = $1) LIMIT $2",
                vec![uuid(account).into(), limit()?.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        if rows.len() > MAX_CONTACTS {
            return Err(Error::Unavailable);
        }
        rows.iter()
            .map(|row| {
                let other: Uuid = row.try_get("", "other").map_err(|_| Error::Unavailable)?;
                Ok(other.into_bytes())
            })
            .collect()
    }
}

/// One row more than an account can hold, so that too many show.
fn limit() -> Result<i64, Error> {
    i64::try_from(MAX_CONTACTS + 1).map_err(|_| Error::Unavailable)
}

/// What changing the social graph concerns, as the push socket addresses it:
/// the accounts whose own view changed, each named once. A socket of an
/// account that is not named hears nothing of the change.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Concerns {
    /// Accounts at least one of whose devices received a delivery (`chat`).
    pub chat: BTreeSet<[u8; 16]>,
    /// Accounts whose contacts, requests, blocks, groups, invitations or own
    /// devices changed (`social`).
    pub social: BTreeSet<[u8; 16]>,
    /// Two accounts whose friendship began or ended: their friend lists are
    /// read again if they hold a socket. Never set by a request, or by a block
    /// or its lifting between accounts that were no friends.
    pub pair: Option<([u8; 16], [u8; 16])>,
}

impl Concerns {
    pub(super) fn social_of(accounts: impl IntoIterator<Item = Uuid>) -> Self {
        Self {
            social: accounts.into_iter().map(Uuid::into_bytes).collect(),
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_friends_query_reads_one_row_past_the_bound() {
        assert_eq!(
            super::limit(),
            Ok(i64::try_from(central_api::social::MAX_CONTACTS).unwrap() + 1)
        );
    }
}
