//! Migration 000011's display names and tags: a registered account draws its
//! tag with its reservation and chooses its name there or later; the bridge
//! carries a moved account's name and tag (`LegacyReserve`, and `LegacyProfile`
//! once only), and a carried tag another account holds draws a new one; the
//! draw widens and retries, seen with the range injected small.
use super::{AccountStore, Error, Identity, ServerKeys, Token, enroll, login};
use central_api::profile::{MAX_TAG, REGISTERED_TAG_FLOOR};
use central_postgres::{Session, TAG_DRAWS, TagDraw};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use std::{
    collections::VecDeque,
    ops::RangeInclusive,
    sync::{Arc, Mutex},
};

/// The stored name, tag and whether the name has its time, of the account
/// whose `column` (a fixed identifier) is `value`.
async fn stored(
    operator: &DatabaseConnection,
    column: &str,
    value: &str,
) -> (Option<String>, Option<i32>, bool) {
    let row = operator
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            format!(
                "SELECT display_name, tag, display_name_at IS NOT NULL AS timed
                 FROM central_identity.accounts WHERE {column} = $1"
            ),
            [value.into()],
        ))
        .await
        .unwrap()
        .unwrap();
    (
        row.try_get("", "display_name").unwrap(),
        row.try_get("", "tag").unwrap(),
        row.try_get("", "timed").unwrap(),
    )
}

async fn legacy(operator: &DatabaseConnection, id: &str) -> (Option<String>, Option<i32>, bool) {
    stored(operator, "legacy_id", id).await
}

async fn registered(
    operator: &DatabaseConnection,
    name: &str,
) -> (Option<String>, Option<i32>, bool) {
    stored(operator, "username_key", name).await
}

/// How many accounts are named `name`.
async fn count(operator: &DatabaseConnection, name: &str) -> i64 {
    operator
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT count(*) AS found FROM central_identity.accounts WHERE username_key = $1",
            [name.into()],
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "found")
        .unwrap()
}

fn four_digits(tag: Option<i32>) -> bool {
    tag.and_then(|t| u32::try_from(t).ok())
        .is_some_and(|t| (REGISTERED_TAG_FLOOR..=0xffff).contains(&t))
}

/// A browser session for `identity`, as a sign-in on the website issues it.
async fn browser(store: &AccountStore, identity: Identity) -> Token {
    let credential = store.credential(identity).await.unwrap().unwrap();
    let token = Token::generate().unwrap();
    store
        .issue_session(&credential, Session::Browser(&token))
        .await
        .unwrap();
    token
}

/// Registers and enrolls `name` with `display_name`.
async fn registration(
    store: &AccountStore,
    operator: &DatabaseConnection,
    keys: &ServerKeys,
    name: &str,
    display_name: Option<&str>,
) -> Identity {
    let key = super::register::issue(operator, 1, name).await;
    let ticket = Token::generate().unwrap();
    let identity = store
        .register(name, display_name, &key.digest(), &ticket)
        .await
        .unwrap();
    let enrolled = super::register::credentials(keys, identity);
    store
        .enroll(
            &ticket,
            &enrolled.upload,
            &enrolled.wrapped,
            &enrolled.document,
            Some(&enrolled.recovery),
        )
        .await
        .unwrap();
    identity
}

pub(super) async fn run(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    chosen(store, operator, keys).await;
    later(store, operator, keys).await;
    waiting(store, operator, keys).await;
    carried(store, operator).await;
    once(store, operator, keys).await;
    drawn(store, operator).await;
}

