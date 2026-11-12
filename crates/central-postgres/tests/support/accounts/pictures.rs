//! Migration 000027's profile pictures against real PostgreSQL: set,
//! replace and remove under new keys; the public read; the upload budget;
//! who learns a key; reports and their bounds; the administrators'
//! decisions with their audit; blocks; the sweep; deletion's cascade; and
//! the runtime's grants both ways. Synthetic accounts and bytes only.
use super::{AccountStore, Error, Identity, ServerKeys, Token, enroll};
use central_api::picture::{
    BlockChoice, PictureDecision, PictureReason, PictureReportState, TaggedPicture, UploadBlock,
};
use central_api::social::{SocialRequest as Request, SocialResponse as Response};
use central_postgres::pictures::{Encoding, PictureFiles, UPLOADS_PER_DAY, UPLOADS_PER_HOUR};
use central_postgres::{Session, appoint_admin, sweep};
use sea_orm::{ConnectionTrait, DatabaseConnection};

const OWNER: u32 = 3_611;
const VIEWER: u32 = 3_612;
const SHUNNED: u32 = 3_613;
const MODERATOR: u32 = 3_614;

async fn count(db: &impl ConnectionTrait, sql: &str) -> i64 {
    crate::scalar(db, sql).await
}

fn uuid(identity: Identity) -> String {
    sea_orm::prelude::Uuid::from_bytes(identity.account).to_string()
}

async fn browser(store: &AccountStore, identity: Identity) -> Token {
    let credential = store.credential(identity).await.unwrap().unwrap();
    let token = Token::generate().unwrap();
    store
        .issue_session(&credential, Session::Browser(&token))
        .await
        .unwrap();
    token
}

/// A converted pair as the service would store it: synthetic bytes within
/// the schema's bounds, and their digest stand-in.
fn pair(n: u8) -> (Vec<u8>, Vec<u8>, [u8; 32]) {
    (vec![n; 100], vec![n.wrapping_add(1); 200], [n; 32])
}

async fn set(
    store: &AccountStore,
    cookie: &Token,
    account: Identity,
    key: u8,
    n: u8,
) -> Result<(), Error> {
    let (avif, png, sha) = pair(n);
    store
        .set_picture(
            cookie,
            account.account,
            [key; 16],
            PictureFiles {
                avif: &avif,
                png: &png,
                sequence: None,
                sha256: sha,
            },
        )
        .await
        .map(|_| ())
}

async fn keys(
    store: &AccountStore,
    cookie: &Token,
    tags: Vec<u32>,
) -> Result<Vec<TaggedPicture>, Error> {
    match store
        .social(Session::Browser(cookie), Request::Pictures { tags })
        .await?
    {
        Response::Pictures { pictures } => Ok(pictures),
        _ => panic!("pictures expected"),
    }
}

async fn report(
    store: &AccountStore,
    cookie: &Token,
    tag: u32,
    key: u8,
    note: Option<&str>,
) -> Result<(), Error> {
    store
        .social(
            Session::Browser(cookie),
            Request::ReportPicture {
                tag,
                key: [key; 16],
                reason: PictureReason::Hate,
                note: note.map(str::to_owned),
            },
        )
        .await
        .map(|_| ())
}

struct People {
    owner: (Identity, Token),
    viewer: (Identity, Token),
    shunned: (Identity, Token),
    moderator: (Identity, Token),
}

