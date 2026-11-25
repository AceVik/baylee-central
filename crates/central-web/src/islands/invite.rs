//! Public friend invitation landing card (`/{lang}/invite/{code}`): displays
//! inviter preview, signed-out signup/signin CTAs, and signed-in
//! accept/decline with optional email opt-in.

use central_api::email::EmailRequest;
use central_api::profile::tag_text;
use central_api::social::{
    FriendInvitePreview, FriendInviteStatus, Person, SocialRequest, SocialResponse,
    valid_friend_invite_code,
};
use central_api::wire::{Request, Response};
use leptos::prelude::*;

use crate::words::{self, Lang};

use super::auth::{Problem, problem_text};
use super::central::{Failure, call};
use super::chat::glyph::{self, glyph};
use super::session;
use super::time;
use super::ui::rank_avatar::{AvatarSize, rank_avatar};

#[derive(Clone, Copy, PartialEq, Eq)]
enum ViewerState {
    SignedOut,
    SignedIn { has_confirmed_email: bool },
}

#[derive(Clone)]
enum PageOutcome {
    Declined,
    Accepted {
        inviter: Person,
        email_pending: bool,
    },
}

#[derive(Clone)]
enum LoadedInvite {
    Invalid(InvalidKind),
    Failed(Problem),
    Active(Box<FriendInvitePreview>),
}

#[derive(Clone)]
enum InvalidKind {
    Unknown,
    Expired,
    Revoked,
    Redeemed(Option<Person>),
}

/// Percent-encode a query parameter value cleanly (RFC 3986 unreserved kept literal).
#[cfg_attr(all(not(target_arch = "wasm32"), not(test)), allow(dead_code))]
pub(super) fn percent_encode_component(raw: &str) -> String {
    use std::fmt::Write as _;

    let mut out = String::with_capacity(raw.len());
    for byte in raw.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

/// Percent-decode a single query parameter value without altering `+`.
#[cfg_attr(all(not(target_arch = "wasm32"), not(test)), allow(dead_code))]
pub(super) fn percent_decode(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(h), Some(l)) = (
                char::from(bytes[i + 1]).to_digit(16),
                char::from(bytes[i + 2]).to_digit(16),
            )
        {
            out.push(u8::try_from((h << 4) | l).unwrap_or(0));
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| raw.to_owned())
}

#[cfg_attr(all(not(target_arch = "wasm32"), not(test)), allow(dead_code))]
pub(super) fn query_param_from_search(search: &str, name: &str) -> Option<String> {
    for pair in search.trim_start_matches('?').split('&') {
        if let Some((k, v)) = pair.split_once('=')
            && k == name
            && !v.is_empty()
        {
            return Some(percent_decode(v));
        }
    }
    None
}

/// Read query parameter `name` from the current browser location.
pub(super) fn query_param(name: &str) -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        let search = web_sys::window()?.location().search().ok()?;
        query_param_from_search(&search, name)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = name;
        None
    }
}

/// Extract a friend invite code from either a raw code or a full `/invite/{code}` URL.
#[must_use]
pub(super) fn normalize_invite_code(raw: &str) -> String {
    let trimmed = raw.trim();
    let no_frag = trimmed.split('#').next().unwrap_or(trimmed);
    let no_query = no_frag.split('?').next().unwrap_or(no_frag);
    let no_slash = no_query.trim_end_matches('/');
    no_slash
        .rsplit_once("/invite/")
        .map_or(no_slash, |(_, code)| code)
        .to_owned()
}

/// Extract the invite code from a clean `/{lang}/invite/{code}` pathname.
#[cfg_attr(all(not(target_arch = "wasm32"), not(test)), allow(dead_code))]
#[must_use]
pub(super) fn invite_code_from_path(pathname: &str) -> Option<String> {
    let (_, rest) = pathname.rsplit_once("/invite/")?;
    let code = rest.trim_end_matches('/');
    if code.is_empty() || code.contains('/') {
        return None;
    }
    Some(code.to_owned())
}