/// A name chosen at registration, with the tag drawn for the reservation;
/// a name a person may not choose is refused, after the key.
async fn chosen(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    let key = super::register::issue(operator, 2, "profile-refusals").await;
    for name in ["Support", "a-d-m-i-n", "_Chosen", "Chosen Name", "ab"] {
        assert_eq!(
            store
                .register(
                    "profile-refused",
                    Some(name),
                    &key.digest(),
                    &Token::generate().unwrap()
                )
                .await
                .err(),
            Some(Error::Rejected),
            "{name}"
        );
    }
    let unknown = central_api::beta_key::BetaKey::generate().unwrap();
    assert_eq!(
        store
            .register(
                "profile-refused",
                Some("Support"),
                &unknown.digest(),
                &Token::generate().unwrap()
            )
            .await
            .err(),
        Some(Error::Rejected),
        "an unknown key is refused as before"
    );
    assert_eq!(
        count(operator, "profile-refused").await,
        0,
        "nothing reserved"
    );
    let identity = registration(store, operator, keys, "profile-chosen", Some("Chosen_Name")).await;
    let (name, tag, timed) = registered(operator, "profile-chosen").await;
    assert_eq!(name.as_deref(), Some("Chosen_Name"), "kept as typed");
    assert!(four_digits(tag) && timed, "{tag:?}");
    let cookie = browser(store, identity).await;
    let summary = store.account_summary(&cookie).await.unwrap();
    assert_eq!(summary.display_name.as_deref(), Some("Chosen_Name"));
    assert_eq!(summary.tag.map(|t| i32::try_from(t).unwrap()), tag);
    // Two reservations never share a tag.
    let other = registration(
        store,
        operator,
        keys,
        "profile-chosen-two",
        Some("Chosen_Name"),
    )
    .await;
    let second = registered(operator, "profile-chosen-two").await.1;
    assert!(four_digits(second) && second != tag);
    let _ = other;
    store.logout(Session::Browser(&cookie)).await.unwrap();
}

/// No name at registration: the account chooses one on the account page,
/// once; the previous binary's registration draws its tag at the next sign-in.
async fn later(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    let identity = registration(store, operator, keys, "profile-later", None).await;
    let (name, tag, timed) = registered(operator, "profile-later").await;
    assert!(name.is_none() && !timed && four_digits(tag));
    let native = login(store, keys, identity).await;
    let cookie = browser(store, identity).await;
    for (token, name) in [
        (&native, "Later_Name"),
        (&cookie, "Admin"),
        (&cookie, "Sys-tem"),
        (&cookie, "x"),
        (&cookie, "Later Name"),
    ] {
        assert_eq!(
            store.set_display_name(token, name).await.err(),
            Some(Error::Rejected),
            "{name}"
        );
    }
    assert_eq!(registered(operator, "profile-later").await.0, None);
    store.set_display_name(&cookie, "Later_Name").await.unwrap();
    store.set_display_name(&cookie, "Later_Name").await.unwrap();
    assert_eq!(
        store.set_display_name(&cookie, "Other_Name").await.err(),
        Some(Error::Conflict),
        "renaming comes later"
    );
    let (name, after, timed) = registered(operator, "profile-later").await;
    assert_eq!(
        (name.as_deref(), after, timed),
        (Some("Later_Name"), tag, true)
    );
    assert_eq!(
        store
            .account_summary(&cookie)
            .await
            .unwrap()
            .display_name
            .as_deref(),
        Some("Later_Name")
    );
    store.logout(Session::Browser(&cookie)).await.unwrap();
    assert_eq!(
        store.set_display_name(&cookie, "Later_Name").await.err(),
        Some(Error::Rejected),
        "no live session"
    );

    // As the binary before 000011 registered one: no tag. The sign-in draws it.
    operator
        .execute_unprepared(
            "UPDATE central_identity.accounts SET tag = NULL WHERE username_key = 'profile-later'",
        )
        .await
        .unwrap();
    let cookie = browser(store, identity).await;
    let drawn = registered(operator, "profile-later").await.1;
    assert!(four_digits(drawn), "{drawn:?}");
    assert_eq!(
        store
            .account_summary(&cookie)
            .await
            .unwrap()
            .tag
            .map(|t| i32::try_from(t).unwrap()),
        drawn
    );
    store.logout(Session::Browser(&cookie)).await.unwrap();
    store.logout(&native).await.unwrap();
}

/// A moved account reserved before 000011: no tag, and no name to choose
/// until the bridge carries its own; its sign-in draws none. One whose name
/// did not keep the rule had only its tag carried, and chooses a name.
async fn waiting(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    let moved = enroll(store, keys, "profile-waiting", "profile-waiting").await;
    let cookie = browser(store, moved).await;
    assert_eq!(
        legacy(operator, "profile-waiting").await,
        (None, None, false)
    );
    assert_eq!(
        store.set_display_name(&cookie, "Waiting_Name").await.err(),
        Some(Error::Rejected)
    );
    store
        .legacy_profile("profile-waiting", Some("Waiting_Name"), 0x301)
        .await
        .unwrap();
    let summary = store.account_summary(&cookie).await.unwrap();
    assert_eq!(
        (summary.display_name.as_deref(), summary.tag),
        (Some("Waiting_Name"), Some(0x301))
    );
    store
        .set_display_name(&cookie, "Waiting_Name")
        .await
        .unwrap();
    assert_eq!(
        store.set_display_name(&cookie, "Other_Name").await.err(),
        Some(Error::Conflict)
    );
    store.logout(Session::Browser(&cookie)).await.unwrap();

    // Its name did not keep the rule, so only its tag was carried.
    let unnamed = enroll(store, keys, "profile-unnamed", "profile-unnamed").await;
    store
        .legacy_profile("profile-unnamed", None, 0x302)
        .await
        .unwrap();
    assert_eq!(
        legacy(operator, "profile-unnamed").await,
        (None, Some(0x302), false)
    );
    let cookie = browser(store, unnamed).await;
    store
        .set_display_name(&cookie, "Unnamed_Now")
        .await
        .unwrap();
    assert_eq!(
        legacy(operator, "profile-unnamed").await,
        (Some("Unnamed_Now".into()), Some(0x302), true)
    );
    store.logout(Session::Browser(&cookie)).await.unwrap();
}

