//! Signing in, registering and the signed-in area: the server renders the
//! frame, the headings and the navigation; the islands do the rest in the
//! browser, where the session and the crypto are.

use leptos::prelude::*;

use super::Built;
use crate::data::Section;
use crate::islands::{Account, Administration, Messenger};
use crate::rich::icon;
use crate::site::icons;
use crate::site::shell::Meta;
use crate::site::{Context, Page};
use crate::words::{self, Lang};

/// One of the three forms at its own address. The form is a dialog every
/// page carries (`shell::auth_dialogs`), so its address shows the start page
/// with that dialog standing open over it (`boot.js` opens the one the
/// document names in `data-auth`). The head is the form's own: its title and
/// its lead, and none of the start page's structured data, because these
/// addresses are in no index.
fn over_start(cx: &Context<'_>, title: &'static str, lead: &'static str) -> Built {
    let (_, body) = super::home::page(cx);
    let mut meta = Meta::new(format!("{title} – Baylee"), lead, "img/og.jpg");
    meta.islands = true;
    (meta, body)
}

pub fn sign_in(cx: &Context<'_>) -> Built {
    over_start(
        cx,
        words::signin_title(cx.lang),
        words::signin_lead(cx.lang),
    )
}

pub fn register(cx: &Context<'_>) -> Built {
    over_start(
        cx,
        words::register_title(cx.lang),
        words::register_lead(cx.lang),
    )
}

/// A new password with an administrator's reset code
/// (`docs/account-reset.md`): reached by its link from the sign-in form and
/// from the reset mail, in no navigation and no sitemap.
pub fn reset(cx: &Context<'_>) -> Built {
    over_start(cx, words::reset_title(cx.lang), words::reset_lead(cx.lang))
}

/// A section's title and its icon.
fn section_name(lang: Lang, section: Section) -> (&'static str, &'static str) {
    match section {
        Section::Home => (words::account_home(lang), icons::HOUSE),
        Section::Chat => (words::account_chat(lang), icons::MESSAGE_CIRCLE),
        Section::Contacts => (words::account_contacts(lang), icons::USERS),
        Section::Profile => (words::account_profile(lang), icons::USER),
        Section::Security => (words::account_security(lang), icons::SHIELD),
        Section::Privacy => (words::account_privacy(lang), icons::FILE_TEXT),
        Section::Gateways => (words::account_gateways(lang), icons::SERVER),
        Section::Notifications => (words::push_title(lang), icons::BELL),
        Section::Admin => (words::admin_reset_title(lang), icons::KEY_ROUND),
        Section::AdminOverview => (words::account_admin(lang), icons::LAYERS),
        Section::AdminUsers => (words::admin_accounts(lang), icons::USERS),
        Section::AdminUser(_) => (words::admin_account(lang), icons::USER),
        Section::AdminGateways => (words::gw_title(lang), icons::SERVER),
        Section::AdminAudit => (words::admin_audit(lang), icons::FILE_TEXT),
        Section::AdminLegacy => (words::admin_legacy(lang), icons::FILE_ARCHIVE),
        Section::AdminPictures => (words::admin_pictures_title(lang), icons::FLAG),
        Section::Feedback => (words::fb_title(lang), icons::MESSAGE_CIRCLE),
        Section::Report => (words::fb_report_title(lang), icons::FILE_TEXT),
    }
}

pub fn area(cx: &Context<'_>, section: Section) -> Built {
    let lang = cx.lang;
    let (title, _) = section_name(lang, section);
    let mut meta = Meta::new(
        format!("{title} – Baylee"),
        words::account_lead(lang),
        "img/og.jpg",
    );
    meta.islands = true;
    let links = Section::ALL
        .into_iter()
        .map(|item| {
            let (name, glyph) = section_name(lang, item);
            let here = item == section;
            view! {
                <li>
                    <a href=Page::Account(item).path(lang) aria-current=here.then_some("page")>
                        {icon(glyph)}
                        <span>{name}</span>
                    </a>
                </li>
            }
        })
        .collect_view();
    let worker = cx.assets.url("crypto-worker.js");
    let messenger = matches!(section, Section::Chat | Section::Contacts);
    // the administration's pages have an island of their own, in a module
    // of their own (`shell::islands_script`)
    let island = if messenger {
        view! { <Messenger lang=lang section=section worker=worker /> }.into_any()
    } else if section.administration() {
        view! { <Administration lang=lang section=section worker=worker /> }.into_any()
    } else {
        view! { <Account lang=lang section=section worker=worker /> }.into_any()
    };
    let body = view! {
        <div class="frame" class:frame-messenger=messenger>
            <nav class="frame-nav" aria-label=words::account_nav(lang)>
                <p class="frame-nav-head" aria-hidden="true">{words::app_account(lang)}</p>
                <ul>{links}</ul>
                // shown by pwa.js where the browser offers to install the app
                <p class="install" data-install-box hidden>
                    <button type="button" class="button" data-install>
                        {icon(icons::DOWNLOAD)}<span>{words::install_app(lang)}</span>
                    </button>
                </p>
            </nav>
            <div class="frame-main">
                <header class="frame-head" class:visually-hidden=messenger>
                    <p class="eyebrow">{words::acct_eyebrow(lang)}</p>
                    <h1>{title}</h1>
                </header>
                {island}
            </div>
        </div>
    }
    .into_any();
    (meta, body)
}
