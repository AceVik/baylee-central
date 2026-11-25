//! Report tokens (migration 000052, the store's `report_tokens.rs` and
//! `reports/actor.rs`): made and revoked by an administrator with a fresh
//! sign-in alone, audited by their id; each does exactly what its rights say
//! and nothing once it is revoked, over, or its maker is no administrator;
//! what one files, writes and changes names it and never looks like a
//! gateway's or an administrator's; and the runtime's exact grants.
use super::{
    AccountStore, Admin, DatabaseConnection, Error, Identity, ServerKeys, Token, all, config,
    count, fresh_id, gzip, intake, sql,
};
use super::{Uuid, signed_in};
use central_api::report_tokens::{
    LIVE_TOKENS, ReportScope, ReportScopes, ReportTokenInfo, TOKENS_PER_DAY,
};
use central_api::reports::{
    BulkAction, Channel, HISTORY, MAX_NOTE, MAX_NOTES_BYTES, Part, RecordOrigin, ReportChange,
    ReportFilter, ReportIntake, ReportKind, ReportSort, ReportStatus,
};
use central_postgres::{FilerKind, GATEWAY_STORED, REPORT_BYTES, ReportCaller, appoint_admin};
use sea_orm::{ConnectionTrait, DbBackend, Statement, TransactionTrait};
use std::time::Duration;

/// A report token the fixture made, and its id.
struct Made {
    id: [u8; 16],
    token: Token,
    scopes: ReportScopes,
}

impl Made {
    fn by(&self) -> ReportCaller<'_> {
        ReportCaller::Token(&self.token)
    }
}

async fn made(store: &AccountStore, cookie: &Token, name: &str, scopes: ReportScopes) -> Made {
    let token = Token::generate().unwrap();
    let (id, _) = store
        .create_report_token(cookie, name, scopes, 30, &token)
        .await
        .unwrap();
    Made { id, token, scopes }
}

/// Every set of rights a token can hold.
fn every_scopes() -> Vec<ReportScopes> {
    (0_u8..16)
        .map(|bits| {
            let held: Vec<ReportScope> = ReportScope::ALL
                .into_iter()
                .enumerate()
                .filter(|(at, _)| bits & (1 << at) != 0)
                .map(|(_, right)| right)
                .collect();
            ReportScopes::of(&held)
        })
        .filter(|scopes| scopes.validate().is_ok())
        .collect()
}

/// A report as a token files it.
fn filed(text: &str) -> ReportIntake {
    let mut report = intake(ReportKind::Bug, text, &gzip(3, 400), Some(&gzip(5, 900)));
    report.channel = Channel::Token;
    report.record.as_mut().unwrap().origin = RecordOrigin::Client;
    report
}

pub(super) async fn run(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    grants(operator).await;
    table(operator).await;
    let (admin, cookie) = signed_in(store, keys, "rt-admin").await;
    let (_, stranger) = signed_in(store, keys, "rt-stranger").await;
    appoint_admin(operator, "rt-admin").await.unwrap();
    making(store, operator, &cookie, &stranger, admin).await;
    stale(store, operator, keys).await;
    confusion(store, &cookie).await;
    rights(store, operator, &cookie).await;
    filing(store, operator, &cookie).await;
    acting(store, &cookie, admin).await;
    bounded(store, operator, &cookie).await;
    ending(store, operator, &cookie, admin).await;
    in_flight(store, operator, &cookie).await;
    used(store, operator, &cookie).await;
    full(store, operator, &cookie).await;
    sql(
        operator,
        "DELETE FROM central_identity.reports WHERE filed_by = 'token'",
        vec![],
    )
    .await;
}

/// Anyone but a fresh administrator is refused, and so is what no token may
/// be; what is stored is the digest, the bounds and the maker, and the audit
/// names the token by its id and the page by its name.
async fn making(
    store: &AccountStore,
    operator: &DatabaseConnection,
    cookie: &Token,
    stranger: &Token,
    admin: Identity,
) {
    let token = Token::generate().unwrap();
    let nobody = Token::generate().unwrap();
    for browser in [stranger, &nobody, &token] {
        let scopes = ReportScopes::ONLY_POST;
        assert_eq!(
            store
                .create_report_token(browser, "CI", scopes, 30, &token)
                .await
                .err(),
            Some(Error::Rejected)
        );
        assert_eq!(
            store.admin_report_tokens(browser).await.err(),
            Some(Error::Rejected)
        );
    }
    let broken = ReportScopes::of(&[ReportScope::Purge, ReportScope::Read]);
    let post = ReportScopes::ONLY_POST;
    for (name, scopes, days) in [
        ("", post, 30),
        (" CI", post, 30),
        ("C\nI", post, 30),
        ("CI", ReportScopes::default(), 30),
        ("CI", broken, 30),
        ("CI", post, 0),
        ("CI", post, 367),
    ] {
        assert_eq!(
            store
                .create_report_token(cookie, name, scopes, days, &token)
                .await
                .err(),
            Some(Error::Rejected),
            "{name:?} {scopes:?} {days}"
        );
    }
    assert_eq!(
        store.admin_report_tokens(cookie).await.unwrap(),
        Vec::<ReportTokenInfo>::new()
    );
    let everything = ReportScopes::EVERYTHING;
    let (id, expires_at) = store
        .create_report_token(cookie, "Nightly build", everything, 366, &token)
        .await
        .unwrap();
    let listed = store.admin_report_tokens(cookie).await.unwrap();
    assert_eq!(listed.len(), 1);
    let one = &listed[0];
    assert_eq!(
        (one.id, one.name.as_str(), one.scopes, one.created_by),
        (id, "Nightly build", everything, admin.account)
    );
    assert_eq!(
        (one.expires_at, one.expires_at - one.created_at),
        (expires_at, 366 * 86_400)
    );
    assert_eq!(
        (
            one.revoked_at,
            one.revoked_by,
            one.last_used_at,
            one.suspended
        ),
        (None, None, None, false)
    );
    // the digest is stored, the token nowhere
    let holds = |column: &str, value: Vec<u8>| {
        let text = format!(
            "SELECT count(*) AS n FROM central_identity.report_tokens t WHERE {column} = $1"
        );
        async move { count(operator, &text, vec![value.into()]).await }
    };
    assert_eq!(holds("t.digest", token.digest().to_vec()).await, 1);
    assert_eq!(holds("t.digest", token.to_secret_bytes().to_vec()).await, 0);
    // revoked by a fresh administrator alone, and once
    for browser in [stranger, &nobody, &token] {
        assert_eq!(
            store.revoke_report_token(browser, id).await.err(),
            Some(Error::Rejected)
        );
    }
    let unknown = *fresh_id(operator).await.as_bytes();
    assert_eq!(
        store.revoke_report_token(cookie, unknown).await.err(),
        Some(Error::Rejected)
    );
    store.revoke_report_token(cookie, id).await.unwrap();
    store.revoke_report_token(cookie, id).await.unwrap();
    let listed = store.admin_report_tokens(cookie).await.unwrap();
    assert!(listed[0].revoked_at.is_some());
    assert_eq!(listed[0].revoked_by, Some(admin.account));
    audited(store, operator, cookie, id, &token).await;
}