/// `LegacyReserve` stores the bridge's name and tag; a renewed reservation
/// refreshes the name and keeps the tag; reserved names are carried as they
/// are; a malformed name or tag is refused; a taken tag draws a new one.
async fn carried(store: &AccountStore, operator: &DatabaseConnection) {
    let reserve = async |id: &str, name: Option<&str>, tag: Option<u32>| {
        store
            .reserve_legacy(id, id, name, tag, &Token::generate().unwrap())
            .await
    };
    let first = reserve("carry-a", Some("Carried_A"), Some(0x311))
        .await
        .unwrap();
    assert_eq!(
        legacy(operator, "carry-a").await,
        (Some("Carried_A".into()), Some(0x311), true)
    );
    let again = reserve("carry-a", Some("Carried_A2"), Some(0x311))
        .await
        .unwrap();
    assert_eq!(again.account, first.account);
    assert_eq!(
        legacy(operator, "carry-a").await,
        (Some("Carried_A2".into()), Some(0x311), true)
    );
    // A bridge from before sends neither: what was carried stays.
    reserve("carry-a", None, None).await.unwrap();
    assert_eq!(
        legacy(operator, "carry-a").await,
        (Some("Carried_A2".into()), Some(0x311), true)
    );
    reserve("carry-admin", Some("Admin"), Some(0x312))
        .await
        .unwrap();
    assert_eq!(
        legacy(operator, "carry-admin").await.0.as_deref(),
        Some("Admin")
    );
    for (name, tag) in [
        (Some("_Carried"), Some(0x313)),
        (Some("seventeen_chars_x"), Some(0x313)),
        (Some("Carried"), Some(0)),
        (Some("Carried"), Some(MAX_TAG + 1)),
    ] {
        assert_eq!(
            reserve("carry-bad", name, tag).await.err(),
            Some(Error::Rejected),
            "{name:?} {tag:?}"
        );
    }
    // Held by carry-a: the account keeps its name and draws a tag.
    let taken = reserve("carry-b", Some("Carried_B"), Some(0x311))
        .await
        .unwrap();
    let (name, tag, _) = legacy(operator, "carry-b").await;
    assert_eq!(name.as_deref(), Some("Carried_B"));
    assert!(four_digits(tag), "{tag:?}");
    assert_eq!(
        reserve("carry-b", Some("Carried_B"), Some(0x311))
            .await
            .unwrap()
            .account,
        taken.account
    );
    assert_eq!(
        legacy(operator, "carry-b").await.1,
        tag,
        "a tag, once held, stays"
    );
    assert_eq!(legacy(operator, "carry-a").await.1, Some(0x311));
}

