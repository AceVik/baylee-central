//! The frame of every page: the document head (title, description,
//! canonical address, languages, sharing cards, structured data), the bar,
//! the footer with the Fan Content notice, and the bottom bar on phones.

use leptos::prelude::*;
use leptos::tachys::html::attribute::custom::custom_attribute;
use serde_json::{Value, json};

use super::icons;
use super::{Context, Page};
use crate::rich::{icon, rich};
use crate::words::{self, Lang};

/// Where Baylee's code lives.
pub const REPOSITORY: &str = "https://github.com/AceVik/baylee";
/// Where Baylee Central's code lives.
pub const CENTRAL_REPOSITORY: &str = "https://github.com/AceVik/baylee-central";
/// Baylee's community on Discord.
pub const DISCORD: &str = "https://discord.gg/4pP3uAwgY";

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

/// The Fan Content Policy's address in a language (Wizards' own pages).
pub fn policy_url(lang: Lang) -> &'static str {
    match lang {
        Lang::En => "https://company.wizards.com/en/legal/fancontentpolicy",
        Lang::De => "https://company.wizards.com/de/legal/fancontentpolicy",
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
            .map(|other| (other.code(), format!("{origin}{}", page.path(other))))
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
    let islands = meta.islands.then(|| {
        view! { <script type="module" src=assets.url("islands.js") integrity=assets.integrity("islands.js") fetchpriority="low"></script> }
    });
    let other = Lang::ALL
        .into_iter()
        .find(|l| *l != lang)
        .unwrap_or(Lang::En);
    let font = |name: &str| {
        view! { <link rel="preload" href=assets.url(name) r#as="font" type="font/woff2" crossorigin="anonymous" /> }
    };
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
            <meta name="theme-color" media="(prefers-color-scheme: light)" content="#f7f3ea" />
            <meta name="theme-color" media="(prefers-color-scheme: dark)" content="#0f1b2d" />
            <meta name="referrer" content="no-referrer" />
            <meta name="tdm-reservation" content="1" />
            {font("fonts/AlegreyaSans-Regular.woff2")}
            {font("fonts/Faustina.woff2")}
            <link rel="stylesheet" href=assets.url("site.css") integrity=assets.integrity("site.css") />
            <script src=assets.url("boot.js") integrity=assets.integrity("boot.js")></script>
            <script type="module" src=assets.url("pwa.js") integrity=assets.integrity("pwa.js")></script>
            {islands}
            <link rel="icon" href="/favicon.ico" sizes="32x32" />
            <link rel="icon" href=assets.url("img/icon.svg") type="image/svg+xml" />
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
            <meta {..custom_attribute("property", "og:locale:alternate")} content=other.locale() />
            <meta name="twitter:card" content="summary_large_image" />
            {verification}
            {structured}
        </head>
    }
}

/// The whole document around a page's `body`.
pub fn document(page: Page, cx: &Context<'_>, meta: &Meta, body: AnyView) -> impl IntoView + use<> {
    let lang = cx.lang;
    view! {
        <html lang=lang.code() dir="ltr">
            {head(page, cx, meta)}
            <body>
                <a class="skip" href="#main">{words::skip(lang)}</a>
                {bar(page, cx)}
                <div class="page">
                    <main id="main" tabindex="-1">
                        {crumbs(lang, &meta.trail)}
                        {body}
                    </main>
                    {footer(page, cx)}
                </div>
                {dock(page, cx)}
                {update(lang)}
            </body>
        </html>
    }
}

/// The address of the same page in `lang`.
fn translated(page: Page, lang: Lang) -> String {
    match page {
        Page::NotFound | Page::Gone | Page::Offline => Page::Home.path(lang),
        _ => page.path(lang),
    }
}

/// A navigation link that says when it is the current page.
fn nav_link(
    page: Page,
    current: Page,
    lang: Lang,
    label: String,
    glyph: &'static str,
) -> impl IntoView + use<> {
    let here = page == current;
    view! {
        <a href=page.path(lang) aria-current=here.then_some("page")>
            {icon(glyph)}
            <span>{label}</span>
        </a>
    }
}

