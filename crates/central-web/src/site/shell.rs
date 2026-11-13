//! The frame of every page: the document head (title, description,
//! canonical address, languages, sharing cards, structured data), the bar
//! with the language menu, the footer with the Fan Content notice, and
//! Baylee's world behind every page but the start.

use leptos::prelude::*;
use leptos::tachys::html::attribute::custom::custom_attribute;
use serde_json::{Value, json};

use super::{Assets, Context, Page, icons};
use crate::islands::{Register, SignIn};
use crate::rich::{icon, rich};
use crate::solid;
use crate::words::{self, Lang};

/// Where Baylee's code lives.
pub const REPOSITORY: &str = "https://github.com/AceVik/baylee";
/// Where Baylee Central's code lives.
pub const CENTRAL_REPOSITORY: &str = "https://github.com/AceVik/baylee-central";
/// Baylee's community on Discord.
pub const DISCORD: &str = "https://discord.gg/4pP3uAwgY";
/// Baylee's community on `WhatsApp`.
pub const WHATSAPP: &str = "https://chat.whatsapp.com/JsWHl71rnPPGEG74w48vk6";

/// What a page says about itself to the head.
pub struct Meta {
    /// The `<title>`.
    pub title: String,
    /// The description for search results and sharing cards.
    pub description: String,
    /// The sharing image's logical asset name.
    pub image: &'static str,
    /// Structured data beyond the breadcrumb.
    pub data: Vec<Value>,
    /// The page has islands: load the WebAssembly.
    pub islands: bool,
    /// The trail from the start page, without it: (name, page).
    pub trail: Vec<(String, Page)>,
}

impl Meta {
    /// A page's head with its title, description and sharing image.
    pub fn new(
        title: impl Into<String>,
        description: impl Into<String>,
        image: &'static str,
    ) -> Self {
        Meta {
            title: title.into(),
            description: description.into(),
            image,
            data: Vec::new(),
            islands: false,
            trail: Vec::new(),
        }
    }
}

/// The Fan Content Policy's address in a language: Wizards' own page in
/// each of them (every one answered on 2026-11-11).
pub fn policy_url(lang: Lang) -> &'static str {
    match lang {
        Lang::En => "https://company.wizards.com/en/legal/fancontentpolicy",
        Lang::De => "https://company.wizards.com/de/legal/fancontentpolicy",
        Lang::Fr => "https://company.wizards.com/fr/legal/fancontentpolicy",
        Lang::It => "https://company.wizards.com/it/legal/fancontentpolicy",
        Lang::Es => "https://company.wizards.com/es/legal/fancontentpolicy",
        Lang::PtBr => "https://company.wizards.com/pt-BR/legal/fancontentpolicy",
        Lang::Ru => "https://company.wizards.com/ru/legal/fancontentpolicy",
        Lang::Ja => "https://company.wizards.com/ja/legal/fancontentpolicy",
        Lang::Ko => "https://company.wizards.com/ko/legal/fancontentpolicy",
        Lang::ZhHans => "https://company.wizards.com/zh-Hans/legal/fancontentpolicy",
        Lang::ZhHant => "https://company.wizards.com/zh-Hant/legal/fancontentpolicy",
    }
}

/// JSON for a `<script type="application/ld+json">`: `<` escaped, so no
/// text in it can close the element.
fn ld_json(value: &Value) -> String {
    value.to_string().replace('<', "\\u003c")
}

/// The breadcrumb's structured data, when the page has a trail.
fn breadcrumb_data(cx: &Context<'_>, trail: &[(String, Page)]) -> Option<Value> {
    if trail.is_empty() {
        return None;
    }
    let (lang, origin) = (cx.lang, cx.origin);
    let mut items = vec![json!({
        "@type": "ListItem", "position": 1,
        "name": words::breadcrumb_home(lang),
        "item": format!("{origin}{}", Page::Home.path(lang)),
    })];
    for (at, (name, step)) in trail.iter().enumerate() {
        items.push(json!({
            "@type": "ListItem", "position": at + 2, "name": name,
            "item": format!("{origin}{}", step.path(lang)),
        }));
    }
    Some(
        json!({"@context": "https://schema.org", "@type": "BreadcrumbList", "itemListElement": items}),
    )
}

