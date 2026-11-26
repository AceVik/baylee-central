//! The operator's steps after a restore (`docs/account-deletion.md`
//! §"The operator's erasure", `docs/operations.md` §"After a restore"),
//! under the migrator login as `central-service` runs them: an erasure takes
//! what the person's own deletion takes, and what a restored backup can
//! bring back besides; it is idempotent and loses no race; and the step of
//! the revocation streams leaves every gateway a gap it cannot miss.
use super::deletion::{admin_revision, count, gateway, id, moved, naming, rows, sql};
use super::{AccountStore, Admit, Error, Identity, ServerKeys, Token, config, enroll, login};
use central_api::gateway::GatewayMetadata;
use central_postgres::{
    Erased, MAX_REVOCATION, RESTORE_STEP, Session, administrators, appoint_admin, erase_account,
    step_revocations,
};
use sea_orm::{
    ConnectionTrait, DatabaseConnection, DbBackend, Statement, TransactionTrait, Value,
    prelude::Uuid,
};
use std::time::Duration;

/// The migrator login a second time, to watch the first one wait.
async fn observer() -> DatabaseConnection {
    central_postgres::connect_dev(config("central_migrator", "CENTRAL_TEST_MIGRATOR_PASSWORD"))
        .await
        .unwrap()
}

/// Until a statement of the migrator's that contains `marker` waits on a lock.
async fn waits_on_lock(observer: &DatabaseConnection, marker: &str) {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let waiting = count(
                observer,
                "SELECT count(*) AS n FROM pg_catalog.pg_stat_activity
                 WHERE datname = current_database() AND usename = 'central_migrator'
                 AND wait_event_type = 'Lock' AND strpos(query, $1) > 0",
                vec![marker.into()],
            )
            .await;
            if waiting > 0 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("the operator's statement must wait on the held lock");
}

/// A row in the legacy store for `legacy_id`: an old password's hash and a
/// source, as the import leaves them.
async fn legacy_row(operator: &DatabaseConnection, legacy_id: &str) {
    sql(
        operator,
        "INSERT INTO central_legacy.accounts (legacy_id, username_key, password_salt,
             password_hash, must_change_password, banned, snapshot, snapshot_digest)
         VALUES ($1, $1, decode(repeat('03', 16), 'hex'), decode(repeat('04', 32), 'hex'),
             false, false, '\\x7b7d', pg_catalog.sha256('\\x7b7d'))",
        vec![legacy_id.into()],
    )
    .await;
}

async fn legacy_rows(operator: &DatabaseConnection, legacy_id: &str) -> i64 {
    count(
        operator,
        "SELECT count(*) AS n FROM central_legacy.accounts WHERE legacy_id = $1",
        vec![legacy_id.into()],
    )
    .await
}

async fn erased_events(operator: &DatabaseConnection, account: &Value) -> i64 {
    count(
        operator,
        "SELECT count(*) AS n FROM central_identity.audit_events
         WHERE account_id = $1 AND action = 'account.erased'
         AND actor_kind = 'operator' AND actor_account IS NULL",
        vec![account.clone()],
    )
    .await
}

pub(super) async fn run(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    everything_once(store, operator, keys).await;
    what_a_restore_brings_back(store, operator, keys).await;
    a_deletion_that_wins_the_race(store, operator, keys).await;
    the_runtime_cannot(store, operator, keys).await;
}

const OWN: &str = "https://erasure-own.invalid";
const FOREIGN: &str = "https://erasure-foreign.invalid";

/// An account with a row in every table that can name it, with a session at
/// its own gateway and at another owner's that it administers.
struct Filled {
    identity: Identity,
    native: Token,
    other_session: Token,
    own: [u8; 16],
    foreign: [u8; 16],
}