pub(super) fn person_handle(person: &Person) -> String {
    format!("{}#{}", person.display_name, tag_text(person.tag))
}

fn preview_to_loaded(preview: FriendInvitePreview) -> LoadedInvite {
    match preview.status {
        FriendInviteStatus::Active => LoadedInvite::Active(Box::new(preview)),
        FriendInviteStatus::Expired => LoadedInvite::Invalid(InvalidKind::Expired),
        FriendInviteStatus::Revoked => LoadedInvite::Invalid(InvalidKind::Revoked),
        FriendInviteStatus::Redeemed => {
            LoadedInvite::Invalid(InvalidKind::Redeemed(preview.redeemed_by))
        }
    }
}

async fn fetch_invite_info(code: &str) -> LoadedInvite {
    if !valid_friend_invite_code(code) {
        return LoadedInvite::Invalid(InvalidKind::Unknown);
    }
    match call(&Request::FriendInviteInfo {
        code: code.to_owned(),
    })
    .await
    {
        Ok(Response::FriendInviteInfo { invite }) => preview_to_loaded(invite),
        Ok(_) => LoadedInvite::Failed(Problem::Failed),
        Err(Failure::Rejected) => LoadedInvite::Invalid(InvalidKind::Unknown),
        Err(failure) => LoadedInvite::Failed(Problem::outside_sign_in(failure)),
    }
}

async fn check_viewer_state() -> ViewerState {
    if session::remembered().is_none() {
        return ViewerState::SignedOut;
    }
    match call(&Request::Email {
        request: EmailRequest::Status,
    })
    .await
    {
        Ok(Response::Email { state }) => ViewerState::SignedIn {
            has_confirmed_email: state.address.is_some(),
        },
        Err(Failure::Rejected) => match call(&Request::AccountSummary).await {
            Err(Failure::Rejected) => {
                session::forget();
                ViewerState::SignedOut
            }
            _ => ViewerState::SignedIn {
                has_confirmed_email: false,
            },
        },
        _ => ViewerState::SignedIn {
            has_confirmed_email: false,
        },
    }
}

fn invalid_card(lang: Lang, reason: InvalidKind) -> AnyView {
    let (body, extra) = match reason {
        InvalidKind::Unknown => (words::invite_page_invalid_body(lang), None),
        InvalidKind::Expired => (words::invite_page_expired_body(lang), None),
        InvalidKind::Revoked => (words::invite_page_revoked_body(lang), None),
        InvalidKind::Redeemed(by) => (
            words::invite_page_used_body(lang),
            by.map(|p| words::invite_redeemed_by(lang, &person_handle(&p))),
        ),
    };
    let home = format!("/{}/", lang.code());
    view! {
        <section class="card invite-card invite-card-status" aria-labelledby="invite-invalid-title">
            <div class="invite-status-head">
                <span class="invite-status-icon warn" aria-hidden="true">{glyph(glyph::SHIELD_ALERT)}</span>
                <h2 id="invite-invalid-title">{words::invite_page_invalid_title(lang)}</h2>
            </div>
            <p>{body}</p>
            {extra.map(|who| view! { <p class="meta">{who}</p> })}
            <p class="invite-actions">
                <a class="button" href=home>{words::error_home(lang)}</a>
            </p>
        </section>
    }
    .into_any()
}

fn accepted_card(lang: Lang, inviter: &Person, email_pending: bool) -> AnyView {
    let handle = person_handle(inviter);
    let chat_href = format!(
        "/{}/account/chat?with={}",
        lang.code(),
        tag_text(inviter.tag)
    );
    view! {
        <section class="card invite-card invite-success" aria-labelledby="invite-accepted-title">
            <div class="invite-hero">
                {rank_avatar(inviter.display_name.clone(), &handle, None, None, AvatarSize::Medium)}
                <div class="invite-hero-text">
                    <h2 id="invite-accepted-title">{words::invite_page_accepted(lang, &handle)}</h2>
                    <p class="meta" translate="no">{handle.clone()}</p>
                </div>
            </div>
            {email_pending.then(|| view! {
                <p class="notice" role="status">{words::invite_page_email_pending_ok(lang)}</p>
            })}
            <p class="invite-actions">
                <a class="button primary" href=chat_href>
                    {glyph(glyph::MESSAGE)}
                    <span>{words::invite_page_open_chat(lang)}</span>
                </a>
            </p>
        </section>
    }
    .into_any()
}

