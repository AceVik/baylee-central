//! The start page: what Baylee is, the download for this machine, the
//! project's state, the card numbers, the table, the gateways, the open
//! source promise, and Baylee's album behind the big logo.

use std::fmt::Write as _;

use leptos::prelude::*;
use leptos::tachys::html::attribute::custom::custom_attribute;
use serde_json::json;

use super::download::download_block;
use super::{Built, section_head};
use crate::data::{CardStatus, Gateway};
use crate::format;
use crate::islands::ContactForm;
use crate::rich::{icon, paragraphs, plain, rich};
use crate::site::icons;
use crate::site::shell::{CENTRAL_REPOSITORY, DISCORD, Meta, REPOSITORY, WHATSAPP, policy_url};
use crate::site::{Context, Page};
use crate::solid;
use crate::words::{self, Lang};

/// The client's screenshots (`npm run content` of the old site, now
/// `cargo xtask site --content`): name, caption, alt text.
type Shot = (
    &'static str,
    fn(Lang) -> Vec<crate::words::Piece>,
    fn(Lang) -> &'static str,
);

const SHOTS: [Shot; 6] = [
    ("table-duel", words::shot_duel, words::shot_duel_alt),
    ("game-log", words::shot_log, words::shot_log_alt),
    ("table-ring", words::shot_ring, words::shot_ring_alt),
    ("lobby", words::shot_lobby, words::shot_lobby_alt),
    ("room", words::shot_room, words::shot_room_alt),
    ("deck-builder", words::shot_builder, words::shot_builder_alt),
];