async fn filled(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) -> Filled {
    let (identity, native) = moved(store, keys, "erasure-all").await;
    let (other, other_session) = moved(store, keys, "erasure-owner").await;
    let credential = store.credential(identity).await.unwrap().unwrap();
    store
        .issue_session(&credential, Session::Browser(&Token::generate().unwrap()))
        .await
        .unwrap();
    let own = gateway(store, operator, &native, OWN).await;
    store
        .set_gateway_admins(&native, own, 1, &[other.account])
        .await
        .unwrap();
    store
        .gateway_grant(&native, &[OWN], true, Admit, sid(61))
        .await
        .unwrap();
    let foreign = gateway(store, operator, &other_session, FOREIGN).await;
    store
        .set_gateway_admins(&other_session, foreign, 1, &[identity.account])
        .await
        .unwrap();
    store
        .gateway_grant(&native, &[FOREIGN], true, Admit, sid(62))
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

/// The digests of the account's sessions, one of each kind.
async fn digests(operator: &DatabaseConnection, account: &Value) -> Vec<Value> {
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
    digests
}

/// An account with a row in every table that can name it is erased with all
/// of them, as its own deletion would be: its sessions' `sid`s reach their
/// gateways as revocations, the other owner rereads its list, the audit
/// stays and gains the operator's event.
async fn everything_once(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    let Filled {
        identity,
        native,
        other_session,
        own,
        foreign,
    } = filled(store, operator, keys).await;
    let account = id(identity);
    let foreign_revision = admin_revision(operator, foreign).await;
    let digests = digests(operator, &account).await;
    let before = naming(operator, &account, own, &digests).await;
    for column in [
        "accounts.id",
        "sessions.account_id",
        "gateways.owner_id",
        "gateway_admins.account_id",
        "private_objects.account_id",
        "migration_archives.account_id",
        "terms_acceptances.account_id",
        "gateway_sids.session_digest",
    ] {
        assert!(
            before.iter().any(|(name, n)| name == column && *n > 0),
            "{column}: {before:?}"
        );
    }
    let own_last = store.last_revocation(OWN).await.unwrap();
    let foreign_last = store.last_revocation(FOREIGN).await.unwrap();

    assert_eq!(
        erase_account(operator, identity.account).await,
        Ok(Erased::Erased)
    );
    for (column, n) in naming(operator, &account, own, &digests).await {
        assert_eq!(n, 0, "{column}");
    }
    // Each gateway hears of the `sid` it held, numbered after what it had.
    for (stream, last, held) in [(OWN, own_last, sid(61)), (FOREIGN, foreign_last, sid(62))] {
        let page = store.revocation_page(stream, last).await.unwrap();
        assert_eq!(
            (page.from, page.sids.clone()),
            (last + 1, vec![held]),
            "{stream}"
        );
    }
    assert_eq!(
        admin_revision(operator, foreign).await,
        foreign_revision + 1
    );
    assert_eq!(erased_events(operator, &account).await, 1);
    // The audit's earlier event about it stays too: ids only.
    assert_eq!(
        count(
            operator,
            "SELECT count(*) AS n FROM central_identity.audit_events WHERE account_id = $1",
            vec![account.clone()],
        )
        .await,
        2
    );
    store.session_status(&other_session).await.unwrap();
    assert!(store.discover("erasure-all").await.unwrap().is_none());
    assert_eq!(
        store.session_status(&native).await.err(),
        Some(Error::Rejected)
    );
    again(store, operator, identity, foreign, foreign_revision + 1).await;
    store.logout(&other_session).await.unwrap();
}

/// Again, the erased account is absent, and nothing more is written; so is
/// an id that never was one.
async fn again(
    store: &AccountStore,
    operator: &DatabaseConnection,
    identity: Identity,
    foreign: [u8; 16],
    revision: i64,
) {
    let account = id(identity);
    let foreign_last = store.last_revocation(FOREIGN).await.unwrap();
    assert_eq!(
        erase_account(operator, identity.account).await,
        Ok(Erased::Absent)
    );
    assert_eq!(erased_events(operator, &account).await, 1);
    assert_eq!(store.last_revocation(FOREIGN).await.unwrap(), foreign_last);
    assert_eq!(admin_revision(operator, foreign).await, revision);
    let never = [
        0x01, 0x93, 0xa5, 0xb2, 0x7c, 0x41, 0x7d, 0x3e, 0x9f, 0, 0, 0, 0, 0, 0xee, 0xee,
    ];
    assert_eq!(erase_account(operator, never).await, Ok(Erased::Absent));
    assert_eq!(
        erased_events(operator, &Uuid::from_bytes(never).into()).await,
        0
    );
}

/// A restored backup can show the account as it was before the person
/// deleted it: an administrator, its move still open, the legacy store's row
/// of its old password still there, or a reservation that never enrolled.
/// Its own deletion would refuse the first two; the erasure takes all of
/// them, and says how many administrators are left.
async fn what_a_restore_brings_back(
    store: &AccountStore,
    operator: &DatabaseConnection,
    keys: &ServerKeys,
) {
    let open = enroll(store, keys, "erasure-open", "erasure-open").await;
    let session = login(store, keys, open).await;
    legacy_row(operator, "erasure-open").await;
    appoint_admin(operator, "erasure-open").await.unwrap();
    let administrators_before = administrators(operator).await.unwrap();
    assert!(administrators_before >= 1);
    assert_eq!(
        store.deletion_start(Session::Native(&session)).await.err(),
        Some(Error::Rejected),
        "its own deletion refuses an open move and an administrator"
    );
    assert_eq!(
        erase_account(operator, open.account).await,
        Ok(Erased::Erased)
    );
    assert_eq!(legacy_rows(operator, "erasure-open").await, 0);
    assert_eq!(
        administrators(operator).await.unwrap(),
        administrators_before - 1
    );
    assert_eq!(
        count(
            operator,
            "SELECT count(*) AS n FROM central_identity.account_roles WHERE account_id = $1",
            vec![id(open)],
        )
        .await,
        0
    );
    assert_eq!(
        store.session_status(&session).await.err(),
        Some(Error::Rejected)
    );

    // A reservation that never enrolled: no credential, a legacy row.
    let reserved = store
        .reserve_legacy(
            "erasure-reserved",
            "erasure-reserved",
            None,
            None,
            &Token::generate().unwrap(),
        )
        .await
        .unwrap();
    legacy_row(operator, "erasure-reserved").await;
    assert_eq!(
        erase_account(operator, reserved.account).await,
        Ok(Erased::Erased)
    );
    assert_eq!(legacy_rows(operator, "erasure-reserved").await, 0);
    assert_eq!(
        count(
            operator,
            "SELECT count(*) AS n FROM central_identity.accounts WHERE id = $1",
            vec![id(reserved)],
        )
        .await,
        0
    );
    // The name is free again, for a later registration.
    assert!(store.discover("erasure-open").await.unwrap().is_none());
}

/// An erasure that waits on the account's lock while the account's own
/// deletion commits finds it absent: no failure, no event.
async fn a_deletion_that_wins_the_race(
    store: &AccountStore,
    operator: &DatabaseConnection,
    keys: &ServerKeys,
) {
    let observer = observer().await;
    let (identity, _native) = moved(store, keys, "erasure-race").await;
    let tx = operator.begin().await.unwrap();
    sql(
        &tx,
        "DELETE FROM central_identity.accounts WHERE id = $1",
        vec![id(identity)],
    )
    .await;
    let erasing = operator.clone();
    let erasure = tokio::spawn(async move { erase_account(&erasing, identity.account).await });
    waits_on_lock(&observer, "SELECT legacy_id FROM central_identity.accounts").await;
    tx.commit().await.unwrap();
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(5), erasure)
            .await
            .unwrap()
            .unwrap(),
        Ok(Erased::Absent)
    );
    assert_eq!(erased_events(operator, &id(identity)).await, 0);

    // One that waits while the account is made an administrator erases it all
    // the same, role and all.
    let (identity, _native) = moved(store, keys, "erasure-appointed").await;
    let tx = operator.begin().await.unwrap();
    sql(
        &tx,
        "SELECT id FROM central_identity.accounts WHERE id = $1 FOR UPDATE",
        vec![id(identity)],
    )
    .await;
    let erasing = operator.clone();
    let erasure = tokio::spawn(async move { erase_account(&erasing, identity.account).await });
    waits_on_lock(&observer, "SELECT legacy_id FROM central_identity.accounts").await;
    sql(
        &tx,
        "INSERT INTO central_identity.account_roles (account_id, role) VALUES ($1, 'admin')",
        vec![id(identity)],
    )
    .await;
    tx.commit().await.unwrap();
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(5), erasure)
            .await
            .unwrap()
            .unwrap(),
        Ok(Erased::Erased)
    );
    assert_eq!(
        count(
            operator,
            "SELECT count(*) AS n FROM central_identity.account_roles WHERE account_id = $1",
            vec![id(identity)],
        )
        .await,
        0
    );
    observer.close().await.unwrap();
}