fn declined_card(lang: Lang, signed_in: bool) -> AnyView {
    let dest = if signed_in {
        format!("/{}/account", lang.code())
    } else {
        format!("/{}/", lang.code())
    };
    let dest_label = if signed_in {
        words::nav_account(lang)
    } else {
        words::error_home(lang)
    };
    view! {
        <section class="card invite-card invite-card-status" aria-labelledby="invite-declined-title">
            <h2 id="invite-declined-title">{words::invite_page_title(lang)}</h2>
            <p role="status">{words::invite_page_declined(lang)}</p>
            <p class="invite-actions">
                <a class="button" href=dest>{dest_label}</a>
            </p>
        </section>
    }
    .into_any()
}

#[derive(Clone, Copy)]
struct InviteCtx {
    lang: Lang,
    invite: RwSignal<Option<LoadedInvite>>,
    viewer: RwSignal<Option<ViewerState>>,
    outcome: RwSignal<Option<PageOutcome>>,
    link_email: RwSignal<bool>,
    busy: RwSignal<bool>,
    action_error: RwSignal<Option<&'static str>>,
}

fn redeem_invite(ctx: InviteCtx, code: String, link_email: bool) {
    if ctx.busy.get_untracked() {
        return;
    }
    ctx.action_error.set(None);
    ctx.busy.set(true);
    leptos::task::spawn_local(async move {
        let request = Request::Social {
            request: SocialRequest::RedeemFriendInvite {
                code: code.clone(),
                link_email,
                lang: Some(ctx.lang.code().to_owned()),
            },
        };
        match call(&request).await {
            Ok(Response::Social {
                response:
                    SocialResponse::FriendInviteRedeemed {
                        inviter,
                        email_pending,
                    },
            }) => {
                ctx.outcome.set(Some(PageOutcome::Accepted {
                    inviter,
                    email_pending,
                }));
            }
            Ok(_) => ctx.action_error.set(Some(words::error_failed(ctx.lang))),
            Err(Failure::Rejected) => {
                let latest_viewer = check_viewer_state().await;
                ctx.viewer.set(Some(latest_viewer));
                if matches!(latest_viewer, ViewerState::SignedOut) {
                    ctx.busy.set(false);
                    return;
                }
                let info = fetch_invite_info(&code).await;
                if matches!(info, LoadedInvite::Active(_)) {
                    ctx.action_error
                        .set(Some(words::invite_page_self_error(ctx.lang)));
                } else {
                    ctx.invite.set(Some(info));
                }
            }
            Err(failure) => {
                ctx.action_error
                    .set(Some(problem_text(ctx.lang, Problem::from(failure))));
            }
        }
        ctx.busy.set(false);
    });
}

fn invite_preview_head(lang: Lang, preview: &FriendInvitePreview) -> AnyView {
    let handle = person_handle(&preview.inviter);
    let expires = time::format_datetime_local(lang, preview.expires_at);
    let note = preview.note.clone().filter(|n| !n.trim().is_empty());
    view! {
        <div class="invite-hero">
            {rank_avatar(preview.inviter.display_name.clone(), &handle, None, None, AvatarSize::Medium)}
            <div class="invite-hero-text">
                <h2 id="invite-active-title">{words::invite_page_heading(lang, &handle)}</h2>
                <p class="meta" translate="no">{handle}</p>
            </div>
        </div>
        <p>{words::invite_page_sub(lang)}</p>
        {note.map(|text| view! {
            <div class="invite-note-box">
                <p class="eyebrow">{words::invite_page_note_label(lang)}</p>
                <blockquote class="invite-note"><bdi>{text}</bdi></blockquote>
            </div>
        })}
        <p class="meta invite-expiry">
            {glyph(glyph::CALENDAR)}
            <span>{words::invite_expires_at(lang, &expires)}</span>
        </p>
    }
    .into_any()
}