/// The album's photographs: file number, caption, width, height (fixed
/// files, family-approved; `web/assets-sunlight.md`).
/// A photograph: its number, its caption, its width and height.
type Photo = (u8, fn(Lang) -> &'static str, u32, u32);

const PHOTOS: [Photo; 10] = [
    (1, words::baylee_photo1, 787, 1400),
    (2, words::baylee_photo2, 1400, 787),
    (3, words::baylee_photo3, 1400, 787),
    (4, words::baylee_photo4, 787, 1400),
    (5, words::baylee_photo5, 787, 1400),
    (6, words::baylee_photo6, 960, 540),
    (7, words::baylee_photo7, 960, 540),
    (8, words::baylee_photo8, 960, 540),
    (9, words::baylee_photo9, 540, 960),
    (10, words::baylee_photo10, 960, 540),
];

type Chapter = (fn(Lang) -> &'static str, fn(Lang) -> &'static str);

const CHAPTERS: [Chapter; 8] = [
    (words::baylee_chapter1_title, words::baylee_chapter1_body),
    (words::baylee_chapter2_title, words::baylee_chapter2_body),
    (words::baylee_chapter3_title, words::baylee_chapter3_body),
    (words::baylee_chapter4_title, words::baylee_chapter4_body),
    (words::baylee_chapter5_title, words::baylee_chapter5_body),
    (words::baylee_chapter6_title, words::baylee_chapter6_body),
    (words::baylee_chapter7_title, words::baylee_chapter7_body),
    (words::baylee_chapter8_title, words::baylee_chapter8_body),
];

pub fn page(cx: &Context<'_>) -> Built {
    let lang = cx.lang;
    let mut meta = Meta::new(
        words::title_home(lang),
        words::desc_home(lang),
        "img/og.jpg",
    );
    meta.data.push(json!({
        "@context": "https://schema.org",
        "@type": "WebSite",
        "name": "Baylee",
        "url": format!("{}{}", cx.origin, Page::Home.path(lang)),
        "inLanguage": lang.code(),
    }));
    meta.data.push(json!({
        "@context": "https://schema.org",
        "@type": "Organization",
        "name": "Baylee",
        "url": format!("{}/", cx.origin),
        "logo": format!("{}{}", cx.origin, cx.assets.url("img/icon-192.webp")),
        "sameAs": [REPOSITORY, CENTRAL_REPOSITORY],
    }));
    meta.data.push(super::download::software(cx));
    meta.islands = true;
    let body = view! {
        {strip(cx)}
        {hero(cx)}
        {gateways(lang, cx.data.gateways.as_deref())}
        {screens(cx)}
        {about(lang)}
        {cards(lang, cx.data.cards.as_ref())}
        {contact(lang)}
        {source(cx)}
        {album(cx)}
    }
    .into_any();
    (meta, body)
}

fn hero(cx: &Context<'_>) -> impl IntoView + use<> {
    let lang = cx.lang;
    let assets = cx.assets;
    let policy = policy_url(lang);
    let fan = rich(words::fan_content(lang), move |tag| {
        (tag == "policy").then(|| policy.to_owned())
    });
    let world = |kind: &str| {
        format!(
            "{} 640w, {} 960w, {} 1600w",
            assets.url(&format!("img/world-640.{kind}")),
            assets.url(&format!("img/world-960.{kind}")),
            assets.url(&format!("img/world-1600.{kind}"))
        )
    };
    view! {
        <section class="hero" aria-labelledby="hero-title">
            <div class="sky" aria-hidden="true">
                <picture class="sky-painting">
                    <source type="image/avif" srcset=world("avif") sizes="100vw" />
                    <img
                        src=assets.url("img/world-1600.webp")
                        srcset=world("webp")
                        sizes="100vw"
                        width="1600"
                        height="900"
                        alt=""
                        fetchpriority="high"
                    />
                </picture>
                <div class="sky-beams"></div>
                <div class="sky-motes"></div>
                <div class="sky-light"></div>
                <div class="sky-mist"></div>
                <div class="sky-veil"></div>
            </div>
            <div class="hero-inner">
                <button type="button" class="memorial-open" commandfor="album" command="show-modal" aria-haspopup="dialog">
                    <span class="halo" aria-hidden="true"></span>
                    <img
                        class="hero-logo"
                        src=assets.url("img/logo-960.webp")
                        srcset=format!("{} 480w, {} 960w", assets.url("img/logo-480.webp"), assets.url("img/logo-960.webp"))
                        sizes="(min-width: 30rem) 19.5rem, 72vw"
                        width="960"
                        height="400"
                        alt=""
                        fetchpriority="low"
                    />
                    <span class="visually-hidden">{words::hero_memorial(lang)}</span>
                </button>
                <ul class="hero-badges" aria-label=words::hero_badges(lang)>
                    <li class="hero-badge"><span class="dot ok live" aria-hidden="true"></span>{words::hero_badge_beta(lang)}</li>
                    <li class="hero-badge">{icon(icons::HEART)}{words::hero_badge_free(lang)}</li>
                    <li class="hero-badge">{solid::icon(solid::GITHUB)}{words::hero_badge_open(lang)}</li>
                    <li class="hero-badge">{icon(icons::SPARKLES)}{words::hero_badge_fan(lang)}</li>
                </ul>
                <h1 id="hero-title">{plain(words::hero_title(lang))}</h1>
                <p class="lead">{words::hero_lead(lang)}</p>
                {download_block(cx, false)}
                <div class="hero-actions">
                    <a class="button discord" href=DISCORD target="_blank" rel="noopener noreferrer">
                        <span class="beacon" aria-hidden="true"></span>
                        {solid::icon_as(solid::DISCORD, "wave")}
                        <span>{words::hero_discord(lang)}</span>
                    </a>
                    <a class="button whatsapp" href=WHATSAPP target="_blank" rel="noopener noreferrer">
                        <span class="beacon" aria-hidden="true"></span>
                        {solid::icon(solid::WHATSAPP)}
                        <span>{words::hero_whatsapp(lang)}</span>
                    </a>
                    <a class="button github" href=REPOSITORY target="_blank" rel="noopener noreferrer">
                        {solid::icon(solid::GITHUB)}
                        <span>{words::hero_github(lang)}</span>
                    </a>
                </div>
            </div>
            // Wizards' Fan Content notice, in the hero where a visitor
            // sees it (owner, 11.11.2026); the footer carries it on every page
            <p class="hero-fan">{solid::icon(solid::SHIELD)}<span>{fan}</span></p>
        </section>
    }
}

/// The strip under the bar: the project's state on the left (the latest
/// build, CI on main, the last commit), its numbers on the right (playable
/// cards, the repository's stars, the gateways online and not). Only what a
/// source said is shown: a number nobody reported is left out, and while
/// GitHub or the directory has not answered, the strip links there instead.
fn strip(cx: &Context<'_>) -> impl IntoView + use<> {
    let lang = cx.lang;
    // `wide`: a stat a phone has no room for. `quiet`: its glyph and its
    // value say what it is; the label is for a screen reader and the
    // pointer's tooltip.
    let stat = |glyph: AnyView, label: &'static str, value: AnyView, wide: bool, quiet: bool| {
        view! {
            <div class="strip-stat" class:strip-opt=wide title=quiet.then_some(label)>
                <dt>{glyph}<span class:visually-hidden=quiet>{label}</span></dt>
                <dd>{value}</dd>
            </div>
        }
    };
    let build = match cx.data.release.as_ref() {
        Some(release) => view! {
            <a href=release.page.clone() target="_blank" rel="noopener noreferrer"><code translate="no">{release.tag.clone()}</code></a>
            <span class="small when">{format::date(lang, &release.date)}</span>
        }
        .into_any(),
        None => view! {
            <a href=format!("{REPOSITORY}/releases") target="_blank" rel="noopener noreferrer">{words::strip_releases(lang)}</a>
        }
        .into_any(),
    };
    let actions = format!("{REPOSITORY}/actions/workflows/ci.yml");
    let ci = match cx.data.repo.ci {
        Some(ok) => {
            let (class, word) = if ok {
                ("dot ok", words::ci_passing(lang))
            } else {
                ("dot bad", words::ci_failing(lang))
            };
            view! { <a href=actions target="_blank" rel="noopener noreferrer"><span class=class aria-hidden="true"></span>{word}</a> }
                .into_any()
        }
        None => {
            view! { <a href=actions target="_blank" rel="noopener noreferrer">{words::strip_on_github(lang)}</a> }
                .into_any()
        }
    };
    let commit = match cx.data.repo.commit.clone() {
        Some((id, date)) => view! {
            <a href=format!("{REPOSITORY}/commit/{id}") target="_blank" rel="noopener noreferrer"><code translate="no">{id.clone()}</code></a>
            <span class="small when">{format::date(lang, &date)}</span>
        }
        .into_any(),
        None => view! {
            <a href=format!("{REPOSITORY}/commits/main") target="_blank" rel="noopener noreferrer">{words::strip_on_github(lang)}</a>
        }
        .into_any(),
    };
    let cards = cx.data.cards.as_ref().map(|status| {
        let s = status.stats;
        let playable = s
            .implemented
            .saturating_add(s.generated)
            .saturating_add(s.partial);
        stat(
            icon(icons::LAYERS).into_any(),
            words::strip_cards(lang),
            view! {
                <a href="#cards"><span class="num">{words::number(lang, playable)}</span><span class="small">{words::strip_cards_unit(lang)}</span></a>
            }
            .into_any(),
            false,
            true,
        )
    });
    let stars = cx.data.repo.stars.map(|stars| {
        stat(
            solid::icon(solid::STAR).into_any(),
            words::strip_stars(lang),
            view! {
                <a href=REPOSITORY target="_blank" rel="noopener noreferrer"><span class="num">{words::number(lang, stars)}</span></a>
            }
            .into_any(),
            true,
            true,
        )
    });
    let gateways = match cx.data.gateways.as_deref() {
        Some(all) => {
            let online = all.iter().filter(|g| g.online).count();
            let offline = all.len() - online;
            let count = |n: usize| words::number(lang, u64::try_from(n).unwrap_or(u64::MAX));
            let dot = if online > 0 { "dot ok live" } else { "dot off" };
            view! {
                <a href="#gateways">
                    <span class="stat-pill num">
                        <span class="stat-part"><span class=dot aria-hidden="true"></span>{count(online)}" "{words::strip_online(lang)}</span>
                        <span class="stat-part"><span class="dot off" aria-hidden="true"></span>{count(offline)}" "{words::strip_offline(lang)}</span>
                    </span>
                </a>
            }
            .into_any()
        }
        None => view! { <a href=Page::Gateways.path(lang)>{words::strip_directory(lang)}</a> }
            .into_any(),
    };
    view! {
        <section class="strip" aria-label=words::status_label(lang)>
            <div class="strip-inner">
                <dl class="strip-group">
                    {stat(icon(icons::PACKAGE).into_any(), words::strip_build(lang), build, false, false)}
                    {stat(icon(icons::CIRCLE_CHECK).into_any(), words::strip_ci(lang), ci, false, false)}
                    {stat(icon(icons::HISTORY).into_any(), words::strip_commit(lang), commit, true, false)}
                </dl>
                <dl class="strip-group strip-right">
                    {cards}
                    {stars}
                    {stat(icon(icons::SERVER).into_any(), words::strip_gateways(lang), gateways, false, true)}
                </dl>
            </div>
        </section>
    }
}