/// The erasure is the migrator's: the runtime login, which has nothing in
/// the legacy store, erases nothing of an account that has a row there.
async fn the_runtime_cannot(
    store: &AccountStore,
    operator: &DatabaseConnection,
    keys: &ServerKeys,
) {
    let runtime =
        central_postgres::connect_dev(config("central_runtime", "CENTRAL_TEST_RUNTIME_PASSWORD"))
            .await
            .unwrap();
    let identity = enroll(store, keys, "erasure-runtime", "erasure-runtime").await;
    let session = login(store, keys, identity).await;
    legacy_row(operator, "erasure-runtime").await;
    assert_eq!(
        erase_account(&runtime, identity.account).await,
        Err(Error::Unavailable)
    );
    store.session_status(&session).await.unwrap();
    assert_eq!(legacy_rows(operator, "erasure-runtime").await, 1);
    assert_eq!(erased_events(operator, &id(identity)).await, 0);
    // The migrator's leaves the store empty for the legacy tests after it.
    assert_eq!(
        erase_account(operator, identity.account).await,
        Ok(Erased::Erased)
    );
    assert_eq!(legacy_rows(operator, "erasure-runtime").await, 0);
    runtime.close().await.unwrap();
}

/// A gateway's stream, as `revocation_page` reads it: `(last, floor)`.
async fn position(store: &AccountStore, stream: &str) -> (u64, u64) {
    let page = store.revocation_page(stream, 0).await.unwrap();
    (page.last, page.floor)
}