pub(super) async fn run(store: &AccountStore, operator: &DatabaseConnection, keys_: &ServerKeys) {
    let mut people = Vec::new();
    for (legacy, name, shown, tag) in [
        ("picture-o", "pictureowner", "PicOwner", OWNER),
        ("picture-v", "pictureviewer", "PicViewer", VIEWER),
        ("picture-s", "pictureshunned", "PicShunned", SHUNNED),
        ("picture-m", "picturemoderator", "PicModerator", MODERATOR),
    ] {
        let identity = enroll(store, keys_, legacy, name).await;
        store
            .legacy_profile(legacy, Some(shown), tag)
            .await
            .unwrap();
        people.push((identity, browser(store, identity).await));
    }
    let mut people = people.into_iter();
    let mut next = || people.next().unwrap();
    let people = People {
        owner: next(),
        viewer: next(),
        shunned: next(),
        moderator: next(),
    };
    appoint_admin(operator, "picturemoderator").await.unwrap();
    pictures(store, &people).await;
    budget(store, operator, &people).await;
    visibility(store, operator, &people).await;
    reports(store, operator, &people).await;
    decisions(store, operator, &people).await;
    grants(store, operator).await;
    retention_and_deletion(store, operator, &people).await;
}

/// Set, replace, remove: a new key each time, the old one finds nothing.
async fn pictures(store: &AccountStore, people: &People) {
    let (owner, cookie) = &people.owner;
    let admission = store.picture_admission(cookie).await.unwrap();
    assert_eq!(admission.account, owner.account);
    assert_eq!((admission.block, admission.room), (None, true));
    assert!(store.my_picture(cookie).await.unwrap().picture.is_none());
    set(store, cookie, *owner, 1, 10).await.unwrap();
    let mine = store.my_picture(cookie).await.unwrap().picture.unwrap();
    assert_eq!(
        (mine.key, mine.side, mine.avif_bytes, mine.png_bytes),
        ([1; 16], 256, 100, 200)
    );
    assert_eq!((mine.sha256, mine.sequence_bytes), ([10; 32], None));
    let (avif, png, sha) = pair(10);
    assert_eq!(
        store.picture_bytes([1; 16], Encoding::Avif).await.unwrap(),
        Some((avif, sha))
    );
    assert_eq!(
        store.picture_bytes([1; 16], Encoding::Png).await.unwrap(),
        Some((png, sha))
    );
    set(store, cookie, *owner, 2, 20).await.unwrap();
    assert_eq!(
        store.picture_bytes([1; 16], Encoding::Avif).await.unwrap(),
        None
    );
    assert!(
        store
            .picture_bytes([2; 16], Encoding::Png)
            .await
            .unwrap()
            .is_some()
    );
    // another account's session, a key taken, an unknown session
    let (_, viewer) = &people.viewer;
    assert_eq!(
        set(store, viewer, *owner, 3, 30).await,
        Err(Error::Rejected)
    );
    assert_eq!(
        set(store, viewer, people.viewer.0, 2, 30).await,
        Err(Error::Conflict)
    );
    let stranger = Token::generate().unwrap();
    assert_eq!(
        store.picture_admission(&stranger).await.err(),
        Some(Error::Rejected)
    );
    assert_eq!(
        store.my_picture(&stranger).await.err(),
        Some(Error::Rejected)
    );
    assert_eq!(
        store
            .picture_bytes([2; 16], Encoding::Sequence)
            .await
            .unwrap(),
        None,
        "a still has no sequence"
    );
    moving(store, cookie, *owner).await;
    store.remove_picture(cookie).await.unwrap();
    for (key, encoding) in [([2; 16], Encoding::Avif), ([11; 16], Encoding::Sequence)] {
        assert_eq!(store.picture_bytes(key, encoding).await.unwrap(), None);
    }
    assert!(store.my_picture(cookie).await.unwrap().picture.is_none());
    store.remove_picture(cookie).await.unwrap();
    set(store, cookie, *owner, 4, 40).await.unwrap();
}

/// A picture that moves: its sequence is stored and served beside the
/// still under the same key, and its size is the owner's to see.
async fn moving(store: &AccountStore, cookie: &Token, owner: Identity) {
    let sequence = vec![0x5e; 300];
    store
        .set_picture(
            cookie,
            owner.account,
            [11; 16],
            PictureFiles {
                avif: &[11; 100],
                png: &[12; 200],
                sequence: Some(&sequence),
                sha256: [11; 32],
            },
        )
        .await
        .unwrap();
    let mine = store.my_picture(cookie).await.unwrap().picture.unwrap();
    assert_eq!((mine.key, mine.sequence_bytes), ([11; 16], Some(300)));
    assert_eq!(
        store
            .picture_bytes([11; 16], Encoding::Sequence)
            .await
            .unwrap(),
        Some((sequence, [11; 32]))
    );
}

