//! Independent named credentials on the very same synthetic, verified gateway.
use super::{ConnectionTrait, DatabaseConnection, DbBackend, Statement, Token, Uuid, api};
use central_postgres::{AccountStore, RankedHostCredential::Registered};
use sea_orm::TransactionTrait;
use std::time::Duration;

/// Revocation which already holds the token row wins even if the host started
/// a request before that revocation became visible to other transactions.
pub(super) async fn revocation_in_flight(
    store: &AccountStore,
    db: &DatabaseConnection,
    host: api::Id,
) -> Token {
    let token = Token::generate().unwrap();
    let digest = token.digest().to_vec();
    db.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "INSERT INTO central_identity.gateway_tokens(gateway_id,name,digest,sso,listing,ranked,expires_at)
         VALUES ($1,'In-flight revocation',$2,false,false,true,now()+interval '1 day')",
         vec![Uuid::from_bytes(host).into(),digest.clone().into()],
    )).await.unwrap();
    let tx = db.begin().await.unwrap();
    tx.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "UPDATE central_identity.gateway_tokens SET revoked_at=clock_timestamp() WHERE digest=$1",
        vec![digest.into()],
    ))
    .await
    .unwrap();
    {
        let ready = store.ranked_host(Registered(&token), api::HostRequest::Ready { capacity: 1 });
        tokio::pin!(ready);
        assert!(
            tokio::time::timeout(Duration::from_millis(100), &mut ready)
                .await
                .is_err()
        );
        tx.commit().await.unwrap();
        assert_eq!(ready.await.unwrap_err(), central_api::Error::Rejected);
    }
    token
}

pub(super) async fn named(db: &DatabaseConnection, host: api::Id) -> (Token, Vec<Token>) {
    let ranked = Token::generate().unwrap();
    let mut denied = Vec::new();
    for (name, sso, listing, has_ranked, expired, revoked) in [
        ("Ranked", false, false, true, false, false),
        ("Sign-in", true, false, false, false, false),
        ("Directory", false, true, false, false, false),
        ("Revoked", false, false, true, false, true),
        ("Expired", false, false, true, true, false),
    ] {
        let token = if name == "Ranked" {
            &ranked
        } else {
            denied.push(Token::generate().unwrap());
            denied.last().unwrap()
        };
        db.execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO central_identity.gateway_tokens
             (gateway_id,name,digest,sso,listing,ranked,created_at,expires_at,revoked_at)
             VALUES ($1,$2,$3,$4,$5,$6,now()-interval '2 days',
             CASE WHEN $7 THEN now()-interval '1 day' ELSE now()+interval '1 day' END,
             CASE WHEN $8 THEN now() ELSE NULL END)",
            vec![
                Uuid::from_bytes(host).into(),
                name.into(),
                token.digest().to_vec().into(),
                sso.into(),
                listing.into(),
                has_ranked.into(),
                expired.into(),
                revoked.into(),
            ],
        ))
        .await
        .unwrap();
    }
    (ranked, denied)
}