/// One event each for the making and the revocation, naming the token by its
/// id and nothing of it in words; the page adds its name beside the events.
async fn audited(
    store: &AccountStore,
    operator: &DatabaseConnection,
    cookie: &Token,
    id: [u8; 16],
    token: &Token,
) {
    for action in ["report_token.created", "report_token.revoked"] {
        assert_eq!(
            count(
                operator,
                "SELECT count(*) AS n FROM central_identity.audit_events
                 WHERE action = $1 AND report_token_id = $2 AND actor_kind = 'admin' AND detail IS NULL
                 AND account_id IS NULL AND gateway_id IS NULL",
                vec![action.into(), Uuid::from_bytes(id).into()],
            )
            .await,
            1,
            "{action}"
        );
    }
    let page = store.admin_audit(cookie, None, None).await.unwrap();
    let named: Vec<(&str, &str)> = page
        .report_tokens
        .iter()
        .map(|named| {
            assert_eq!(named.token.id, id);
            let event = page.events.iter().find(|e| e.id == named.event).unwrap();
            (event.action.as_str(), named.token.name.as_str())
        })
        .collect();
    assert_eq!(
        named,
        [
            ("report_token.revoked", "Nightly build"),
            ("report_token.created", "Nightly build")
        ]
    );
    // the row of the audit, as it is stored, holds neither token nor digest
    let digest = central_api::reports::hex(&token.digest());
    let secret = token.hex();
    assert_eq!(
        count(
            operator,
            "SELECT count(*) AS n FROM central_identity.audit_events e
             WHERE strpos(e::text, $1) > 0 OR strpos(e::text, $2) > 0",
            vec![digest.into(), secret.as_str().into()],
        )
        .await,
        0
    );
}

/// A sign-in of half an hour ago makes and revokes nothing; it reads.
async fn stale(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    let (identity, cookie) = signed_in(store, keys, "rt-stale-admin").await;
    appoint_admin(operator, "rt-stale-admin").await.unwrap();
    let token = made(store, &cookie, "Before", ReportScopes::ONLY_READ).await;
    sql(
        operator,
        "UPDATE central_identity.sessions SET created_at = created_at - interval '31 minutes'
         WHERE account_id = $1 AND kind = 'browser'",
        vec![Uuid::from_bytes(identity.account).into()],
    )
    .await;
    let next = Token::generate().unwrap();
    assert_eq!(
        store
            .create_report_token(&cookie, "After", ReportScopes::ONLY_READ, 30, &next)
            .await
            .err(),
        Some(Error::Rejected)
    );
    assert_eq!(
        store.revoke_report_token(&cookie, token.id).await.err(),
        Some(Error::Rejected)
    );
    assert!(store.admin_report_tokens(&cookie).await.is_ok());
}

/// Each credential is one kind: a report token is no session, no gateway and
/// no administrator, and neither a cookie nor a gateway's token is a report
/// token.
async fn confusion(store: &AccountStore, cookie: &Token) {
    let one = made(store, cookie, "Confused", ReportScopes::EVERYTHING).await;
    let (list, newest) = (all(), ReportSort::Received);
    assert!(
        store
            .reports(one.by(), &list, newest, true, 0, 1)
            .await
            .is_ok()
    );
    for caller in [Admin(&one.token), ReportCaller::Token(cookie)] {
        assert_eq!(
            store.reports(caller, &list, newest, true, 0, 1).await.err(),
            Some(Error::Rejected)
        );
        assert_eq!(
            store.report_admins(caller).await.err(),
            Some(Error::Rejected)
        );
    }
    assert_eq!(
        store.report_filer(&one.token).await.err(),
        Some(Error::Rejected)
    );
    assert_eq!(
        store.report_filer(cookie).await.err(),
        Some(Error::Rejected)
    );
    assert_eq!(
        store.report_token_filer(cookie).await.err(),
        Some(Error::Rejected)
    );
    assert_eq!(
        store.gateway_status(&one.token).await.err(),
        Some(Error::Rejected)
    );
    assert_eq!(
        store.admin_report_tokens(&one.token).await.err(),
        Some(Error::Rejected)
    );
    assert_eq!(
        store.report_views(&one.token).await.err(),
        Some(Error::Rejected)
    );
    store.revoke_report_token(cookie, one.id).await.unwrap();
}

