//! The download page: the file for this machine first, then every
//! platform's files with their checksums and signatures, then what a first
//! start needs. Links go straight to GitHub's files: a download is a
//! navigation the visitor chooses, not a request the page makes.

use leptos::prelude::*;
use serde_json::{Value, json};

use super::Built;
use crate::data::{Kind, Platform, Release, ReleaseFile};
use crate::format;
use crate::rich::{plain, rich};
use crate::site::shell::{Meta, REPOSITORY};
use crate::site::{Context, Page};
use crate::solid;
use crate::words::{self, Lang};

/// A platform's name and its line beneath.
pub fn platform_name(lang: Lang, platform: Platform) -> (&'static str, &'static str) {
    match platform {
        Platform::WindowsX64 => (
            words::plat_windows_x64(lang),
            words::plat_windows_x64_sub(lang),
        ),
        Platform::WindowsArm64 => (
            words::plat_windows_arm64(lang),
            words::plat_windows_arm64_sub(lang),
        ),
        Platform::MacArm64 => (words::plat_mac_arm64(lang), words::plat_mac_arm64_sub(lang)),
        Platform::MacX64 => (words::plat_mac_x64(lang), words::plat_mac_x64_sub(lang)),
        Platform::LinuxX64 => (words::plat_linux_x64(lang), words::plat_linux_x64_sub(lang)),
        Platform::LinuxArm64 => (
            words::plat_linux_arm64(lang),
            words::plat_linux_arm64_sub(lang),
        ),
    }
}

fn kind_name(lang: Lang, kind: Kind) -> &'static str {
    match kind {
        Kind::Installer => words::dl_installer(lang),
        Kind::Deb => words::dl_deb(lang),
        Kind::Archive => words::dl_archive(lang),
        Kind::Fallback => words::dl_fallback(lang),
    }
}

fn releases_link(tag: &str) -> Option<String> {
    (tag == "releases").then(|| format!("{REPOSITORY}/releases"))
}

/// What a platform's build runs on, as the old site wrote it.
fn arch(platform: Platform) -> &'static str {
    match platform {
        Platform::MacArm64 => "Apple Silicon (ARM64)",
        Platform::MacX64 => "Intel (x64)",
        Platform::WindowsArm64 | Platform::LinuxArm64 => "ARM64",
        Platform::WindowsX64 | Platform::LinuxX64 => "x64",
    }
}