/// One card number, large, with what it means behind a small button.
fn stat(
    lang: Lang,
    id: &'static str,
    value: u64,
    label: &'static str,
    help: &'static str,
) -> impl IntoView + use<> {
    let tip = format!("tip-{id}");
    let what = format!("{} – {label}", words::cards_help(lang));
    view! {
        <div class=format!("number glass reveal number-{id}")>
            <dt class="num" data-count=value.to_string()>{words::number(lang, value)}</dt>
            <dd>
                {label}
                <button type="button" class="tip" popovertarget=tip.clone() {..custom_attribute("interestfor", tip.clone())} aria-label=what>
                    {icon(icons::INFO)}
                </button>
                <span id=tip class="tip-body" popover="auto" role="note">{help}</span>
            </dd>
        </div>
    }
}

fn cards(lang: Lang, status: Option<&CardStatus>) -> impl IntoView + use<> {
    let body = match status {
        Some(status) => {
            let s = status.stats;
            let playable = s
                .implemented
                .saturating_add(s.generated)
                .saturating_add(s.partial);
            view! {
                <dl class="numbers stagger">
                    {stat(lang, "implemented", s.implemented, words::cards_implemented(lang), words::cards_implemented_help(lang))}
                    {stat(lang, "generated", s.generated, words::cards_generated(lang), words::cards_generated_help(lang))}
                    {stat(lang, "partial", s.partial, words::cards_partial(lang), words::cards_partial_help(lang))}
                    {stat(lang, "total", s.total, words::cards_total(lang), words::cards_total_help(lang))}
                </dl>
                <div class="share glass reveal">
                    <p>{words::cards_share(lang, &words::number(lang, playable), &words::number(lang, s.total))}</p>
                    <progress class="share-bar" max=s.total.to_string() value=playable.min(s.total).to_string() aria-hidden="true"></progress>
                    <p class="small">{words::cards_as_of(lang, &status.tag, &format::date(lang, &status.date))}</p>
                </div>
            }
            .into_any()
        }
        None => view! { <p class="notice">{words::cards_missing(lang)}</p> }.into_any(),
    };
    view! {
        <section class="band" id="cards" aria-labelledby="cards-title">
            <div class="wrap">
                {section_head(words::cards_eyebrow(lang), words::cards_title(lang), "cards-title")}
                <p class="lead">{words::cards_lead(lang)}</p>
                {body}
            </div>
        </section>
    }
}

/// The first piece in bold of a caption: the screenshot's title.
fn title_of(caption: &[crate::words::Piece]) -> String {
    caption
        .iter()
        .find(|piece| piece.tag.as_deref() == Some("b"))
        .or_else(|| caption.first())
        .map(|piece| piece.text.trim_end_matches(':').to_owned())
        .unwrap_or_default()
}

