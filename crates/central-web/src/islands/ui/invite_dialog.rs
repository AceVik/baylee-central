//! Modal friend-invite manager (`ui/invite_dialog.rs`) with slot counter,
//! filter pills, creation form, instant sharing (Copy Link/Code, Web Share,
//! `mailto:`, toggleable QR SVG), and live-synced invite list with revocation.

use std::time::Duration;

use central_api::social::{
    DEFAULT_FRIEND_INVITE_DAYS, FriendInviteStatus, FriendInviteSummary, MAX_FRIEND_INVITE_DAYS,
    MAX_INVITE_LABEL, MAX_INVITE_NOTE,
};
use leptos::prelude::*;

use super::super::chat::glyph::{self, glyph};
use super::super::chat::model::handle;
use super::super::chat::talk::open_modal_dialog;
use super::super::chat::wire::{
    self, CreatedFriendInvite, FriendInvitesList, encode_query_component,
};
use super::super::chat::{Ctl, time};
use super::super::{clip, focus};
use super::badges::recruiter_medal_banner;
use super::qr;
use crate::words::{self, Lang};

const DIALOG_ID: &str = "friend-invite-dialog";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InviteFilter {
    All,
    Active,
    Used,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CopiedTarget {
    Link,
    Code,
}

fn origin_prefix() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        leptos::web_sys::window()
            .and_then(|w| w.location().origin().ok())
            .unwrap_or_default()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        String::new()
    }
}

fn invite_share_url(lang: Lang, code: &str) -> String {
    let origin = origin_prefix();
    if origin.is_empty() {
        format!("/{}/invite/{code}", lang.code())
    } else {
        format!("{origin}/{}/invite/{code}", lang.code())
    }
}

fn format_unix_secs(lang: Lang, secs: i64) -> String {
    let Ok(secs_u64) = u64::try_from(secs.max(0)) else {
        return String::new();
    };
    let ms = secs_u64.saturating_mul(1_000);
    let iso = time::machine(ms);
    let date_iso = iso.split('T').next().unwrap_or(&iso);
    format!(
        "{}, {}",
        crate::format::date(lang, date_iso),
        time::clock(ms)
    )
}

fn mailto_href(lang: Lang, to_email: Option<&str>, url: &str) -> String {
    let to: String = to_email
        .unwrap_or_default()
        .trim()
        .chars()
        .filter(|&c| !matches!(c, '?' | '&' | '#' | '\r' | '\n' | ' '))
        .collect();
    let subject = encode_query_component(words::invite_mailto_subject(lang));
    let body = encode_query_component(&words::invite_mailto_body(lang, url));
    format!("mailto:{to}?subject={subject}&body={body}")
}

#[cfg(target_arch = "wasm32")]
fn web_share_supported() -> bool {
    use js_sys::{Function, Reflect};
    use wasm_bindgen::{JsCast, JsValue};
    let Some(win) = leptos::web_sys::window() else {
        return false;
    };
    let nav: JsValue = win.navigator().into();
    Reflect::get(&nav, &JsValue::from_str("share"))
        .ok()
        .and_then(|v| v.dyn_into::<Function>().ok())
        .is_some()
}

#[cfg(not(target_arch = "wasm32"))]
const fn web_share_supported() -> bool {
    false
}