/// What a token is asked to do, each apart.
#[derive(Clone, Copy, Debug)]
enum Act {
    File,
    Announced,
    Part,
    List,
    Facets,
    Crashes,
    Stats,
    Detail,
    Download,
    Admins,
    Change,
    BulkChange,
    Trash,
    Restore,
    Note,
    RemoveNote,
    Purge,
    BulkPurge,
}

const ACTS: [(Act, ReportScope); 18] = [
    (Act::File, ReportScope::File),
    (Act::Announced, ReportScope::File),
    (Act::Part, ReportScope::File),
    (Act::List, ReportScope::Read),
    (Act::Facets, ReportScope::Read),
    (Act::Crashes, ReportScope::Read),
    (Act::Stats, ReportScope::Read),
    (Act::Detail, ReportScope::Read),
    (Act::Download, ReportScope::Read),
    (Act::Admins, ReportScope::Triage),
    (Act::Change, ReportScope::Triage),
    (Act::BulkChange, ReportScope::Triage),
    (Act::Trash, ReportScope::Triage),
    (Act::Restore, ReportScope::Triage),
    (Act::Note, ReportScope::Triage),
    (Act::RemoveNote, ReportScope::Triage),
    (Act::Purge, ReportScope::Purge),
    (Act::BulkPurge, ReportScope::Purge),
];

/// What an act needs to be there, made by those who may.
struct Scene<'a> {
    store: &'a AccountStore,
    operator: &'a DatabaseConnection,
    /// An administrator's session.
    cookie: &'a Token,
    /// A token with every right, that files what the others act on.
    helper: &'a Made,
}

impl Scene<'_> {
    /// A report of the helper's, its client part stored.
    async fn report(&self) -> [u8; 16] {
        let filer = self
            .store
            .report_token_filer(&self.helper.token)
            .await
            .unwrap();
        let report = self
            .store
            .file_report(&filer, &filed("Acted on"))
            .await
            .unwrap();
        self.store
            .file_report_part(&filer, report, Part::Client, &gzip(3, 400))
            .await
            .unwrap();
        report
    }

    async fn trashed(&self) -> [u8; 16] {
        let report = self.report().await;
        self.store
            .bin_report(Admin(self.cookie), report, &BulkAction::Trash)
            .await
            .unwrap();
        report
    }

    /// `one` tries `act`.
    async fn tries(&self, one: &Made, act: Act) -> Result<(), Error> {
        let (store, by) = (self.store, one.by());
        match act {
            Act::File => {
                let filer = store.report_token_filer(&one.token).await?;
                store.file_report(&filer, &filed("Filed")).await.map(drop)
            }
            Act::Announced | Act::Part => {
                let filer = store.report_token_filer(&one.token).await?;
                let report = store.file_report(&filer, &filed("In parts")).await?;
                store
                    .report_part_expected(&filer, report, Part::Client)
                    .await?;
                if matches!(act, Act::Part) {
                    store
                        .file_report_part(&filer, report, Part::Client, &gzip(3, 400))
                        .await?;
                }
                Ok(())
            }
            Act::List => {
                let page = store
                    .reports(by, &all(), ReportSort::Received, true, 0, 5)
                    .await;
                page.map(drop)
            }
            Act::Facets => store.report_facets(by, &all()).await.map(drop),
            Act::Crashes => store.crash_groups(by, &all()).await.map(drop),
            Act::Stats => store.report_stats(by).await.map(drop),
            Act::Detail => store.report_detail(by, self.report().await).await.map(drop),
            Act::Download => {
                let part = store
                    .report_part(by, self.report().await, Part::Client)
                    .await?;
                assert_eq!(part, (gzip(3, 400), None));
                Ok(())
            }
            Act::Admins => store.report_admins(by).await.map(drop),
            Act::Change => {
                let change = status(ReportStatus::Triaged);
                let report = self.report().await;
                store
                    .set_report(by, report, &change, Some(0))
                    .await
                    .map(drop)
            }
            Act::BulkChange => {
                let change = status(ReportStatus::InProgress);
                let reports = [self.report().await];
                let how = BulkAction::Change { change };
                store.reports_bulk(by, &reports, &how).await.map(drop)
            }
            Act::Trash => {
                store
                    .bin_report(by, self.report().await, &BulkAction::Trash)
                    .await
            }
            Act::Restore => {
                store
                    .bin_report(by, self.trashed().await, &BulkAction::Restore)
                    .await
            }
            Act::Note => {
                let report = self.report().await;
                store.add_report_note(by, report, "A note").await.map(drop)
            }
            Act::RemoveNote => {
                // its own, written for it: a note a token may remove is its own
                let report = self.report().await;
                let note: Uuid = self
                    .operator
                    .query_one_raw(Statement::from_sql_and_values(
                        DbBackend::Postgres,
                        "INSERT INTO central_identity.report_notes (report_id, author, token_name, text)
                         VALUES ($1, $2, 'CI', 'Its own') RETURNING id",
                        vec![Uuid::from_bytes(report).into(), Uuid::from_bytes(one.id).into()],
                    ))
                    .await
                    .unwrap()
                    .unwrap()
                    .try_get("", "id")
                    .unwrap();
                store
                    .remove_report_note(by, report, *note.as_bytes())
                    .await
                    .map(drop)
            }
            Act::Purge => {
                store
                    .bin_report(by, self.trashed().await, &BulkAction::Purge)
                    .await
            }
            Act::BulkPurge => {
                let reports = [self.trashed().await];
                store
                    .reports_bulk(by, &reports, &BulkAction::Purge)
                    .await
                    .map(drop)
            }
        }
    }
}