/// `LegacyProfile` sets name and tag once, for an account reserved without
/// them; a taken tag draws a new one; anything else is refused.
async fn once(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    let identity = enroll(store, keys, "carry-c", "carry-c").await;
    assert_eq!(
        store
            .legacy_profile("carry-nobody", Some("Nobody"), 0x321)
            .await
            .err(),
        Some(Error::Rejected)
    );
    for (name, tag) in [
        (Some("_Carried"), 0x321),
        (Some("Carried"), 0),
        (Some("Carried"), MAX_TAG + 1),
    ] {
        assert_eq!(
            store.legacy_profile("carry-c", name, tag).await.err(),
            Some(Error::Rejected)
        );
    }
    store
        .legacy_profile("carry-c", Some("Carried_C"), 0x321)
        .await
        .unwrap();
    for (name, tag) in [
        (Some("Carried_C"), 0x321),
        (Some("Other"), 0x322),
        (None, 0x323),
    ] {
        assert_eq!(
            store.legacy_profile("carry-c", name, tag).await.err(),
            Some(Error::Conflict),
            "once only"
        );
    }
    assert_eq!(
        legacy(operator, "carry-c").await,
        (Some("Carried_C".into()), Some(0x321), true)
    );
    // Its tag held already: the account keeps its name and draws a tag.
    enroll(store, keys, "carry-d", "carry-d").await;
    store
        .legacy_profile("carry-d", Some("Carried_D"), 0x321)
        .await
        .unwrap();
    let (name, tag, _) = legacy(operator, "carry-d").await;
    assert_eq!(name.as_deref(), Some("Carried_D"));
    assert!(four_digits(tag), "{tag:?}");
    // A registered account has no legacy ID to name it by.
    assert_eq!(
        store
            .legacy_profile("profile-chosen", Some("Chosen"), 0x324)
            .await
            .err(),
        Some(Error::Rejected)
    );
    let _ = identity;
}

/// The range a registered account draws from widens once half of it is
/// taken, a taken draw is drawn again up to [`TAG_DRAWS`] times, and then
/// the registration is `Busy` and reserves nothing; so it is, too, once no
/// range is left. Seen with the floor injected (1: the tags 1 to 15, then
/// 16 to 255) and the draws given.
async fn drawn(store: &AccountStore, operator: &DatabaseConnection) {
    let given: Arc<Mutex<VecDeque<u32>>> = Arc::default();
    let asked: Arc<Mutex<Vec<RangeInclusive<u32>>>> = Arc::default();
    let draw: TagDraw = {
        let (given, asked) = (given.clone(), asked.clone());
        Arc::new(move |range: &RangeInclusive<u32>| {
            asked.lock().unwrap().push(range.clone());
            Ok(given.lock().unwrap().pop_front().unwrap_or(*range.start()))
        })
    };
    let small = store.clone().with_tag_draw(1, draw.clone());
    let key = super::register::issue(operator, 16, "profile-draws").await;
    let register = async |name: &str, draws: &[u32]| {
        given.lock().unwrap().clear();
        given.lock().unwrap().extend(draws);
        asked.lock().unwrap().clear();
        let result = small
            .register(name, None, &key.digest(), &Token::generate().unwrap())
            .await;
        let ranges: Vec<_> = asked.lock().unwrap().drain(..).collect();
        (result.map(|_| ()), ranges)
    };
    // Seven of fifteen taken: still fewer than half.
    for tag in 1..=7 {
        let (result, ranges) = register(&format!("draw-{tag}"), &[tag]).await;
        result.unwrap();
        assert!(ranges.iter().all(|r| *r == (1..=15)), "{ranges:?}");
        assert_eq!(
            registered(operator, &format!("draw-{tag}")).await.1,
            Some(i32::try_from(tag).unwrap())
        );
    }
    // Taken draws are drawn again.
    let (result, _) = register("draw-8", &[1, 2, 3, 8]).await;
    result.unwrap();
    assert_eq!(registered(operator, "draw-8").await.1, Some(8));
    // Eight of fifteen taken: the next range.
    let (result, ranges) = register("draw-9", &[16]).await;
    result.unwrap();
    assert!(ranges.iter().all(|r| *r == (16..=255)), "{ranges:?}");
    assert_eq!(registered(operator, "draw-9").await.1, Some(16));
    // Every draw taken: busy, and nothing reserved.
    let (result, ranges) = register("draw-busy", &[16; TAG_DRAWS]).await;
    assert_eq!(result.err(), Some(Error::Busy));
    assert_eq!(ranges.len(), TAG_DRAWS);
    assert_eq!(count(operator, "draw-busy").await, 0);
    // A draw outside its range is no tag.
    let (result, _) = register("draw-outside", &[300]).await;
    assert_eq!(result.err(), Some(Error::Unavailable));
    // The last range, MAX_TAG alone: once it is taken, no range is left.
    let last = store.clone().with_tag_draw(MAX_TAG, draw);
    last.register(
        "draw-last",
        None,
        &key.digest(),
        &Token::generate().unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(
        registered(operator, "draw-last").await.1,
        Some(i32::try_from(MAX_TAG).unwrap())
    );
    assert_eq!(
        last.register(
            "draw-none",
            None,
            &key.digest(),
            &Token::generate().unwrap()
        )
        .await
        .err(),
        Some(Error::Busy)
    );
}