fn signed_out_actions(lang: Lang, code: &str, invite_only: bool) -> AnyView {
    let lcode = lang.code();
    let signup_href = format!(
        "/{lcode}/register?invite={}",
        percent_encode_component(code)
    );
    let next_target = format!("/{lcode}/invite/{code}");
    let signin_href = format!(
        "/{lcode}/signin?next={}",
        percent_encode_component(&next_target)
    );
    view! {
        {invite_only.then(|| view! {
            <p class="notice">{words::invite_page_invite_only_note(lang)}</p>
        })}
        <div class="invite-actions">
            <a class="button primary" href=signup_href>
                {glyph(glyph::USER_PLUS)}
                <span>{words::invite_page_signup(lang)}</span>
            </a>
            <a class="button" href=signin_href>
                {glyph(glyph::FORWARD)}
                <span>{words::invite_page_signin(lang)}</span>
            </a>
        </div>
    }
    .into_any()
}

fn signed_in_actions(
    ctx: InviteCtx,
    code: String,
    preview: &FriendInvitePreview,
    has_confirmed_email: bool,
) -> AnyView {
    let lang = ctx.lang;
    let can_opt_in = !has_confirmed_email;
    let opt_in = preview
        .invited_email
        .as_ref()
        .filter(|_| can_opt_in)
        .map(|addr| {
            view! {
                <div class="invite-email-optin">
                    <div class="check">
                        <input
                            id="invite-link-email"
                            type="checkbox"
                            prop:checked=move || ctx.link_email.get()
                            on:change=move |ev| ctx.link_email.set(event_target_checked(&ev))
                        />
                        <label for="invite-link-email">{words::invite_page_link_email_existing(lang, addr)}</label>
                    </div>
                    <p class="hint">{words::invite_page_email_verify_hint(lang)}</p>
                </div>
            }
        });
    let accept_code = code;
    view! {
        {opt_in}
        {move || ctx.action_error.get().map(|err| view! {
            <p class="error" role="alert">{err}</p>
        })}
        <div class="invite-actions">
            <button
                type="button"
                class="button primary"
                aria-disabled=move || ctx.busy.get().then_some("true")
                on:click=move |_| {
                    let link = can_opt_in && ctx.link_email.get_untracked();
                    redeem_invite(ctx, accept_code.clone(), link);
                }
            >
                {glyph(glyph::CHECK)}
                <span>{words::invite_page_accept(lang)}</span>
            </button>
            <button
                type="button"
                class="button quiet"
                aria-disabled=move || ctx.busy.get().then_some("true")
                on:click=move |_| {
                    if !ctx.busy.get_untracked() {
                        ctx.outcome.set(Some(PageOutcome::Declined));
                    }
                }
            >
                {words::invite_page_decline(lang)}
            </button>
        </div>
    }
    .into_any()
}