#[cfg(target_arch = "wasm32")]
fn invoke_web_share(title: &str, text: &str, url: &str) {
    use js_sys::{Function, Object, Promise, Reflect};
    use wasm_bindgen::{JsCast, JsValue};
    let Some(win) = leptos::web_sys::window() else {
        return;
    };
    let nav: JsValue = win.navigator().into();
    let Ok(share_fn) = Reflect::get(&nav, &JsValue::from_str("share"))
        .and_then(wasm_bindgen::JsCast::dyn_into::<Function>)
    else {
        return;
    };
    let data = Object::new();
    let _ = Reflect::set(
        &data,
        &JsValue::from_str("title"),
        &JsValue::from_str(title),
    );
    let _ = Reflect::set(&data, &JsValue::from_str("text"), &JsValue::from_str(text));
    let _ = Reflect::set(&data, &JsValue::from_str("url"), &JsValue::from_str(url));
    if let Ok(ret) = share_fn.call1(&nav, &data)
        && let Ok(promise) = ret.dyn_into::<Promise>()
    {
        wasm_bindgen_futures::spawn_local(async move {
            let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
        });
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn invoke_web_share(title: &str, text: &str, url: &str) {
    let _ = (title, text, url);
}

#[derive(Clone, Copy)]
struct InviteDialogState {
    lang: Lang,
    ctl: Ctl,
    open: RwSignal<bool>,
    list: RwSignal<Option<Result<FriendInvitesList, ()>>>,
    loading: RwSignal<bool>,
    busy: RwSignal<bool>,
    error: RwSignal<Option<&'static str>>,
    filter: RwSignal<InviteFilter>,
    label: RwSignal<String>,
    note: RwSignal<String>,
    email: RwSignal<String>,
    days: RwSignal<u8>,
    created: RwSignal<Option<CreatedFriendInvite>>,
    show_qr: RwSignal<bool>,
    copied: RwSignal<Option<CopiedTarget>>,
    copy_timer: StoredValue<Option<TimeoutHandle>>,
}

impl InviteDialogState {
    fn new(lang: Lang, ctl: Ctl, open: RwSignal<bool>) -> Self {
        let copy_timer = StoredValue::new(None::<TimeoutHandle>);
        on_cleanup(move || {
            if let Some(Some(handle)) = copy_timer.try_get_value() {
                handle.clear();
            }
        });
        Self {
            lang,
            ctl,
            open,
            list: RwSignal::new(None),
            loading: RwSignal::new(false),
            busy: RwSignal::new(false),
            error: RwSignal::new(None),
            filter: RwSignal::new(InviteFilter::All),
            label: RwSignal::new(String::new()),
            note: RwSignal::new(String::new()),
            email: RwSignal::new(String::new()),
            days: RwSignal::new(DEFAULT_FRIEND_INVITE_DAYS),
            created: RwSignal::new(None),
            show_qr: RwSignal::new(false),
            copied: RwSignal::new(None),
            copy_timer,
        }
    }

    fn reload(self) {
        self.loading.set(true);
        leptos::task::spawn_local(async move {
            let res = wire::friend_invites().await;
            self.loading.set(false);
            match res {
                Ok(data) => self.list.set(Some(Ok(data))),
                Err(_) => self.list.set(Some(Err(()))),
            }
        });
    }

    fn close(self) {
        self.open.set(false);
        focus::after_render("#people-invite-btn");
    }

    fn copy_value(self, target: CopiedTarget, value: String) {
        leptos::task::spawn_local(async move {
            if clip::write(&value).await {
                if let Some(Some(prev)) = self.copy_timer.try_get_value() {
                    prev.clear();
                }
                self.copied.set(Some(target));
                let handle = set_timeout(
                    move || {
                        self.copied.set(None);
                    },
                    Duration::from_millis(1_800),
                )
                .ok();
                let _ = self.copy_timer.try_set_value(handle);
            }
        });
    }

    fn create_invite(self) {
        if self.busy.get_untracked() {
            return;
        }
        self.busy.set(true);
        self.error.set(None);
        let (label, note, email, days, lang_code) = (
            Some(self.label.get_untracked()),
            Some(self.note.get_untracked()),
            Some(self.email.get_untracked()),
            Some(self.days.get_untracked()),
            self.lang.code(),
        );
        leptos::task::spawn_local(async move {
            let outcome = wire::create_friend_invite(label, note, email, lang_code, days).await;
            self.busy.set(false);
            match outcome {
                Ok(res) => {
                    self.label.set(String::new());
                    self.note.set(String::new());
                    self.email.set(String::new());
                    self.show_qr.set(false);
                    self.copied.set(None);
                    self.created.set(Some(res));
                    self.reload();
                }
                Err(_) => self.error.set(Some(words::people_failed(self.lang))),
            }
        });
    }

    fn revoke_invite(self, id: [u8; 16]) {
        if self.busy.get_untracked() {
            return;
        }
        self.busy.set(true);
        self.error.set(None);
        leptos::task::spawn_local(async move {
            let res = wire::revoke_friend_invite(id).await;
            self.busy.set(false);
            match res {
                Ok(()) => self.reload(),
                Err(_) => self.error.set(Some(words::people_failed(self.lang))),
            }
        });
    }
}

fn create_form_view(state: &InviteDialogState, slots_full: Memo<bool>) -> AnyView {
    let state = *state;
    let lang = state.lang;
    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if !slots_full.get_untracked() {
            state.create_invite();
        }
    };
    let day_options = (1..=MAX_FRIEND_INVITE_DAYS)
        .map(|d| {
            let val = d.to_string();
            let text = words::invite_expiry_days(lang, u64::from(d));
            view! { <option value=val selected=move || state.days.get() == d>{text}</option> }
        })
        .collect_view();

    view! {
        <form class="invite-form" on:submit=on_submit>
            <div class="field">
                <label for="invite-label">{words::invite_label_field(lang)}</label>
                <div class="inset">
                    <span class="inset-mark">{glyph(glyph::PENCIL)}</span>
                    <input
                        id="invite-label"
                        type="text"
                        maxlength=MAX_INVITE_LABEL.to_string()
                        placeholder=words::invite_label_placeholder(lang)
                        prop:value=move || state.label.get()
                        on:input=move |ev| state.label.set(event_target_value(&ev))
                    />
                </div>
            </div>
            <div class="field">
                <label for="invite-note">{words::invite_note_field(lang)}</label>
                <textarea
                    id="invite-note"
                    rows="2"
                    maxlength=MAX_INVITE_NOTE.to_string()
                    placeholder=words::invite_note_placeholder(lang)
                    prop:value=move || state.note.get()
                    on:input=move |ev| state.note.set(event_target_value(&ev))
                ></textarea>
            </div>
            <div class="invite-form-row">
                <div class="field">
                    <label for="invite-email">{words::invite_email_field(lang)}</label>
                    <div class="inset">
                        <span class="inset-mark">{glyph(glyph::MAIL)}</span>
                        <input
                            id="invite-email"
                            type="email"
                            autocomplete="off"
                            placeholder=words::invite_email_placeholder(lang)
                            aria-describedby="invite-email-hint"
                            prop:value=move || state.email.get()
                            on:input=move |ev| state.email.set(event_target_value(&ev))
                        />
                    </div>
                </div>
                <div class="field invite-days-field">
                    <label for="invite-days">{words::invite_expiry_field(lang)}</label>
                    <div class="inset">
                        <span class="inset-mark">{glyph(glyph::CLOCK)}</span>
                        <select
                            id="invite-days"
                            aria-label=words::invite_expiry_field(lang)
                            prop:value=move || state.days.get().to_string()
                            on:change=move |ev| {
                                if let Ok(v) = event_target_value(&ev).parse::<u8>() {
                                    state.days.set(v.clamp(1, MAX_FRIEND_INVITE_DAYS));
                                }
                            }
                        >
                            {day_options}
                        </select>
                    </div>
                </div>
            </div>
            <p id="invite-email-hint" class="hint">{words::invite_email_hint(lang)}</p>
            <p class="error" role="alert">{move || state.error.get()}</p>
            <div class="invite-form-actions">
                <button
                    type="submit"
                    class="button small primary"
                    aria-disabled=move || (state.busy.get() || slots_full.get()).then_some("true")
                >
                    {glyph(glyph::USER_PLUS)}
                    <span>
                        {move || if state.busy.get() {
                            words::invite_creating(lang)
                        } else {
                            words::invite_create_btn(lang)
                        }}
                    </span>
                </button>
            </div>
        </form>
    }
    .into_any()
}

fn share_actions_row(
    state: &InviteDialogState,
    url: &str,
    code: String,
    invited_email: Option<&str>,
) -> AnyView {
    let state = *state;
    let lang = state.lang;
    let url_for_copy = url.to_owned();
    let url_for_share = url.to_owned();
    let mailto = mailto_href(lang, invited_email, url);
    let has_share = web_share_supported();

    view! {
        <div class="invite-share-actions">
            <button
                type="button"
                class="button small primary"
                on:click=move |_| state.copy_value(CopiedTarget::Link, url_for_copy.clone())
            >
                {move || if state.copied.get() == Some(CopiedTarget::Link) {
                    glyph(glyph::CHECK)
                } else {
                    glyph(glyph::LINK)
                }}
                <span>
                    {move || if state.copied.get() == Some(CopiedTarget::Link) {
                        words::invite_copied(lang)
                    } else {
                        words::invite_copy_link(lang)
                    }}
                </span>
            </button>
            <button
                type="button"
                class="button small quiet"
                on:click=move |_| state.copy_value(CopiedTarget::Code, code.clone())
            >
                {move || if state.copied.get() == Some(CopiedTarget::Code) {
                    glyph(glyph::CHECK)
                } else {
                    glyph(glyph::COPY)
                }}
                <span>
                    {move || if state.copied.get() == Some(CopiedTarget::Code) {
                        words::invite_copied(lang)
                    } else {
                        words::invite_copy_code(lang)
                    }}
                </span>
            </button>
            {has_share.then(|| view! {
                <button
                    type="button"
                    class="button small quiet"
                    on:click=move |_| {
                        let title = words::invite_mailto_subject(lang);
                        let text = words::invite_mailto_body(lang, &url_for_share);
                        invoke_web_share(title, &text, &url_for_share);
                    }
                >
                    {glyph(glyph::SHARE_2)}
                    <span>{words::invite_share_native(lang)}</span>
                </button>
            })}
            <a class="button small quiet" href=mailto>
                {glyph(glyph::MAIL)}
                <span>{words::invite_open_mailto(lang)}</span>
            </a>
            <button
                type="button"
                class="button small quiet"
                aria-expanded=move || state.show_qr.get().to_string()
                on:click=move |_| state.show_qr.update(|v| *v = !*v)
            >
                {glyph(glyph::QR_CODE)}
                <span>
                    {move || if state.show_qr.get() {
                        words::invite_hide_qr(lang)
                    } else {
                        words::invite_show_qr(lang)
                    }}
                </span>
            </button>
        </div>
    }
    .into_any()
}

fn created_share_panel(state: &InviteDialogState, created: CreatedFriendInvite) -> AnyView {
    let state = *state;
    let lang = state.lang;
    let url = invite_share_url(lang, &created.code);
    let url_for_qr = url.clone();
    let actions = share_actions_row(
        &state,
        &url,
        created.code.clone(),
        created.invite.invited_email.as_deref(),
    );

    let mail_note = if created.mail_sent {
        Some(view! { <p class="hint invite-mail-note">{words::invite_mail_sent(lang)}</p> })
    } else if created.invite.invited_email.is_some() {
        Some(view! { <p class="hint invite-mail-note">{words::invite_mail_skipped(lang)}</p> })
    } else {
        None
    };

    view! {
        <section class="invite-created-card" aria-live="polite">
            <div class="invite-created-head">
                <span class="invite-created-icon">{glyph(glyph::SPARKLES)}</span>
                <strong>{words::invite_created_title(lang)}</strong>
            </div>
            {mail_note}
            <div class="invite-link-box" translate="no">
                <code>{url}</code>
                <span class="invite-code-tag">{created.code}</span>
            </div>
            {actions}
            {move || state.show_qr.get().then(|| {
                let qr_url = url_for_qr.clone();
                view! {
                    <div class="invite-qr-panel">
                        {qr::qr_svg(qr_url, words::invite_qr_alt(lang).to_owned())}
                    </div>
                }
            })}
        </section>
    }
    .into_any()
}

fn invite_status_pill(lang: Lang, status: FriendInviteStatus) -> AnyView {
    let (cls, text) = match status {
        FriendInviteStatus::Active => ("pill ok", words::invite_status_active(lang)),
        FriendInviteStatus::Redeemed => ("pill accent", words::invite_status_redeemed(lang)),
        FriendInviteStatus::Revoked => ("pill warn", words::invite_status_revoked(lang)),
        FriendInviteStatus::Expired => ("pill off", words::invite_status_expired(lang)),
    };
    view! { <span class=cls>{text}</span> }.into_any()
}

fn invite_row_view(state: &InviteDialogState, item: &FriendInviteSummary) -> AnyView {
    let state = *state;
    let lang = state.lang;
    let id = item.id;
    let status = item.status;
    let title = item
        .label
        .as_deref()
        .filter(|s| !s.is_empty())
        .or_else(|| item.invited_email.as_deref().filter(|s| !s.is_empty()))
        .map_or_else(|| format_unix_secs(lang, item.created_at), str::to_owned);
    let email_line = item
        .invited_email
        .as_ref()
        .filter(|e| item.label.as_ref().is_some_and(|l| l != *e))
        .cloned();
    let note_line = item.note.clone();
    let expires_text = words::invite_expires_at(lang, &format_unix_secs(lang, item.expires_at));
    let redeemed_text = item
        .redeemed_by
        .as_ref()
        .map(|who| words::invite_redeemed_by(lang, &handle(who)));

    view! {
        <li class="invite-row" data-status=match status {
            FriendInviteStatus::Active => "active",
            FriendInviteStatus::Redeemed => "redeemed",
            FriendInviteStatus::Revoked => "revoked",
            FriendInviteStatus::Expired => "expired",
        }>
            <div class="invite-row-main">
                <div class="invite-row-top">
                    <strong class="invite-row-title" translate="no">{title}</strong>
                    {invite_status_pill(lang, status)}
                </div>
                {email_line.map(|addr| view! { <span class="invite-row-email" translate="no">{addr}</span> })}
                {note_line.map(|text| view! { <p class="invite-row-note">{text}</p> })}
                <div class="invite-row-meta">
                    <span>{expires_text}</span>
                    {redeemed_text.map(|who| view! { <span>{who}</span> })}
                </div>
            </div>
            {(status == FriendInviteStatus::Active).then(|| view! {
                <button
                    type="button"
                    class="button small danger invite-revoke-btn"
                    aria-disabled=move || state.busy.get().then_some("true")
                    on:click=move |_| state.revoke_invite(id)
                >
                    {glyph(glyph::TRASH_2)}
                    <span>{words::invite_revoke(lang)}</span>
                </button>
            })}
        </li>
    }
    .into_any()
}

fn invites_list_section(state: &InviteDialogState) -> AnyView {
    let state = *state;
    let lang = state.lang;
    let pill = move |which: InviteFilter, text: &'static str| {
        let is_on = move || state.filter.get() == which;
        view! {
            <button
                type="button"
                class="pill-button"
                data-active=move || is_on().then_some("true")
                aria-pressed=move || is_on().to_string()
                on:click=move |_| state.filter.set(which)
            >
                <span>{text}</span>
            </button>
        }
    };

    let body = move || {
        if state.loading.get() && state.list.with_untracked(Option::is_none) {
            return view! { <p class="meta" role="status">{words::account_loading(lang)}</p> }
                .into_any();
        }
        match state.list.get() {
            None => view! { <p class="meta">{words::account_loading(lang)}</p> }.into_any(),
            Some(Err(())) => view! {
                <p class="banner bad" role="alert">
                    {glyph(glyph::INFO)}
                    <span>{words::people_failed(lang)}</span>
                    <button type="button" class="button small quiet" on:click=move |_| state.reload()>
                        {glyph(glyph::REFRESH_CW)}
                        <span>{words::chat_retry(lang)}</span>
                    </button>
                </p>
            }
            .into_any(),
            Some(Ok(data)) => {
                let current_filter = state.filter.get();
                let filtered: Vec<FriendInviteSummary> = data
                    .invites
                    .into_iter()
                    .filter(|item| match current_filter {
                        InviteFilter::All => true,
                        InviteFilter::Active => item.status == FriendInviteStatus::Active,
                        InviteFilter::Used => item.status != FriendInviteStatus::Active,
                    })
                    .collect();
                if filtered.is_empty() {
                    view! { <p class="meta">{words::invite_empty(lang)}</p> }.into_any()
                } else {
                    let rows = filtered
                        .iter()
                        .map(|item| invite_row_view(&state, item))
                        .collect_view();
                    view! { <ul class="invite-list">{rows}</ul> }.into_any()
                }
            }
        }
    };

    view! {
        <section class="invite-list-section" aria-labelledby="invite-list-title">
            <div class="invite-list-head">
                <h4 id="invite-list-title">{words::invite_list_title(lang)}</h4>
                <div class="invite-filter-pills" role="group" aria-label=words::invite_list_title(lang)>
                    {pill(InviteFilter::All, words::invite_filter_all(lang))}
                    {pill(InviteFilter::Active, words::invite_filter_active(lang))}
                    {pill(InviteFilter::Used, words::invite_filter_used(lang))}
                </div>
            </div>
            {body}
        </section>
    }
    .into_any()
}

