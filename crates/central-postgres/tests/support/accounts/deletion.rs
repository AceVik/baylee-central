//! The store's half of deleting and exporting an account
//! (`docs/account-deletion.md`): who may delete, what one deletion takes with
//! it under the runtime's own grants, what it leaves, the races it loses on
//! purpose, and what an export holds.
use super::{
    AccountStore, Error, Identity, ServerKeys, TicketPurpose, Token, config, enroll, login,
};
use central_api::{admin::AccountState, export::EXPORT_LIST, gateway::GatewayMetadata};
use central_postgres::Session;
use sea_orm::{
    ConnectionTrait, DatabaseConnection, DbBackend, Statement, TransactionTrait, Value,
    prelude::Uuid,
};
use std::time::Duration;

async fn sql(db: &impl ConnectionTrait, query: &str, values: Vec<Value>) {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        query,
        values,
    ))
    .await
    .unwrap();
}

async fn count(db: &impl ConnectionTrait, query: &str, values: Vec<Value>) -> i64 {
    db.query_one_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        query,
        values,
    ))
    .await
    .unwrap()
    .unwrap()
    .try_get("", "n")
    .unwrap()
}

fn id(identity: Identity) -> Value {
    Uuid::from_bytes(identity.account).into()
}

/// A moved account, restored and closed, with a native session.
async fn moved(store: &AccountStore, keys: &ServerKeys, name: &str) -> (Identity, Token) {
    let identity = enroll(store, keys, name, name).await;
    let native = login(store, keys, identity).await;
    store.confirm_restore(&native, 1).await.unwrap();
    store.close_legacy(name, identity).await.unwrap();
    (identity, native)
}

/// A registered and approved gateway of the session's account.
async fn gateway(
    store: &AccountStore,
    operator: &DatabaseConnection,
    session: &Token,
    origin: &str,
) -> [u8; 16] {
    let gateway = store
        .register_gateway(
            session,
            origin,
            &GatewayMetadata {
                name: "Synthetic gateway".into(),
                version: "1.0-test".into(),
                website: format!("{origin}/status"),
            },
            &Token::generate().unwrap(),
            &Token::generate().unwrap(),
            false,
        )
        .await
        .unwrap();
    sql(
        operator,
        "INSERT INTO central_identity.gateway_approvals(gateway_id, enabled) VALUES ($1, true)",
        vec![Uuid::from_bytes(gateway).into()],
    )
    .await;
    gateway
}

async fn admin_revision(operator: &DatabaseConnection, gateway: [u8; 16]) -> i64 {
    count(
        operator,
        "SELECT admin_revision AS n FROM central_identity.gateways WHERE id = $1",
        vec![Uuid::from_bytes(gateway).into()],
    )
    .await
}

pub(super) async fn run(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    refusals(store, operator, keys).await;
    appointed_while_waiting(store, operator, keys).await;
    replaced_while_waiting(store, operator, keys).await;
    everything(store, operator, keys).await;
}