/// The visible breadcrumb above a page's head.
fn crumbs(lang: Lang, trail: &[(String, Page)]) -> Option<impl IntoView + use<>> {
    if trail.is_empty() {
        return None;
    }
    let steps = trail
        .iter()
        .enumerate()
        .map(|(at, (name, step))| {
            let name = name.clone();
            if at + 1 == trail.len() {
                view! { <li><span aria-current="page">{name}</span></li> }.into_any()
            } else {
                view! { <li><a href=step.path(lang)>{name}</a></li> }.into_any()
            }
        })
        .collect_view();
    Some(view! {
        <nav class="crumbs wrap" aria-label=words::crumbs_label(lang)>
            <ol>
                <li><a href=Page::Home.path(lang)>{words::breadcrumb_home(lang)}</a></li>
                {steps}
            </ol>
        </nav>
    })
}

/// What a page asks for before its stylesheet and scripts: the start
/// page's painting (its largest paint) and the faces the first screen
/// is set in.
fn preloads(page: Page, lang: Lang, assets: &Assets) -> impl IntoView + use<> {
    let painting = (page == Page::Home).then(|| {
        let set = format!(
            "{} 640w, {} 960w, {} 1600w",
            assets.url("img/world-640.avif"),
            assets.url("img/world-960.avif"),
            assets.url("img/world-1600.avif")
        );
        view! {
            <link rel="preload" r#as="image" type="image/avif" imagesrcset=set imagesizes="100vw" fetchpriority="high" />
        }
    });
    let font = |name: &str| {
        view! { <link rel="preload" href=assets.url(name) r#as="font" type="font/woff2" crossorigin="anonymous" /> }
    };
    // the start page's title ends in italic: its face, too, before the
    // title wraps around a stand-in. (The download button's bold and
    // medium faces move it by less than CLS 0.05 as they arrive; asked for
    // early, they took the painting's bandwidth: LCP was later.)
    //
    // The faces a language's words are set in: Cyrillic for Russian; the
    // Latin ones for every other, which Chinese, Japanese and Korean use
    // for names and numbers beside their own system faces (and they have
    // no italic).
    let (subset, slanted) = match lang {
        Lang::Ru => ("cyrillic", true),
        Lang::Ja | Lang::Ko | Lang::ZhHans | Lang::ZhHant => ("latin", false),
        _ => ("latin", true),
    };
    let italic = (page == Page::Home && slanted)
        .then(|| font(&format!("fonts/Literata-Italic-{subset}.woff2")));
    view! {
        {painting}
        {font(&format!("fonts/Inter-{subset}.woff2"))}
        {font(&format!("fonts/Literata-{subset}.woff2"))}
        {italic}
    }
}

/// Everything in `<head>`.
fn head(page: Page, cx: &Context<'_>, meta: &Meta) -> impl IntoView + use<> {
    let lang = cx.lang;
    let origin = cx.origin.to_owned();
    let assets = cx.assets;
    let canonical = format!("{origin}{}", page.path(lang));
    let listed = Page::LISTED.contains(&page);
    let alternates = listed.then(|| {
        let mut links: Vec<(&'static str, String)> = Lang::ALL
            .into_iter()
            .map(|other| (other.tag(), format!("{origin}{}", page.path(other))))
            .collect();
        links.push(("x-default", format!("{origin}{}", page.path(Lang::En))));
        links
            .into_iter()
            .map(|(code, href)| view! { <link rel="alternate" hreflang=code href=href /> })
            .collect_view()
    });
    let image = format!("{origin}{}", assets.url(meta.image));
    let structured = meta
        .data
        .iter()
        .cloned()
        .chain(breadcrumb_data(cx, &meta.trail))
        .map(|value| view! { <script type="application/ld+json" inner_html=ld_json(&value)></script> })
        .collect_view();
    let robots = (!page.indexed())
        .then(|| view! { <meta name="robots" content="noindex, noarchive, nosnippet" /> });
    let verification = cx
        .verification
        .map(|value| view! { <meta name="google-site-verification" content=value.to_owned() /> });
    // a page's own script: the start page's album and screenshots, the
    // gateway directory's finding
    let own = match page {
        Page::Home => Some("home.js"),
        Page::Gateways => Some("directory.js"),
        _ => None,
    }
    .map(|name| view! { <script type="module" src=assets.url(name) integrity=assets.integrity(name)></script> });
    // the start page's sky: clouds the pointer stirs; the script waits for
    // the page to have loaded and the browser to be idle before it begins.
    // A phone has no pointer to stir with and is not sent it.
    let sky = (page == Page::Home && !cx.machine.mobile).then(|| {
        view! { <script type="module" src=assets.url("clouds.js") integrity=assets.integrity("clouds.js") fetchpriority="low"></script> }
    });
    // every page that can open the sign-in over itself has its islands
    let islands = (meta.islands || has_auth_dialogs(page)).then(|| {
        let name = islands_script(page);
        view! { <script type="module" src=assets.url(name) integrity=assets.integrity(name) fetchpriority="low"></script> }
    });
    let others = Lang::ALL
        .into_iter()
        .filter(|other| *other != lang)
        .map(|other| view! { <meta {..custom_attribute("property", "og:locale:alternate")} content=other.locale() /> })
        .collect_view();
    view! {
        <head>
            <meta charset="utf-8" />
            <meta
                name="viewport"
                content="width=device-width, initial-scale=1, viewport-fit=cover, interactive-widget=resizes-content"
            />
            <title>{meta.title.clone()}</title>
            <meta name="description" content=meta.description.clone() />
            {listed.then(|| view! { <link rel="canonical" href=canonical.clone() /> })}
            {alternates}
            {robots}
            <meta name="color-scheme" content="light dark" />
            <meta name="theme-color" media="(prefers-color-scheme: light)" content="#f5f2e9" />
            <meta name="theme-color" media="(prefers-color-scheme: dark)" content="#0f1b2d" />
            <meta name="referrer" content="no-referrer" />
            <meta name="tdm-reservation" content="1" />
            {preloads(page, lang, assets)}
            <link rel="stylesheet" href=assets.url("site.css") integrity=assets.integrity("site.css") />
            <script src=assets.url("boot.js") integrity=assets.integrity("boot.js")></script>
            <script type="module" src=assets.url("pwa.js") integrity=assets.integrity("pwa.js")></script>
            <script type="module" src=assets.url("hint.js") integrity=assets.integrity("hint.js") fetchpriority="low"></script>
            {own}
            {sky}
            {islands}
            <link rel="icon" href="/favicon.ico" sizes="32x32" />
            <link rel="icon" href=assets.url("img/icon-32.png") type="image/png" sizes="32x32" />
            <link rel="icon" href=assets.url("img/icon-192.webp") type="image/webp" sizes="192x192" />
            <link rel="apple-touch-icon" href="/apple-touch-icon.png" />
            <link rel="manifest" href=format!("/{}/manifest.webmanifest", lang.code()) />
            <meta {..custom_attribute("property", "og:type")} content="website" />
            <meta {..custom_attribute("property", "og:site_name")} content="Baylee" />
            <meta {..custom_attribute("property", "og:title")} content=meta.title.clone() />
            <meta {..custom_attribute("property", "og:description")} content=meta.description.clone() />
            {listed.then(|| view! { <meta {..custom_attribute("property", "og:url")} content=canonical.clone() /> })}
            <meta {..custom_attribute("property", "og:image")} content=image />
            <meta {..custom_attribute("property", "og:image:width")} content="1200" />
            <meta {..custom_attribute("property", "og:image:height")} content="630" />
            <meta {..custom_attribute("property", "og:image:alt")} content=meta.title.clone() />
            <meta {..custom_attribute("property", "og:locale")} content=lang.locale() />
            {others}
            <meta name="twitter:card" content="summary_large_image" />
            {verification}
            {structured}
        </head>
    }
}

/// The whole document around a page's `body`.
pub fn document(page: Page, cx: &Context<'_>, meta: &Meta, body: AnyView) -> impl IntoView + use<> {
    let lang = cx.lang;
    // the start page has the world in its hero; every other one behind it
    let world = (page != Page::Home).then(|| world(cx));
    view! {
        <html lang=lang.tag() dir="ltr">
            {head(page, cx, meta)}
            <body>
                <a class="skip" href="#main">{words::skip(lang)}</a>
                {world}
                {bar(page, cx)}
                <div class="page">
                    <main id="main" tabindex="-1">
                        {crumbs(lang, &meta.trail)}
                        {body}
                    </main>
                    {footer(page, cx)}
                </div>
                {update(lang)}
                {auth_dialogs(page, cx)}
            </body>
        </html>
    }
}

/// The bootstrap of the islands' module a page loads (`cargo xtask site`
/// writes both from `js/islands.js`): the administration's pages
/// (`/{lang}/account/admin…`) load the administration's own module, every
/// other page with islands the public one, which carries none of the
/// administration. A page loads one module, never both.
pub fn islands_script(page: Page) -> &'static str {
    match page {
        Page::Account(section) if section.administration() => "admin-islands.js",
        _ => "islands.js",
    }
}

/// Whether signing in and registering open over this page. Not over their
/// own pages, not in the signed-in area, and not on the page a device
/// shows without a connection.
fn has_auth_dialogs(page: Page) -> bool {
    !matches!(
        page,
        Page::SignIn | Page::Register | Page::Reset | Page::Account(_) | Page::Offline
    )
}

/// One of the two dialogs: Baylee's mark, the title, a line, the form.
fn auth_dialog(
    cx: &Context<'_>,
    id: &'static str,
    title: &'static str,
    lead: &'static str,
    form: AnyView,
    more: Option<AnyView>,
) -> impl IntoView + use<> {
    let lang = cx.lang;
    let title_id = format!("{id}-title");
    let labelled = title_id.clone();
    view! {
        <dialog {..custom_attribute("closedby", "any")} class="auth-dialog" id=id aria-labelledby=labelled>
            <div class="auth-card">
                <img class="auth-mark" src=cx.assets.url("img/icon-192.webp") alt="" width="192" height="192" loading="lazy" decoding="async" />
                <p class="eyebrow">{words::acct_eyebrow(lang)}</p>
                <h2 id=title_id>{title}</h2>
                <p class="auth-lead">{lead}</p>
                {form}
                {more}
                // last in the card, so the dialog opens with the keyboard
                // in its first field; drawn in the card's corner
                <button type="button" class="round-btn auth-close" data-auth-close aria-label=words::dialog_close(lang) title=words::dialog_close(lang)>
                    {icon(icons::X)}
                </button>
            </div>
        </dialog>
    }
}

/// Signing in and registering, over the page that asked for them: a link
/// to either opens its dialog (`boot.js`); without script, with a modifier
/// key, or on those pages themselves, the link leads to the page as
/// before. The forms are the pages' own islands, taken over when their
/// dialog first opens (`islands.js`), so a page that never opens one
/// fetches no WebAssembly for it.
fn auth_dialogs(page: Page, cx: &Context<'_>) -> Option<impl IntoView + use<>> {
    if !has_auth_dialogs(page) {
        return None;
    }
    let lang = cx.lang;
    let worker = cx.assets.url("crypto-worker.js");
    let pow = cx.assets.url("pow-worker.js");
    let security = Page::Security.path(lang);
    let link = rich(words::sec_signin_link(lang), move |tag| {
        (tag == "security").then(|| security.clone())
    });
    Some(view! {
        {auth_dialog(
            cx,
            "auth-signin",
            words::signin_title(lang),
            words::signin_lead(lang),
            view! { <SignIn lang=lang worker=worker.clone() /> }.into_any(),
            Some(view! { <p class="hint auth-more">{link}</p> }.into_any()),
        )}
        {auth_dialog(
            cx,
            "auth-register",
            words::register_title(lang),
            words::register_lead(lang),
            view! { <Register lang=lang worker=worker pow=pow /> }.into_any(),
            None,
        )}
    })
}

/// The address of the same page in `lang`.
fn translated(page: Page, lang: Lang) -> String {
    match page {
        Page::NotFound | Page::Gone | Page::Offline => Page::Home.path(lang),
        _ => page.path(lang),
    }
}

/// Baylee's world, quiet behind the page: decorative, small, late.
fn world(cx: &Context<'_>) -> impl IntoView + use<> {
    let assets = cx.assets;
    view! {
        <picture class="world" aria-hidden="true">
            <source type="image/avif" srcset=format!("{} 640w, {} 960w", assets.url("img/world-640.avif"), assets.url("img/world-960.avif")) sizes="100vw" />
            <img
                src=assets.url("img/world-960.webp")
                srcset=format!("{} 640w, {} 960w", assets.url("img/world-640.webp"), assets.url("img/world-960.webp"))
                sizes="100vw"
                width="960"
                height="540"
                alt=""
                decoding="async"
                fetchpriority="low"
            />
        </picture>
    }
}

/// A section in the bar, saying when it is the current page.
fn nav_link(page: Page, current: Page, lang: Lang, label: &'static str) -> impl IntoView + use<> {
    view! {
        <a href=page.path(lang) aria-current=(page == current).then_some("page")>{label}</a>
    }
}

/// The colour switch: one press shows the scheme that is not on screen
/// (boot.js turns it and names it; without script it stays hidden and the
/// device's scheme decides).
fn theme_switch(lang: Lang, in_menu: bool) -> impl IntoView + use<> {
    let class = if in_menu {
        "round-btn theme-btn theme-in-menu"
    } else {
        "round-btn theme-btn theme-in-bar"
    };
    let word =
        in_menu.then(|| view! { <span class="theme-word">{words::theme_label(lang)}</span> });
    view! {
        <button
            type="button"
            class=class
            data-theme-switch=""
            data-light=words::theme_light(lang)
            data-dark=words::theme_dark(lang)
            aria-label=words::theme_label(lang)
            hidden=true
        >
            <span class="when-light">{solid::icon(solid::SUN)}</span>
            <span class="when-dark">{solid::icon(solid::MOON)}</span>
            {word}
        </button>
    }
}

/// A language's mark in the switch: a flag, drawn plainly, where one flag
/// stands for the language; the two ways of writing Chinese by the
/// character that names each (简, 繁), which is how readers tell them apart
/// and which belongs to no country.
fn flag(lang: Lang) -> AnyView {
    // three bands side by side, or one above the other
    let upright = |a: &'static str, b: &'static str, c: &'static str| {
        view! {
            <rect width="8" height="16" fill=a></rect>
            <rect width="8" height="16" x="8" fill=b></rect>
            <rect width="8" height="16" x="16" fill=c></rect>
        }
        .into_any()
    };
    let level = |a: &'static str, b: &'static str, c: &'static str| {
        view! {
            <rect width="24" height="5.34" y="0" fill=a></rect>
            <rect width="24" height="5.34" y="5.33" fill=b></rect>
            <rect width="24" height="5.34" y="10.66" fill=c></rect>
        }
        .into_any()
    };
    let drawn = match lang {
        Lang::ZhHans | Lang::ZhHant => {
            let script = if lang == Lang::ZhHans { "简" } else { "繁" };
            return view! { <span class="flag script" aria-hidden="true">{script}</span> }
                .into_any();
        }
        Lang::De => level("#18181b", "#dc2626", "#facc15"),
        Lang::Ru => level("#ffffff", "#1d4ed8", "#dc2626"),
        Lang::Fr => upright("#1e40af", "#ffffff", "#dc2626"),
        Lang::It => upright("#15803d", "#ffffff", "#dc2626"),
        Lang::Es => view! {
            <rect width="24" height="16" fill="#b91c1c"></rect>
            <rect width="24" height="8" y="4" fill="#facc15"></rect>
        }
        .into_any(),
        Lang::PtBr => view! {
            <rect width="24" height="16" fill="#15803d"></rect>
            <path d="M12 2L21.5 8L12 14L2.5 8Z" fill="#facc15"></path>
            <circle cx="12" cy="8" r="3.3" fill="#1e3a8a"></circle>
        }
        .into_any(),
        Lang::Ja => view! {
            <rect width="24" height="16" fill="#ffffff"></rect>
            <circle cx="12" cy="8" r="4.6" fill="#be123c"></circle>
        }
        .into_any(),
        Lang::Ko => view! {
            <rect width="24" height="16" fill="#ffffff"></rect>
            <g transform="rotate(33.7 12 8)">
                <path d="M8 8A4 4 0 0 1 16 8Z" fill="#be123c"></path>
                <path d="M8 8A4 4 0 0 0 16 8Z" fill="#1e3a8a"></path>
            </g>
            <path d="M3.6 4.6L6 1.6M18 14.4L20.4 11.4M18 1.6L20.4 4.6M3.6 11.4L6 14.4" stroke="#18181b" stroke-width="1.5"></path>
        }
        .into_any(),
        Lang::En => view! {
            <rect width="24" height="16" fill="#1e3a8a"></rect>
            <path d="M0 0L24 16M24 0L0 16" stroke="#ffffff" stroke-width="3.2"></path>
            <path d="M0 0L24 16M24 0L0 16" stroke="#dc2626" stroke-width="1.6"></path>
            <path d="M12 0V16M0 8H24" stroke="#ffffff" stroke-width="5.2"></path>
            <path d="M12 0V16M0 8H24" stroke="#dc2626" stroke-width="3"></path>
        }
        .into_any(),
    };
    view! {
        <span class="flag" aria-hidden="true">
            <svg viewBox="0 0 24 16" width="21" height="14" focusable="false">
                {drawn}
                <rect width="24" height="16" fill="none" stroke="rgb(128 128 128 / 0.45)" stroke-width="1.2"></rect>
            </svg>
        </span>
    }
    .into_any()
}