/// The client's screenshots, each opening large in `#shot-viewer` (the
/// arrow keys step). `home.js` draws each picture once its card comes
/// near: the still (WebP), and its recording (a short AVIF that loops, a
/// few hundred kB) for those who did not ask for less motion. On a phone
/// the cards start a screen below the first, inside the browser's own
/// lazy-loading reach, and six stills taken then delayed the painting.
/// Without script the `<noscript>` picture shows the still.
fn screens(cx: &Context<'_>) -> Option<impl IntoView + use<>> {
    let lang = cx.lang;
    let assets = cx.assets;
    if !assets.has("media/shot-table-duel-1200.webp") {
        return None;
    }
    let sizes = "(max-width: 600px) 92vw, (max-width: 1000px) 46vw, 380px";
    let figures = SHOTS
        .iter()
        .enumerate()
        .map(|(at, (name, caption, alt))| {
            let still = |width: u32| assets.url(&format!("media/shot-{name}-{width}.webp"));
            let moving = |width: u32| assets.url(&format!("media/shot-{name}-{width}.avif"));
            let label = words::shots_show_large(lang, &title_of(&caption(lang)));
            view! {
                <li class="glass reveal">
                    <figure>
                        <button type="button" class="shot-open" commandfor="shot-viewer" command="show-modal" aria-haspopup="dialog"
                            aria-label=label data-shot=at.to_string() data-alt=alt(lang) data-sizes=sizes
                            data-still=still(1200) data-stills=format!("{} 640w, {} 1200w", still(640), still(1200))
                            data-moving=moving(1200) data-movings=format!("{} 640w, {} 1200w", moving(640), moving(1200))>
                            <noscript>
                                <picture>
                                    <img src=still(1200) srcset=format!("{} 640w, {} 1200w", still(640), still(1200)) sizes=sizes width="1200" height="703" alt=alt(lang) loading="lazy" decoding="async" />
                                </picture>
                            </noscript>
                            <span class="shot-zoom" aria-hidden="true">{solid::icon(solid::EXTERNAL)}</span>
                        </button>
                        <figcaption>{plain(caption(lang))}</figcaption>
                    </figure>
                </li>
            }
        })
        .collect_view();
    Some(view! {
        <section class="band" id="screens" aria-labelledby="screens-title">
            <div class="wrap">
                {section_head(words::shots_eyebrow(lang), words::shots_title(lang), "screens-title")}
                <p class="lead reveal">{words::shots_lead(lang)}</p>
                <ul class="shots stagger">{figures}</ul>
                <p class="small shots-credit">{words::shots_credit(lang)}</p>
            </div>
            <dialog id="shot-viewer" class="lightbox" aria-label=words::shots_dialog(lang)>
                <picture data-shot-picture="">
                    <img src=assets.url("media/shot-table-duel-1200.webp") width="1200" height="703" alt="" loading="lazy" decoding="async" data-shot-image="" />
                </picture>
                <p class="lightbox-cap">
                    <span data-shot-caption=""></span>
                    <span class="lightbox-credit">{words::shots_credit(lang)}</span>
                </p>
                <button type="button" class="round-btn lightbox-prev" data-shot-step="-1" aria-label=words::shots_previous(lang)>{solid::icon(solid::PREV)}</button>
                <button type="button" class="round-btn lightbox-next" data-shot-step="1" aria-label=words::shots_next(lang)>{solid::icon(solid::NEXT)}</button>
                <form method="dialog">
                    <button type="submit" class="round-btn lightbox-close" aria-label=words::shots_close(lang)>{solid::icon(solid::CLOSE)}</button>
                </form>
            </dialog>
        </section>
    })
}

/// What Baylee does: nine things, an icon and two messages each.
type Feature = (
    &'static str,
    fn(Lang) -> &'static str,
    fn(Lang) -> &'static str,
);

const FEATURES: [Feature; 9] = [
    (
        solid::EYE_OFF,
        words::feature_hidden_title,
        words::feature_hidden,
    ),
    (
        solid::PEOPLE,
        words::feature_seats_title,
        words::feature_seats,
    ),
    (
        solid::CROWN,
        words::feature_commander_title,
        words::feature_commander,
    ),
    (solid::CHIP, words::feature_ai_title, words::feature_ai),
    (
        solid::GRID,
        words::feature_process_title,
        words::feature_process,
    ),
    (
        solid::GLOBE,
        words::feature_languages_title,
        words::feature_languages,
    ),
    (solid::EYE, words::feature_watch_title, words::feature_watch),
    (
        solid::WIFI,
        words::feature_connection_title,
        words::feature_connection,
    ),
    (
        solid::SLIDERS,
        words::feature_tables_title,
        words::feature_tables,
    ),
];

/// The language models a seat can take, by their makers' names.
const PROVIDERS: [&str; 5] = [
    "Google Gemini",
    "OpenAI ChatGPT",
    "Anthropic Claude",
    "DeepSeek",
    "LM Studio",
];

fn feature_icon(path: &'static str) -> impl IntoView + use<> {
    view! { <span class="feat-icon">{solid::icon(path)}</span> }
}

fn about(lang: Lang) -> impl IntoView + use<> {
    let features = FEATURES
        .iter()
        .map(|(glyph, title, text)| {
            view! {
                <li class="glass reveal">
                    <div class="feat-head">{feature_icon(glyph)}<h3>{title(lang)}</h3></div>
                    <p>{text(lang)}</p>
                </li>
            }
        })
        .collect_view();
    view! {
        <section class="band" id="about" aria-labelledby="about-title">
            <div class="wrap">
                {section_head(words::about_eyebrow(lang), words::about_title(lang), "about-title")}
                <div class="two-col about-arch-grid">
                    <div class="about-arch-card glass reveal">
                        <div class="about-arch-head">{feature_icon(solid::SHIELD)}<h3>{words::about_engine_title(lang)}</h3></div>
                        <p>{words::about_engine(lang)}</p>
                        <p>{words::about_deterministic(lang)}</p>
                    </div>
                    <div class="about-arch-card glass reveal">
                        <div class="about-arch-head">{feature_icon(solid::GRID)}<h3>{words::about_table_title(lang)}</h3></div>
                        <p>{words::about_table(lang)}</p>
                        <p>{words::about_keys(lang)}</p>
                    </div>
                </div>
                <ul class="features stagger">{features}</ul>
                {ai(lang)}
            </div>
        </section>
    }
}