async fn signed_in(store: &AccountStore, keys: &ServerKeys, name: &str) -> Token {
    let identity = enroll(store, keys, name, name).await;
    let session = login(store, keys, identity).await;
    store.confirm_restore(&session, 1).await.unwrap();
    store.close_legacy(name, identity).await.unwrap();
    session
}

/// Its own `sid`s, apart from the other tests'.
fn sid(number: u8) -> [u8; 16] {
    let mut id = [number; 16];
    id[0] = 0x5e;
    id
}

const KNOWN: &str = "https://step-known.invalid";
const QUIET: &str = "https://step-quiet.invalid";
const OFFICIAL: &str = "https://step-official.invalid";
const FULL: &str = "https://step-full.invalid";

/// The step after a restore, last of all: it moves every stream. Refused as a
/// whole when one stream would pass the bound; then every stream, those the
/// backup never saw among them, ends a step past its last number with
/// nothing held below it, so a gateway behind or ahead finds the gap; a
/// session's end that raced it is covered; nothing is due to be pushed; and
/// numbering goes on from there for the sessions that stay.
pub(super) async fn step(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    let (staying, racing, owner) = before_the_step(store, operator, keys).await;
    refused_at_the_bound(store, operator).await;
    let moved = raced(operator, &racing).await;
    after_the_step(store, operator, moved).await;
    // Central's own sessions stay: the one that stayed refreshes its `sid`,
    // and its end is the next number.
    let known = 3 + u64::try_from(RESTORE_STEP).unwrap();
    store.session_status(&staying).await.unwrap();
    assert_eq!(
        store
            .gateway_grant(&staying, &[KNOWN], false, Admit, sid(9))
            .await
            .unwrap()
            .sid,
        sid(3)
    );
    store.logout(&staying).await.unwrap();
    let page = store.revocation_page(KNOWN, known).await.unwrap();
    assert_eq!(
        (page.from, page.sids, page.last, page.floor),
        (known + 1, vec![sid(3)], known + 1, known + 1)
    );
    store.logout(&owner).await.unwrap();
}

async fn streams(operator: &DatabaseConnection, origin: &str) -> i64 {
    count(
        operator,
        "SELECT count(*) AS n FROM central_identity.gateway_streams WHERE origin = $1",
        vec![origin.into()],
    )
    .await
}

/// Two revocations held at `KNOWN`, a session there that stays and one that
/// ends during the step, its push waiting; a registered gateway that never
/// held a session and the official one have no stream yet.
async fn before_the_step(
    store: &AccountStore,
    operator: &DatabaseConnection,
    keys: &ServerKeys,
) -> (Token, Token, Token) {
    for number in 1..=2 {
        let session = signed_in(store, keys, &format!("step-ended-{number}")).await;
        store
            .gateway_grant(&session, &[KNOWN], false, Admit, sid(number))
            .await
            .unwrap();
        store.logout(&session).await.unwrap();
    }
    let staying = signed_in(store, keys, "step-staying").await;
    store
        .gateway_grant(&staying, &[KNOWN], false, Admit, sid(3))
        .await
        .unwrap();
    let racing = signed_in(store, keys, "step-racing").await;
    store
        .gateway_grant(&racing, &[KNOWN], false, Admit, sid(4))
        .await
        .unwrap();
    assert_eq!(position(store, KNOWN).await, (2, 1));
    let owner = signed_in(store, keys, "step-owner").await;
    store
        .register_gateway(
            &owner,
            QUIET,
            &GatewayMetadata {
                name: "Quiet gateway".into(),
                version: "1.0-test".into(),
                website: format!("{QUIET}/status"),
            },
            &Token::generate().unwrap(),
            &Token::generate().unwrap(),
            false,
        )
        .await
        .unwrap();
    assert_eq!(
        (
            streams(operator, QUIET).await,
            streams(operator, OFFICIAL).await
        ),
        (0, 0)
    );
    sql(
        operator,
        "UPDATE central_identity.gateway_streams SET attempts = 3,
         next_attempt_at = pg_catalog.clock_timestamp() + interval '1 hour' WHERE origin = $1",
        vec![KNOWN.into()],
    )
    .await;
    (staying, racing, owner)
}