/// What to say about the visitor's machine, if anything.
pub fn machine_note(cx: &Context<'_>) -> Option<&'static str> {
    let lang = cx.lang;
    match (cx.machine.mobile, cx.machine.platform, cx.machine.sure) {
        (true, _, _) => Some(words::dl_mobile(lang)),
        (false, None, _) => Some(words::dl_unknown(lang)),
        (false, Some(Platform::MacArm64), false) => Some(words::dl_mac_unsure(lang)),
        (false, Some(Platform::WindowsX64), false) => Some(words::dl_windows_unsure(lang)),
        (false, Some(Platform::LinuxX64), false) => Some(words::dl_linux_unsure(lang)),
        _ => None,
    }
}

fn sep() -> impl IntoView + use<> {
    view! { <span class="sep" aria-hidden="true">"·"</span> }
}

/// The download menu's rows: each system's first file.
fn menu_items(lang: Lang, release: &Release, mine: Option<Platform>) -> impl IntoView + use<> {
    Platform::ALL
        .into_iter()
        .filter_map(|platform| release.best(platform).map(|file| (platform, file.clone())))
        .map(|(platform, file)| {
            let (name, _) = platform_name(lang, platform);
            let is_mine = mine == Some(platform);
            view! {
                <li>
                    <a class="menu-item" class:is-mine=is_mine href=file.url.clone() rel="noopener noreferrer">
                        <span class="dl-plat">
                            <strong>{name}</strong>
                            <span>{arch(platform)}</span>
                        </span>
                        <span class="dl-file num">
                            {kind_name(lang, file.kind)}
                            {sep()}
                            {words::dl_size(lang, &format::megabytes(lang, file.size))}
                        </span>
                    </a>
                </li>
            }
        })
        .collect_view()
}

/// The big button, on the start page and the download page: the file for
/// this machine when there is one, else the release; its version and
/// architecture inside it, every other system in a menu beside it, and the
/// checksum, signature and release notes below. Every link goes straight to
/// GitHub. Without a release, the releases page.
/// The lines below the button: the release, the file and its size and
/// date; the checksum, the signature, the notes and the other systems.
fn lines(
    lang: Lang,
    release: &Release,
    file: Option<&ReleaseFile>,
    others: String,
) -> impl IntoView + use<> {
    let meta = file.map(|file| {
        view! {
            {sep()}
            {kind_name(lang, file.kind)}
            {sep()}
            <span class="num">{words::dl_size(lang, &format::megabytes(lang, file.size))}</span>
        }
    });
    let checksum = file.and_then(|file| file.checksum.clone()).map(|url| {
        view! { <a href=url rel="noopener noreferrer">{words::dl_checksum(lang)}</a>{sep()} }
    });
    let signature = file.and_then(|file| file.signature.clone()).map(|url| {
        view! { <a href=url rel="noopener noreferrer">{words::dl_signature(lang)}</a>{sep()} }
    });
    view! {
        <p class="dl-meta">
            <span translate="no">{release.tag.clone()}</span>
            {meta}
            {sep()}
            {format::date(lang, &release.date)}
        </p>
        <p class="dl-sub">
            {checksum}
            {signature}
            <a href=release.page.clone() rel="noopener noreferrer">{words::dl_notes(lang)}</a>
            {sep()}
            <a href=others>{words::dl_others(lang)}</a>
        </p>
    }
}

pub fn download_block(cx: &Context<'_>, on_page: bool) -> impl IntoView + use<> {
    let lang = cx.lang;
    let others = if on_page {
        "#platforms".to_owned()
    } else {
        format!("{}#platforms", Page::Download.path(lang))
    };
    let Some(release) = cx.data.release.as_ref() else {
        let fallback = words::dl_button(lang);
        return view! {
            <div class="dl">
                <div class="dl-split">
                    <a class="button button-big dl-main" href=format!("{REPOSITORY}/releases") rel="noopener noreferrer">
                        {solid::icon(solid::DOWNLOAD)}
                        <span class="dl-text"><span class="dl-title">{fallback}</span></span>
                    </a>
                </div>
                <p class="dl-meta">{rich(words::dl_none(lang), releases_link)}</p>
            </div>
        }
        .into_any();
    };
    let mine = cx.machine.platform;
    let best =
        mine.and_then(|platform| release.best(platform).map(|file| (platform, file.clone())));
    let (href, title, badge) = match &best {
        Some((platform, file)) => (
            file.url.clone(),
            words::dl_for(lang, platform_name(lang, *platform).0),
            format!("{} · {}", release.tag, arch(*platform)),
        ),
        None => (
            release.page.clone(),
            words::dl_button(lang).to_owned(),
            release.tag.clone(),
        ),
    };
    let items = menu_items(lang, release, mine);
    let archive = best
        .as_ref()
        .filter(|(_, file)| file.kind == Kind::Fallback)
        .map(|_| view! { <p class="dl-arch">{words::dl_fallback_help(lang)}</p> });
    let note = machine_note(cx).map(|note| view! { <p class="dl-arch">{note}</p> });
    view! {
        <div class="dl">
            <div class="dl-split">
                <a class="button button-big dl-main" href=href rel="noopener noreferrer">
                    {solid::icon(solid::DOWNLOAD)}
                    <span class="dl-text">
                        <span class="dl-title">{title}</span>
                        <span class="dl-badge">{badge}</span>
                    </span>
                </a>
                <details class="dl-more" data-menu="">
                    <summary class="button" aria-label=words::dl_quick(lang) title=words::dl_quick(lang)>
                        {solid::icon_as(solid::DOWN, "chevron")}
                    </summary>
                    <div class="menu">
                        <ul>{items}</ul>
                        <p class="menu-foot">
                            <a href=others.clone()>{words::dl_others(lang)}</a>
                            {sep()}
                            <a href=release.page.clone() rel="noopener noreferrer">{words::dl_notes(lang)}</a>
                        </p>
                    </div>
                </details>
            </div>
            {lines(lang, release, best.as_ref().map(|(_, file)| file), others)}
            {note}
            {archive}
        </div>
    }
    .into_any()
}

/// Structured data for the game itself.
pub fn software(cx: &Context<'_>) -> Value {
    let mut app = json!({
        "@context": "https://schema.org",
        "@type": "SoftwareApplication",
        "name": "Baylee",
        "applicationCategory": "GameApplication",
        "operatingSystem": "Windows, macOS, Linux",
        "url": format!("{}{}", cx.origin, Page::Download.path(cx.lang)),
        "license": "https://www.gnu.org/licenses/agpl-3.0.html",
        "isAccessibleForFree": true,
        "offers": {"@type": "Offer", "price": "0", "priceCurrency": "EUR"},
        "description": words::desc_home(cx.lang),
    });
    if let Some(release) = &cx.data.release {
        app["softwareVersion"] = json!(release.tag.trim_start_matches('v'));
        app["downloadUrl"] = json!(release.page);
        app["datePublished"] = json!(release.date);
    }
    app
}

/// One file of a platform's card: its name, what it is and how large, and
/// its checksum and signature, as the old site listed them.
fn file_row(lang: Lang, file: &ReleaseFile) -> impl IntoView + use<> {
    let checksum = file.checksum.clone().map(|url| {
        view! { <a href=url rel="noopener noreferrer" translate="no">".sha256"</a> }
    });
    let signature = file.signature.clone().map(|url| {
        view! { <a href=url rel="noopener noreferrer" translate="no">".sig"</a> }
    });
    let between = (checksum.is_some() && signature.is_some()).then_some(" · ");
    let aux = (file.checksum.is_some() || file.signature.is_some())
        .then(|| view! { <span class="aux">{checksum}{between}{signature}</span> });
    let help = (file.kind == Kind::Fallback)
        .then(|| view! { <span class="help">{words::dl_fallback_help(lang)}</span> });
    view! {
        <li>
            <a class="file" href=file.url.clone() rel="noopener noreferrer" translate="no">{file.name.clone()}</a>
            <span class="kind">{kind_name(lang, file.kind)}</span>
            <span class="size num">{words::dl_size(lang, &format::megabytes(lang, file.size))}</span>
            {aux}
            {help}
        </li>
    }
}

/// What a first start needs.
fn install(lang: Lang) -> impl IntoView + use<> {
    view! {
        <section class="band alt" id="install" aria-labelledby="install-title">
            <div class="wrap">
                <p class="eyebrow">{words::install_eyebrow(lang)}</p>
                <h2 id="install-title">{words::install_title(lang)}</h2>
                <div class="two-col">
                    <div class="reveal">
                        <h3>{words::install_first_title(lang)}</h3>
                        <p>{plain(words::install_first(lang))}</p>
                        <h3>{words::install_check_title(lang)}</h3>
                        <p>{plain(words::install_check(lang))}</p>
                    </div>
                    <div class="reveal">
                        <h3>{words::install_needs_title(lang)}</h3>
                        <ul class="plain">
                            <li>{plain(words::install_windows(lang))}</li>
                            <li>{plain(words::install_mac(lang))}</li>
                            <li>{plain(words::install_linux(lang))}</li>
                            <li>{plain(words::install_cpu(lang))}</li>
                        </ul>
                        <p><a class="more-link" href=format!("{REPOSITORY}/releases") rel="noopener noreferrer">{words::install_releases(lang)}</a></p>
                    </div>
                </div>
            </div>
        </section>
    }
}

/// Every file of the release, a card per platform, the visitor's marked.
fn platforms(cx: &Context<'_>, release: &Release) -> impl IntoView + use<> {
    let lang = cx.lang;
    let cards = Platform::ALL
        .into_iter()
        .map(|platform| {
            let (name, sub) = platform_name(lang, platform);
            let yours = cx.machine.platform == Some(platform);
            let files = release.for_platform(platform);
            let list = if files.is_empty() {
                view! { <p class="plat-none">{words::plat_none(lang)}</p> }.into_any()
            } else {
                let rows = files
                    .into_iter()
                    .map(|file| file_row(lang, file))
                    .collect_view();
                view! { <ul class="plat-files">{rows}</ul> }.into_any()
            };
            let mark =
                yours.then(|| view! { <span class="mine-tag">{words::dl_yours(lang)}</span> });
            view! {
                <li class="plat glass reveal" id=platform.slug() class:mine=yours>
                    <h3 class="plat-name">{name}{mark}</h3>
                    <p class="plat-sub">{sub}</p>
                    {list}
                </li>
            }
        })
        .collect_view();
    let prerelease = release
        .prerelease
        .then(|| view! { <span class="tag">{words::dl_prerelease(lang)}</span> });
    view! {
        <section class="band" id="platforms" aria-labelledby="platforms-title">
            <div class="wrap">
                <p class="eyebrow">{words::dl_page_files_eyebrow(lang)}</p>
                <h2 id="platforms-title">{words::dl_page_files_title(lang)}</h2>
                <p class="release-head">
                    <span class="ver" translate="no">{release.tag.clone()}</span>
                    {prerelease}
                    <span>{format::date(lang, &release.date)}</span>
                    <a href=release.page.clone() rel="noopener noreferrer">{words::release_notes(lang)}</a>
                    <a href=format!("{REPOSITORY}/releases") rel="noopener noreferrer">{words::release_all(lang)}</a>
                </p>
                <ul class="platforms stagger">{cards}</ul>
            </div>
        </section>
    }
}

pub fn page(cx: &Context<'_>) -> Built {
    let lang = cx.lang;
    let mut meta = Meta::new(
        words::title_download(lang),
        words::desc_download(lang),
        "img/og.jpg",
    );
    meta.trail
        .push((words::nav_download(lang).to_owned(), Page::Download));
    meta.data.push(software(cx));
    let (first, all) = match &cx.data.release {
        Some(release) => {
            let missing = cx
                .machine
                .platform
                .filter(|platform| release.best(*platform).is_none())
                .map(|_| view! { <p class="notice">{rich(words::dl_missing(lang), releases_link)}</p> });
            (
                view! { <div class="page-dl">{download_block(cx, true)}</div>{missing} }.into_any(),
                Some(platforms(cx, release)),
            )
        }
        None => (
            view! { <p class="notice">{rich(words::dl_none(lang), releases_link)}</p> }.into_any(),
            None,
        ),
    };
    let body = view! {
        <section class="page-head wrap" aria-labelledby="download-title">
            <p class="eyebrow">{words::dl_page_eyebrow(lang)}</p>
            <h1 id="download-title">{words::dl_page_title(lang)}</h1>
            <p class="lead">{words::dl_page_lead(lang)}</p>
            {first}
        </section>
        {all}
        {install(lang)}
    }
    .into_any();
    (meta, body)
}
