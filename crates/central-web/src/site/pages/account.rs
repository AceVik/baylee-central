//! Signing in, registering and the signed-in area: the server renders the
//! frame, the headings and the navigation; the islands do the rest in the
//! browser, where the session and the crypto are.

use leptos::prelude::*;

use super::Built;
use crate::data::Section;
use crate::islands::{Account, Messenger, Register, SignIn};
use crate::rich::icon;
use crate::site::icons;
use crate::site::shell::Meta;
use crate::site::{Context, Page};
use crate::words::{self, Lang};

/// The forms explain themselves without script: the password is processed
/// in the browser, so there is nothing to send without it.
fn noscript(lang: Lang) -> impl IntoView + use<> {
    view! { <noscript><p class="notice">{words::signin_noscript(lang)}</p></noscript> }
}

fn form_page(
    cx: &Context<'_>,
    title: &'static str,
    lead: &'static str,
    page: Page,
    form: AnyView,
) -> Built {
    let lang = cx.lang;
    let mut meta = Meta::new(format!("{title} – Baylee"), lead, "img/og.jpg");
    meta.islands = true;
    meta.trail.push((title.to_owned(), page));
    let body = view! {
        <div class="wrap auth">
            <section class="page-head">
                <h1>{title}</h1>
                <p class="lead">{lead}</p>
            </section>
            <div class="auth-grid">
                <div class="auth-form">
                    {form}
                    {noscript(lang)}
                </div>
                <img class="auth-cloud" src=cx.assets.url("img/cloud-600.webp") width="600" height="400" alt="" loading="lazy" decoding="async" />
            </div>
        </div>
    }
    .into_any();
    (meta, body)
}

pub fn sign_in(cx: &Context<'_>) -> Built {
    let lang = cx.lang;
    let worker = cx.assets.url("crypto-worker.js");
    form_page(
        cx,
        words::signin_title(lang),
        words::signin_lead(lang),
        Page::SignIn,
        view! { <SignIn lang=lang worker=worker /> }.into_any(),
    )
}

pub fn register(cx: &Context<'_>) -> Built {
    let lang = cx.lang;
    let worker = cx.assets.url("crypto-worker.js");
    form_page(
        cx,
        words::register_title(lang),
        words::register_lead(lang),
        Page::Register,
        view! { <Register lang=lang worker=worker /> }.into_any(),
    )
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
    let island = if messenger {
        view! { <Messenger lang=lang section=section worker=worker /> }.into_any()
    } else {
        view! { <Account lang=lang section=section worker=worker /> }.into_any()
    };
    let body = view! {
        <div class="frame" class:frame-messenger=messenger>
            <nav class="frame-nav" aria-label=words::account_nav(lang)>
                <ul>{links}</ul>
                // shown by pwa.js where the browser offers to install the app
                <p class="install" data-install-box hidden>
                    <button type="button" class="button" data-install>
                        {icon(icons::DOWNLOAD)}<span>{words::install_app(lang)}</span>
                    </button>
                </p>
            </nav>
            <div class="frame-main">
                <h1 class=messenger.then_some("visually-hidden")>{title}</h1>
                {island}
            </div>
        </div>
    }
    .into_any();
    (meta, body)
}