/// A language's two letters on its chip: `EN`, `PT`, `ZH`.
fn chip(lang: Lang) -> String {
    lang.code()
        .chars()
        .take(2)
        .collect::<String>()
        .to_uppercase()
}

/// The language menu: this page in every language the site speaks.
fn languages(page: Page, lang: Lang) -> impl IntoView + use<> {
    let choices = Lang::ALL
        .into_iter()
        .map(|other| {
            let here = other == lang;
            view! {
                <li>
                    <a
                        class="menu-item"
                        href=translated(page, other)
                        hreflang=other.tag()
                        lang=other.tag()
                        aria-current=here.then_some("true")
                    >
                        {flag(other)}
                        <span class="lang-code">{chip(other)}</span>
                        <span class="grow">{other.own_name()}</span>
                        {here.then(|| solid::icon_as(solid::CHECK, "check"))}
                    </a>
                </li>
            }
        })
        .collect_view();
    view! {
        <details class="lang" data-menu="">
            <summary>
                <span class="visually-hidden">{words::lang_label(lang)}": "</span>
                <span class="lang-main">
                    {flag(lang)}
                    <span class="lang-code">{chip(lang)}</span>
                    <span class="lang-name">{lang.own_name()}</span>
                </span>
                <span class="lang-toggle">{solid::icon_as(solid::DOWN, "chevron")}</span>
            </summary>
            <div class="menu">
                <ul>{choices}</ul>
            </div>
        </details>
    }
}