/// The opponents: the house AI, language models, hosted seats, and the
/// native AI in training.
fn ai(lang: Lang) -> impl IntoView + use<> {
    let providers = PROVIDERS
        .iter()
        .map(|name| view! { <li class="ai-provider-chip" translate="no">{*name}</li> })
        .collect_view();
    view! {
        <div id="ai" class="ai-showcase">
            <div class="ai-showcase-head">
                <p class="eyebrow">{words::ai_eyebrow(lang)}</p>
                <h2 class="ai-showcase-title">{words::ai_title(lang)}</h2>
            </div>
            <div class="ai-cards-grid stagger">
                <div class="ai-card glass reveal">
                    <div class="ai-card-top">{feature_icon(solid::CHIP)}<span class="pill">{words::ai_house_pill(lang)}</span></div>
                    <h3>{words::ai_house_title(lang)}</h3>
                    <p>{words::ai_house(lang)}</p>
                </div>
                <div class="ai-card glass reveal">
                    <div class="ai-card-top">{feature_icon(solid::GLOBE)}<span class="pill gold" translate="no">"Seat-Bridge · Bring Your Own Model"</span></div>
                    <h3>{words::ai_model_title(lang)}</h3>
                    <p>{plain(words::ai_model(lang))}</p>
                    <ul class="ai-provider-badges" aria-label=words::ai_providers(lang)>{providers}</ul>
                </div>
                <div class="ai-card glass reveal">
                    <div class="ai-card-top">{feature_icon(solid::SERVER)}<span class="pill">{words::ai_hosts_pill(lang)}</span></div>
                    <h3>{words::ai_hosts_title(lang)}</h3>
                    <p>{words::ai_hosts(lang)}</p>
                </div>
            </div>
            <div class="ai-training-banner glass reveal">
                <div class="ai-training-badge-col">
                    <span class="pill ok ai-training-pill"><span class="dot ok live" aria-hidden="true"></span>{words::ai_training_pill(lang)}</span>
                    <h3>{words::ai_training_title(lang)}</h3>
                    <p>{words::ai_training(lang)}</p>
                </div>
                <div class="ai-training-actions">
                    <a class="button button-ghost" href=format!("{REPOSITORY}{}", words::link_readme_model(lang)) target="_blank" rel="noopener noreferrer">
                        {solid::icon(solid::EXTERNAL)}<span>{words::ai_setup(lang)}</span>
                    </a>
                    <a class="button button-ghost" href=format!("{}#host", Page::Gateways.path(lang))>
                        {solid::icon(solid::SERVER)}<span>{words::ai_hosting_link(lang)}</span>
                    </a>
                </div>
            </div>
        </div>
    }
}

/// A gateway's card, shared with the directory page: its name and version
/// are the operator's words, shown as text; the official mark comes from
/// the origin alone. Copying the address (`boot.js`, `[data-copy]`) shows
/// only where the browser can copy.
/// `h2` names the card with a second-level heading (the directory, right
/// under the page's title), else a third (the start page's section).
/// `baylee://gateway/add?origin=` and the origin, percent-encoded: the link
/// Baylee's client reads since 0.1.1-beta.9b (Baylee `docs/client.md`, "The
/// links"; it shows the origin and asks before it adds anything). Only for
/// an `https` origin without a path, which is all the client takes.
fn client_link(origin: &str) -> Option<String> {
    let host = origin.strip_prefix("https://")?;
    if host.is_empty() || host.contains(['/', '?', '#', '@']) {
        return None;
    }
    let mut link = String::from("baylee://gateway/add?origin=");
    for byte in origin.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            link.push(char::from(byte));
        } else {
            let _ = write!(link, "%{byte:02X}");
        }
    }
    Some(link)
}

pub fn gateway_card(lang: Lang, gateway: &Gateway, h2: bool) -> impl IntoView + use<> {
    let state = if gateway.online {
        view! { <p class="gw-state pill ok"><span class="dot ok live" aria-hidden="true"></span>{words::gws_online(lang)}</p> }.into_any()
    } else {
        view! { <p class="gw-state pill off"><span class="dot off" aria-hidden="true"></span>{words::gws_offline(lang)}</p> }.into_any()
    };
    let official = gateway
        .official
        .then(|| view! { <span class="pill gold gw-official">{words::gws_official(lang)}</span> });
    let class = if gateway.official {
        "gw card card-link reveal gw-is-official"
    } else {
        "gw card card-link reveal"
    };
    let website = gateway.website.clone();
    // an offline gateway cannot be joined: its link would open a client
    // that then finds nobody there
    let open = client_link(&gateway.origin)
        .filter(|_| gateway.online)
        .map(|link| {
            view! {
                <a class="button gw-join" href=link>
                    {solid::icon(solid::EXTERNAL)}<span>{words::gws_open_client(lang)}</span>
                </a>
            }
        });
    let find = format!("{} {}", gateway.name, gateway.origin).to_lowercase();
    let name = gateway.name.clone();
    let name = if h2 {
        view! { <h2 class="gw-name"><bdi>{name}</bdi></h2> }.into_any()
    } else {
        view! { <h3 class="gw-name"><bdi>{name}</bdi></h3> }.into_any()
    };
    view! {
        <li class=class data-find=find data-online=if gateway.online { "true" } else { "false" }>
            <div class="gw-head">
                {name}
                {official}
            </div>
            {state}
            <dl class="gw-facts">
                <div>
                    <dt>{words::gws_address(lang)}</dt>
                    <dd class="gw-address"><code translate="no">{gateway.origin.clone()}</code></dd>
                </div>
                <div>
                    <dt>{words::gws_version(lang)}</dt>
                    <dd><code translate="no">{gateway.version.clone()}</code></dd>
                </div>
            </dl>
            <div class="gw-actions">
                {open}
                <button type="button" class="button button-ghost gw-open" hidden=true
                    data-copy=gateway.origin.clone() data-copied-label=words::gws_copied_short(lang) data-copied-said=words::gws_copied(lang) data-copy-failed=words::gws_copy_failed(lang)>
                    {solid::icon_as(solid::COPY, "copy-idle")}{solid::icon_as(solid::CHECK, "copy-done")}<span>{words::gws_copy(lang)}</span>
                </button>
                <a class="button button-ghost" href=website target="_blank" rel="noopener noreferrer nofollow">
                    {solid::icon(solid::EXTERNAL)}<span>{words::gws_website(lang)}</span>
                </a>
            </div>
        </li>
    }
}