/// Who may not delete: an open move, an administrator, a session of the
/// other kind or of another account, an unknown token, and a credential or
/// epoch changed since the proof began. Each changes nothing.
async fn refusals(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    let open = enroll(store, keys, "deletion-open", "deletion-open").await;
    let session = login(store, keys, open).await;
    assert_eq!(
        store.deletion_start(Session::Native(&session)).await.err(),
        Some(Error::Rejected),
        "an open move"
    );
    let (identity, native) = moved(store, keys, "deletion-refused").await;
    let (credential, gateways) = store
        .deletion_start(Session::Native(&native))
        .await
        .unwrap();
    assert_eq!(credential.identity.account, identity.account);
    assert!(gateways.is_empty());
    for session in [
        Session::Browser(&native),
        Session::Native(&Token::generate().unwrap()),
    ] {
        assert_eq!(
            store.deletion_start(session).await.err(),
            Some(Error::Rejected)
        );
        assert_eq!(
            store.delete_account(session, &credential).await.err(),
            Some(Error::Rejected)
        );
    }
    // Another account's session with this account's credential.
    let (_, stranger) = moved(store, keys, "deletion-stranger").await;
    assert_eq!(
        store
            .delete_account(Session::Native(&stranger), &credential)
            .await
            .err(),
        Some(Error::Rejected)
    );
    for stale in [
        central_postgres::Credential {
            identity: credential.identity,
            revision: credential.revision + 1,
            record: credential.record.clone(),
        },
        central_postgres::Credential {
            identity: Identity {
                account: identity.account,
                epoch: identity.epoch + 1,
            },
            revision: credential.revision,
            record: credential.record.clone(),
        },
    ] {
        assert_eq!(
            store
                .delete_account(Session::Native(&native), &stale)
                .await
                .err(),
            Some(Error::Rejected)
        );
    }
    // An administrator is refused at the start and, appointed after it, at the finish.
    central_postgres::appoint_admin(operator, "deletion-refused")
        .await
        .unwrap();
    assert_eq!(
        store.deletion_start(Session::Native(&native)).await.err(),
        Some(Error::Rejected)
    );
    assert_eq!(
        store
            .delete_account(Session::Native(&native), &credential)
            .await
            .err(),
        Some(Error::Rejected)
    );
    // An administrator's export says so, with the operator's appointment.
    let export = store
        .account_export(Session::Native(&native))
        .await
        .unwrap();
    assert!(export.admin);
    assert_eq!(export.audit.len(), 1);
    assert_eq!(export.audit[0].action, "account.admin_granted");
    assert_eq!(export.audit[0].actor, None);
    for (account, what) in [(open, "the open move"), (identity, "the administrator")] {
        assert_eq!(
            count(
                operator,
                "SELECT count(*) AS n FROM central_identity.accounts WHERE id = $1",
                vec![id(account)],
            )
            .await,
            1,
            "{what} stays"
        );
    }
    store.session_status(&native).await.unwrap();
}

/// A deletion that waited on the account's lock while the account was made
/// an administrator deletes nothing: the role is checked after the lock.
async fn appointed_while_waiting(
    store: &AccountStore,
    operator: &DatabaseConnection,
    keys: &ServerKeys,
) {
    let observer =
        central_postgres::connect_dev(config("central_runtime", "CENTRAL_TEST_RUNTIME_PASSWORD"))
            .await
            .unwrap();
    let (identity, native) = moved(store, keys, "deletion-appointed").await;
    let (credential, _) = store
        .deletion_start(Session::Native(&native))
        .await
        .unwrap();
    // The role's insert holds the account row's key-share lock until it commits.
    let tx = operator.begin().await.unwrap();
    sql(
        &tx,
        "INSERT INTO central_identity.account_roles (account_id, role) VALUES ($1, 'admin')",
        vec![id(identity)],
    )
    .await;
    let waiting = store.clone();
    let waiter = tokio::spawn(async move {
        waiting
            .delete_account(Session::Native(&native), &credential)
            .await
    });
    super::revocation_race::wait_for_account_lock(&observer).await;
    tx.commit().await.unwrap();
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(5), waiter)
            .await
            .unwrap()
            .unwrap(),
        Err(Error::Rejected)
    );
    assert_eq!(
        count(
            operator,
            "SELECT count(*) AS n FROM central_identity.account_roles WHERE account_id = $1",
            vec![id(identity)],
        )
        .await,
        1,
        "the administrator and its role stay"
    );
    observer.close().await.unwrap();
}

/// A deletion that waited on the account's lock while its session was
/// replaced deletes nothing: the session is checked again after the lock.
async fn replaced_while_waiting(
    store: &AccountStore,
    operator: &DatabaseConnection,
    keys: &ServerKeys,
) {
    let observer =
        central_postgres::connect_dev(config("central_runtime", "CENTRAL_TEST_RUNTIME_PASSWORD"))
            .await
            .unwrap();
    let (identity, native) = moved(store, keys, "deletion-replaced").await;
    let (credential, _) = store
        .deletion_start(Session::Native(&native))
        .await
        .unwrap();
    let tx = operator.begin().await.unwrap();
    sql(
        &tx,
        "SELECT id FROM central_identity.accounts WHERE id = $1 FOR UPDATE",
        vec![id(identity)],
    )
    .await;
    let waiting = store.clone();
    let waiter = tokio::spawn(async move {
        waiting
            .delete_account(Session::Native(&native), &credential)
            .await
    });
    super::revocation_race::wait_for_account_lock(&observer).await;
    sql(
        &tx,
        "DELETE FROM central_identity.sessions WHERE account_id = $1",
        vec![id(identity)],
    )
    .await;
    tx.commit().await.unwrap();
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(5), waiter)
            .await
            .unwrap()
            .unwrap(),
        Err(Error::Rejected)
    );
    assert_eq!(
        count(
            operator,
            "SELECT count(*) AS n FROM central_identity.accounts WHERE id = $1",
            vec![id(identity)],
        )
        .await,
        1
    );
    observer.close().await.unwrap();
}