fn bar(page: Page, cx: &Context<'_>) -> impl IntoView + use<> {
    let lang = cx.lang;
    let signing = matches!(
        page,
        Page::SignIn | Page::Register | Page::Reset | Page::Account(_)
    );
    view! {
        <header class="top">
            <div class="top-inner">
                <a class="brand" href=Page::Home.path(lang)>
                    <img src=cx.assets.url("img/icon-192.webp") alt="" width="192" height="192" fetchpriority="low" />
                    <span>{words::site_name(lang)}</span>
                </a>
                <button type="button" class="round-btn menu-btn" popovertarget="nav">
                    <span class="bars" aria-hidden="true"></span>
                    <span class="visually-hidden">{words::nav_menu(lang)}</span>
                </button>
                <nav class="nav" id="nav" popover="auto" aria-label=words::nav_label(lang)>
                    {nav_link(Page::Download, page, lang, words::nav_download(lang))}
                    {nav_link(Page::Gateways, page, lang, words::nav_gateways(lang))}
                    {nav_link(Page::Challenges, page, lang, words::ch_title(lang))}
                    {nav_link(Page::Security, page, lang, words::nav_security(lang))}
                    {nav_link(Page::Faq, page, lang, words::nav_faq(lang))}
                    {nav_link(Page::Contact, page, lang, words::nav_contact(lang))}
                    <a
                        class="nav-account"
                        href=format!("/{}/signin", lang.code())
                        aria-current=signing.then_some("page")
                        data-account=words::nav_account(lang)
                        data-account-href=format!("/{}/account", lang.code())
                    >
                        <span>{words::nav_sign_in(lang)}</span>
                    </a>
                    {theme_switch(lang, true)}
                </nav>
                <div class="top-tools">
                    {theme_switch(lang, false)}
                    {languages(page, lang)}
                </div>
            </div>
        </header>
    }
}