/// Ten uploads an hour, thirty a day, in a row that survives restarts.
async fn budget(store: &AccountStore, operator: &DatabaseConnection, people: &People) {
    let (viewer, cookie) = &people.viewer;
    for _ in 0..UPLOADS_PER_HOUR {
        store.charge_picture(viewer.account).await.unwrap();
    }
    assert_eq!(store.charge_picture(viewer.account).await, Err(Error::Busy));
    assert!(!store.picture_admission(cookie).await.unwrap().room);
    let id = uuid(*viewer);
    operator
        .execute_unprepared(&format!(
            "UPDATE central_identity.picture_budgets SET hour_start = hour_start - interval '61 minutes'
             WHERE account_id = '{id}'"
        ))
        .await
        .unwrap();
    assert!(store.picture_admission(cookie).await.unwrap().room);
    store.charge_picture(viewer.account).await.unwrap();
    operator
        .execute_unprepared(&format!(
            "UPDATE central_identity.picture_budgets SET hour_count = 0, day_count = {UPLOADS_PER_DAY}
             WHERE account_id = '{id}'"
        ))
        .await
        .unwrap();
    assert_eq!(store.charge_picture(viewer.account).await, Err(Error::Busy));
    assert!(!store.picture_admission(cookie).await.unwrap().room);
    operator
        .execute_unprepared(&format!(
            "UPDATE central_identity.picture_budgets SET day_start = day_start - interval '25 hours'
             WHERE account_id = '{id}'"
        ))
        .await
        .unwrap();
    store.charge_picture(viewer.account).await.unwrap();
}

/// Who learns a key: anyone signed in, by tag, paying the lookup budget
/// outside contacts and groups; never across a block.
async fn visibility(store: &AccountStore, operator: &DatabaseConnection, people: &People) {
    let (_, owner) = &people.owner;
    let (viewer_id, viewer) = &people.viewer;
    let (_, shunned) = &people.shunned;
    let lookups = |id: Identity| {
        format!(
            "SELECT COALESCE((SELECT lookups::bigint FROM central_identity.social_budgets WHERE account_id = '{}'), 0) AS value",
            uuid(id)
        )
    };
    let owner_id = people.owner.0;
    let held = count(
        operator,
        &format!(
            "SELECT count(*)::bigint AS value FROM central_identity.pictures WHERE account_id = '{}'",
            uuid(owner_id)
        ),
    )
    .await;
    let tag = count(
        operator,
        &format!(
            "SELECT COALESCE(tag, -1)::bigint AS value FROM central_identity.accounts WHERE id = '{}'",
            uuid(owner_id)
        ),
    )
    .await;
    assert_eq!(
        (held, tag),
        (1, i64::from(OWNER)),
        "the owner's picture and tag"
    );
    let before = count(operator, &lookups(*viewer_id)).await;
    let found = keys(store, viewer, vec![OWNER, 999_999, VIEWER, OWNER])
        .await
        .unwrap();
    assert_eq!(
        found,
        vec![TaggedPicture {
            tag: OWNER,
            key: [4; 16],
            moving: false,
        }]
    );
    // the owner's tag and the unknown one are outside; its own is not
    assert_eq!(count(operator, &lookups(*viewer_id)).await, before + 2);
    // one's own key, free
    assert_eq!(
        keys(store, owner, vec![OWNER]).await.unwrap()[0].key,
        [4; 16]
    );
    // a contact costs nothing
    for (token, tag, change) in [
        (viewer, OWNER, central_api::social::ContactAction::Request),
        (owner, VIEWER, central_api::social::ContactAction::Accept),
    ] {
        store
            .social(Session::Browser(token), Request::Contact { tag, change })
            .await
            .unwrap();
    }
    let paid = count(operator, &lookups(*viewer_id)).await;
    keys(store, viewer, vec![OWNER]).await.unwrap();
    assert_eq!(count(operator, &lookups(*viewer_id)).await, paid);
    // a block, either way, withholds the key
    store
        .social(
            Session::Browser(owner),
            Request::Contact {
                tag: SHUNNED,
                change: central_api::social::ContactAction::Block,
            },
        )
        .await
        .unwrap();
    assert_eq!(keys(store, shunned, vec![OWNER]).await.unwrap(), []);
    assert_eq!(keys(store, owner, vec![SHUNNED]).await.unwrap(), []);
    // bounds: too many tags, a spent budget (all or nothing)
    assert_eq!(
        keys(store, viewer, vec![1; 301]).await,
        Err(Error::Rejected)
    );
    operator
        .execute_unprepared(&format!(
            "UPDATE central_identity.social_budgets SET lookups = 119 WHERE account_id = '{}'",
            uuid(*viewer_id)
        ))
        .await
        .unwrap();
    assert_eq!(
        keys(store, viewer, vec![SHUNNED, MODERATOR]).await,
        Err(Error::Busy)
    );
    assert_eq!(count(operator, &lookups(*viewer_id)).await, 119);
    operator
        .execute_unprepared(&format!(
            "UPDATE central_identity.social_budgets SET lookups = 0 WHERE account_id = '{}'",
            uuid(*viewer_id)
        ))
        .await
        .unwrap();
}

