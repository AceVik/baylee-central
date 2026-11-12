//! Signing in, registering and the signed-in area: the server renders the
//! frame, the headings and the navigation; the islands do the rest in the
//! browser, where the session and the crypto are.

use leptos::prelude::*;

use super::Built;
use crate::data::Section;
use crate::islands::{Account, Messenger, Register, Reset, SignIn};
use crate::rich::{icon, rich};
use crate::site::icons;
use crate::site::shell::Meta;
use crate::site::{Context, Page};
use crate::words::{self, Lang};

/// The forms explain themselves without script: the password is processed
/// in the browser, so there is nothing to send without it. A reset says so
/// of the recovery code as well.
fn noscript(lang: Lang, page: Page) -> impl IntoView + use<> {
    let words = if page == Page::Reset {
        words::reset_noscript(lang)
    } else {
        words::signin_noscript(lang)
    };
    view! { <noscript><p class="notice">{words}</p></noscript> }
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
    // as the old site set it: the words and the form on the left, Baylee
    // on her cloud beside them, floating
    let cloud = format!(
        "{} 600w, {} 1200w",
        cx.assets.url("img/cloud-600.webp"),
        cx.assets.url("img/cloud-1200.webp")
    );
    // signing in says where its password goes
    let more = (page == Page::SignIn).then(|| {
        let security = Page::Security.path(lang);
        let link = rich(words::sec_signin_link(lang), move |tag| {
            (tag == "security").then(|| security.clone())
        });
        view! { <p class="hint">{link}</p> }
    });
    let body = view! {
        <div class="wrap auth">
            <section class="page-head auth-head" aria-labelledby="auth-title">
                <p class="eyebrow">{words::acct_eyebrow(lang)}</p>
                <h1 id="auth-title">{title}</h1>
                <p class="lead">{lead}</p>
                <div class="auth-form glass">
                    {form}
                    {noscript(lang, page)}
                </div>
                {more}
            </section>
            <img class="auth-cloud" src=cx.assets.url("img/cloud-600.webp") srcset=cloud sizes="min(40vw, 40rem)"
                width="600" height="400" alt="" loading="lazy" decoding="async" />
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
    let pow = cx.assets.url("pow-worker.js");
    form_page(
        cx,
        words::register_title(lang),
        words::register_lead(lang),
        Page::Register,
        view! { <Register lang=lang worker=worker pow=pow /> }.into_any(),
    )
}

/// A new password with an administrator's reset code
/// (`docs/account-reset.md`): reached by its link from the sign-in form,
/// in no navigation and no sitemap.
pub fn reset(cx: &Context<'_>) -> Built {
    let lang = cx.lang;
    let worker = cx.assets.url("crypto-worker.js");
    form_page(
        cx,
        words::reset_title(lang),
        words::reset_lead(lang),
        Page::Reset,
        view! { <Reset lang=lang worker=worker /> }.into_any(),
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
        Section::Admin => (words::admin_reset_title(lang), icons::KEY_ROUND),
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
    let island = if messenger {
        view! { <Messenger lang=lang section=section worker=worker /> }.into_any()
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
