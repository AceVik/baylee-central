//! Credential, canonical origin and the administrator's verification grant
//! are all held under lock until the host operation's transaction finishes.
use super::{Error, RankedHostCredential, column, host::Host, statement, unavailable};
use sea_orm::{
    ConnectionTrait, DatabaseTransaction, QueryResult,
    prelude::{TimeDateTimeWithTimeZone, Uuid},
};

pub(super) struct Authority {
    pub(super) host: Host,
    expires_at: Option<TimeDateTimeWithTimeZone>,
}

impl Authority {
    // A request may wait for another settlement. Time can expire a named token
    // even while its locked row prevents modifications or revocation.
    pub(super) async fn check_expiry(&self, db: &DatabaseTransaction) -> Result<(), Error> {
        if let Some(expires) = self.expires_at {
            let row = db
                .query_one_raw(statement(
                    "SELECT pg_catalog.clock_timestamp() < $1::timestamptz AS live",
                    vec![expires.into()],
                ))
                .await
                .map_err(unavailable)?
                .ok_or(Error::Unavailable)?;
            if !column::<bool>(&row, "live")? {
                return Err(Error::Rejected);
            }
        }
        Ok(())
    }
}

async fn registered(
    db: &DatabaseTransaction,
    token: &central_api::Token,
) -> Result<(QueryResult, Option<TimeDateTimeWithTimeZone>), Error> {
    let digest = token.digest();
    // Account deletion locks the gateway before cascading to its token rows.
    // Lock in that same order and recheck the digest on the exact token row.
    let gateway = db
        .query_one_raw(statement(
            "SELECT g.id,g.origin,g.credential_digest=$1 AS legacy
         FROM central_identity.gateways g WHERE NOT g.official AND
         (g.credential_digest=$1 OR g.id=(SELECT t.gateway_id FROM central_identity.gateway_tokens t
         WHERE t.digest=$1 AND t.ranked)) FOR SHARE OF g",
            vec![digest.to_vec().into()],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Rejected)?;
    if column::<Option<bool>>(&gateway, "legacy")? == Some(true) {
        return Ok((gateway, None));
    }
    let id: Uuid = column(&gateway, "id")?;
    let token = db
        .query_one_raw(statement(
            &format!(
                "SELECT t.expires_at FROM central_identity.gateway_tokens t
         WHERE t.gateway_id=$1 AND t.digest=$2 AND t.ranked AND {} FOR SHARE OF t",
                super::super::tokens::LIVE_TOKEN
            ),
            vec![id.into(), digest.to_vec().into()],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Rejected)?;
    Ok((gateway, Some(column(&token, "expires_at")?)))
}

pub(super) async fn authorize(
    db: &DatabaseTransaction,
    credential: RankedHostCredential<'_>,
) -> Result<Authority, Error> {
    let (gateway, expires_at) = match credential {
        RankedHostCredential::Registered(token) => registered(db, token).await?,
        RankedHostCredential::Official(origin) => {
            let gateway=db.query_one_raw(statement(
                "SELECT id,origin FROM central_identity.gateways WHERE official AND origin=$1 FOR SHARE",
                vec![origin.into()],
            )).await.map_err(unavailable)?.ok_or(Error::Rejected)?;
            (gateway, None)
        }
    };
    let id: Uuid = column(&gateway, "id")?;
    let grant=db.query_one_raw(statement(
        "SELECT ranked_epoch FROM central_identity.gateway_approvals WHERE gateway_id=$1 AND enabled AND ranked_verified FOR SHARE",
        vec![id.into()],
    )).await.map_err(unavailable)?.ok_or(Error::Rejected)?;
    Ok(Authority {
        host: Host {
            id,
            origin: column(&gateway, "origin")?,
            epoch: column(&grant, "ranked_epoch")?,
        },
        expires_at,
    })
}