fn gateways(lang: Lang, all: Option<&[Gateway]>) -> impl IntoView + use<> {
    let host = format!("{}#host", Page::Gateways.path(lang));
    let offline = rich(words::hero_offline(lang), move |tag| {
        (tag == "host").then(|| host.clone())
    });
    let code = lang.code();
    let account = rich(words::hero_account(lang), move |tag| match tag {
        "register" => Some(format!("/{code}/register")),
        "signin" => Some(format!("/{code}/signin")),
        _ => None,
    });
    let listed = all.unwrap_or_default();
    let online = listed.iter().filter(|gateway| gateway.online).count();
    let count = match all {
        Some([]) => words::gw_none(lang).to_owned(),
        Some(all) => words::gw_count(
            lang,
            u64::try_from(all.len()).unwrap_or(u64::MAX),
            &words::number(lang, u64::try_from(online).unwrap_or(u64::MAX)),
        ),
        None => words::gws_failed(lang).to_owned(),
    };
    let shown: Vec<_> = listed.iter().take(4).collect();
    let n = shown.len();
    let grid = (n > 0).then(|| {
        let cards = shown
            .into_iter()
            .map(|gateway| gateway_card(lang, gateway, false))
            .collect_view();
        view! { <ul class="gateways hgw-grid stagger" data-count=n.to_string()>{cards}</ul> }
    });
    view! {
        <section class="band" id="gateways" aria-labelledby="gw-title">
            <div class="wrap">
                {section_head(words::gw_eyebrow(lang), words::gw_title(lang), "gw-title")}
                <p class="lead">{words::home_gw_lead(lang)}</p>
                // what playing online takes, before the gateways themselves:
                // what works without an account, and how to get one
                <div class="play-notes">
                    <p class="play-note glass">{icon(icons::WIFI_OFF)}<span>{offline}</span></p>
                    <p class="play-note glass" data-signed-out="">{icon(icons::USER_PLUS)}<span>{account}</span></p>
                </div>
                <p class="small hgw-count">{count}</p>
                {grid}
                <p class="small hgw-help">{words::gws_open_client_help(lang)}</p>
                <p class="hgw-foot">
                    <a class="button" href=Page::Gateways.path(lang)>{words::beta_open(lang)}</a>
                    <a class="button button-ghost" href=format!("{}#host", Page::Gateways.path(lang))>{words::beta_own(lang)}</a>
                </p>
            </div>
        </section>
    }
}

/// Writing to the people behind Baylee: the channels, and the form; on the
/// start page and on the contact page. `reports` is where "Reports &
/// privacy" leads.
pub fn contact_columns(lang: Lang, reports: String) -> impl IntoView + use<> {
    let impressum = Page::Impressum.path(lang);
    let operator = rich(words::contact_operator(lang), move |tag| {
        (tag == "impressum").then(|| impressum.clone())
    });
    let privacy = Page::Privacy.path(lang);
    let note = rich(words::contact_privacy(lang), move |tag| {
        (tag == "privacy").then(|| privacy.clone())
    });
    let channel = |href: String, glyph: &'static str, name: &'static str, what: &'static str| {
        view! {
            <li>
                <a
                    class="glass"
                    href=href.clone()
                    target=href.starts_with("https://").then_some("_blank")
                    rel=href.starts_with("https://").then_some("noopener noreferrer")
                >
                    {solid::icon(glyph)}
                    <span>{name}<small>{what}</small></span>
                </a>
            </li>
        }
    };
    view! {
        <div class="two-col">
            <div class="reveal">
                <p>{words::contact_intro(lang)}</p>
                <p>{plain(words::contact_f8(lang))}</p>
                <ul class="channels">
                    {channel(DISCORD.to_owned(), solid::DISCORD, "Discord", words::contact_discord(lang))}
                    {channel(WHATSAPP.to_owned(), solid::WHATSAPP, "WhatsApp", words::contact_whatsapp_sub(lang))}
                    {channel(format!("{REPOSITORY}/issues"), solid::GITHUB, words::contact_issues(lang), words::contact_issues_sub(lang))}
                    {channel(reports, solid::CHAT, words::contact_reports_title(lang), words::contact_reports_sub(lang))}
                </ul>
                <p class="small">{operator}</p>
            </div>
            <div class="contact-card glass reveal">
                <ContactForm lang=lang />
                <noscript><p class="notice">{words::contact_noscript(lang)}</p></noscript>
                <p class="small privacy-note">{note}</p>
            </div>
        </div>
    }
}

fn contact(lang: Lang) -> impl IntoView + use<> {
    view! {
        <section class="band alt" id="contact" aria-labelledby="contact-title">
            <div class="wrap">
                {section_head(words::contact_eyebrow(lang), words::contact_title(lang), "contact-title")}
                {contact_columns(lang, format!("{}#reports", Page::Contact.path(lang)))}
            </div>
        </section>
    }
}