/// Renders the interactive friend-invite creation, sharing, QR, and management
/// modal dialog controlled by `open`.
pub fn invite_dialog(lang: Lang, ctl: Ctl, open: RwSignal<bool>) -> AnyView {
    let state = InviteDialogState::new(lang, ctl, open);

    Effect::new(move |_| {
        let _rev = state.ctl.invites_rev.get();
        if state.open.get() {
            state.reload();
        }
    });

    let active_counts = Memo::new(move |_| {
        state.list.with(|l| match l {
            Some(Ok(data)) => {
                let used = data
                    .invites
                    .iter()
                    .filter(|i| i.status == FriendInviteStatus::Active)
                    .count();
                (used, usize::try_from(data.active_limit).unwrap_or(10))
            }
            _ => (0, 10),
        })
    });
    let slots_full = Memo::new(move |_| {
        let (used, max) = active_counts.get();
        used >= max
    });

    view! {
        {move || {
            if !open.get() {
                return None;
            }
            open_modal_dialog(DIALOG_ID);
            let backdrop_down = RwSignal::new(false);
            let slots_label = move || {
                let (used, max) = active_counts.get();
                words::invite_slots(lang, &used.to_string(), &max.to_string())
            };

            Some(view! {
                <dialog
                    id=DIALOG_ID
                    class="conv-details-dialog invite-dialog"
                    aria-labelledby="invite-dialog-title"
                    on:pointerdown=move |ev: leptos::ev::PointerEvent| {
                        backdrop_down.set(ev.target() == ev.current_target());
                    }
                    on:click=move |ev: leptos::ev::MouseEvent| {
                        if backdrop_down.get_untracked() && ev.target() == ev.current_target() {
                            state.close();
                        }
                    }
                    on:close=move |_| state.close()
                >
                    <div class="conv-details-card invite-dialog-card">
                        <header class="conv-details-head">
                            <div class="invite-dialog-title-wrap">
                                <h3 id="invite-dialog-title">{words::invite_title(lang)}</h3>
                                <span class="pill" data-full=move || slots_full.get().then_some("true")>
                                    {slots_label}
                                </span>
                            </div>
                            <button
                                type="button"
                                class="dialog-close"
                                title=words::chat_close(lang)
                                aria-label=words::chat_close(lang)
                                on:click=move |_| state.close()
                            >
                                {glyph(glyph::CLOSE)}
                            </button>
                        </header>
                        <div class="conv-details-body invite-dialog-body">
                            <p class="invite-lead">{words::invite_lead(lang)}</p>
                            {move || state.list.with(|l| match l {
                                Some(Ok(data)) => data.recruiter_badge.as_ref().map(|b| recruiter_medal_banner(lang, b)),
                                _ => None,
                            })}
                            {create_form_view(&state, slots_full)}
                            {move || state.created.get().map(|res| created_share_panel(&state, res))}
                            {invites_list_section(&state)}
                        </div>
                    </div>
                </dialog>
            })
        }}
    }
    .into_any()
}