/// An account with a row in every table that can name it, and the account
/// owning the other gateway it administers.
struct Filled {
    identity: Identity,
    native: Token,
    other_session: Token,
    own: [u8; 16],
    foreign: [u8; 16],
}

async fn fill(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) -> Filled {
    let (identity, native) = moved(store, keys, "deletion-all").await;
    let (other, other_session) = moved(store, keys, "deletion-owner").await;
    let credential = store.credential(identity).await.unwrap().unwrap();
    let browser = Token::generate().unwrap();
    store
        .issue_session(&credential, Session::Browser(&browser))
        .await
        .unwrap();
    // Its own gateway, with the other account on its list and a lease on it.
    let own = gateway(store, operator, &native, "https://deletion-own.invalid").await;
    store
        .set_gateway_admins(&native, own, 1, &[other.account])
        .await
        .unwrap();
    let ticket = Token::generate().unwrap();
    store
        .registered_ticket(
            &native,
            "https://deletion-own.invalid",
            &ticket,
            TicketPurpose::Admit,
        )
        .await
        .unwrap();
    store
        .redeem_lease(
            &ticket,
            "https://deletion-own.invalid",
            TicketPurpose::Admit,
            Some(own),
            Some(&Token::generate().unwrap()),
        )
        .await
        .unwrap();
    // The other account's gateway, with this one on its list and a ticket
    // outstanding for it.
    let foreign = gateway(
        store,
        operator,
        &other_session,
        "https://deletion-foreign.invalid",
    )
    .await;
    store
        .set_gateway_admins(&other_session, foreign, 1, &[identity.account])
        .await
        .unwrap();
    store
        .registered_ticket(
            &native,
            "https://deletion-foreign.invalid",
            &Token::generate().unwrap(),
            TicketPurpose::Admit,
        )
        .await
        .unwrap();
    rows(operator, &id(identity)).await;
    Filled {
        identity,
        native,
        other_session,
        own,
        foreign,
    }
}

/// The rows a store call cannot easily make: objects, a reset code, an
/// archive, an acceptance and an audit event.
async fn rows(operator: &DatabaseConnection, account: &Value) {
    sql(
        operator,
        "INSERT INTO central_identity.private_objects (account_id, revision, document)
         VALUES ($1, 1, decode(repeat('ab', 41), 'hex')), ($1, 0, NULL)",
        vec![account.clone()],
    )
    .await;
    sql(
        operator,
        "INSERT INTO central_identity.private_objects (account_id, revision, deleted)
         VALUES ($1, 2, true)",
        vec![account.clone()],
    )
    .await;
    sql(
        operator,
        "UPDATE central_identity.accounts SET object_bytes = 41,
         reset_digest = decode(repeat('11', 32), 'hex'),
         reset_expires = pg_catalog.clock_timestamp() + interval '1 day' WHERE id = $1",
        vec![account.clone()],
    )
    .await;
    sql(
        operator,
        "INSERT INTO central_identity.migration_archives
             (account_id, vault_epoch, credential, wrapped_master, document, document_revision)
         VALUES ($1, 9, '\\x01', decode(repeat('02', 73), 'hex'), decode(repeat('03', 41), 'hex'), 1)",
        vec![account.clone()],
    )
    .await;
    sql(
        operator,
        "INSERT INTO central_identity.terms_acceptances (account_id, subject, version, digest)
         VALUES ($1, 'central', 'synthetic-1', decode(repeat('44', 32), 'hex'))",
        vec![account.clone()],
    )
    .await;
    sql(
        operator,
        "INSERT INTO central_identity.audit_events (actor_kind, action, account_id)
         VALUES ('operator', 'account.reset_issued', $1)",
        vec![account.clone()],
    )
    .await;
    // One it acted in, as an administrator would, about nobody.
    sql(
        operator,
        "INSERT INTO central_identity.audit_events (actor_kind, actor_account, action)
         VALUES ('admin', $1, 'account.admin_removed')",
        vec![account.clone()],
    )
    .await;
}