/// The open-source promise, and the repository as it stands.
fn source(cx: &Context<'_>) -> impl IntoView + use<> {
    let lang = cx.lang;
    let repo = &cx.data.repo;
    let rules = rich(words::source_rust(lang), |tag| match tag {
        "agents" => Some(format!("{REPOSITORY}/blob/main/AGENTS.md")),
        "claude" => Some(format!("{REPOSITORY}/blob/main/CLAUDE.md")),
        _ => None,
    });
    let actions = format!("{REPOSITORY}/actions/workflows/ci.yml?query=branch%3Amain");
    let ci = match repo.ci {
        Some(passed) => {
            let (class, word) = if passed {
                ("dot ok", words::ci_passing(lang))
            } else {
                ("dot bad", words::ci_failing(lang))
            };
            view! { <a href=actions target="_blank" rel="noopener noreferrer"><span class=class aria-hidden="true"></span>{word}</a> }.into_any()
        }
        None => "–".into_any(),
    };
    let commit = match &repo.commit {
        Some((sha, date)) => {
            let subject = repo
                .subject
                .clone()
                .map(|subject| view! { " · " <bdi class="small">{subject}</bdi> });
            view! {
                <a href=format!("{REPOSITORY}/commit/{sha}") target="_blank" rel="noopener noreferrer" translate="no">{sha.clone()}</a>
                " · "{format::date(lang, date)}
                {subject}
            }
            .into_any()
        }
        None => "–".into_any(),
    };
    let count = |value: Option<u64>, href: String| {
        match value {
        Some(value) => view! { <a href=href target="_blank" rel="noopener noreferrer" class="num">{words::number(lang, value)}</a> }.into_any(),
        None => "–".into_any(),
    }
    };
    let stars = count(repo.stars, format!("{REPOSITORY}/stargazers"));
    let issues = count(repo.issues, format!("{REPOSITORY}/issues"));
    let link = |href: String, words: AnyView| {
        view! { <a class="button button-ghost" href=href target="_blank" rel="noopener noreferrer">{words}</a> }
    };
    view! {
        <section class="band" id="source" aria-labelledby="source-title">
            <div class="wrap">
                {section_head(words::source_eyebrow(lang), words::source_title(lang), "source-title")}
                <div class="two-col">
                    <div class="reveal">
                        <p class="promise">{words::source_promise(lang)}</p>
                        <p>{rules}</p>
                        <p class="links-row">
                            {link(REPOSITORY.to_owned(), view! { {solid::icon(solid::GITHUB)}<span translate="no">"github.com/AceVik/baylee"</span> }.into_any())}
                            {link(CENTRAL_REPOSITORY.to_owned(), view! { {solid::icon(solid::GITHUB)}<span>{words::open_central(lang)}</span> }.into_any())}
                            {link(DISCORD.to_owned(), view! { {solid::icon(solid::DISCORD)}<span>{words::source_discord(lang)}</span> }.into_any())}
                            {link(WHATSAPP.to_owned(), view! { {solid::icon(solid::WHATSAPP)}<span>{words::source_whatsapp(lang)}</span> }.into_any())}
                            {link(format!("{REPOSITORY}/blob/main/LICENSE"), view! { <span translate="no">"AGPL-3.0"</span> }.into_any())}
                            {link(format!("{REPOSITORY}/blob/main/docs/architecture.md"), view! { <span>{words::source_architecture(lang)}</span> }.into_any())}
                        </p>
                    </div>
                    <div class="repo-card glass reveal">
                        <h3>{words::source_repo_title(lang)}</h3>
                        <dl class="kv">
                            <div><dt>{words::strip_ci(lang)}</dt><dd>{ci}</dd></div>
                            <div><dt>{words::strip_commit(lang)}</dt><dd>{commit}</dd></div>
                            <div><dt>{words::source_stars(lang)}</dt><dd>{stars}</dd></div>
                            <div><dt>{words::source_issues(lang)}</dt><dd>{issues}</dd></div>
                        </dl>
                        <p class="small">{words::source_note(lang)}</p>
                    </div>
                </div>
            </div>
        </section>
    }
}