fn theme_switch(lang: Lang) -> impl IntoView + use<> {
    view! {
        <button
            type="button"
            class="tool theme"
            data-theme-switch=""
            data-system=words::theme_system(lang)
            data-light=words::theme_light(lang)
            data-dark=words::theme_dark(lang)
            aria-label=words::theme_system(lang)
            hidden=true
        >
            <span class="when-system">{icon(icons::SUN_MOON)}</span>
            <span class="when-light">{icon(icons::SUN)}</span>
            <span class="when-dark">{icon(icons::MOON)}</span>
        </button>
    }
}

fn bar(page: Page, cx: &Context<'_>) -> impl IntoView + use<> {
    let lang = cx.lang;
    let other = Lang::ALL
        .into_iter()
        .find(|l| *l != lang)
        .unwrap_or(Lang::En);
    view! {
        <header class="bar">
            <div class="bar-in">
                <a class="brand" href=Page::Home.path(lang)>
                    <img src=cx.assets.url("img/icon.svg") alt="" width="40" height="40" />
                    <span>{words::site_name(lang)}</span>
                </a>
                <nav class="nav" aria-label=words::nav_label(lang)>
                    <ul>
                        <li>{nav_link(Page::Download, page, lang, words::nav_download(lang).into(), icons::DOWNLOAD)}</li>
                        <li>{nav_link(Page::Gateways, page, lang, words::nav_gateways(lang).into(), icons::SERVER)}</li>
                        <li>{nav_link(Page::Faq, page, lang, words::nav_faq(lang).into(), icons::CIRCLE_HELP)}</li>
                        <li>{nav_link(Page::Contact, page, lang, words::nav_contact(lang).into(), icons::MAIL)}</li>
                    </ul>
                </nav>
                <div class="tools">
                    <a class="tool lang" href=translated(page, other) hreflang=other.code() lang=other.code()>
                        {icon(icons::LANGUAGES)}
                        <span>{words::lang_switch(lang)}</span>
                    </a>
                    {theme_switch(lang)}
                    <button type="button" class="tool menu-tool" popovertarget="more" aria-haspopup="dialog" aria-label=words::nav_more(lang)>
                        {icon(icons::MENU)}
                    </button>
                    <a class="button quiet sign-in" href=format!("/{}/signin", lang.code()) data-account=words::nav_account(lang) data-account-href=format!("/{}/account", lang.code())>
                        {icon(icons::LOG_IN)}
                        <span>{words::nav_sign_in(lang)}</span>
                    </a>
                </div>
            </div>
        </header>
    }
}