/// Its export holds what Central stores, and its deletion, under the
/// runtime's grants, leaves no row naming the account or its gateway, apart
/// from the audit.
async fn everything(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    let Filled {
        identity,
        native,
        other_session,
        own,
        foreign,
    } = fill(store, operator, keys).await;
    let account = id(identity);
    export(store, operator, &native, identity, own, foreign).await;
    let foreign_revision = admin_revision(operator, foreign).await;
    // The deletion itself.
    let (credential, gateways) = store
        .deletion_start(Session::Native(&native))
        .await
        .unwrap();
    assert_eq!(gateways.len(), 1);
    assert_eq!(
        (
            gateways[0].id,
            gateways[0].origin.as_str(),
            gateways[0].approved
        ),
        (own, "https://deletion-own.invalid", true)
    );
    let digests: Vec<Value> = operator
        .query_all_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT digest FROM central_identity.sessions WHERE account_id = $1",
            vec![account.clone()],
        ))
        .await
        .unwrap()
        .iter()
        .map(|row| row.try_get::<Vec<u8>>("", "digest").unwrap().into())
        .collect();
    assert_eq!(digests.len(), 2, "both kinds");
    // Every table the deletion must empty holds a row first.
    let named = naming(operator, &account, own, &digests).await;
    for column in [
        "accounts.id",
        "sessions.account_id",
        "gateways.owner_id",
        "gateways.id",
        "gateway_approvals.gateway_id",
        "gateway_admins.account_id",
        "gateway_admins.gateway_id",
        "gateway_leases.gateway_id",
        "private_objects.account_id",
        "migration_archives.account_id",
        "terms_acceptances.account_id",
        "gateway_tickets.session_digest",
        "gateway_leases.session_digest",
    ] {
        assert!(
            named.iter().any(|(name, n)| name == column && *n > 0),
            "{column}: {named:?}"
        );
    }
    store
        .delete_account(Session::Native(&native), &credential)
        .await
        .unwrap();
    // No column of any table names the account or its gateway any more,
    // apart from the audit, which names ids only.
    for (column, n) in naming(operator, &account, own, &digests).await {
        assert_eq!(n, 0, "{column}");
    }
    assert_eq!(
        count(
            operator,
            "SELECT count(*) AS n FROM central_identity.audit_events WHERE account_id = $1",
            vec![account.clone()],
        )
        .await,
        1,
        "the audit stays"
    );
    // The other owner rereads its list; its gateway and session stay.
    assert_eq!(
        admin_revision(operator, foreign).await,
        foreign_revision + 1
    );
    store.session_status(&other_session).await.unwrap();
    assert!(store.discover("deletion-all").await.unwrap().is_none());
    assert_eq!(
        store.session_status(&native).await.err(),
        Some(Error::Rejected)
    );
}

/// How many rows of each column name the account, its gateway or its
/// sessions: every uuid column of the schema (apart from the audit and who
/// granted a role, which keep ids on purpose), and the session digests.
async fn naming(
    operator: &DatabaseConnection,
    account: &Value,
    own: [u8; 16],
    digests: &[Value],
) -> Vec<(String, i64)> {
    let columns = operator
        .query_all_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT table_name, column_name FROM information_schema.columns
             WHERE table_schema = 'central_identity' AND data_type = 'uuid'
             AND table_name <> 'audit_events' AND NOT (table_name = 'account_roles' AND column_name = 'granted_by')",
        ))
        .await
        .unwrap();
    assert!(columns.len() >= 12, "{}", columns.len());
    let mut named = Vec::new();
    for column in &columns {
        let table: String = column.try_get("", "table_name").unwrap();
        let name: String = column.try_get("", "column_name").unwrap();
        let n = count(
            operator,
            &format!(
                "SELECT count(*) AS n FROM central_identity.{table} WHERE {name} = $1 OR {name} = $2"
            ),
            vec![account.clone(), Uuid::from_bytes(own).into()],
        )
        .await;
        named.push((format!("{table}.{name}"), n));
    }
    for (table, column) in [
        ("sessions", "digest"),
        ("gateway_tickets", "session_digest"),
        ("gateway_leases", "session_digest"),
    ] {
        let mut n = 0;
        for digest in digests {
            n += count(
                operator,
                &format!("SELECT count(*) AS n FROM central_identity.{table} WHERE {column} = $1"),
                vec![digest.clone()],
            )
            .await;
        }
        named.push((format!("{table}.{column}"), n));
    }
    named
}

