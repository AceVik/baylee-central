//! The signed-in area (`docs/proposals/frontend-rewrite.md` §6.2): one island
//! per section page, reading the account from Central in the browser. The
//! server renders the frame and the headings; it never sees the session.
//! The administration's pages are drawn by its own island in its own
//! module (`admin::Administration`, §27), inside the same [`frame`]: this
//! island never reaches into it.

use std::fmt::Write as _;

use central_api::email::MailCode;
use central_api::owner::{GatewayScopes, OwnerAnswer, OwnerRequest, SessionInfo, SessionKind};
use central_api::picture::PictureKey;
use central_api::profile::{
    MIN_BIRTH_AGE_YEARS, ProfileDetails, PublicBadges, is_at_least_age, parse_iso_date, tag_text,
    utc_date_from_unix_secs, validate_birth_date, validate_person_name,
};
use central_api::ranked::{Rank, Track};
use central_api::wire::{AccountSource, Request, Response};
use leptos::prelude::*;
use serde_json::json;

use super::auth::sign_in;
pub(super) use super::auth::{Problem, problem_text};
use super::central::{Failure, call};
use super::chat::glyph;
use super::crypto::{Crypto, field_bytes};
use super::{fragment, session};
use crate::data::Section;
use crate::words::{self, Lang};

/// What `AccountSummary` says, as the pages use it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Summary {
    /// The account this browser is signed in as (the administration's
    /// pages and overview creation date read it).
    pub(super) account: [u8; 16],
    /// The sign-in name, for a password manager.
    pub(super) username: String,
    display_name: Option<String>,
    tag: Option<u32>,
    moved: bool,
    recovery: bool,
    /// Whether Central grants this account the administration.
    pub(super) admin: bool,
}