fn render_invite_body(ctx: InviteCtx, code_sig: RwSignal<String>, reload: Callback<()>) -> AnyView {
    let lang = ctx.lang;
    let Some(viewer) = ctx.viewer.get() else {
        return view! {
            <section class="card invite-card">
                <p class="meta" role="status">{words::account_loading(lang)}</p>
            </section>
        }
        .into_any();
    };
    if let Some(done) = ctx.outcome.get() {
        return match done {
            PageOutcome::Declined => {
                declined_card(lang, matches!(viewer, ViewerState::SignedIn { .. }))
            }
            PageOutcome::Accepted {
                inviter,
                email_pending,
            } => accepted_card(lang, &inviter, email_pending),
        };
    }
    match ctx.invite.get() {
        None => view! {
            <section class="card invite-card">
                <p class="meta" role="status">{words::account_loading(lang)}</p>
            </section>
        }
        .into_any(),
        Some(LoadedInvite::Invalid(reason)) => invalid_card(lang, reason),
        Some(LoadedInvite::Failed(prob)) => view! {
            <section class="card invite-card">
                <p class="error" role="alert">{problem_text(lang, prob)}</p>
                <p class="invite-actions">
                    <button type="button" class="button primary" on:click=move |_| {
                        ctx.invite.set(None);
                        reload.run(());
                    }>{words::error_again(lang)}</button>
                </p>
            </section>
        }
        .into_any(),
        Some(LoadedInvite::Active(preview)) => {
            let code = code_sig.get();
            view! {
                <section class="card invite-card" aria-labelledby="invite-active-title">
                    {invite_preview_head(lang, &preview)}
                    {match viewer {
                        ViewerState::SignedOut => signed_out_actions(lang, &code, preview.invite_only),
                        ViewerState::SignedIn { has_confirmed_email } => {
                            signed_in_actions(ctx, code, &preview, has_confirmed_email)
                        }
                    }}
                </section>
            }
            .into_any()
        }
    }
}

/// Interactive friend invite landing island mounted on `/{lang}/invite/{code}`.
#[island]
pub fn InviteCard(lang: Lang) -> impl IntoView {
    let code_sig = RwSignal::new(String::new());
    let ctx = InviteCtx {
        lang,
        invite: RwSignal::new(None),
        viewer: RwSignal::new(None),
        outcome: RwSignal::new(None),
        link_email: RwSignal::new(false),
        busy: RwSignal::new(false),
        action_error: RwSignal::new(None),
    };

    let reload = Callback::new(move |()| {
        #[cfg(target_arch = "wasm32")]
        {
            let path = web_sys::window()
                .and_then(|w| w.location().pathname().ok())
                .unwrap_or_default();
            let Some(code) = invite_code_from_path(&path) else {
                ctx.invite
                    .set(Some(LoadedInvite::Invalid(InvalidKind::Unknown)));
                ctx.viewer.set(Some(ViewerState::SignedOut));
                return;
            };
            code_sig.set(code.clone());
            leptos::task::spawn_local(async move {
                ctx.invite.set(Some(fetch_invite_info(&code).await));
            });
            leptos::task::spawn_local(async move {
                ctx.viewer.set(Some(check_viewer_state().await));
            });
        }
    });
    #[cfg(target_arch = "wasm32")]
    Effect::new(move |_| reload.run(()));

    view! { <div class="stack">{move || render_invite_body(ctx, code_sig, reload)}</div> }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_and_normalizes_invite_codes() {
        assert_eq!(
            invite_code_from_path("/de/invite/AAAAAAAAAAAAAAAAAAAAAA"),
            Some("AAAAAAAAAAAAAAAAAAAAAA".to_owned())
        );
        assert_eq!(
            invite_code_from_path("/en/invite/AAAAAAAAAAAAAAAAAAAAAA/"),
            Some("AAAAAAAAAAAAAAAAAAAAAA".to_owned())
        );
        assert_eq!(invite_code_from_path("/en/invite/"), None);
        assert_eq!(invite_code_from_path("/en/invite/a/b"), None);

        assert_eq!(
            normalize_invite_code(
                "  https://baylee.example/de/invite/AAAAAAAAAAAAAAAAAAAAAA?x=1#y "
            ),
            "AAAAAAAAAAAAAAAAAAAAAA"
        );
        assert_eq!(
            normalize_invite_code("AAAAAAAAAAAAAAAAAAAAAA"),
            "AAAAAAAAAAAAAAAAAAAAAA"
        );
    }

    #[test]
    fn parses_query_params_and_encodes_safely() {
        let encoded = percent_encode_component("/de/invite/AbC_12-x");
        let search = format!("?foo=1&next={encoded}");
        assert_eq!(
            query_param_from_search(&search, "next"),
            Some("/de/invite/AbC_12-x".to_owned())
        );
        assert_eq!(query_param_from_search(&search, "missing"), None);
    }
}