fn status(status: ReportStatus) -> ReportChange {
    ReportChange {
        status: Some(status),
        ..ReportChange::default()
    }
}

/// Every set of rights a token can hold does exactly its own.
async fn rights(store: &AccountStore, operator: &DatabaseConnection, cookie: &Token) {
    let helper = made(store, cookie, "Helper", ReportScopes::EVERYTHING).await;
    let scene = Scene {
        store,
        operator,
        cookie,
        helper: &helper,
    };
    let mut tokens = Vec::new();
    for scopes in every_scopes() {
        tokens.push(made(store, cookie, "Tried", scopes).await);
    }
    assert_eq!(tokens.len(), 7);
    for one in &tokens {
        for (act, right) in ACTS {
            let expected = if one.scopes.allows(right) {
                Ok(())
            } else {
                Err(Error::Rejected)
            };
            assert_eq!(
                scene.tries(one, act).await,
                expected,
                "{:?} {act:?}",
                one.scopes
            );
        }
        for right in ReportScope::ALL {
            assert_eq!(
                store.report_token(&one.token, right).await.is_ok(),
                one.scopes.allows(right)
            );
        }
        store.revoke_report_token(cookie, one.id).await.unwrap();
    }
    store.revoke_report_token(cookie, helper.id).await.unwrap();
}

/// What a token files: by its own channel alone, never a record as a
/// gateway's own, weighed by its own id; listed as a token's, found by it and
/// by no gateway's name, even its own.
async fn filing(store: &AccountStore, operator: &DatabaseConnection, cookie: &Token) {
    // named as the fixture's gateway is
    let one = made(store, cookie, "Synthetic reports", ReportScopes::ONLY_POST).await;
    let other = made(store, cookie, "Another", ReportScopes::ONLY_POST).await;
    let filer = store.report_token_filer(&one.token).await.unwrap();
    assert_eq!((filer.id, filer.kind), (one.id, FilerKind::Token));
    let mut gateways = filed("As a gateway");
    gateways.channel = Channel::Gateway;
    let mut direct = filed("Direct");
    direct.channel = Channel::Direct;
    let mut own = filed("A gateway's record");
    own.record.as_mut().unwrap().origin = RecordOrigin::Gateway;
    for report in [&gateways, &direct, &own] {
        assert_eq!(
            store.file_report(&filer, report).await.err(),
            Some(Error::Rejected),
            "{}",
            report.text
        );
    }
    let official = store.official_filer().await.unwrap();
    assert_eq!(
        store
            .file_report(&official, &filed("As a token"))
            .await
            .err(),
        Some(Error::Rejected),
        "a token's report is only a token's"
    );
    let report = store
        .file_report(&filer, &filed("From a script"))
        .await
        .unwrap();
    // its parts are its own to send
    let elsewhere = store.report_token_filer(&other.token).await.unwrap();
    for stranger in [&elsewhere, &official] {
        assert_eq!(
            store
                .report_part_expected(stranger, report, Part::Record)
                .await
                .err(),
            Some(Error::Rejected)
        );
        assert_eq!(
            store
                .file_report_part(stranger, report, Part::Record, &gzip(5, 900))
                .await
                .err(),
            Some(Error::Rejected)
        );
    }
    store
        .file_report_part(&filer, report, Part::Record, &gzip(5, 900))
        .await
        .unwrap();
    listed(store, cookie, &one, report).await;
    // weighed as a registered gateway is, by its own id
    let (theirs, _) = store.report_levels(one.id).await.unwrap();
    assert_eq!(theirs, REPORT_BYTES + 400 + 900);
    let room = (GATEWAY_STORED - theirs) / (REPORT_BYTES + 64 * 1024 * 1024);
    sql(
        operator,
        "INSERT INTO central_identity.reports (gateway_id, gateway_label, filed_by, channel, reporter, kind, text,
             record_bytes, record_sha256, record_complete, record_origin)
         SELECT $1, 'Piled', 'gateway', 'gateway', 'ab', 'other', 'piled', 67108864,
             pg_catalog.sha256(''::bytea), false, 'gateway' FROM pg_catalog.generate_series(1, $2::bigint)",
        vec![
            Uuid::from_bytes(one.id).into(),
            i64::try_from(room).unwrap().into(),
        ],
    )
    .await;
    let mut large = filed("One too many");
    large.record.as_mut().unwrap().bytes = 4 * 1024 * 1024;
    let refused = loop {
        match store.file_report(&filer, &large).await {
            Ok(_) => {}
            Err(error) => break error,
        }
    };
    assert_eq!(refused, Error::Busy);
    assert!(store.report_levels(one.id).await.unwrap().0 <= GATEWAY_STORED);
    // another token is not refused for it
    store
        .file_report(&elsewhere, &filed("Elsewhere"))
        .await
        .unwrap();
    sql(
        operator,
        "DELETE FROM central_identity.reports WHERE gateway_id = $1",
        vec![Uuid::from_bytes(one.id).into()],
    )
    .await;
    for token in [one, other] {
        store.revoke_report_token(cookie, token.id).await.unwrap();
    }
}