/// A report holds the digest; refusals look alike; duplicates and rates.
async fn reports(store: &AccountStore, operator: &DatabaseConnection, people: &People) {
    let (owner_id, owner) = &people.owner;
    let (viewer_id, viewer) = &people.viewer;
    let (_, shunned) = &people.shunned;
    report(store, viewer, OWNER, 4, Some("Synthetic note."))
        .await
        .unwrap();
    assert_eq!(
        report(store, viewer, OWNER, 4, None).await,
        Err(Error::Conflict)
    );
    // one's own, a stale key, across a block, an unknown tag: alike
    for (token, tag, key) in [
        (owner, OWNER, 4),
        (viewer, OWNER, 1),
        (shunned, OWNER, 4),
        (viewer, 999_999, 4),
    ] {
        assert_eq!(
            report(store, token, tag, key, None).await,
            Err(Error::Rejected)
        );
    }
    let long = "x".repeat(1_001);
    assert_eq!(
        report(store, viewer, OWNER, 4, Some(&long)).await,
        Err(Error::Rejected)
    );
    assert_eq!(
        report(store, viewer, OWNER, 4, Some("\u{1b}[31m")).await,
        Err(Error::Rejected)
    );
    let filed = store.my_picture_reports(viewer).await.unwrap();
    assert_eq!(filed.len(), 1);
    assert_eq!(
        (filed[0].reason, filed[0].state),
        (PictureReason::Hate, PictureReportState::Open)
    );
    assert_eq!(filed[0].note.as_deref(), Some("Synthetic note."));
    // the reported person never learns of a report
    assert!(store.my_picture(owner).await.unwrap().notice.is_none());
    // twenty an hour from one reporter
    operator
        .execute_unprepared(&format!(
            "INSERT INTO central_identity.picture_reports (account_id, sha256, reporter_id, reason, state, decided_at)
             SELECT '{}', decode(lpad(to_hex(n), 64, '0'), 'hex'), '{}', 'other', 'gone', clock_timestamp()
             FROM generate_series(1, 19) n",
            uuid(*owner_id),
            uuid(*viewer_id)
        ))
        .await
        .unwrap();
    let (moderator_id, _) = &people.moderator;
    set(store, &people.moderator.1, *moderator_id, 5, 50)
        .await
        .unwrap();
    assert_eq!(
        report(store, viewer, MODERATOR, 5, None).await,
        Err(Error::Busy)
    );
    operator
        .execute_unprepared(&format!(
            "UPDATE central_identity.picture_reports SET created_at = created_at - interval '2 hours'
             WHERE reporter_id = '{}' AND state = 'gone'",
            uuid(*viewer_id)
        ))
        .await
        .unwrap();
    report(store, viewer, MODERATOR, 5, None).await.unwrap();
}