/// Baylee's album: a book the big logo opens (no page of its own, owner
/// 10.11.2026), as the old site drew it: her photographs on the left, her
/// story on the right, one chapter at a time. The server writes the whole
/// book; `home.js` turns its pages (without it every chapter stands one
/// after the other). Photographs load only once the album is open: lazy
/// images in a closed dialog are not fetched.
fn album(cx: &Context<'_>) -> impl IntoView + use<> {
    let lang = cx.lang;
    let assets = cx.assets;
    let count = CHAPTERS.len();
    let photos = PHOTOS
        .iter()
        .enumerate()
        .map(|(at, (n, caption, width, height))| {
            let later = at > 0;
            view! {
                <img
                    class=format!("focal-{n}")
                    src=assets.url(&format!("img/memorial/baylee-{n}.jpg"))
                    data-full=assets.url(&format!("img/memorial/full/baylee-{n}.jpg"))
                    width=width.to_string()
                    height=height.to_string()
                    alt=caption(lang)
                    loading="lazy"
                    decoding="async"
                    hidden=later
                />
            }
        })
        .collect_view();
    let (tabs, chapters) = story(lang);
    view! {
        <dialog id="album" class="book album" aria-labelledby="album-title" data-photos=PHOTOS.len().to_string()>
            <header class="album-masthead">
                <div class="album-light" aria-hidden="true"></div>
                <img class="album-cloud" src=assets.url("img/cloud-600.webp") width="600" height="400" alt="" loading="lazy" decoding="async" />
                <div class="album-heading">
                    <p class="eyebrow">{words::baylee_eyebrow(lang)}</p>
                    <h2 id="album-title">{words::site_name(lang)}</h2>
                    <p class="album-love">{words::baylee_story_title(lang)}</p>
                    <p class="album-dates">{words::baylee_together(lang)}</p>
                </div>
                <form method="dialog">
                    <button type="submit" class="round-btn album-dismiss" aria-label=words::baylee_close(lang)>
                        <span aria-hidden="true">"×"</span>
                    </button>
                </form>
            </header>
            <div class="album-layout">
                <figure class="album-photo">
                    <button type="button" class="album-photo-open" commandfor="photo-viewer" command="show-modal" aria-haspopup="dialog">
                        <span class="photo-preview">{photos}</span>
                        <span class="photo-open-hint">{words::baylee_photo_open(lang)}" ↗"</span>
                    </button>
                    <div class="album-photo-caption">
                        <figcaption aria-live="polite" data-caption="">{words::baylee_photo1(lang)}</figcaption>
                        <div class="album-controls">
                            <button type="button" class="round-btn" data-photo-step="-1" aria-label=words::baylee_previous(lang)>
                                <span aria-hidden="true">"←"</span>
                            </button>
                            <span class="num" aria-hidden="true" data-photo-count="">{format!("1 / {}", PHOTOS.len())}</span>
                            <button type="button" class="round-btn" data-photo-step="1" aria-label=words::baylee_next(lang)>
                                <span aria-hidden="true">"→"</span>
                            </button>
                        </div>
                        <p class="album-photo-note">{words::baylee_photos(lang)}</p>
                    </div>
                </figure>
                <div class="album-reading">
                    <div class="chapter-nav" role="tablist" aria-label=words::baylee_chapters(lang)>{tabs}</div>
                    <div class="story-leaves">
                        <article class="story-page" tabindex="0">{chapters}</article>
                    </div>
                    <footer class="chapter-footer">
                        <button type="button" class="button button-ghost" data-chapter-step="-1" disabled=true aria-label=words::baylee_chapter_previous(lang)>
                            <span aria-hidden="true">"← "</span>
                            <span>{words::baylee_chapter_previous(lang)}</span>
                        </button>
                        <span class="chapter-count num" aria-live="polite" data-chapter-count="">{format!("1 / {count}")}</span>
                        <button type="button" class="button button-ghost" data-chapter-step="1" aria-label=words::baylee_chapter_next(lang)>
                            <span>{words::baylee_chapter_next(lang)}</span>
                            <span aria-hidden="true">" →"</span>
                        </button>
                    </footer>
                </div>
            </div>
        </dialog>
        {viewer(lang, assets.url("img/memorial/full/baylee-1.jpg"))}
    }
}

/// The story's tabs and its chapters, every chapter written out.
fn story(lang: Lang) -> (impl IntoView + use<>, impl IntoView + use<>) {
    let count = CHAPTERS.len();
    let tabs = CHAPTERS
        .iter()
        .enumerate()
        .map(|(at, (title, _))| {
            let first = at == 0;
            view! {
                <button
                    type="button"
                    role="tab"
                    id=format!("chapter-tab-{}", at + 1)
                    aria-controls=format!("chapter-{}", at + 1)
                    aria-selected=if first { "true" } else { "false" }
                    tabindex=if first { "0" } else { "-1" }
                    aria-label=format!("{:02} · {}", at + 1, title(lang))
                    title=title(lang)
                >
                    {format!("{:02}", at + 1)}
                </button>
            }
        })
        .collect_view();
    let chapters = CHAPTERS
        .iter()
        .enumerate()
        .map(|(at, (title, body))| {
            view! {
                <section
                    class="story-ink"
                    id=format!("chapter-{}", at + 1)
                    role="tabpanel"
                    aria-labelledby=format!("chapter-tab-{}", at + 1)
                >
                    <p class="eyebrow">{format!("{} · {} / {count}", words::baylee_chapters(lang), at + 1)}</p>
                    <h3>{title(lang)}</h3>
                    {paragraphs(body(lang))}
                    <p class="story-flower" aria-hidden="true">"❧"</p>
                </section>
            }
        })
        .collect_view();
    (tabs, chapters)
}

/// The full photograph, larger, to zoom into and move about.
fn viewer(lang: Lang, full: String) -> impl IntoView + use<> {
    view! {
        <dialog id="photo-viewer" class="book photo-viewer" aria-label=words::baylee_photos(lang)>
            <header class="photo-viewer-header">
                <p data-viewer-caption="">{words::baylee_photo1(lang)}</p>
                <form method="dialog">
                    <button type="submit" class="round-btn" aria-label=words::baylee_photo_close(lang)>
                        <span aria-hidden="true">"×"</span>
                    </button>
                </form>
            </header>
            <div class="photo-viewport" data-viewport="">
                <img src=full alt=words::baylee_photo1(lang) width="787" height="1400" loading="lazy" decoding="async" draggable="false" data-viewer-image="" />
            </div>
            <footer class="photo-viewer-tools">
                <div class="photo-step-controls">
                    <button type="button" class="round-btn" data-photo-step="-1" aria-label=words::baylee_previous(lang)>
                        <span aria-hidden="true">"←"</span>
                    </button>
                    <span class="num" data-photo-count="">"1 / 10"</span>
                    <button type="button" class="round-btn" data-photo-step="1" aria-label=words::baylee_next(lang)>
                        <span aria-hidden="true">"→"</span>
                    </button>
                </div>
                <div class="photo-zoom-controls">
                    <button type="button" class="round-btn" data-zoom="-1" aria-label=words::baylee_zoom_out(lang) disabled=true>
                        <span aria-hidden="true">"−"</span>
                    </button>
                    <output class="num" aria-live="polite" data-zoom-level="">"100 %"</output>
                    <button type="button" class="round-btn" data-zoom="1" aria-label=words::baylee_zoom_in(lang)>
                        <span aria-hidden="true">"+"</span>
                    </button>
                    <button type="button" class="button button-ghost" data-zoom="0">{words::baylee_fit(lang)}</button>
                </div>
            </footer>
        </dialog>
    }
}
