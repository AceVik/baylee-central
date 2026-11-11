//! The start page: what Baylee is, the download for this machine, the
//! project's state, the card numbers, the table, the gateways, the open
//! source promise, and Baylee's album behind the big logo.

use leptos::prelude::*;
use leptos::tachys::html::attribute::custom::custom_attribute;
use serde_json::json;

use super::download::download_block;
use super::{Built, section_head};
use crate::data::{CardStatus, Gateway};
use crate::format;
use crate::rich::{icon, paragraphs, plain, rich};
use crate::site::icons;
use crate::site::shell::{CENTRAL_REPOSITORY, DISCORD, Meta, REPOSITORY, WHATSAPP};
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
    let body = view! {
        {strip(cx)}
        {hero(cx)}
        {cards(lang, cx.data.cards.as_ref())}
        {screens(cx)}
        {about(lang)}
        {gateways(lang, cx.data.gateways.as_deref())}
        {open(lang)}
        {album(cx)}
    }
    .into_any();
    (meta, body)
}

fn hero(cx: &Context<'_>) -> impl IntoView + use<> {
    let lang = cx.lang;
    let assets = cx.assets;
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
                        sizes="(min-width: 30rem) 22rem, 78vw"
                        width="960"
                        height="400"
                        alt=""
                        fetchpriority="low"
                    />
                    <span class="visually-hidden">{words::hero_memorial(lang)}</span>
                </button>
                <h1 id="hero-title">{plain(words::hero_title(lang))}</h1>
                <p class="lead">{words::hero_lead(lang)}</p>
                {download_block(cx, false)}
                <p class="hero-note">{offline}</p>
                <p class="hero-signup" data-signed-out="">{account}</p>
                <div class="hero-actions">
                    <a class="button discord" href=DISCORD rel="noopener noreferrer">
                        <span class="beacon" aria-hidden="true"></span>
                        {solid::icon_as(solid::DISCORD, "wave")}
                        <span>{words::hero_discord(lang)}</span>
                    </a>
                    <a class="button whatsapp" href=WHATSAPP rel="noopener noreferrer">
                        <span class="beacon" aria-hidden="true"></span>
                        {solid::icon(solid::WHATSAPP)}
                        <span>{words::hero_whatsapp(lang)}</span>
                    </a>
                    <a class="button github" href=REPOSITORY rel="noopener noreferrer">
                        {solid::icon(solid::GITHUB)}
                        <span>{words::hero_github(lang)}</span>
                    </a>
                </div>
                <p class="kicker">{words::hero_kicker(lang)}</p>
            </div>
        </section>
    }
}