/// Keep and remove, each needing a fresh administrator; the audit holds
/// reason codes only; blocks refuse uploads until they end or are lifted.
async fn decisions(store: &AccountStore, operator: &DatabaseConnection, people: &People) {
    let (owner_id, _) = &people.owner;
    let (_, moderator) = &people.moderator;
    let (_, viewer) = &people.viewer;
    let audit = |action: &str| audit(*owner_id, action);
    // not an administrator
    assert_eq!(
        store.admin_picture_reports(viewer, None, None).await.err(),
        Some(Error::Rejected)
    );
    let (queue, more) = store
        .admin_picture_reports(moderator, Some(PictureReportState::Open), None)
        .await
        .unwrap();
    assert!(!more);
    let ours: Vec<_> = queue
        .iter()
        .filter(|r| r.account.tag == Some(OWNER))
        .collect();
    assert_eq!(ours.len(), 1);
    assert_eq!(ours[0].reporter.as_ref().and_then(|r| r.tag), Some(VIEWER));
    assert_eq!(ours[0].current, Some([4; 16]));
    let sha = ours[0].sha256;
    // a stale sign-in may read but not decide
    operator
        .execute_unprepared(&format!(
            "UPDATE central_identity.sessions SET created_at = created_at - interval '31 minutes'
             WHERE account_id = '{}' AND kind = 'browser'",
            uuid(people.moderator.0)
        ))
        .await
        .unwrap();
    assert_eq!(
        store
            .decide_picture(moderator, owner_id.account, sha, &PictureDecision::Keep)
            .await,
        Err(Error::Rejected)
    );
    let moderator = &browser(store, people.moderator.0).await;
    store
        .decide_picture(moderator, owner_id.account, sha, &PictureDecision::Keep)
        .await
        .unwrap();
    assert_eq!(count(operator, &audit("picture.kept")).await, 1);
    // nothing open is left to keep
    assert_eq!(
        store
            .decide_picture(moderator, owner_id.account, sha, &PictureDecision::Keep)
            .await,
        Err(Error::Conflict)
    );
    removal(store, operator, people, moderator, sha).await;
    blocks(store, operator, people, moderator).await;
}

/// The audit events of `action` about `owner`, by an administrator.
fn audit(owner: Identity, action: &str) -> String {
    format!(
        "SELECT count(*)::bigint AS value FROM central_identity.audit_events WHERE action = '{action}'
         AND account_id = '{}' AND actor_kind = 'admin'",
        uuid(owner)
    )
}