impl Summary {
    fn handle(&self) -> Option<String> {
        self.display_name
            .as_deref()
            .zip(self.tag)
            .map(|(name, tag)| central_api::profile::handle(name, tag))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Load {
    Loading,
    SignedOut,
    Ready(Summary),
    Failed(Problem),
}

async fn summary() -> Load {
    let Some(stamp) = session::login_stamp() else {
        return Load::SignedOut;
    };
    let answer = call(&Request::AccountSummary).await;
    if session::login_stamp().as_deref() != Some(stamp.as_str()) {
        return Load::SignedOut;
    }
    match answer {
        Ok(Response::AccountSummary {
            identity,
            username,
            display_name,
            tag,
            source,
            recovery,
            admin,
            ..
        }) => {
            // the header names the account on every page from here on
            session::note_name(display_name.as_deref(), tag);
            Load::Ready(Summary {
                account: identity.account,
                username,
                display_name,
                tag,
                moved: source == AccountSource::Legacy,
                recovery,
                admin,
            })
        }
        Ok(_) => Load::Failed(Problem::Failed),
        Err(Failure::Rejected) => {
            session::forget();
            Load::SignedOut
        }
        Err(failure) => Load::Failed(failure.into()),
    }
}

/// A Unix time as the page's language writes a date and time in the browser's local timezone.
pub fn when(lang: Lang, seconds: i64) -> String {
    super::time::format_datetime_local(lang, seconds)
}

/// The day of a Unix time as the page's language writes a date in the browser's local timezone.
#[cfg_attr(not(feature = "admin"), allow(dead_code))]
pub fn day(lang: Lang, seconds: i64) -> String {
    super::time::format_unix_date_local(lang, seconds)
}

/// Signs out here: Central ends the session and clears the cookie, the
/// browser forgets the session's end and the chat's kept identity.
async fn sign_out(worker: &str, lang: Lang) {
    // this device stops getting the account's notifications first
    let _ = super::push::disable().await;
    let _ = call(&Request::Logout).await;
    session::forget();
    if let Some(crypto) = Crypto::start(worker) {
        let _ = crypto.ask("forget", json!({})).await;
    }
    session::go(&format!("/{}/", lang.code()));
}

/// Reads the account's remembered highest rank from the browser's local
/// session note so overview and profile avatars draw their ranked border
/// immediately while `own_badges()` is in flight.
#[cfg(target_arch = "wasm32")]
fn remembered_rank() -> Option<Rank> {
    use central_api::ranked::League;
    let of = session::marker()?;
    let text = session::read(session::OWN)?;
    let held: serde_json::Value = serde_json::from_str(&text).ok()?;
    if held.get("of").and_then(serde_json::Value::as_str) != Some(of.as_str()) {
        return None;
    }
    let rank = held.get("rank")?.as_object()?;
    let tier = usize::try_from(rank.get("tier")?.as_u64()?).ok()?;
    let league = if tier == 11 {
        League::Unranked
    } else {
        *League::PLACED.get(tier)?
    };
    let division = rank
        .get("div")
        .and_then(serde_json::Value::as_u64)
        .and_then(|d| u8::try_from(d).ok())
        .filter(|&d| d > 0);
    let champion_place = rank
        .get("podium")
        .and_then(serde_json::Value::as_u64)
        .and_then(|p| u8::try_from(p).ok())
        .filter(|&p| (1..=3).contains(&p));
    Some(Rank {
        league,
        division,
        champion_place,
    })
}

/// On the server there is no remembered rank in local storage.
#[cfg(not(target_arch = "wasm32"))]
const fn remembered_rank() -> Option<Rank> {
    None
}

/// The password form that signs in again before a change that needs a
/// fresh sign-in (making a token, exporting).
pub(super) fn reauth(lang: Lang, worker: String, done: Callback<()>) -> impl IntoView {
    // only an action asks for it, so it takes the focus
    super::focus::when_shown("#reauth-name");
    let password = RwSignal::new(String::new());
    let error = RwSignal::new(None::<&'static str>);
    let busy = RwSignal::new(false);
    let username = RwSignal::new(String::new());
    let submit = move |event: leptos::ev::SubmitEvent| {
        event.prevent_default();
        if busy.get_untracked() {
            return;
        }
        let (name, secret, worker) = (
            username.get_untracked(),
            password.get_untracked(),
            worker.clone(),
        );
        password.set(String::new());
        busy.set(true);
        leptos::task::spawn_local(async move {
            match sign_in(&worker, &name, secret).await {
                Ok(_) => {
                    error.set(None);
                    done.run(());
                }
                Err(problem) => error.set(Some(problem_text(lang, problem))),
            }
            busy.set(false);
        });
    };
    view! {
        <form class="form reauth" method="post" novalidate=true on:submit=submit>
            <p>{words::reauth_lead(lang)}</p>
            <div class="field">
                <label for="reauth-name">{words::signin_name(lang)}</label>
                <div class="inset">
                    {super::field::mark(glyph::USER)}
                    <input id="reauth-name" type="text" autocomplete="username" autocapitalize="none" on:input=move |ev| username.set(event_target_value(&ev)) />
                </div>
            </div>
            {super::field::password_box(
                lang,
                "reauth-password",
                words::signin_password(lang),
                "current-password",
                password,
                None,
                None,
                "1024",
            )}
            <p class="error" role="alert">{move || error.get()}</p>
            <button type="submit" class="button" aria-disabled=move || busy.get().then_some("true")>{words::reauth_submit(lang)}</button>
        </form>
    }
}

fn uuid_v7_unix_secs(id: &[u8; 16]) -> Option<i64> {
    if !super::central::uuid_v7(id) {
        return None;
    }
    let mut millis_bytes = [0_u8; 8];
    millis_bytes[2..8].copy_from_slice(&id[0..6]);
    let millis = u64::from_be_bytes(millis_bytes);
    i64::try_from(millis / 1000).ok()
}

fn badge_summary(lang: Lang, badges: &PublicBadges) -> (String, u32, u32, usize) {
    let career_games: u32 = badges
        .ranked_standings
        .iter()
        .filter(|s| s.track == Track::Player)
        .map(|s| s.career.games)
        .sum();
    let career_wins: u32 = badges
        .ranked_standings
        .iter()
        .filter(|s| s.track == Track::Player)
        .map(|s| s.career.wins)
        .sum();
    let unlocked = badges.ranked_standings.len()
        + badges.ranked_awards.len()
        + badges
            .challenge_badges
            .iter()
            .filter(|b| b.tier.is_some())
            .count()
        + badges
            .tower_badges
            .iter()
            .filter(|b| b.tier.is_some())
            .count()
        + badges
            .echoes_badges
            .iter()
            .filter(|b| b.tier.is_some())
            .count()
        + badges
            .social_badges
            .iter()
            .filter(|b| b.tier.is_some())
            .count();
    let best_rank = badges.best_rank.map_or_else(
        || words::rank_league_unranked(lang).to_owned(),
        |r| super::ui::badges::rank_title(lang, r),
    );
    (best_rank, career_games, career_wins, unlocked)
}

type OverviewBadgeStats = (String, u32, u32, usize);

fn overview_stats(lang: Lang, stats: Memo<Option<OverviewBadgeStats>>) -> impl IntoView {
    view! {
        <dl class="overview-stats">
            <div>
                <dt>{words::player_best_rank(lang)}</dt>
                <dd>{move || stats.with(|s| s.as_ref().map_or_else(|| "–".to_owned(), |s| s.0.clone()))}</dd>
            </div>
            <div>
                <dt>{words::player_career_games(lang)}</dt>
                <dd translate="no">{move || stats.with(|s| s.as_ref().map_or_else(|| "–".to_owned(), |s| s.1.to_string()))}</dd>
            </div>
            <div>
                <dt>{words::player_career_wins(lang)}</dt>
                <dd translate="no">{move || stats.with(|s| s.as_ref().map_or_else(|| "–".to_owned(), |s| s.2.to_string()))}</dd>
            </div>
            <div>
                <dt>{words::player_badges_count(lang)}</dt>
                <dd translate="no">{move || stats.with(|s| s.as_ref().map_or_else(|| "–".to_owned(), |s| s.3.to_string()))}</dd>
            </div>
        </dl>
    }
}

fn home_admin_cards(lang: Lang, code: &'static str) -> impl IntoView {
    let to = |slug: &'static str, name: &'static str| {
        view! { <li><a class="button button-ghost" href=format!("/{code}/account/admin{slug}")>{name}</a></li> }
    };
    view! {
        <section class="card admin-card" aria-labelledby="admin-card-title">
            <h2 id="admin-card-title">{words::account_admin(lang)}</h2>
            <ul class="admin-links">
                {to("/overview", words::account_home(lang))}
                {to("/feedback", words::fb_title(lang))}
                {to("/pictures", words::admin_pictures_title(lang))}
                {to("/users", words::admin_accounts(lang))}
                {to("", words::admin_reset_title(lang))}
            </ul>
        </section>
    }
}

fn home(lang: Lang, summary: &Summary) -> AnyView {
    let code = lang.code();
    let badges = RwSignal::new(None::<PublicBadges>);
    let pic = RwSignal::new(super::picture::remembered_tuple());
    let initial_rank = remembered_rank();
    Effect::new(move |_| {
        leptos::task::spawn_local(async move {
            if let Ok(found) = super::central::own_badges().await {
                session::note_rank(found.best_rank.as_ref());
                badges.set(Some(found));
            }
        });
        leptos::task::spawn_local(async move {
            pic.set(super::picture::own_tuple().await);
        });
    });
    let stats = Memo::new(move |_| badges.with(|b| b.as_ref().map(|b| badge_summary(lang, b))));
    let needs = (!summary.recovery).then(|| {
        view! { <li><a href=format!("/{code}/account/profile")>{words::account_needs_code(lang)}</a></li> }
    });
    let name = summary.display_name.is_none().then(|| {
        view! { <li><a href=format!("/{code}/account/profile")>{words::account_needs_name(lang)}</a></li> }
    });
    let attention = (needs.is_some() || name.is_some()).then(|| {
        view! {
            <section class="card needs" aria-labelledby="needs-title">
                <h2 id="needs-title">{words::account_needs_title(lang)}</h2>
                <ul>{needs}{name}</ul>
            </section>
        }
    });
    let admin = summary.admin.then(|| home_admin_cards(lang, code));
    let named = summary.display_name.clone().zip(summary.tag);
    let face_name = summary
        .display_name
        .clone()
        .unwrap_or_else(|| summary.username.clone());
    let seed = summary.handle().unwrap_or_else(|| summary.username.clone());
    let handle = match named {
        Some((name, tag)) => view! {
            <div class="me-handle" id="me-name">
                {super::ui::copy_handle::handle_with_copy(lang, name, tag_text(tag), true)}
            </div>
        }
        .into_any(),
        None => view! { <p class="me-handle" id="me-name">{words::account_unnamed(lang)}</p> }
            .into_any(),
    };
    let role = summary
        .admin
        .then(|| view! { <span class="me-role">{words::admin_pill(lang)}</span> });
    let member_since = uuid_v7_unix_secs(&summary.account).map_or_else(
        || "–".to_owned(),
        |secs| super::time::format_unix_date_local(lang, secs),
    );
    view! {
        <section class="card me" aria-labelledby="me-name">
            {move || super::ui::rank_avatar::rank_avatar(
                face_name.clone(),
                &seed,
                pic.get(),
                badges.with(|b| b.as_ref().map_or(initial_rank, |b| b.best_rank)),
                super::ui::rank_avatar::AvatarSize::Large,
            )}
            <div class="me-text">
                {handle}
                <p class="meta">{words::account_signin_name(lang, &summary.username)}{role}</p>
            </div>
            <div class="me-aside">
                <p class="me-since">
                    <span class="me-since-label">{words::admin_created(lang)}</span>
                    <span aria-hidden="true">{" · "}</span>
                    <time translate="no">{member_since}</time>
                </p>
                <a class="button button-ghost me-edit" href=format!("/{code}/account/profile") title=words::account_profile(lang)>
                    {glyph::glyph(glyph::PENCIL)}
                    <span>{words::account_profile(lang)}</span>
                </a>
            </div>
            {overview_stats(lang, stats)}
        </section>
        {attention}
        {admin}
        {move || badges.with(|b| b.as_ref().map(|b| view! {
            <section class="card" aria-labelledby="badges-title">
                <h2 id="badges-title">{words::badges_title(lang)}</h2>
                {super::ui::badges::badge_wall(lang, b)}
            </section>
        }))}
    }
    .into_any()
}

fn display_name_chooser(
    lang: Lang,
    reload: Callback<()>,
    focus_next: StoredValue<Option<&'static str>>,
) -> impl IntoView {
    let chosen = RwSignal::new(String::new());
    let error = RwSignal::new(None::<&'static str>);
    let saving = RwSignal::new(false);
    let save = move |event: leptos::ev::SubmitEvent| {
        event.prevent_default();
        if saving.get_untracked() {
            return;
        }
        let name = chosen.get_untracked();
        if !central_api::profile::choosable(&name) {
            error.set(Some(words::register_error_display(lang)));
            return;
        }
        saving.set(true);
        leptos::task::spawn_local(async move {
            match call(&Request::SetDisplayName { display_name: name }).await {
                Ok(_) => {
                    // the form goes with the reload; the new handle takes the focus
                    focus_next.set_value(Some("#profile-who"));
                    reload.run(());
                }
                Err(Failure::Conflict) => error.set(Some(words::profile_name_taken(lang))),
                Err(failure) => error.set(Some(problem_text(lang, failure.into()))),
            }
            saving.set(false);
        });
    };
    view! {
        <form class="form" method="post" novalidate=true on:submit=save>
            <h3>{words::profile_choose_title(lang)}</h3>
            <div class="field">
                <label for="profile-name">{words::register_display(lang)}</label>
                <div class="inset">
                    {super::field::mark(glyph::ID_CARD)}
                    <input id="profile-name" type="text" maxlength="16" aria-describedby="profile-name-hint" on:input=move |ev| chosen.set(event_target_value(&ev)) />
                    <button
                        type="submit"
                        class="inset-end"
                        title=words::profile_name_save(lang)
                        aria-label=words::profile_name_save(lang)
                        aria-disabled=move || saving.get().then_some("true")
                    >
                        {glyph::glyph(glyph::CHECK)}
                    </button>
                </div>
                <p class="hint" id="profile-name-hint">{words::register_display_hint(lang)}</p>
            </div>
            <p class="error" role="alert">{move || error.get()}</p>
        </form>
    }
}

#[derive(Clone, Copy)]
struct ProfileDetailsState {
    details: RwSignal<Option<ProfileDetails>>,
    first_name: RwSignal<String>,
    last_name: RwSignal<String>,
    birth_date: RwSignal<Option<String>>,
    public_profile: RwSignal<bool>,
    rank: RwSignal<Option<Rank>>,
}

fn recovery_code_block(lang: Lang, worker: &str, kept: bool) -> AnyView {
    if kept {
        return view! { <p>{words::profile_code_kept(lang)}</p> }.into_any();
    }
    let asking = RwSignal::new(false);
    let worker = worker.to_owned();
    view! {
        <p>{words::profile_code_missing(lang)}</p>
        <Show when=move || asking.get() fallback=move || view! {
            <button type="button" class="button primary" on:click=move |_| asking.set(true)>{words::profile_code_make(lang)}</button>
        }>
            {
                let worker = worker.clone();
                let lang_code = lang.code();
                reauth(lang, worker, Callback::new(move |()| session::go(&format!("/{lang_code}/account/profile"))))
            }
        </Show>
    }
    .into_any()
}

fn profile_identity_actions(
    lang: Lang,
    worker: &str,
    named: Option<(String, u32)>,
) -> impl IntoView + use<> {
    let worker_logout = worker.to_owned();
    let open_profile = named.map(|(name, tag)| {
        let href = super::ui::copy_handle::player_path(lang, &name, &tag_text(tag));
        view! {
            <a class="button button-ghost" href=href title=words::player_open_profile(lang)>
                {glyph::glyph(glyph::EXTERNAL_LINK)}
                <span>{words::player_open_profile(lang)}</span>
            </a>
        }
    });
    view! {
        <p class="actions profile-identity-actions">
            {open_profile}
            <button
                type="button"
                class="button button-ghost"
                title=words::profile_sign_out(lang)
                on:click=move |_| {
                    let worker = worker_logout.clone();
                    leptos::task::spawn_local(async move { sign_out(&worker, lang).await });
                }
            >
                {glyph::glyph(glyph::LOG_OUT)}
                <span>{words::profile_sign_out(lang)}</span>
            </button>
        </p>
    }
}

fn public_profile_row(lang: Lang, state: ProfileDetailsState) -> impl IntoView {
    let public_busy = RwSignal::new(false);
    let public_error = RwSignal::new(None::<&'static str>);
    let toggle_public = move |ev| {
        if public_busy.get_untracked() || state.details.with_untracked(Option::is_none) {
            return;
        }
        let next = event_target_checked(&ev);
        state.public_profile.set(next);
        public_error.set(None);
        public_busy.set(true);
        let mut payload = state.details.get_untracked().unwrap_or_default();
        payload.public_profile = next;
        leptos::task::spawn_local(async move {
            match super::central::set_profile_details(payload.clone()).await {
                Ok(()) => state.details.set(Some(payload)),
                Err(failure) => {
                    state.public_profile.set(!next);
                    public_error.set(Some(problem_text(lang, failure.into())));
                }
            }
            public_busy.set(false);
        });
    };
    view! {
        <div class="profile-public-row">
            <div>
                <label for="profile-public">{words::profile_public_toggle(lang)}</label>
                <p class="hint" id="profile-public-hint">{words::profile_public_hint(lang)}</p>
            </div>
            <input
                id="profile-public"
                type="checkbox"
                role="switch"
                aria-describedby="profile-public-hint"
                prop:checked=move || state.public_profile.get()
                disabled=move || state.details.with(Option::is_none) || public_busy.get()
                on:change=toggle_public
            />
        </div>
        <p class="error" role="alert">{move || public_error.get()}</p>
    }
}

fn profile_identity_card(
    lang: Lang,
    worker: &str,
    summary: &Summary,
    reload: Callback<()>,
    focus_next: StoredValue<Option<&'static str>>,
    state: ProfileDetailsState,
) -> impl IntoView {
    let named = summary.display_name.clone().zip(summary.tag);
    let handle_cell = match named.clone() {
        Some((name, tag)) => {
            super::ui::copy_handle::handle_with_copy(lang, name, tag_text(tag), true)
        }
        None => view! { <span>{"–"}</span> }.into_any(),
    };
    let rank_sig = Signal::derive(move || state.rank.get());
    let (picture_side, picture_editor) = match summary.display_name.clone() {
        Some(display_name) => {
            let handle = summary.handle().unwrap_or_else(|| summary.username.clone());
            let parts = super::picture::card(lang, display_name, handle, rank_sig);
            (Some(parts.side), Some(parts.editor))
        }
        None => (None, None),
    };
    let choose = summary
        .display_name
        .is_none()
        .then(|| display_name_chooser(lang, reload, focus_next));
    let identity_body = view! {
        <div class="stack">
            <dl class="facts">
                <dt>{words::profile_handle(lang)}</dt>
                <dd>{handle_cell}</dd>
                <dt>{words::profile_signin_name(lang)}</dt>
                <dd translate="no">{summary.username.clone()}</dd>
                <dt>{words::profile_source(lang)}</dt>
                <dd>{if summary.moved { words::profile_source_moved(lang) } else { words::profile_source_registered(lang) }}</dd>
            </dl>
            {profile_identity_actions(lang, worker, named)}
            {public_profile_row(lang, state)}
            <div class="profile-code-block" aria-labelledby="profile-code">
                <h3 id="profile-code">{words::profile_code_title(lang)}</h3>
                {recovery_code_block(lang, worker, summary.recovery)}
            </div>
        </div>
    };
    let content: AnyView = match picture_side {
        Some(side) => view! {
            <div class="profile-identity-grid">
                {side}
                {identity_body}
            </div>
        }
        .into_any(),
        None => identity_body.into_any(),
    };
    view! {
        <section class="card profile-merged" aria-labelledby="profile-who">
            <h2 id="profile-who" tabindex="-1">{words::profile_identity_title(lang)}</h2>
            {choose}
            {content}
            {picture_editor}
        </section>
    }
}

fn person_name_field(
    id: &'static str,
    label: &'static str,
    auto: &'static str,
    value: RwSignal<String>,
    saved: RwSignal<bool>,
) -> impl IntoView {
    view! {
        <div class="field">
            <label for=id>{label}</label>
            <div class="inset">
                {super::field::mark(glyph::USER)}
                <input
                    id=id
                    type="text"
                    maxlength="80"
                    autocomplete=auto
                    prop:value=move || value.get()
                    on:input=move |ev| {
                        saved.set(false);
                        value.set(event_target_value(&ev));
                    }
                />
            </div>
        </div>
    }
}

fn profile_personal_card(
    lang: Lang,
    worker: &str,
    username: &str,
    signed_again: Callback<()>,
    reload: Callback<()>,
    handed: StoredValue<Option<MailCode>>,
    state: ProfileDetailsState,
) -> impl IntoView {
    let personal_busy = RwSignal::new(false);
    let personal_error = RwSignal::new(None::<&'static str>);
    let personal_saved = RwSignal::new(false);
    let age_error = Signal::derive(move || {
        let val = state.birth_date.get()?;
        let birth = parse_iso_date(&val).ok()?;
        let today = utc_date_from_unix_secs(super::ui::datepicker::unix_today_secs());
        (!is_at_least_age(birth, today, MIN_BIRTH_AGE_YEARS))
            .then(|| words::date_picker_age_error(lang))
    });
    let save_personal = move |event: leptos::ev::SubmitEvent| {
        event.prevent_default();
        if personal_busy.get_untracked() || state.details.with_untracked(Option::is_none) {
            return;
        }
        personal_saved.set(false);
        personal_error.set(None);
        let raw_first = state.first_name.get_untracked();
        let raw_last = state.last_name.get_untracked();
        let raw_birth = state.birth_date.get_untracked();
        let Ok(valid_first) = validate_person_name(Some(&raw_first)) else {
            personal_error.set(Some(problem_text(lang, Problem::Failed)));
            return;
        };
        let Ok(valid_last) = validate_person_name(Some(&raw_last)) else {
            personal_error.set(Some(problem_text(lang, Problem::Failed)));
            return;
        };
        let today = utc_date_from_unix_secs(super::ui::datepicker::unix_today_secs());
        let Ok(valid_birth) = validate_birth_date(raw_birth.as_deref(), today) else {
            personal_error.set(Some(words::date_picker_age_error(lang)));
            return;
        };
        let payload = ProfileDetails {
            first_name: valid_first,
            last_name: valid_last,
            birth_date: valid_birth,
            public_profile: state.public_profile.get_untracked(),
        };
        personal_busy.set(true);
        leptos::task::spawn_local(async move {
            match super::central::set_profile_details(payload.clone()).await {
                Ok(()) => {
                    state
                        .first_name
                        .set(payload.first_name.clone().unwrap_or_default());
                    state
                        .last_name
                        .set(payload.last_name.clone().unwrap_or_default());
                    state.birth_date.set(payload.birth_date.clone());
                    state.details.set(Some(payload));
                    personal_saved.set(true);
                }
                Err(failure) => personal_error.set(Some(problem_text(lang, failure.into()))),
            }
            personal_busy.set(false);
        });
    };
    view! {
        <section class="card" aria-labelledby="profile-personal-title">
            <h2 id="profile-personal-title">{words::profile_personal_title(lang)}</h2>
            <form class="form profile-personal-form" method="post" novalidate=true on:submit=save_personal>
                {person_name_field("profile-first-name", words::profile_first_name(lang), "given-name", state.first_name, personal_saved)}
                {person_name_field("profile-last-name", words::profile_last_name(lang), "family-name", state.last_name, personal_saved)}
                {super::ui::datepicker::date_picker(
                    lang,
                    "profile-birth-date",
                    words::profile_birth_date(lang),
                    state.birth_date,
                    age_error,
                )}
                <p class="error" role="alert">{move || personal_error.get()}</p>
                <p class="meta" role="status">{move || personal_saved.get().then(|| words::profile_personal_saved(lang))}</p>
                <p class="actions">
                    <button type="submit" class="button primary" aria-disabled=move || (state.details.with(Option::is_none) || personal_busy.get()).then_some("true")>
                        <span>{words::profile_personal_save(lang)}</span>
                    </button>
                </p>
            </form>
            {super::email::section(lang, worker.to_owned(), username.to_owned(), signed_again, reload, handed)}
        </section>
    }
}

fn profile(
    lang: Lang,
    worker: &str,
    summary: &Summary,
    reload: Callback<()>,
    focus_next: StoredValue<Option<&'static str>>,
    handed: StoredValue<Option<MailCode>>,
    prefetched_badges: Option<PublicBadges>,
) -> AnyView {
    let needs_badge_fetch = prefetched_badges.is_none();
    let initial_rank = prefetched_badges
        .as_ref()
        .and_then(|b| b.best_rank)
        .or_else(remembered_rank);
    let badges_data = RwSignal::new(prefetched_badges);
    let state = ProfileDetailsState {
        details: RwSignal::new(None),
        first_name: RwSignal::new(String::new()),
        last_name: RwSignal::new(String::new()),
        birth_date: RwSignal::new(None),
        public_profile: RwSignal::new(false),
        rank: RwSignal::new(initial_rank),
    };
    Effect::new(move |_| {
        leptos::task::spawn_local(async move {
            if let Ok(loaded) = super::central::profile_details().await {
                state
                    .first_name
                    .set(loaded.first_name.clone().unwrap_or_default());
                state
                    .last_name
                    .set(loaded.last_name.clone().unwrap_or_default());
                state.birth_date.set(loaded.birth_date.clone());
                state.public_profile.set(loaded.public_profile);
                state.details.set(Some(loaded));
            }
        });
        if needs_badge_fetch {
            leptos::task::spawn_local(async move {
                if let Ok(found) = super::central::own_badges().await {
                    session::note_rank(found.best_rank.as_ref());
                    state.rank.set(found.best_rank);
                    badges_data.set(Some(found));
                }
            });
        }
    });
    let noop = Callback::new(|()| {});
    view! {
        {profile_identity_card(lang, worker, summary, reload, focus_next, state)}
        {move || badges_data.with(|b| b.as_ref().map(|b| view! {
            <section class="card profile-badges" aria-labelledby="profile-badges-title">
                <h2 id="profile-badges-title">{words::badges_title(lang)}</h2>
                {super::ui::badges::badge_wall(lang, b)}
            </section>
        }))}
        {profile_personal_card(lang, worker, &summary.username, noop, reload, handed, state)}
        {super::password::section(lang, worker.to_owned(), summary.username.clone(), noop, reload)}
    }
    .into_any()
}

fn session_row(lang: Lang, info: &SessionInfo) -> impl IntoView + use<> {
    let kind = match info.kind {
        SessionKind::Browser => words::security_kind_browser(lang),
        SessionKind::Native => words::security_kind_client(lang),
    };
    let current = info
        .current
        .then(|| view! { <span class="pill accent">{words::security_this(lang)}</span> });
    let gateways = (!info.gateways.is_empty()).then(|| {
        let list = info
            .gateways
            .iter()
            .map(|origin| view! { <li><code translate="no">{origin.clone()}</code></li> })
            .collect_view();
        view! { <dt>{words::security_gateways(lang)}</dt><dd><ul>{list}</ul></dd> }
    });
    view! {
        <li class="card">
            <h3>{kind} {current}</h3>
            <dl class="facts">
                <dt>{words::security_signed_in(lang)}</dt>
                <dd>{super::time::format_datetime_local(lang, info.signed_in_at)}</dd>
                <dt>{words::security_ends(lang)}</dt>
                <dd>{super::time::format_datetime_local(lang, info.expires_at)}</dd>
                {gateways}
            </dl>
        </li>
    }
}

/// Active browser and client sessions for this account.
fn security(lang: Lang, worker: String) -> AnyView {
    let sessions = RwSignal::new(None::<Result<Vec<SessionInfo>, Problem>>);
    let notice = RwSignal::new(None::<&'static str>);
    let load = move || {
        leptos::task::spawn_local(async move {
            let found = match call(&Request::Owner {
                request: OwnerRequest::Sessions,
            })
            .await
            {
                Ok(Response::Owner {
                    answer: OwnerAnswer::Sessions { sessions },
                }) => Ok(sessions),
                Ok(_) => Err(Problem::Failed),
                Err(failure) => Err(failure.into()),
            };
            sessions.set(Some(found));
        });
    };
    Effect::new(move |_| load());
    let end_client = move |_| {
        leptos::task::spawn_local(async move {
            match call(&Request::Owner {
                request: OwnerRequest::EndClientSession,
            })
            .await
            {
                Ok(_) => {
                    notice.set(Some(words::security_client_ended(lang)));
                    load();
                }
                Err(failure) => notice.set(Some(problem_text(lang, failure.into()))),
            }
        });
    };
    let leave = move |_| {
        let worker = worker.clone();
        leptos::task::spawn_local(async move { sign_out(&worker, lang).await });
    };
    view! {
        <section class="card" aria-labelledby="sessions-title">
            <h2 id="sessions-title">{words::security_sessions(lang)}</h2>
            <p>{words::security_lead(lang)}</p>
            {move || sessions.with(|found| match found {
                None => view! { <p role="status" class="meta">{words::account_loading(lang)}</p> }.into_any(),
                Some(Err(problem)) => view! { <p class="error" role="alert">{problem_text(lang, *problem)}</p> }.into_any(),
                Some(Ok(list)) => {
                    let rows = list.iter().map(|info| session_row(lang, info)).collect_view();
                    view! { <ul class="sessions">{rows}</ul> }.into_any()
                }
            })}
            <p class="meta" role="status">{move || notice.get()}</p>
            <p class="actions">
                <button type="button" class="button danger" on:click=end_client>
                    {glyph::glyph(glyph::LOG_OUT)}
                    <span>{words::security_end_client(lang)}</span>
                </button>
                <button type="button" class="button button-ghost" on:click=leave>
                    {glyph::glyph(glyph::LOG_OUT)}
                    <span>{words::security_sign_out(lang)}</span>
                </button>
            </p>
        </section>
    }
    .into_any()
}

/// The export as a JSON file the browser saves.
fn download_link(lang: Lang, json: &str) -> impl IntoView + use<> {
    let mut href = String::from("data:application/json;charset=utf-8,");
    for byte in json.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.~:,{}[]\"".contains(&byte) {
            href.push(char::from(byte));
        } else {
            let _ = write!(href, "%{byte:02X}");
        }
    }
    view! {
        <a class="button" id="export-save" href=href download="baylee-account.json">
            {glyph::glyph(glyph::DOWNLOAD)}
            <span>{words::privacy_export_save(lang)}</span>
        </a>
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Deletion {
    Idle,
    Asking,
    Proven { gateways: Vec<String> },
    Deleted,
}

fn export_result_view(
    lang: Lang,
    result: Option<Result<String, Problem>>,
    without_email: RwSignal<bool>,
    picture: RwSignal<Option<PictureKey>>,
    busy: RwSignal<bool>,
    run_export: impl Fn() + Clone + 'static,
) -> AnyView {
    match result {
        Some(Ok(json)) => {
            super::focus::when_shown("#export-save");
            let lacks = without_email.get().then(|| {
                view! { <p class="meta" id="export-no-email">{words::privacy_export_no_email(lang)}</p> }
            });
            let png = picture.get().map(|key| {
                view! {
                    <a class="button button-ghost" id="export-picture" href=super::picture::url(&key, false)
                        download="baylee-profile-picture.png">
                        {glyph::glyph(glyph::DOWNLOAD)}
                        <span>{words::privacy_export_picture(lang)}</span>
                    </a>
                }
            });
            view! { <div class="actions">{download_link(lang, &json)}{png}</div>{lacks} }.into_any()
        }
        Some(Err(problem)) => {
            super::focus::when_shown("#export-error");
            view! {
                <p class="error" id="export-error" role="alert" tabindex="-1">{problem_text(lang, problem)}</p>
                <button type="button" class="button" on:click=move |_| run_export()>
                    {glyph::glyph(glyph::DOWNLOAD)}
                    <span>{words::privacy_export_make(lang)}</span>
                </button>
            }
            .into_any()
        }
        None => view! {
            <button type="button" class="button" disabled=move || busy.get() on:click=move |_| run_export()>
                {glyph::glyph(glyph::DOWNLOAD)}
                <span>{words::privacy_export_make(lang)}</span>
            </button>
        }
        .into_any(),
    }
}

/// The export, and beside it in the same file what Central holds of the
/// account's e-mail address (`email::export_file`): the export's own
/// answer does not carry it. Where the address cannot be read the file is
/// the export alone, and the page says that the address is not in it.
fn export_view(lang: Lang, worker: String) -> impl IntoView {
    let export = RwSignal::new(None::<Result<String, Problem>>);
    let busy = RwSignal::new(false);
    let generation = RwSignal::new(0_u64);
    let without_email = RwSignal::new(false);
    let picture = RwSignal::new(None::<PictureKey>);
    let needs_reauth = RwSignal::new(false);
    session::on_change(move || {
        generation.update(|value| *value = value.wrapping_add(1));
        export.set(None);
        picture.set(None);
        without_email.set(false);
        needs_reauth.set(false);
        busy.set(false);
    });
    let run_export = move || {
        if busy.get_untracked() {
            return;
        }
        let Some(stamp) = session::login_stamp() else {
            needs_reauth.set(true);
            return;
        };
        let started = generation.get_untracked();
        export.set(None);
        busy.set(true);
        leptos::task::spawn_local(async move {
            let result = call(&Request::AccountExport).await;
            if generation.get_untracked() != started {
                return;
            }
            if session::login_stamp().as_ref() != Some(&stamp) {
                busy.set(false);
                return;
            }
            match result {
                Ok(Response::AccountExport { export: data }) => {
                    // asked once the export answered: a refusal here is not the export's
                    let email = super::email::for_export().await;
                    let (held, filed) = super::picture::for_export().await;
                    let file = super::email::export_file(&data, email.as_ref()).and_then(|file| {
                        super::picture::merged(&file, held.as_ref(), filed.as_deref())
                    });
                    let file = match file {
                        Some(file) => super::export::merged(&file).await.ok(),
                        None => None,
                    };
                    if generation.get_untracked() != started {
                        return;
                    }
                    if session::login_stamp().as_ref() != Some(&stamp) {
                        busy.set(false);
                        return;
                    }
                    without_email.set(email.is_none());
                    picture.set(held.as_ref().map(|p| p.key));
                    export.set(Some(file.ok_or(Problem::Failed)));
                }
                Ok(_) => export.set(Some(Err(Problem::Failed))),
                Err(Failure::Rejected) => needs_reauth.set(true),
                Err(failure) => export.set(Some(Err(failure.into()))),
            }
            busy.set(false);
        });
    };
    view! {
        <section class="card" aria-labelledby="export-title">
            <h2 id="export-title">{words::privacy_export_title(lang)}</h2>
            <p>{words::privacy_export_lead(lang)}</p>
            <span class="meta" role="status">{move || busy.get().then(||words::account_loading(lang))}</span>
            {move || export_result_view(lang, export.get(), without_email, picture, busy, run_export)}
            <Show when=move || needs_reauth.get()>
                {reauth(lang, worker.clone(), Callback::new(move |()| { needs_reauth.set(false); run_export(); }))}
            </Show>
        </section>
    }
}

fn deletion_proven_view(
    lang: Lang,
    gateways: Vec<String>,
    busy: RwSignal<bool>,
    deletion: RwSignal<Deletion>,
    delete: Callback<()>,
) -> AnyView {
    let owned = (!gateways.is_empty()).then(|| {
        let list = gateways
            .into_iter()
            .map(|origin| view! { <li><code translate="no">{origin}</code></li> })
            .collect_view();
        view! { <p>{words::privacy_delete_gateways(lang)}</p><ul>{list}</ul> }
    });
    // the last word before the end, not the button that ends it
    super::focus::when_shown("#delete-last");
    view! {
        <p class="notice" id="delete-last" role="alert" tabindex="-1">{words::privacy_delete_last(lang)}</p>
        {owned}
        <p class="actions">
            <button type="button" class="button danger" aria-disabled=move || busy.get().then_some("true") on:click=move |_| delete.run(())>
                {glyph::glyph(glyph::TRASH_2)}
                <span>{words::privacy_delete_action(lang)}</span>
            </button>
            <button type="button" class="button button-ghost" on:click=move |_| {
                deletion.set(Deletion::Idle);
                super::focus::after_render("#delete-start");
            }>{words::privacy_delete_cancel(lang)}</button>
        </p>
    }
    .into_any()
}

fn deletion_asking_view(
    lang: Lang,
    password: RwSignal<String>,
    busy: RwSignal<bool>,
    prove: impl Fn(leptos::ev::SubmitEvent) + 'static,
) -> AnyView {
    super::focus::when_shown("#delete-password");
    view! {
        <form class="form" method="post" novalidate=true on:submit=prove>
            {super::field::password_box(
                lang,
                "delete-password",
                words::privacy_delete_password(lang),
                "current-password",
                password,
                None,
                None,
                "1024",
            )}
            <button type="submit" class="button danger" aria-disabled=move || busy.get().then_some("true")>
                {glyph::glyph(glyph::TRASH_2)}
                <span>{words::privacy_delete_prove(lang)}</span>
            </button>
        </form>
    }
    .into_any()
}

fn deletion_view(lang: Lang, worker: String) -> impl IntoView {
    let deletion = RwSignal::new(Deletion::Idle);
    let password = RwSignal::new(String::new());
    let error = RwSignal::new(None::<&'static str>);
    let busy = RwSignal::new(false);
    let proof = StoredValue::new_local(None::<(central_api::Token, Vec<u8>)>);
    let worker_delete = worker.clone();
    let prove = move |event: leptos::ev::SubmitEvent| {
        event.prevent_default();
        if busy.get_untracked() {
            return;
        }
        let (secret, worker) = (password.get_untracked(), worker_delete.clone());
        password.set(String::new());
        busy.set(true);
        leptos::task::spawn_local(async move {
            match prove_deletion(&worker, secret).await {
                Ok((attempt, kept, gateways)) => {
                    proof.set_value(Some((attempt, kept)));
                    error.set(None);
                    deletion.set(Deletion::Proven { gateways });
                }
                Err(problem) => error.set(Some(problem_text(lang, problem))),
            }
            busy.set(false);
        });
    };
    let delete = Callback::new(move |()| {
        if busy.get_untracked() {
            return;
        }
        let Some((attempt, kept)) = proof.try_update_value(Option::take).flatten() else {
            return;
        };
        let worker = worker.clone();
        busy.set(true);
        leptos::task::spawn_local(async move {
            match call(&Request::DeleteAccountFinish {
                attempt,
                proof: kept,
            })
            .await
            {
                Ok(_) => {
                    session::forget();
                    if let Some(crypto) = Crypto::start(&worker) {
                        let _ = crypto.ask("forget", json!({})).await;
                    }
                    deletion.set(Deletion::Deleted);
                }
                Err(failure) => {
                    error.set(Some(problem_text(lang, failure.into())));
                    deletion.set(Deletion::Idle);
                    super::focus::after_render("#delete-start");
                }
            }
            busy.set(false);
        });
    });
    let code = lang.code();
    view! {
        <section class="card danger-zone" aria-labelledby="delete-title">
            <h2 id="delete-title">{words::privacy_delete_title(lang)}</h2>
            <p>{words::privacy_delete_lead(lang)}</p>
            {move || match deletion.get() {
                Deletion::Idle => view! {
                    <button type="button" class="button danger" id="delete-start" on:click=move |_| deletion.set(Deletion::Asking)>
                        {glyph::glyph(glyph::TRASH_2)}
                        <span>{words::privacy_delete_start(lang)}</span>
                    </button>
                }.into_any(),
                Deletion::Asking => deletion_asking_view(lang, password, busy, prove.clone()),
                Deletion::Proven { gateways } => deletion_proven_view(lang, gateways, busy, deletion, delete),
                Deletion::Deleted => {
                    super::focus::when_shown("#deleted");
                    view! {
                        <p role="status" id="deleted" tabindex="-1">{words::privacy_deleted(lang)}</p>
                        <p><a class="button" href=format!("/{code}/")>{words::error_home(lang)}</a></p>
                    }.into_any()
                }
            }}
            <p class="error" role="alert">{move || error.get()}</p>
        </section>
    }
}

fn privacy(lang: Lang, worker: String) -> AnyView {
    view! {
        {export_view(lang, worker.clone())}
        {deletion_view(lang, worker)}
    }
    .into_any()
}

/// Proves the password for a deletion: KE1 in a fresh Worker,
/// `DeleteAccountStart`, KE3 back. The attempt lives 60 seconds.
async fn prove_deletion(
    worker: &str,
    password: String,
) -> Result<(central_api::Token, Vec<u8>, Vec<String>), Problem> {
    let crypto = Crypto::start(worker).ok_or(Problem::Failed)?;
    let Response::AccountSummary { identity, .. } = call(&Request::AccountSummary).await? else {
        return Err(Problem::Failed);
    };
    let start = crypto
        .ask(
            "login_start",
            json!({ "password": password, "account": identity.account, "epoch": identity.epoch }),
        )
        .await?;
    drop(password);
    let message = field_bytes(&start, "message", Some(96))?;
    let Response::DeleteChallenge {
        attempt,
        login,
        gateways,
    } = call(&Request::DeleteAccountStart { login: message }).await?
    else {
        return Err(Problem::Failed);
    };
    let finish = crypto
        .ask("login_finish", json!({ "response": login }))
        .await?;
    let proof = field_bytes(&finish, "message", Some(64))?;
    Ok((
        attempt,
        proof,
        gateways.into_iter().map(|gateway| gateway.origin).collect(),
    ))
}

#[cfg_attr(not(feature = "admin"), allow(dead_code))]
pub(super) fn scope_text(lang: Lang, scopes: GatewayScopes) -> String {
    let mut parts = Vec::with_capacity(2);
    match (scopes.sso, scopes.listing) {
        (true, true) => parts.push(words::tokens_both(lang)),
        (true, false) => parts.push(words::tokens_sso(lang)),
        (false, true) => parts.push(words::tokens_listing(lang)),
        (false, false) => {}
    }
    if scopes.ranked {
        parts.push(words::tokens_ranked(lang));
    }
    parts.join(" · ")
}

/// The account's categories, each a switch.
fn push_switches(
    lang: Lang,
    admin: bool,
    now: RwSignal<Option<Result<super::push::Now, Problem>>>,
    busy: RwSignal<bool>,
    error: RwSignal<Option<&'static str>>,
) -> AnyView {
    use super::push;

    let switches = push::categories(admin)
        .into_iter()
        .map(|category| {
            let id = format!("push-{}", push::bit(category));
            let on = move || now.with(|n| matches!(n, Some(Ok(n)) if n.enabled & push::bit(category) != 0));
            let change = move |event: leptos::ev::Event| {
                let wanted = event_target_checked(&event);
                let Some(Ok(current)) = now.get_untracked() else {
                    return;
                };
                if busy.get_untracked() {
                    // one change at a time: this one is undone
                    event_target::<leptos::web_sys::HtmlInputElement>(&event).set_checked(!wanted);
                    return;
                }
                busy.set(true);
                error.set(None);
                leptos::task::spawn_local(async move {
                    match push::set(&current, category, wanted).await {
                        Ok(enabled) => now.update(|n| {
                            if let Some(Ok(n)) = n {
                                n.enabled = enabled;
                            }
                        }),
                        Err(_) => error.set(Some(words::push_failed(lang))),
                    }
                    busy.set(false);
                });
            };
            let name = match category {
                central_api::notifications::Category::Chat => words::push_chat(lang),
                central_api::notifications::Category::Contacts => words::push_contacts(lang),
                central_api::notifications::Category::Updates => words::push_updates(lang),
                central_api::notifications::Category::Feedback => words::push_feedback(lang),
                central_api::notifications::Category::Gateways => words::push_gateways(lang),
            };
            view! {
                <div class="check">
                    <input id=id.clone() type="checkbox" role="switch" prop:checked=on on:change=change
                        aria-disabled=move || busy.get().then_some("true")
                        disabled=move || now.with(|n| !matches!(n, Some(Ok(_)))) />
                    <label for=id>{name}</label>
                </div>
            }
        })
        .collect_view();
    switches.into_any()
}

fn push_install_box(
    lang: Lang,
    standalone: RwSignal<bool>,
    needs_install: RwSignal<bool>,
) -> impl IntoView {
    view! {
        <div class="push-install-box" hidden=move || standalone.get()>
            <p class="meta">
                {move || if needs_install.get() {
                    words::notify_install_needed(lang)
                } else {
                    words::pwa_install_tooltip(lang)
                }}
            </p>
            <button
                type="button"
                class="button"
                data-pwa-install=""
                title=words::pwa_install_tooltip(lang)
                hidden=true
            >
                {glyph::glyph(glyph::DOWNLOAD)}
                <span>{words::install_app(lang)}</span>
            </button>
        </div>
    }
}

fn push_permission_row(
    lang: Lang,
    perm: RwSignal<String>,
    needs_install: RwSignal<bool>,
    request_permission: impl Fn(leptos::ev::MouseEvent) + Copy + 'static,
) -> AnyView {
    if needs_install.get() || !super::push::browser::has_notification_api() {
        return ().into_any();
    }
    match perm.get().as_str() {
        "default" => view! {
            <div class="push-permission-row">
                <button type="button" class="button" on:click=request_permission>
                    {glyph::glyph(glyph::BELL)}
                    <span>{words::notify_request_permission(lang)}</span>
                </button>
            </div>
        }
        .into_any(),
        "granted" => view! {
            <div class="push-permission-row">
                <p class="meta" role="status">
                    {glyph::glyph(glyph::CIRCLE_CHECK)}
                    <span>{words::notify_permission_granted(lang)}</span>
                </p>
            </div>
        }
        .into_any(),
        "denied" => view! {
            <div class="push-permission-row">
                <p class="notice" role="status">
                    {glyph::glyph(glyph::SHIELD_ALERT)}
                    <span>{words::notify_permission_denied(lang)}</span>
                </p>
            </div>
        }
        .into_any(),
        _ => ().into_any(),
    }
}

fn toggle_push_device(
    lang: Lang,
    now: RwSignal<Option<Result<super::push::Now, Problem>>>,
    busy: RwSignal<bool>,
    error: RwSignal<Option<&'static str>>,
    perm: RwSignal<String>,
    active: bool,
    public_key: Option<String>,
) {
    use super::push::{self, Refusal};

    error.set(None);
    if active {
        busy.set(true);
        leptos::task::spawn_local(async move {
            if push::disable().await.is_err() {
                error.set(Some(words::push_failed(lang)));
            }
            now.update(|n| {
                if let Some(Ok(n)) = n {
                    n.active = false;
                }
            });
            busy.set(false);
        });
        return;
    }
    let Some(public_key) = public_key else {
        return;
    };
    // the browser asks inside the click, before anything is awaited
    let asked = push::browser::ask();
    busy.set(true);
    leptos::task::spawn_local(async move {
        match push::enable(asked, lang, &public_key).await {
            Ok(enabled) => now.update(|n| {
                if let Some(Ok(n)) = n {
                    n.enabled = enabled;
                    n.active = true;
                }
                perm.set(push::browser::permission());
            }),
            Err(Refusal::Denied) => {
                perm.set(push::browser::permission());
                error.set(Some(words::push_denied(lang)));
            }
            Err(Refusal::Failed) => error.set(Some(words::push_failed(lang))),
        }
        busy.set(false);
    });
}

/// Web Push on this device, PWA install guidance, and notification categories.
fn notifications(lang: Lang, admin: bool) -> AnyView {
    use super::push;

    let now = RwSignal::new(None::<Result<push::Now, Problem>>);
    let busy = RwSignal::new(false);
    let error = RwSignal::new(None::<&'static str>);
    let perm = RwSignal::new(push::browser::permission());
    let standalone = RwSignal::new(push::browser::is_standalone());
    let needs_install = RwSignal::new(push::browser::requires_install());
    push::browser::on_pwa_change(move |_can, inst| {
        standalone.set(inst);
        needs_install.set(push::browser::requires_install());
        perm.set(push::browser::permission());
    });
    Effect::new(move |_| {
        standalone.set(push::browser::is_standalone());
        needs_install.set(push::browser::requires_install());
        perm.set(push::browser::permission());
        leptos::task::spawn_local(async move {
            now.set(Some(push::load().await.map_err(Problem::from)));
        });
    });
    let supported = push::browser::supported();
    let active = move || now.with(|n| matches!(n, Some(Ok(n)) if n.active));
    let key = move || {
        now.with(|n| {
            n.as_ref()
                .and_then(|n| n.as_ref().ok())
                .and_then(|n| n.public_key.clone())
        })
    };
    let request_permission = move |_| {
        if push::browser::requires_install() || !push::browser::has_notification_api() {
            return;
        }
        let asked = push::browser::ask();
        leptos::task::spawn_local(async move {
            let answer = push::browser::answer(asked).await;
            perm.set(if answer.is_empty() {
                push::browser::permission()
            } else {
                answer
            });
            needs_install.set(push::browser::requires_install());
            standalone.set(push::browser::is_standalone());
        });
    };
    let device = move |_| {
        if !busy.get_untracked() && supported {
            toggle_push_device(lang, now, busy, error, perm, active(), key());
        }
    };
    let switches = push_switches(lang, admin, now, busy, error);
    view! {
        <section class="card" aria-labelledby="push-title">
            <h2 id="push-title">{words::push_title(lang)}</h2>
            <p>{words::push_lead(lang)}</p>
            {push_install_box(lang, standalone, needs_install)}
            {move || (!supported && !needs_install.get()).then(|| view! {
                <p class="meta">{words::push_unsupported(lang)}</p>
            })}
            {move || push_permission_row(lang, perm, needs_install, request_permission)}
            {move || matches!(now.get(), Some(Ok(ref n)) if n.public_key.is_none())
                .then(|| view! { <p class="meta">{words::push_unconfigured(lang)}</p> })}
            {move || matches!(now.get(), Some(Err(_))).then(|| view! { <p class="error" role="alert">{words::push_failed(lang)}</p> })}
            <p class="error" role="alert">{move || error.get()}</p>
            <div class="push-device">
                <button type="button" class="button button-ghost" role="switch" aria-checked=move || active().to_string()
                    aria-disabled=move || (busy.get() || !supported || key().is_none()).then_some("true")
                    on:click=device>{words::push_device(lang)}</button>
                <p class="meta">{move || if active() { words::push_on(lang) } else { words::push_off(lang) }}</p>
            </div>
            <fieldset class="push-categories">
                <legend>{words::push_categories(lang)}</legend>
                {switches}
            </fieldset>
            <p class="meta">{words::push_scope(lang)}</p>
        </section>
    }
    .into_any()
}

/// What a page of the area is drawn with once the account is read.
#[derive(Clone, Copy)]
pub struct Area {
    /// Reads the account again and draws the page anew; also what a page
    /// runs when this browser turns out not to be signed in.
    pub reload: Callback<()>,
    /// Where the focus goes once a reload an action asked for is drawn.
    focus_next: StoredValue<Option<&'static str>>,
    /// The confirmation code a mail's link handed the security page.
    handed: StoredValue<Option<MailCode>>,
    /// Prefetched badges for the profile page so the showcase renders with
    /// `Load::Ready` at zero layout shift.
    badges: StoredValue<Option<PublicBadges>>,
}

async fn prefetch_profile_badges(section: Section, load: &Load) -> Option<PublicBadges> {
    if section != Section::Profile || !matches!(load, Load::Ready(_)) {
        return None;
    }
    let loaded = super::central::own_badges().await.ok()?;
    session::note_rank(loaded.best_rank.as_ref());
    Some(loaded)
}

/// The frame of every page of the area but the messenger's: the account
/// read from Central in the browser (on the server, "loading"), a way to
/// sign in where there is none, a failure with "again", and, once the
/// account is read, the page `draw` makes of it. Both islands drawn in it
/// call it: [`Account`] and the administration's (`admin::Administration`),
/// which only the administration's module has.
pub fn frame<F>(lang: Lang, section: Section, draw: F) -> impl IntoView
where
    F: Fn(Summary, Area) -> AnyView + Send + Sync + 'static,
{
    let state = RwSignal::new(Load::Loading);
    let generation = RwSignal::new(0_u64);
    // where the focus goes once a reload an action asked for is drawn
    let focus_next = StoredValue::new(None::<&'static str>);
    // A confirmation code the mail's link handed in the address's
    // fragment (`fragment`): gone from the address bar at once, on every
    // page of the area, and kept on the profile page (redirecting legacy
    // `/account/security#code=...` links automatically) until the e-mail
    // section consumes it. One that arrives while the page is open is
    // removed and fills nothing (`fragment::watch`).
    fragment::watch();
    let mail_code = fragment::code(&fragment::take(), MailCode::parse);
    if section == Section::Security
        && let Some(ref mail_code) = mail_code
    {
        session::go(&format!(
            "/{}/account/profile#code={}",
            lang.code(),
            mail_code.display().as_str()
        ));
    }
    let handed = StoredValue::new(mail_code.filter(|_| section == Section::Profile));
    let badges = StoredValue::new(None::<PublicBadges>);
    let reload = Callback::new(move |()| {
        generation.update(|value| *value = value.wrapping_add(1));
        let started = generation.get_untracked();
        leptos::task::spawn_local(async move {
            let load = summary().await;
            if generation.get_untracked() != started {
                return;
            }
            let loaded_badges = prefetch_profile_badges(section, &load).await;
            if generation.get_untracked() != started {
                return;
            }
            badges.set_value(loaded_badges);
            if !matches!(load, Load::Ready(_)) {
                handed.set_value(None);
            }
            state.set(load);
        });
    });
    // Effects run in the browser only: the server renders "loading".
    Effect::new(move |_| {
        reload.run(());
        #[cfg(target_arch = "wasm32")]
        session::on_change(move || reload.run(()));
    });
    let area = Area {
        reload,
        focus_next,
        handed,
        badges,
    };
    let code = lang.code();
    move || {
        match state.get() {
        Load::Loading => view! {
            <section class="card account-loading-shell" role="status" aria-label=words::account_loading(lang)>
                <p class="meta">{words::account_loading(lang)}</p>
            </section>
        }
        .into_any(),
        Load::SignedOut => view! {
            <section class="card app-empty">
                <p>{words::account_signed_out(lang)}</p>
                <p><a class="button primary" href=format!("/{code}/signin")>{words::nav_sign_in(lang)}</a></p>
            </section>
        }
        .into_any(),
        Load::Failed(problem) => view! {
            <p class="error" role="alert">{problem_text(lang, problem)}</p>
            <p><button type="button" class="button" on:click=move |_| reload.run(())>{words::error_again(lang)}</button></p>
        }
        .into_any(),
        Load::Ready(summary) => {
            if let Some(target) = focus_next.try_update_value(Option::take).flatten() {
                super::focus::after_render(target);
            }
            draw(summary, area)
        }
    }
    }
}

/// The signed-in account island driving overview, profile, security, privacy,
/// gateways, and notifications views.
#[island]
pub fn Account(lang: Lang, section: Section, worker: String) -> impl IntoView {
    super::central::start_refresh_keepalive();
    frame(lang, section, move |summary, area| match section {
        Section::Home => home(lang, &summary),
        Section::Profile => profile(
            lang,
            &worker,
            &summary,
            area.reload,
            area.focus_next,
            area.handed,
            area.badges.get_value(),
        ),
        Section::Security => security(lang, worker.clone()),
        Section::Privacy => privacy(lang, worker.clone()),
        Section::Gateways => super::gateways::view(lang, worker.clone()),
        Section::Notifications => notifications(lang, summary.admin),
        // The messenger and the administration are islands of their own
        // (`chat::Messenger`, `admin::Administration`, the latter in the
        // administration's module only): the server never draws their
        // pages with this one, and this one never reaches into theirs.
        Section::Chat
        | Section::Contacts
        | Section::Admin
        | Section::AdminOverview
        | Section::AdminUsers
        | Section::AdminUser(_)
        | Section::AdminGateways
        | Section::AdminAudit
        | Section::AdminLegacy
        | Section::AdminPictures
        | Section::Feedback
        | Section::Report => ().into_any(),
    })
}