/// One stream a step from the bound refuses the whole step, and nothing
/// changes; at the bound exactly, it may still go.
async fn refused_at_the_bound(store: &AccountStore, operator: &DatabaseConnection) {
    let full = MAX_REVOCATION - RESTORE_STEP;
    sql(
        operator,
        "INSERT INTO central_identity.gateway_streams (origin, last_seq, pushed_seq)
         VALUES ($1, $2, $2)",
        vec![FULL.into(), (full + 1).into()],
    )
    .await;
    assert_eq!(
        step_revocations(operator, OFFICIAL).await,
        Err(Error::Rejected)
    );
    assert_eq!(position(store, KNOWN).await, (2, 1), "nothing moved");
    assert_eq!(
        (
            streams(operator, QUIET).await,
            streams(operator, OFFICIAL).await
        ),
        (0, 0)
    );
    sql(
        operator,
        "UPDATE central_identity.gateway_streams SET last_seq = $2, pushed_seq = $2
         WHERE origin = $1",
        vec![FULL.into(), full.into()],
    )
    .await;
}

/// The step waits for a session's end that holds the stream, and covers the
/// number that end took. Returns how many streams it moved.
async fn raced(operator: &DatabaseConnection, racing: &Token) -> u64 {
    let observer = observer().await;
    let tx = operator.begin().await.unwrap();
    sql(
        &tx,
        "DELETE FROM central_identity.sessions WHERE digest = $1",
        vec![racing.digest().to_vec().into()],
    )
    .await;
    let stepping = operator.clone();
    let stepped = tokio::spawn(async move { step_revocations(&stepping, OFFICIAL).await });
    waits_on_lock(&observer, "ORDER BY origin FOR UPDATE").await;
    tx.commit().await.unwrap();
    let moved = tokio::time::timeout(Duration::from_secs(5), stepped)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    observer.close().await.unwrap();
    moved
}

/// Every stream moved, a step past its last number, nothing held below it
/// and nothing due to be pushed.
async fn after_the_step(store: &AccountStore, operator: &DatabaseConnection, moved: u64) {
    let step = u64::try_from(RESTORE_STEP).unwrap();
    let all = count(
        operator,
        "SELECT count(*) AS n FROM central_identity.gateway_streams",
        vec![],
    )
    .await;
    assert_eq!(moved, u64::try_from(all).unwrap(), "every stream moved");
    let known = 3 + step;
    assert_eq!(position(store, KNOWN).await, (known, known + 1));
    for stream in [QUIET, OFFICIAL] {
        assert_eq!(position(store, stream).await, (step, step + 1), "{stream}");
    }
    let max = u64::try_from(MAX_REVOCATION).unwrap();
    assert_eq!(position(store, FULL).await, (max, max + 1));
    assert_eq!(
        count(
            operator,
            "SELECT count(*) AS n FROM central_identity.gateway_revocations",
            vec![],
        )
        .await,
        0
    );
    // A gateway behind finds the gap, and so does one that had gone on past
    // the backup's numbers.
    for after in [0, 2, 4, 3 + 1_000_000] {
        let page = store.revocation_page(KNOWN, after).await.unwrap();
        assert_eq!(page.sids, Vec::<[u8; 16]>::new());
        assert!(after + 1 < page.floor, "{after}");
    }
    assert_eq!(
        count(
            operator,
            "SELECT count(*) AS n FROM central_identity.gateway_streams
             WHERE pushed_seq <> last_seq OR attempts <> 0 OR next_attempt_at IS NOT NULL",
            vec![],
        )
        .await,
        0
    );
    assert!(store.claim_gateway_push().await.unwrap().is_none());
}
