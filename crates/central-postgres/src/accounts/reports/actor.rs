//! Who reads or changes reports: an administrator by their browser session,
//! or a report token with the right the act needs. Either is checked in the
//! transaction that acts, and what it did names it as what it was.
use super::super::report_tokens::{Hold, ReportToken, authenticated};
use super::{AccountStore, Error, Token};
use central_api::report_tokens::ReportScope;
use sea_orm::{DatabaseTransaction, prelude::Uuid};

/// Who asks: never both kinds by one credential, since a session's digest
/// and a token's are looked for in their own tables alone.
#[derive(Clone, Copy)]
pub enum ReportCaller<'a> {
    /// An administrator, by the browser session's cookie.
    Admin(&'a Token),
    /// A report token.
    Token(&'a Token),
}

/// Who acts, as the acting transaction found them.
pub(super) struct Actor {
    /// The administrator's account, or the report token.
    pub(super) id: Uuid,
    /// A report token's name now; an administrator has none here.
    pub(super) token_name: Option<String>,
}

impl From<&ReportToken> for Actor {
    fn from(token: &ReportToken) -> Self {
        Self {
            id: Uuid::from_bytes(token.id),
            token_name: Some(token.name.clone()),
        }
    }
}

impl Actor {
    /// The caller in `tx`, if it may do what `right` names. An administrator
    /// may do all of it, and purges only with a password sign-in less than
    /// 30 minutes old; a report token does what it holds, and what it
    /// changes holds it ([`Hold::ToTheEnd`]).
    ///
    /// # Errors
    /// `Rejected` for anyone else; storage failure.
    pub(super) async fn check(
        tx: &DatabaseTransaction,
        caller: ReportCaller<'_>,
        right: ReportScope,
    ) -> Result<Self, Error> {
        match caller {
            ReportCaller::Admin(browser) => {
                let (id, recent) = AccountStore::administrator(tx, browser).await?;
                if right == ReportScope::Purge && !recent {
                    return Err(Error::Rejected);
                }
                Ok(Self {
                    id,
                    token_name: None,
                })
            }
            ReportCaller::Token(token) => {
                let hold = if right == ReportScope::Read {
                    Hold::Now
                } else {
                    Hold::ToTheEnd
                };
                Ok(Self::from(&authenticated(tx, token, right, hold).await?))
            }
        }
    }
}