fn footer(page: Page, cx: &Context<'_>) -> impl IntoView + use<> {
    let lang = cx.lang;
    let policy = policy_url(lang);
    let fan = rich(words::fan_content(lang), move |tag| {
        (tag == "policy").then(|| policy.to_owned())
    });
    let colophon = rich(words::foot_colophon(lang), |tag| {
        (tag == "notices").then(|| "/notices.txt".to_owned())
    });
    let link = |target: Page, label: &'static str| {
        view! { <li><a href=target.path(lang) aria-current=(target == page).then_some("page")>{label}</a></li> }
    };
    let languages = Lang::ALL
        .into_iter()
        .map(|other| {
            let here = other == lang;
            view! {
                <li>
                    <a href=translated(page, other) hreflang=other.code() lang=other.code() aria-current=here.then_some("true")>
                        {other.own_name()}
                    </a>
                </li>
            }
        })
        .collect_view();
    view! {
        <footer class="foot" id="site-menu">
            <div class="foot-in">
                <nav class="foot-nav" aria-label=words::foot_label(lang)>
                    <div>
                        <h2>{words::foot_play(lang)}</h2>
                        <ul>
                            {link(Page::Download, words::nav_download(lang))}
                            {link(Page::Gateways, words::nav_gateways(lang))}
                            <li><a href=format!("{REPOSITORY}/releases") rel="noopener noreferrer">{words::foot_releases(lang)}</a></li>
                            <li><a href=REPOSITORY rel="noopener noreferrer">{words::foot_repository(lang)}</a></li>
                        </ul>
                    </div>
                    <div>
                        <h2>{words::foot_about(lang)}</h2>
                        <ul>
                            {link(Page::Faq, words::nav_faq(lang))}
                            {link(Page::Contact, words::nav_contact(lang))}
                            <li><a href=format!("{REPOSITORY}/issues") rel="noopener noreferrer">{words::foot_issues(lang)}</a></li>
                            <li><a href=DISCORD rel="noopener noreferrer">{words::foot_discord(lang)}</a></li>
                            <li><a href=CENTRAL_REPOSITORY rel="noopener noreferrer">{words::foot_central_source(lang)}</a></li>
                        </ul>
                    </div>
                    <div>
                        <h2>{words::foot_legal(lang)}</h2>
                        <ul>
                            {link(Page::Impressum, words::title_impressum(lang))}
                            {link(Page::Privacy, words::title_privacy(lang))}
                            {link(Page::Terms, words::title_terms(lang))}
                            {link(Page::FanContent, words::title_fan_content(lang))}
                        </ul>
                    </div>
                    <div>
                        <h2>{words::lang_label(lang)}</h2>
                        <ul>{languages}</ul>
                    </div>
                </nav>
                <p class="fan-content">{fan}</p>
                <p class="dedication">{words::foot_dedication(lang)}</p>
                <p class="colophon">{colophon}</p>
            </div>
        </footer>
    }
}

/// The bottom bar on phones, within thumb reach; "More" opens a sheet
/// (`popover`, no script needed) with the rest.
fn dock(page: Page, cx: &Context<'_>) -> impl IntoView + use<> {
    let lang = cx.lang;
    let other = Lang::ALL
        .into_iter()
        .find(|l| *l != lang)
        .unwrap_or(Lang::En);
    view! {
        <nav class="dock" aria-label=words::nav_label(lang)>
            <ul>
                <li>{nav_link(Page::Home, page, lang, words::nav_home(lang).into(), icons::HOUSE)}</li>
                <li>{nav_link(Page::Download, page, lang, words::nav_download(lang).into(), icons::DOWNLOAD)}</li>
                <li>{nav_link(Page::Gateways, page, lang, words::nav_gateways(lang).into(), icons::SERVER)}</li>
                <li>
                    <button type="button" popovertarget="more" aria-haspopup="dialog">
                        {icon(icons::MENU)}
                        <span>{words::nav_more(lang)}</span>
                    </button>
                </li>
            </ul>
        </nav>
        <div id="more" class="sheet" popover="auto" role="dialog" aria-label=words::nav_more(lang)>
            <div class="sheet-head">
                <button type="button" class="tool" popovertarget="more" popovertargetaction="hide" aria-label=words::nav_close(lang)>
                    {icon(icons::X)}
                </button>
            </div>
            <ul class="sheet-list">
                <li>{nav_link(Page::Faq, page, lang, words::nav_faq(lang).into(), icons::CIRCLE_HELP)}</li>
                <li>{nav_link(Page::Contact, page, lang, words::nav_contact(lang).into(), icons::MAIL)}</li>
                <li>
                    <a href=format!("/{}/signin", lang.code())>
                        {icon(icons::LOG_IN)}
                        <span>{words::nav_sign_in(lang)}</span>
                    </a>
                </li>
                <li>
                    <a href=translated(page, other) hreflang=other.code() lang=other.code()>
                        {icon(icons::LANGUAGES)}
                        <span>{words::lang_switch(lang)}</span>
                    </a>
                </li>
            </ul>
            <ul class="sheet-legal">
                <li><a href=Page::Impressum.path(lang)>{words::title_impressum(lang)}</a></li>
                <li><a href=Page::Privacy.path(lang)>{words::title_privacy(lang)}</a></li>
                <li><a href=Page::Terms.path(lang)>{words::title_terms(lang)}</a></li>
                <li><a href=Page::FanContent.path(lang)>{words::title_fan_content(lang)}</a></li>
            </ul>
        </div>
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