/// The footer's way home: Baylee's BIMI mark and the sunlight wordmark.
fn foot_logo(lang: Lang, assets: &Assets) -> impl IntoView + use<> {
    let wordmark = assets.url("img/logo-480.webp");
    view! {
        <a class="foot-logo-link" href=Page::Home.path(lang) aria-label=words::site_name(lang)>
            // The BIMI mark, the very file mail providers fetch
            // (`/bimi/baylee.svg`), shown on every page: a mark
            // certificate asks that it was public on the domain
            // for a year. Not lazy, so that archives take it.
            <img
                class="foot-mark"
                src="/bimi/baylee.svg"
                width="72"
                height="72"
                alt="Baylee"
                decoding="async"
            />
            <img
                class="foot-logo"
                src=wordmark
                width="480"
                height="200"
                alt=""
                loading="lazy"
                decoding="async"
            />
        </a>
    }
}

fn footer(page: Page, cx: &Context<'_>) -> impl IntoView + use<> {
    let lang = cx.lang;
    let assets = cx.assets;
    let policy = policy_url(lang);
    let fan = rich(words::fan_content(lang), move |tag| {
        (tag == "policy").then(|| policy.to_owned())
    });
    let colophon = rich(words::foot_colophon(lang), |tag| {
        (tag == "notices").then(|| "/notices.txt".to_owned())
    });
    // in a language a machine translated, the footer says so
    let machine = lang.machine_translated().then(|| {
        let contact = format!("{}#contact", Page::Home.path(lang));
        let note = rich(words::machine_note(lang), move |tag| {
            (tag == "link").then(|| contact.clone())
        });
        view! { <p class="machine-note">{note}</p> }
    });
    let link = |target: Page, label: &'static str| {
        view! { <li><a href=target.path(lang) aria-current=(target == page).then_some("page")>{label}</a></li> }
    };
    let away = |href: String, label: &'static str| {
        view! { <li><a href=href target="_blank" rel="noopener noreferrer">{label}</a></li> }
    };
    view! {
        <footer class="foot">
            <div class="foot-inner">
                <div class="foot-top">
                    <div class="foot-brand">
                        {foot_logo(lang, assets)}
                        <p class="dedication">{words::foot_dedication(lang)}</p>
                        <p class="foot-summary">{words::foot_summary(lang)}</p>
                        <div class="foot-social">
                            <a class="social-pill" href=DISCORD target="_blank" rel="noopener noreferrer">{solid::icon(solid::DISCORD)}<span>"Discord"</span></a>
                            <a class="social-pill" href=WHATSAPP target="_blank" rel="noopener noreferrer">{solid::icon(solid::WHATSAPP)}<span>"WhatsApp"</span></a>
                            <a class="social-pill" href=REPOSITORY target="_blank" rel="noopener noreferrer">{solid::icon(solid::GITHUB)}<span>"GitHub"</span></a>
                        </div>
                    </div>
                    <nav class="foot-map" aria-label=words::foot_label(lang)>
                        <section aria-labelledby="foot-play">
                            <h2 class="foot-head" id="foot-play">{words::foot_play(lang)}</h2>
                            <ul class="foot-list">
                                {link(Page::Download, words::nav_download(lang))}
                                {link(Page::Gateways, words::nav_gateways(lang))}
                                {link(Page::Challenges, words::ch_title(lang))}
                                {link(Page::Faq, words::nav_faq(lang))}
                                {link(Page::Security, words::nav_security(lang))}
                                {away(DISCORD.to_owned(), words::foot_discord(lang))}
                                {away(WHATSAPP.to_owned(), words::hero_whatsapp(lang))}
                                {link(Page::Contact, words::nav_contact(lang))}
                            </ul>
                        </section>
                        <section aria-labelledby="foot-project">
                            <h2 class="foot-head" id="foot-project">{words::foot_project(lang)}</h2>
                            <ul class="foot-list">
                                {away(REPOSITORY.to_owned(), words::foot_source(lang))}
                                {away(CENTRAL_REPOSITORY.to_owned(), words::foot_central(lang))}
                                {away(format!("{CENTRAL_REPOSITORY}/tree/main/crates/central-web"), words::foot_central_source(lang))}
                                {away(format!("{REPOSITORY}/releases"), words::foot_releases(lang))}
                                {away(format!("{REPOSITORY}/issues"), words::foot_issues(lang))}
                            </ul>
                        </section>
                        <section aria-labelledby="foot-legal">
                            <h2 class="foot-head" id="foot-legal">{words::foot_legal(lang)}</h2>
                            <ul class="foot-list">
                                {link(Page::Terms, words::title_terms(lang))}
                                {link(Page::Privacy, words::title_privacy(lang))}
                                {link(Page::Impressum, words::title_impressum(lang))}
                                {link(Page::FanContent, words::title_fan_content(lang))}
                            </ul>
                        </section>
                    </nav>
                </div>
                <div class="foot-bottom">
                    <div class="foot-notices">
                        <p>{fan}</p>
                        {machine}
                    </div>
                    <div class="foot-colophon">
                        <p class="colophon">{colophon}</p>
                        <ul class="foot-badges">
                            <li>"AGPL-3.0"</li>
                            <li>"Rust & Bevy"</li>
                        </ul>
                    </div>
                </div>
            </div>
        </footer>
    }
}

/// The update offer: hidden until `pwa.js` finds a new version waiting.
/// The polite status beside it says so once.
fn update(lang: Lang) -> impl IntoView + use<> {
    view! {
        <div id="update" class="update" hidden>
            <p>{words::update_ready(lang)}</p>
            <p class="update-unsent" hidden>{words::update_unsent(lang)}</p>
            <button type="button" class="button primary" data-update>{words::update_reload(lang)}</button>
        </div>
        <p id="update-said" class="visually-hidden" role="status" data-said=words::update_ready(lang)></p>
    }
}