/// `report` of `one`, as an administrator's list, filters and page show it.
async fn listed(store: &AccountStore, cookie: &Token, one: &Made, report: [u8; 16]) {
    let found = |filter: ReportFilter| async move {
        let (page, _) = store
            .reports(Admin(cookie), &filter, ReportSort::Received, true, 0, 200)
            .await
            .unwrap();
        page
    };
    let by_token = found(ReportFilter {
        token: Some(one.id),
        ..all()
    })
    .await;
    assert_eq!(by_token.len(), 1);
    assert_eq!(
        (by_token[0].id, by_token[0].channel, by_token[0].token),
        (report, Channel::Token, Some(one.id))
    );
    assert_eq!(by_token[0].gateway, "Synthetic reports");
    let channel = |channel| ReportFilter {
        channel: Some(channel),
        ..all()
    };
    let tokens = found(channel(Channel::Token)).await;
    assert!(
        tokens
            .iter()
            .all(|r| r.channel == Channel::Token && r.token.is_some())
    );
    assert!(tokens.iter().any(|r| r.id == report));
    for other in [Channel::Gateway, Channel::Direct] {
        let page = found(channel(other)).await;
        assert!(page.iter().all(|r| r.channel == other && r.token.is_none()));
    }
    // a gateway of the same name is another source
    let gateways = found(ReportFilter {
        gateway: Some("Synthetic reports".into()),
        ..all()
    })
    .await;
    assert!(gateways.iter().all(|r| r.token.is_none()));
    let facets = store.report_facets(Admin(cookie), &all()).await.unwrap();
    let mine = facets.tokens.iter().find(|c| c.value.id == one.id).unwrap();
    assert_eq!(
        (mine.value.name.as_str(), mine.count),
        ("Synthetic reports", 1)
    );
    let counted = |channel| {
        facets
            .channels
            .iter()
            .find(|c| c.value == channel)
            .map_or(0, |c| c.count)
    };
    assert_eq!(
        counted(Channel::Token),
        u64::try_from(tokens.len()).unwrap()
    );
    assert_eq!(
        facets.gateways.iter().map(|c| c.count).sum::<u64>(),
        counted(Channel::Gateway) + counted(Channel::Direct),
        "the gateways' facet counts no token's"
    );
    // its history says who filed it, as what it was
    let detail = store.report_detail(Admin(cookie), report).await.unwrap();
    let line = &detail.audit[0];
    assert_eq!(
        (line.action.as_str(), line.actor, line.token.as_deref()),
        ("report.filed", Some(one.id), Some("Synthetic reports"))
    );
    assert!(detail.record.as_ref().is_some_and(|part| part.received));
}

/// What a token changes and writes names it, beside an administrator's, and
/// its notes are its own.
async fn acting(store: &AccountStore, cookie: &Token, admin: Identity) {
    let all_rights = ReportScopes::EVERYTHING;
    let one = made(store, cookie, "Agent", all_rights).await;
    let other = made(store, cookie, "Other agent", all_rights).await;
    let filer = store.report_token_filer(&one.token).await.unwrap();
    let report = store.file_report(&filer, &filed("Triaged")).await.unwrap();
    let change = status(ReportStatus::Triaged);
    let now = store
        .set_report(one.by(), report, &change, Some(0))
        .await
        .unwrap();
    assert_eq!(
        (now.summary.status, now.summary.revision),
        (ReportStatus::Triaged, 1)
    );
    assert_eq!(
        store
            .set_report(one.by(), report, &change, Some(0))
            .await
            .err(),
        Some(Error::Conflict),
        "an older revision"
    );
    let change = status(ReportStatus::InProgress);
    store
        .set_report(Admin(cookie), report, &change, Some(1))
        .await
        .unwrap();
    // an assignee is an administrator, whoever assigns; a token is none
    let assign = |who: [u8; 16]| ReportChange {
        assignee: Some(Some(who)),
        ..ReportChange::default()
    };
    assert_eq!(
        store
            .set_report(one.by(), report, &assign(one.id), None)
            .await
            .err(),
        Some(Error::Rejected)
    );
    store
        .set_report(one.by(), report, &assign(admin.account), None)
        .await
        .unwrap();
    noting(store, cookie, admin, [&one, &other], report).await;
    // a purge is the trash's alone, and its history stays
    assert_eq!(
        store
            .bin_report(one.by(), report, &BulkAction::Purge)
            .await
            .err(),
        Some(Error::Rejected),
        "not in the trash"
    );
    for how in [BulkAction::Trash, BulkAction::Purge] {
        store.bin_report(one.by(), report, &how).await.unwrap();
    }
    assert_eq!(
        store.report_detail(Admin(cookie), report).await.err(),
        Some(Error::Rejected)
    );
    for token in [one, other] {
        store.revoke_report_token(cookie, token.id).await.unwrap();
    }
}

/// Notes of a token's beside an administrator's, each its author's alone, and
/// the history that names them.
async fn noting(
    store: &AccountStore,
    cookie: &Token,
    admin: Identity,
    [one, other]: [&Made; 2],
    report: [u8; 16],
) {
    let detail = store
        .add_report_note(one.by(), report, "By the agent")
        .await
        .unwrap();
    let agents = detail.notes[0].id;
    let detail = store
        .add_report_note(Admin(cookie), report, "By a person")
        .await
        .unwrap();
    let persons = detail.notes[1].id;
    assert_eq!(
        detail
            .notes
            .iter()
            .map(|n| (n.author, n.token.as_deref()))
            .collect::<Vec<_>>(),
        [(one.id, Some("Agent")), (admin.account, None)]
    );
    let who: Vec<_> = detail
        .audit
        .iter()
        .map(|row| (row.action.as_str(), row.actor, row.token.as_deref()))
        .collect();
    let agent = (Some(one.id), Some("Agent"));
    let person = (Some(admin.account), None);
    assert_eq!(
        who,
        [
            ("report.filed", agent.0, agent.1),
            ("report.status", agent.0, agent.1),
            ("report.status", person.0, person.1),
            ("report.assignee", agent.0, agent.1),
            ("report.note", agent.0, agent.1),
            ("report.note", person.0, person.1),
        ]
    );
    // a note goes by its author's hand alone
    for (caller, note) in [
        (one.by(), persons),
        (other.by(), agents),
        (other.by(), persons),
        (Admin(cookie), agents),
    ] {
        assert_eq!(
            store.remove_report_note(caller, report, note).await.err(),
            Some(Error::Rejected)
        );
    }
    store
        .remove_report_note(one.by(), report, agents)
        .await
        .unwrap();
    store
        .remove_report_note(Admin(cookie), report, persons)
        .await
        .unwrap();
}