/// A removal with a reason, a note and a block: the bytes go at once, the
/// person is told, the audit holds the reason code only.
async fn removal(
    store: &AccountStore,
    operator: &DatabaseConnection,
    people: &People,
    moderator: &Token,
    sha: [u8; 32],
) {
    let (owner_id, owner) = &people.owner;
    let (_, viewer) = &people.viewer;
    let audit = |action: &str| audit(*owner_id, action);
    report(store, viewer, OWNER, 4, None).await.unwrap();
    let remove = PictureDecision::Remove {
        reason: PictureReason::Hate,
        note: Some("Synthetic reason sentence.".into()),
        block: BlockChoice::Days { days: 7 },
    };
    assert_eq!(
        store
            .decide_picture(moderator, owner_id.account, [0; 32], &remove)
            .await,
        Err(Error::Conflict)
    );
    for bad in [
        BlockChoice::Days { days: 0 },
        BlockChoice::Days { days: 366 },
    ] {
        let wrong = PictureDecision::Remove {
            reason: PictureReason::Hate,
            note: None,
            block: bad,
        };
        assert_eq!(
            store
                .decide_picture(moderator, owner_id.account, sha, &wrong)
                .await,
            Err(Error::Rejected)
        );
    }
    store
        .decide_picture(moderator, owner_id.account, sha, &remove)
        .await
        .unwrap();
    assert_eq!(
        store.picture_bytes([4; 16], Encoding::Avif).await.unwrap(),
        None
    );
    let own = store.my_picture(owner).await.unwrap();
    assert!(own.picture.is_none());
    let notice = own.notice.unwrap();
    assert_eq!(
        (notice.reason, notice.note.as_deref()),
        (PictureReason::Hate, Some("Synthetic reason sentence."))
    );
    assert!(matches!(own.block, Some(UploadBlock::Until { .. })));
    assert_eq!(count(operator, &audit("picture.removed")).await, 1);
    assert_eq!(count(operator, &audit("picture.blocked")).await, 1);
    assert_eq!(
        count(operator, &format!(
            "SELECT count(*)::bigint AS value FROM central_identity.audit_events WHERE action = 'picture.removed'
             AND detail = '{{\"reason\": \"hate\"}}'::jsonb AND account_id = '{}'", uuid(*owner_id))).await,
        1,
        "the reason code, never the note"
    );
    // the bytes went at once; only the digest stays with the reports
    assert_eq!(
        count(
            operator,
            &format!(
                "SELECT count(*)::bigint AS value FROM central_identity.picture_reports
             WHERE account_id = '{}' AND sha256 = '\\x{}' AND state = 'removed'",
                uuid(*owner_id),
                "28".repeat(32)
            )
        )
        .await,
        1
    );
    assert_eq!(
        count(operator, &format!(
            "SELECT count(*)::bigint AS value FROM central_identity.pictures WHERE account_id = '{}'",
            uuid(*owner_id))).await,
        0
    );
    assert!(
        store
            .picture_admission(owner)
            .await
            .unwrap()
            .block
            .is_some()
    );
    assert_eq!(
        set(store, owner, *owner_id, 6, 60).await,
        Err(Error::Conflict)
    );
}

/// A block until lifted, then lifted; a replaced picture's open reports are
/// gone.
async fn blocks(
    store: &AccountStore,
    operator: &DatabaseConnection,
    people: &People,
    moderator: &Token,
) {
    let (owner_id, owner) = &people.owner;
    let (_, viewer) = &people.viewer;
    let audit = |action: &str| audit(*owner_id, action);
    let forever = PictureDecision::Remove {
        reason: PictureReason::Other,
        note: None,
        block: BlockChoice::UntilLifted,
    };
    operator
        .execute_unprepared(&format!(
            "UPDATE central_identity.picture_notices SET blocked_until = NULL WHERE account_id = '{}'",
            uuid(*owner_id)
        ))
        .await
        .unwrap();
    set(store, owner, *owner_id, 7, 70).await.unwrap();
    assert!(
        store.my_picture(owner).await.unwrap().notice.is_none(),
        "a new picture ends the notice"
    );
    store
        .decide_picture(moderator, owner_id.account, [70; 32], &forever)
        .await
        .unwrap();
    assert_eq!(
        store.my_picture(owner).await.unwrap().block,
        Some(UploadBlock::UntilLifted)
    );
    store
        .lift_picture_block(moderator, owner_id.account)
        .await
        .unwrap();
    assert_eq!(store.my_picture(owner).await.unwrap().block, None);
    assert_eq!(count(operator, &audit("picture.unblocked")).await, 1);
    let case = store
        .admin_picture_case(moderator, owner_id.account)
        .await
        .unwrap();
    assert_eq!(case.account.tag, Some(OWNER));
    assert!(
        case.reports
            .iter()
            .any(|r| r.state == PictureReportState::Kept)
    );
    // a replaced picture's open reports are gone
    set(store, owner, *owner_id, 8, 80).await.unwrap();
    report(store, viewer, OWNER, 8, None).await.unwrap();
    set(store, owner, *owner_id, 9, 90).await.unwrap();
    assert!(
        store
            .my_picture_reports(viewer)
            .await
            .unwrap()
            .iter()
            .any(|r| r.state == PictureReportState::Gone)
    );
    // the same picture back (same digest): its gone report is open again
    set(store, owner, *owner_id, 10, 80).await.unwrap();
    assert!(
        store
            .my_picture_reports(viewer)
            .await
            .unwrap()
            .iter()
            .any(|r| r.state == PictureReportState::Open),
        "reopened"
    );
    // a current picture no report names can be removed all the same, by
    // its digest, from the account's case; keeping it needs a report
    set(store, owner, *owner_id, 12, 120).await.unwrap();
    assert_eq!(
        store
            .decide_picture(
                moderator,
                owner_id.account,
                [120; 32],
                &PictureDecision::Keep
            )
            .await,
        Err(Error::Conflict)
    );
    let unreported = PictureDecision::Remove {
        reason: PictureReason::Private,
        note: None,
        block: BlockChoice::None,
    };
    store
        .decide_picture(moderator, owner_id.account, [120; 32], &unreported)
        .await
        .unwrap();
    assert!(store.my_picture(owner).await.unwrap().picture.is_none());
    assert_eq!(count(operator, &audit("picture.removed")).await, 3);
}

/// The runtime cannot delete a report, rewrite its words or move a picture
/// to another account.
async fn grants(store: &AccountStore, operator: &DatabaseConnection) {
    let _ = store;
    let runtime = central_postgres::connect_dev(super::config(
        "central_runtime",
        "CENTRAL_TEST_RUNTIME_PASSWORD",
    ))
    .await
    .unwrap();
    for refused in [
        "DELETE FROM central_identity.picture_reports",
        "UPDATE central_identity.picture_reports SET reason = 'other'",
        "UPDATE central_identity.picture_reports SET note = 'x'",
        "UPDATE central_identity.pictures SET account_id = account_id",
        "TRUNCATE central_identity.pictures",
    ] {
        assert!(
            runtime.execute_unprepared(refused).await.is_err(),
            "{refused}"
        );
    }
    assert!(
        count(
            operator,
            "SELECT count(*)::bigint AS value FROM central_identity.picture_reports"
        )
        .await
            > 0
    );
}

/// The sweep deletes decided reports after a year; deleting an account
/// takes its picture and the reports about it, and its own reports lose
/// their reporter, so no row names it.
async fn retention_and_deletion(
    store: &AccountStore,
    operator: &DatabaseConnection,
    people: &People,
) {
    let (owner_id, _) = &people.owner;
    let (viewer_id, _) = &people.viewer;
    let decided = "SELECT count(*)::bigint AS value FROM central_identity.picture_reports WHERE state <> 'open'";
    let before = count(operator, decided).await;
    operator
        .execute_unprepared(
            "UPDATE central_identity.picture_reports SET decided_at = decided_at - interval '366 days'
             WHERE state = 'kept'",
        )
        .await
        .unwrap();
    let swept = sweep(operator).await.unwrap();
    assert!(swept.picture_reports >= 1);
    assert_eq!(
        count(operator, decided).await,
        before - i64::try_from(swept.picture_reports).unwrap()
    );
    let _ = store;
    for (id, column) in [(owner_id, "account_id"), (viewer_id, "reporter_id")] {
        operator
            .execute_unprepared(&format!(
                "DELETE FROM central_identity.accounts WHERE id = '{}'",
                uuid(*id)
            ))
            .await
            .unwrap();
        for table in ["pictures", "picture_notices", "picture_budgets"] {
            assert_eq!(
                count(operator, &format!(
                    "SELECT count(*)::bigint AS value FROM central_identity.{table} WHERE account_id = '{}'",
                    uuid(*id)
                ))
                .await,
                0,
                "{table}"
            );
        }
        assert_eq!(
            count(
                operator,
                &format!(
                    "SELECT count(*)::bigint AS value FROM central_identity.picture_reports
                 WHERE account_id = '{0}' OR reporter_id = '{0}' OR decided_by = '{0}'",
                    uuid(*id)
                )
            )
            .await,
            0,
            "{column}"
        );
    }
}