/// The strip under the bar: the latest build, CI on main and the last
/// commit, and the gateways online and not. Only what a source said is
/// shown; while a source has not answered, the strip links to it instead.
fn strip(cx: &Context<'_>) -> impl IntoView + use<> {
    let lang = cx.lang;
    let build = match cx.data.release.as_ref() {
        Some(release) => view! {
            <a href=release.page.clone() rel="noopener noreferrer"><code translate="no">{release.tag.clone()}</code></a>
            <span class="small">"· "{format::date(lang, &release.date)}</span>
        }
        .into_any(),
        None => view! {
            <a href=format!("{REPOSITORY}/releases") rel="noopener noreferrer">{words::strip_releases(lang)}</a>
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
            view! { <a href=actions rel="noopener noreferrer"><span class=class aria-hidden="true"></span>{word}</a> }
                .into_any()
        }
        None => {
            view! { <a href=actions rel="noopener noreferrer">{words::strip_on_github(lang)}</a> }
                .into_any()
        }
    };
    let commit = match cx.data.repo.commit.clone() {
        Some((id, date)) => view! {
            <a href=format!("{REPOSITORY}/commit/{id}") rel="noopener noreferrer"><code translate="no">{id.clone()}</code></a>
            <span class="small">"· "{format::date(lang, &date)}</span>
        }
        .into_any(),
        None => view! {
            <a href=format!("{REPOSITORY}/commits/main") rel="noopener noreferrer">{words::strip_on_github(lang)}</a>
        }
        .into_any(),
    };
    let gateways = match cx.data.gateways.as_deref() {
        Some(all) => {
            let online = all.iter().filter(|g| g.online).count();
            let offline = all.len() - online;
            let count = |n: usize| words::number(lang, u64::try_from(n).unwrap_or(u64::MAX));
            let dot = if online > 0 { "dot ok" } else { "dot off" };
            view! {
                <a href=Page::Gateways.path(lang)>
                    <span class="stat-pill num">
                        <span class=dot aria-hidden="true"></span>
                        {count(online)}" "{words::strip_online(lang)}
                        <span class="small">"·"</span>
                        <span class="dot off" aria-hidden="true"></span>
                        {count(offline)}" "{words::strip_offline(lang)}
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
                    <div class="strip-stat"><dt>{words::strip_build(lang)}</dt><dd>{build}</dd></div>
                    <div class="strip-stat"><dt>{words::strip_ci(lang)}</dt><dd>{ci}</dd></div>
                    <div class="strip-stat strip-commit"><dt>{words::strip_commit(lang)}</dt><dd>{commit}</dd></div>
                </dl>
                <dl class="strip-group strip-right">
                    <div class="strip-stat"><dt>{words::strip_gateways(lang)}</dt><dd>{gateways}</dd></div>
                </dl>
            </div>
        </section>
    }
}

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
        <div class="stat">
            <p class="stat-value num">{words::number(lang, value)}</p>
            <p class="stat-label">
                {label}
                <button type="button" class="tip" popovertarget=tip.clone() {..custom_attribute("interestfor", tip.clone())} aria-label=what>
                    {icon(icons::INFO)}
                </button>
            </p>
            <div id=tip class="tip-body" popover="auto" role="note">{help}</div>
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
                <div class="stats">
                    {stat(lang, "implemented", s.implemented, words::cards_implemented(lang), words::cards_implemented_help(lang))}
                    {stat(lang, "generated", s.generated, words::cards_generated(lang), words::cards_generated_help(lang))}
                    {stat(lang, "partial", s.partial, words::cards_partial(lang), words::cards_partial_help(lang))}
                    {stat(lang, "total", s.total, words::cards_total(lang), words::cards_total_help(lang))}
                </div>
                <p class="share">{words::cards_share(lang, &words::number(lang, playable), &words::number(lang, s.total))}</p>
                <p class="meta">{words::cards_as_of(lang, &status.tag, &format::date(lang, &status.date))}</p>
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

fn screens(cx: &Context<'_>) -> Option<impl IntoView + use<>> {
    let lang = cx.lang;
    let assets = cx.assets;
    if !assets.has("media/shot-table-duel-1200.webp") {
        return None;
    }
    let figures = SHOTS
        .iter()
        .map(|(name, caption, alt)| {
            let still = |width: u32| assets.url(&format!("media/shot-{name}-{width}.webp"));
            let moving = |width: u32| assets.url(&format!("media/shot-{name}-{width}.avif"));
            view! {
                <figure class="shot">
                    <picture>
                        <source media="(prefers-reduced-motion: reduce)" type="image/webp" srcset=format!("{} 640w, {} 1200w", still(640), still(1200)) sizes="(min-width: 60rem) 30rem, 100vw" />
                        <source type="image/avif" srcset=format!("{} 640w, {} 1200w", moving(640), moving(1200)) sizes="(min-width: 60rem) 30rem, 100vw" />
                        <img src=still(1200) srcset=format!("{} 640w, {} 1200w", still(640), still(1200)) sizes="(min-width: 60rem) 30rem, 100vw" width="1200" height="703" alt=alt(lang) loading="lazy" decoding="async" />
                    </picture>
                    <figcaption>{plain(caption(lang))}</figcaption>
                </figure>
            }
        })
        .collect_view();
    Some(view! {
        <section class="band alt" aria-labelledby="screens-title">
            <div class="wrap">
                {section_head(words::screens_eyebrow(lang), words::screens_title(lang), "screens-title")}
                <div class="shots">{figures}</div>
                <p class="meta">{words::screens_credit(lang)}</p>
            </div>
        </section>
    })
}

fn about(lang: Lang) -> impl IntoView + use<> {
    view! {
        <section class="band" aria-labelledby="about-title">
            <div class="wrap">
                {section_head(words::about_eyebrow(lang), words::about_title(lang), "about-title")}
                <div class="cards-grid">
                    <article class="card">
                        {icon(icons::SCALE)}
                        <h3>{words::about_engine_title(lang)}</h3>
                        <p>{words::about_engine(lang)}</p>
                    </article>
                    <article class="card">
                        {icon(icons::LAYERS)}
                        <h3>{words::about_table_title(lang)}</h3>
                        <p>{words::about_table(lang)}</p>
                    </article>
                    <article class="card">
                        {icon(icons::BOT)}
                        <h3>{words::about_ai_title(lang)}</h3>
                        <p>{words::about_ai(lang)}</p>
                    </article>
                </div>
            </div>
        </section>
    }
}

/// A gateway's card, shared with the directory page.
pub fn gateway_card(lang: Lang, gateway: &Gateway) -> impl IntoView + use<> {
    let state = if gateway.online {
        view! { <span class="pill ok"><span class="dot" aria-hidden="true"></span>{words::gws_online(lang)}</span> }.into_any()
    } else {
        view! { <span class="pill off"><span class="dot" aria-hidden="true"></span>{words::gws_offline(lang)}</span> }.into_any()
    };
    let official = gateway
        .official
        .then(|| view! { <span class="pill accent">{icon(icons::SHIELD_CHECK)}{words::gws_official(lang)}</span> });
    let website = gateway.website.clone();
    view! {
        <article class="card gateway">
            <h3><bdi>{gateway.name.clone()}</bdi></h3>
            <p class="pills">{official}{state}</p>
            <dl class="facts">
                <dt>{words::gws_address(lang)}</dt>
                <dd><code class="address" translate="no">{gateway.origin.clone()}</code></dd>
                <dt>{words::gws_version(lang)}</dt>
                <dd><code translate="no">{gateway.version.clone()}</code></dd>
                <dt>{words::gws_website(lang)}</dt>
                <dd><a href=website.clone() rel="noopener noreferrer nofollow">{website.clone()}</a></dd>
            </dl>
        </article>
    }
}

fn gateways(lang: Lang, all: Option<&[Gateway]>) -> Option<impl IntoView + use<>> {
    let all = all?;
    let first = all
        .iter()
        .take(4)
        .map(|gateway| gateway_card(lang, gateway))
        .collect_view();
    Some(view! {
        <section class="band alt" aria-labelledby="gw-title">
            <div class="wrap">
                {section_head(words::gw_eyebrow(lang), words::gw_title(lang), "gw-title")}
                <p class="lead">{words::gw_lead(lang)}</p>
                <div class="cards-grid">{first}</div>
                <p class="actions">
                    <a class="button quiet" href=Page::Gateways.path(lang)>{words::gw_all(lang)}</a>
                    <a class="button quiet" href=format!("{}#host", Page::Gateways.path(lang))>{words::gw_host(lang)}</a>
                </p>
            </div>
        </section>
    })
}

fn open(lang: Lang) -> impl IntoView + use<> {
    view! {
        <section class="band" aria-labelledby="open-title">
            <div class="wrap narrow">
                {section_head(words::open_eyebrow(lang), words::open_title(lang), "open-title")}
                <p class="lead">{words::open_lead(lang)}</p>
                <p class="actions">
                    <a class="button quiet" href=REPOSITORY rel="noopener noreferrer">{icon(icons::EXTERNAL_LINK)}{words::open_repository(lang)}</a>
                    <a class="button quiet" href=CENTRAL_REPOSITORY rel="noopener noreferrer">{icon(icons::EXTERNAL_LINK)}{words::open_central(lang)}</a>
                </p>
            </div>
        </section>
    }
}

/// Baylee's album: a book the big logo opens (no page of its own, owner
/// 10.11.2026), as the old site drew it: her photographs on the left, her
/// story on the right, one chapter at a time. The server writes the whole
/// book; `album.js` turns its pages (without it every chapter stands one
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
        {viewer(lang)}
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
fn viewer(lang: Lang) -> impl IntoView + use<> {
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
                <img src="" alt="" draggable="false" data-viewer-image="" />
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