/// A report's notes and its history are bounded, whoever writes them.
async fn bounded(store: &AccountStore, operator: &DatabaseConnection, cookie: &Token) {
    let one = made(store, cookie, "Talkative", ReportScopes::EVERYTHING).await;
    let filer = store.report_token_filer(&one.token).await.unwrap();
    let report = store
        .file_report(&filer, &filed("Talked about"))
        .await
        .unwrap();
    let note = "n".repeat(MAX_NOTE);
    for _ in 0..MAX_NOTES_BYTES / MAX_NOTE {
        store
            .add_report_note(one.by(), report, &note)
            .await
            .unwrap();
    }
    for caller in [one.by(), Admin(cookie)] {
        assert_eq!(
            store.add_report_note(caller, report, &note).await.err(),
            Some(Error::Busy)
        );
    }
    let rest = "n".repeat(MAX_NOTES_BYTES % MAX_NOTE);
    store
        .add_report_note(one.by(), report, &rest)
        .await
        .unwrap();
    assert_eq!(
        store
            .add_report_note(Admin(cookie), report, "n")
            .await
            .err(),
        Some(Error::Busy)
    );
    // the newest lines, oldest first, and that there are older ones
    let detail = store.report_detail(one.by(), report).await.unwrap();
    assert!(!detail.older && detail.audit.len() < HISTORY);
    sql(
        operator,
        "INSERT INTO central_identity.report_audit (report_id, action, detail)
         SELECT $1, 'report.synthetic', n::text FROM pg_catalog.generate_series(1, $2::bigint) n",
        vec![
            Uuid::from_bytes(report).into(),
            i64::try_from(HISTORY).unwrap().into(),
        ],
    )
    .await;
    let detail = store.report_detail(Admin(cookie), report).await.unwrap();
    assert!(detail.older);
    assert_eq!(detail.audit.len(), HISTORY);
    assert_eq!(detail.audit[HISTORY - 1].detail, HISTORY.to_string());
    assert!(
        detail
            .audit
            .iter()
            .all(|row| row.action == "report.synthetic")
    );
    store.revoke_report_token(cookie, one.id).await.unwrap();
}

/// A token that is revoked, over or whose maker is no administrator does
/// nothing from that moment, whatever was begun; with the role its maker's
/// tokens work again.
async fn ending(
    store: &AccountStore,
    operator: &DatabaseConnection,
    cookie: &Token,
    admin: Identity,
) {
    let helper = made(store, cookie, "Helper", ReportScopes::EVERYTHING).await;
    let scene = Scene {
        store,
        operator,
        cookie,
        helper: &helper,
    };
    let nothing = |one: Made| {
        let scene = &scene;
        async move {
            for (act, _) in ACTS {
                assert_eq!(
                    scene.tries(&one, act).await,
                    Err(Error::Rejected),
                    "{act:?}"
                );
            }
            for right in ReportScope::ALL {
                assert_eq!(
                    store.report_token(&one.token, right).await.err(),
                    Some(Error::Rejected)
                );
            }
        }
    };
    // begun before the revocation, stored after it: refused
    let one = made(store, cookie, "Revoked", ReportScopes::EVERYTHING).await;
    let filer = store.report_token_filer(&one.token).await.unwrap();
    let report = store.file_report(&filer, &filed("Begun")).await.unwrap();
    store.revoke_report_token(cookie, one.id).await.unwrap();
    assert_eq!(
        store.file_report(&filer, &filed("Too late")).await.err(),
        Some(Error::Rejected)
    );
    assert_eq!(
        store
            .file_report_part(&filer, report, Part::Client, &gzip(3, 400))
            .await
            .err(),
        Some(Error::Rejected)
    );
    nothing(one).await;
    // over
    let one = made(store, cookie, "Over", ReportScopes::EVERYTHING).await;
    sql(
        operator,
        "UPDATE central_identity.report_tokens SET created_at = created_at - interval '31 days',
             expires_at = expires_at - interval '31 days' WHERE id = $1",
        vec![Uuid::from_bytes(one.id).into()],
    )
    .await;
    nothing(one).await;
    maker(store, operator, cookie, admin).await;
    store.revoke_report_token(cookie, helper.id).await.unwrap();
}

/// A maker who is no administrator any more: their token is suspended, for
/// what it began as well, and works again with the role.
async fn maker(
    store: &AccountStore,
    operator: &DatabaseConnection,
    cookie: &Token,
    admin: Identity,
) {
    let one = made(store, cookie, "Suspended", ReportScopes::EVERYTHING).await;
    let filer = store.report_token_filer(&one.token).await.unwrap();
    sql(
        operator,
        "DELETE FROM central_identity.account_roles WHERE account_id = $1",
        vec![Uuid::from_bytes(admin.account).into()],
    )
    .await;
    for right in ReportScope::ALL {
        assert_eq!(
            store.report_token(&one.token, right).await.err(),
            Some(Error::Rejected)
        );
    }
    assert_eq!(
        store.file_report(&filer, &filed("Suspended")).await.err(),
        Some(Error::Rejected)
    );
    assert_eq!(
        store.report_stats(one.by()).await.err(),
        Some(Error::Rejected)
    );
    appoint_admin(operator, "rt-admin").await.unwrap();
    let mine = |tokens: Vec<ReportTokenInfo>| tokens.into_iter().find(|t| t.id == one.id).unwrap();
    assert!(!mine(store.admin_report_tokens(cookie).await.unwrap()).suspended);
    assert!(store.report_stats(one.by()).await.is_ok());
    // another administrator sees it so
    sql(
        operator,
        "UPDATE central_identity.report_tokens SET created_by = pg_catalog.uuidv7() WHERE id = $1",
        vec![Uuid::from_bytes(one.id).into()],
    )
    .await;
    let seen = mine(store.admin_report_tokens(cookie).await.unwrap());
    assert!(seen.suspended && seen.revoked_at.is_none());
    assert_eq!((seen.created_by_name, seen.created_by_tag), (None, None));
    store.revoke_report_token(cookie, one.id).await.unwrap();
}