/// The export of the account `everything` filled, field by field; then a
/// stale sign-in is refused and a list past its bound is cut.
async fn export(
    store: &AccountStore,
    operator: &DatabaseConnection,
    native: &Token,
    identity: Identity,
    own: [u8; 16],
    foreign: [u8; 16],
) {
    let export = store.account_export(Session::Native(native)).await.unwrap();
    assert_eq!(export.account, identity.account);
    assert_eq!(export.username, "deletion-all");
    assert_eq!(export.legacy_id.as_deref(), Some("deletion-all"));
    assert_eq!(export.source, central_api::wire::AccountSource::Legacy);
    assert_eq!(export.state, AccountState::Restored);
    assert_eq!(
        (export.vault_epoch, export.document_revision),
        (identity.epoch, 1)
    );
    assert!(!export.admin && !export.recovery);
    assert!(export.reset_until.is_some());
    assert!(export.last_sign_in_at.is_some());
    assert_eq!(
        (export.objects, export.object_bytes, export.archives),
        (2, 41, 1)
    );
    assert_eq!(export.sessions.len(), 2);
    assert_eq!(export.terms.len(), 1);
    assert_eq!(
        (
            export.terms[0].subject.as_str(),
            export.terms[0].version.as_str()
        ),
        ("central", "synthetic-1")
    );
    assert_eq!(export.gateways.len(), 1);
    assert_eq!(
        (export.gateways[0].id, export.gateways[0].origin.as_str()),
        (own, "https://deletion-own.invalid")
    );
    assert!(export.gateways[0].approved);
    assert_eq!(export.gateways[0].name, "Synthetic gateway");
    assert_eq!(export.administers.len(), 1);
    assert_eq!(export.administers[0].id, foreign);
    // Newest first: the event it acted in as an administrator, then the one
    // about it.
    assert_eq!(export.audit.len(), 2);
    assert_eq!(
        (export.audit[0].actor, export.audit[0].account),
        (Some(identity.account), None)
    );
    assert_eq!(export.audit[1].account, Some(identity.account));
    assert!(export.complete);
    // A sign-in 31 minutes old exports nothing.
    sql(
        operator,
        "UPDATE central_identity.sessions SET created_at = created_at - interval '31 minutes'
         WHERE account_id = $1 AND kind = 'native'",
        vec![id(identity)],
    )
    .await;
    assert_eq!(
        store.account_export(Session::Native(native)).await.err(),
        Some(Error::Rejected)
    );
    sql(
        operator,
        "UPDATE central_identity.sessions SET created_at = created_at + interval '31 minutes'
         WHERE account_id = $1 AND kind = 'native'",
        vec![id(identity)],
    )
    .await;
    // A token of the other kind or of nobody exports nothing.
    for session in [
        Session::Browser(native),
        Session::Native(&Token::generate().unwrap()),
    ] {
        assert_eq!(
            store.account_export(session).await.err(),
            Some(Error::Rejected)
        );
    }
    // A list past its bound is cut and says so.
    let past = i64::try_from(EXPORT_LIST).unwrap() + 1;
    sql(
        operator,
        "INSERT INTO central_identity.terms_acceptances (account_id, subject, version, digest)
         SELECT $1, 'central', 'bound-' || n, decode(repeat('55', 32), 'hex')
         FROM generate_series(1, $2::bigint) n",
        vec![id(identity), past.into()],
    )
    .await;
    let export = store.account_export(Session::Native(native)).await.unwrap();
    assert_eq!(export.terms.len(), EXPORT_LIST);
    assert!(!export.complete);
}