/// A change and a revocation at once: the change waits for a revocation that
/// is being written and is refused by it, and goes on when there was none.
async fn in_flight(store: &AccountStore, operator: &DatabaseConnection, cookie: &Token) {
    let one = made(store, cookie, "In flight", ReportScopes::EVERYTHING).await;
    let filer = store.report_token_filer(&one.token).await.unwrap();
    let report = store.file_report(&filer, &filed("Raced")).await.unwrap();
    for (commits, expected) in [(false, None), (true, Some(Error::Rejected))] {
        let revoking = operator.begin().await.unwrap();
        revoking
            .execute_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "UPDATE central_identity.report_tokens SET revoked_at = pg_catalog.clock_timestamp()
                 WHERE id = $1",
                vec![Uuid::from_bytes(one.id).into()],
            ))
            .await
            .unwrap();
        let next = if commits {
            ReportStatus::Resolved
        } else {
            ReportStatus::Triaged
        };
        let change = status(next);
        let acting = store.set_report(one.by(), report, &change, None);
        tokio::pin!(acting);
        assert!(
            tokio::time::timeout(Duration::from_millis(300), &mut acting)
                .await
                .is_err(),
            "a change waits for the revocation being written"
        );
        if commits {
            revoking.commit().await.unwrap();
        } else {
            revoking.rollback().await.unwrap();
        }
        assert_eq!(acting.await.err(), expected);
    }
    let detail = store.report_detail(Admin(cookie), report).await.unwrap();
    assert_eq!(detail.summary.status, ReportStatus::Triaged);
}

/// A use is noted, to the minute.
async fn used(store: &AccountStore, operator: &DatabaseConnection, cookie: &Token) {
    let one = made(store, cookie, "Used", ReportScopes::ONLY_READ).await;
    let last = || async {
        store
            .admin_report_tokens(cookie)
            .await
            .unwrap()
            .into_iter()
            .find(|t| t.id == one.id)
            .unwrap()
            .last_used_at
    };
    // a right it does not hold is no use of it
    assert!(
        store
            .report_token(&one.token, ReportScope::File)
            .await
            .is_err()
    );
    assert_eq!(last().await, None);
    store
        .report_token(&one.token, ReportScope::Read)
        .await
        .unwrap();
    let first = last().await.unwrap();
    let back = |seconds: i64| {
        sql(
            operator,
            "UPDATE central_identity.report_tokens
             SET last_used_at = pg_catalog.clock_timestamp() - pg_catalog.make_interval(secs => $2) WHERE id = $1",
            vec![
                Uuid::from_bytes(one.id).into(),
                f64::from(i32::try_from(seconds).unwrap()).into(),
            ],
        )
    };
    back(50).await;
    store
        .report_token(&one.token, ReportScope::Read)
        .await
        .unwrap();
    assert!(last().await.unwrap() < first - 40, "within the minute");
    back(70).await;
    store
        .report_token(&one.token, ReportScope::Read)
        .await
        .unwrap();
    assert!(last().await.unwrap() >= first, "a minute later");
    store.revoke_report_token(cookie, one.id).await.unwrap();
}

/// At most so many live ones, and so many new ones a day, whatever became of
/// them.
async fn full(store: &AccountStore, operator: &DatabaseConnection, cookie: &Token) {
    let older = "UPDATE central_identity.report_tokens
         SET created_at = created_at - interval '2 days', expires_at = expires_at - interval '2 days'";
    sql(operator, older, vec![]).await;
    let live = || async {
        let now = count(
            operator,
            "SELECT count(*) AS n FROM central_identity.report_tokens
             WHERE revoked_at IS NULL AND expires_at > pg_catalog.clock_timestamp()",
            vec![],
        )
        .await;
        usize::try_from(now).unwrap()
    };
    let try_one = || async {
        let token = Token::generate().unwrap();
        store
            .create_report_token(cookie, "Many", ReportScopes::ONLY_POST, 30, &token)
            .await
            .map(|(id, _)| id)
    };
    let mut last = [0; 16];
    while live().await < LIVE_TOKENS {
        last = try_one().await.unwrap();
    }
    sql(operator, older, vec![]).await;
    assert_eq!(try_one().await.err(), Some(Error::Busy), "live ones");
    // one revoked makes room for one, until the day's are spent
    store.revoke_report_token(cookie, last).await.unwrap();
    let mut today = 0;
    loop {
        match try_one().await {
            Ok(id) => {
                today += 1;
                store.revoke_report_token(cookie, id).await.unwrap();
            }
            Err(error) => {
                assert_eq!(error, Error::Busy);
                break;
            }
        }
    }
    assert_eq!(today, TOKENS_PER_DAY);
    assert!(live().await < LIVE_TOKENS, "the day's, not the live ones'");
    // and the list is bounded, the live ones first
    let listed = store.admin_report_tokens(cookie).await.unwrap();
    assert!(listed.len() <= central_api::report_tokens::LISTED_TOKENS);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let works =
        |t: &ReportTokenInfo| t.revoked_at.is_none() && u64::try_from(t.expires_at).unwrap() > now;
    assert_eq!(
        listed.iter().take_while(|t| works(t)).count(),
        live().await,
        "the live ones first"
    );
    assert!(!listed.iter().skip(live().await).any(works));
}

/// The table holds a token's rights to their needs, its life to its bound
/// and a report of a token's to what a token may file, whoever writes.
async fn table(operator: &DatabaseConnection) {
    let insert = |rights: &str, expires: &str, digest: &str| {
        format!(
            "INSERT INTO central_identity.report_tokens
                 (name, digest, file, read, triage, purge, created_by, created_at, expires_at)
             VALUES ('CI', {digest}, {rights}, pg_catalog.uuidv7(), now(), {expires})"
        )
    };
    let (post, month) = ("true, false, false, false", "now() + interval '30 days'");
    let digest = "pg_catalog.sha256('x'::bytea)";
    for refused in [
        insert("false, false, false, false", month, digest),
        insert("false, false, true, false", month, digest),
        insert("false, true, false, true", month, digest),
        insert("true, false, true, true", month, digest),
        insert(post, "now()", digest),
        insert(post, "now() + interval '367 days'", digest),
        insert(post, month, "'\\x00'::bytea"),
    ] {
        assert!(
            operator.execute_unprepared(&refused).await.is_err(),
            "{refused}"
        );
    }
    let report = |channel: &str, origin: &str| {
        format!(
            "INSERT INTO central_identity.reports (gateway_id, gateway_label, filed_by, channel, reporter, kind, text,
                 record_bytes, record_sha256, record_complete, record_origin)
             VALUES (pg_catalog.uuidv7(), 'CI', 'token', '{channel}', 'ab', 'other', 't', 4,
                 pg_catalog.sha256(''::bytea), true, '{origin}')"
        )
    };
    for refused in [
        report("gateway", "gateway"),
        report("direct", "client"),
        report("token", "client"),
    ] {
        assert!(
            operator.execute_unprepared(&refused).await.is_err(),
            "{refused}"
        );
    }
    assert!(
        operator
            .execute_unprepared(
                "INSERT INTO central_identity.report_audit (report_id, token_name, action, detail)
                 VALUES (pg_catalog.uuidv7(), 'CI', 'report.status', '')"
            )
            .await
            .is_err(),
        "a token's line names the token"
    );
}

/// The runtime holds exactly its grants on the tokens: each wider and each
/// missing privilege refuses startup; it cannot change what a token may do,
/// when it ends, what it was made as, or delete one, nor say afterwards who
/// filed, wrote or changed something.
async fn grants(operator: &DatabaseConnection) {
    let on = "ON central_identity.report_tokens";
    for (change, undo) in [
        ("GRANT DELETE", "REVOKE DELETE"),
        ("GRANT UPDATE (purge)", "REVOKE UPDATE (purge)"),
        ("GRANT UPDATE (expires_at)", "REVOKE UPDATE (expires_at)"),
        ("GRANT UPDATE (digest)", "REVOKE UPDATE (digest)"),
        ("GRANT INSERT (revoked_at)", "REVOKE INSERT (revoked_at)"),
        ("GRANT INSERT (id)", "REVOKE INSERT (id)"),
        ("REVOKE INSERT (digest)", "GRANT INSERT (digest)"),
        ("REVOKE UPDATE (revoked_by)", "GRANT UPDATE (revoked_by)"),
        (
            "REVOKE UPDATE (last_used_at)",
            "GRANT UPDATE (last_used_at)",
        ),
        ("REVOKE SELECT", "GRANT SELECT"),
    ] {
        let side = |said: &str| {
            if said.starts_with("GRANT") {
                "TO"
            } else {
                "FROM"
            }
        };
        let said = |said: &str| format!("{said} {on} {} central_runtime", side(said));
        operator.execute_unprepared(&said(change)).await.unwrap();
        let refused =
            AccountStore::connect_dev(config("central_runtime", "CENTRAL_TEST_RUNTIME_PASSWORD"))
                .await
                .is_err();
        operator.execute_unprepared(&said(undo)).await.unwrap();
        assert!(refused, "{change}");
    }
    let runtime =
        central_postgres::connect_dev(config("central_runtime", "CENTRAL_TEST_RUNTIME_PASSWORD"))
            .await
            .unwrap();
    for statement in [
        "UPDATE central_identity.report_tokens SET purge = true",
        "UPDATE central_identity.report_tokens SET name = 'x'",
        "UPDATE central_identity.report_tokens SET expires_at = expires_at + interval '1 day'",
        "UPDATE central_identity.report_tokens SET created_by = pg_catalog.uuidv7()",
        "DELETE FROM central_identity.report_tokens",
        "TRUNCATE central_identity.report_tokens",
        "INSERT INTO central_identity.report_tokens (id, name, digest, file, read, triage, purge, created_by, expires_at)
         VALUES (pg_catalog.uuidv7(), 'x', pg_catalog.sha256('x'::bytea), true, false, false, false,
             pg_catalog.uuidv7(), now() + interval '1 day')",
        "UPDATE central_identity.reports SET filed_by = 'gateway'",
        "UPDATE central_identity.reports SET gateway_id = pg_catalog.uuidv7()",
        "UPDATE central_identity.report_audit SET token_name = NULL",
        "UPDATE central_identity.report_notes SET token_name = NULL",
        "UPDATE central_identity.audit_events SET report_token_id = NULL",
    ] {
        assert!(
            runtime.execute_unprepared(statement).await.is_err(),
            "{statement}"
        );
    }
    runtime.close().await.unwrap();
}
