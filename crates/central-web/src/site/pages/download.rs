//! The download page: the file for this machine first, then every
//! platform's files with their checksums and signatures, then what a first
//! start needs. Links go straight to GitHub's files: a download is a
//! navigation the visitor chooses, not a request the page makes.

use leptos::prelude::*;
use serde_json::{Value, json};

use super::Built;
use crate::data::{Kind, Platform, Release, ReleaseFile};
use crate::format;
use crate::rich::{icon, plain, rich};
use crate::site::icons;
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

/// The x86-64 level the main build of a platform needs: v3 (AVX2, 2013-2015
/// and later). `None` for ARM, whose builds have no such split.
fn cpu_level(platform: Platform) -> Option<&'static str> {
    match platform {
        Platform::WindowsX64 | Platform::MacX64 | Platform::LinuxX64 => Some("x86-64-v3"),
        _ => None,
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

/// The download menu: every system under its mark, each architecture's
/// first file a row; where the release has a build for older processors
/// (`-v2`, without AVX2), it stands under its system as a row of its own.
fn menu_items(lang: Lang, release: &Release, mine: Option<Platform>) -> impl IntoView + use<> {
    let systems: [(&'static str, solid::Mark, [Platform; 2]); 3] = [
        (
            "Windows",
            solid::WINDOWS,
            [Platform::WindowsX64, Platform::WindowsArm64],
        ),
        (
            "macOS",
            solid::APPLE,
            [Platform::MacArm64, Platform::MacX64],
        ),
        (
            "Linux",
            solid::LINUX,
            [Platform::LinuxX64, Platform::LinuxArm64],
        ),
    ];
    systems
        .into_iter()
        .filter_map(|(system, mark, platforms)| {
            let mut rows: Vec<(Platform, ReleaseFile, bool)> = Vec::new();
            for platform in platforms {
                let Some(best) = release.best(platform) else {
                    continue;
                };
                rows.push((platform, best.clone(), best.kind == Kind::Fallback));
                if best.kind != Kind::Fallback
                    && let Some(old) = release
                        .for_platform(platform)
                        .into_iter()
                        .find(|file| file.kind == Kind::Fallback)
                {
                    rows.push((platform, old.clone(), true));
                }
            }
            if rows.is_empty() {
                return None;
            }
            let rows = rows
                .into_iter()
                .map(|(platform, file, old)| {
                    let is_mine = mine == Some(platform) && !old;
                    let (title, level) = if old {
                        (words::dl_v2_name(lang).to_owned(), None)
                    } else {
                        (arch(platform).to_owned(), cpu_level(platform))
                    };
                    let kind = if old {
                        words::dl_archive(lang)
                    } else {
                        kind_name(lang, file.kind)
                    };
                    let yours = is_mine
                        .then(|| view! { <span class="mine-tag">{words::dl_yours(lang)}</span> });
                    let level = level.map(|level| view! { <span class="dl-level" translate="no">{level}</span> });
                    let note = old.then(|| view! { <li class="menu-note">{words::dl_v2_help(lang)}</li> });
                    view! {
                        <li>
                            <a class="menu-item dl-item" class:is-mine=is_mine class:is-old=old href=file.url.clone() target="_blank" rel="noopener noreferrer">
                                <span class="dl-plat">
                                    <strong>{title}{level}</strong>
                                    <span class="dl-file num">
                                        {kind}
                                        {sep()}
                                        {words::dl_size(lang, &format::megabytes(lang, file.size))}
                                    </span>
                                    <span class="dl-name" translate="no">{file.name.clone()}</span>
                                </span>
                                {yours}
                                {solid::icon_as(solid::DOWNLOAD, "dl-get")}
                            </a>
                        </li>
                        {note}
                    }
                })
                .collect_view();
            Some(view! {
                <li class="menu-group">
                    <p class="menu-group-name">{solid::mark(mark, "os")}<span translate="no">{system}</span></p>
                    <ul aria-label=system>{rows}</ul>
                </li>
            })
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
        view! { <a href=url target="_blank" rel="noopener noreferrer">{words::dl_checksum(lang)}</a>{sep()} }
    });
    let signature = file.and_then(|file| file.signature.clone()).map(|url| {
        view! { <a href=url target="_blank" rel="noopener noreferrer">{words::dl_signature(lang)}</a>{sep()} }
    });
    view! {
        <p class="dl-meta">
            <span class="dl-facts">
                <span translate="no">{release.tag.clone()}</span>
                {meta}
                {sep()}
                <span class="dl-date">{format::date(lang, &release.date)}</span>
            </span>
            <span class="dl-sub">
                {checksum}
                {signature}
                <a href=release.page.clone() target="_blank" rel="noopener noreferrer">{words::dl_notes(lang)}</a>
                {sep()}
                <a href=others>{words::dl_others(lang)}</a>
            </span>
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
                    <a class="button button-big dl-main" href=format!("{REPOSITORY}/releases") target="_blank" rel="noopener noreferrer">
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
        .map(|_| view! { <p class="dl-arch">{words::dl_v2_help(lang)}</p> });
    let note = machine_note(cx).map(|note| view! { <p class="dl-arch">{note}</p> });
    view! {
        <div class="dl">
            <div class="dl-split">
                <a class="button button-big dl-main" href=href target="_blank" rel="noopener noreferrer">
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
                        <div class="menu-head">
                            <p class="menu-title">{words::dl_menu_title(lang)}</p>
                            <p class="menu-release">
                                <code translate="no">{release.tag.clone()}</code>
                                <span class="dl-date">{format::date(lang, &release.date)}</span>
                            </p>
                        </div>
                        <ul class="menu-groups">{items}</ul>
                        <p class="menu-foot">
                            <a href=others.clone()>{words::dl_others(lang)}</a>
                            {sep()}
                            <a href=release.page.clone() target="_blank" rel="noopener noreferrer">{words::dl_notes(lang)}</a>
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

/// One file of an architecture's list: its name, what it is and how large,
/// and its checksum and signature where the release has them.
fn file_row(lang: Lang, file: &ReleaseFile) -> impl IntoView + use<> {
    let checksum = file.checksum.clone().map(|url| {
        view! { <a href=url target="_blank" rel="noopener noreferrer" translate="no">".sha256"</a> }
    });
    let signature = file.signature.clone().map(|url| {
        view! { <a href=url target="_blank" rel="noopener noreferrer" translate="no">".sig"</a> }
    });
    let aux = (checksum.is_some() || signature.is_some())
        .then(|| view! { <span class="aux">{checksum}{signature}</span> });
    let kind = if file.kind == Kind::Fallback {
        words::dl_archive(lang)
    } else {
        kind_name(lang, file.kind)
    };
    view! {
        <li>
            <a class="file" href=file.url.clone() target="_blank" rel="noopener noreferrer" translate="no">
                {solid::icon_as(solid::DOWNLOAD, "dl-get")}
                <span class="file-name">{file.name.clone()}</span>
            </a>
            <span class="kind">{kind}</span>
            <span class="size num">{words::dl_size(lang, &format::megabytes(lang, file.size))}</span>
            {aux}
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
                        <p><a class="more-link" href=format!("{REPOSITORY}/releases") target="_blank" rel="noopener noreferrer">{words::install_releases(lang)}</a></p>
                    </div>
                </div>
            </div>
        </section>
    }
}

/// The systems of the page, in the order of the menu: the mark, the name,
/// what it needs, its two architectures and the steps of a first start.
type System = (
    &'static str,
    solid::Mark,
    fn(Lang) -> &'static str,
    [Platform; 2],
    [AnyView; 3],
);

fn systems(lang: Lang) -> [System; 3] {
    [
        (
            "Windows",
            solid::WINDOWS,
            words::dl_os_windows,
            [Platform::WindowsX64, Platform::WindowsArm64],
            [
                words::dl_howto_windows_1(lang).into_any(),
                plain(words::dl_howto_windows_2(lang)),
                words::dl_howto_windows_3(lang).into_any(),
            ],
        ),
        (
            "macOS",
            solid::APPLE,
            words::dl_os_mac,
            [Platform::MacArm64, Platform::MacX64],
            [
                plain(words::dl_howto_mac_1(lang)),
                plain(words::dl_howto_mac_2(lang)),
                plain(words::dl_howto_mac_3(lang)),
            ],
        ),
        (
            "Linux",
            solid::LINUX,
            words::dl_os_linux,
            [Platform::LinuxX64, Platform::LinuxArm64],
            [
                plain(words::dl_howto_linux_1(lang)),
                plain(words::dl_howto_linux_2(lang)),
                plain(words::dl_howto_linux_3(lang)),
            ],
        ),
    ]
}

/// One architecture of a system: the main files (installer, package,
/// archive), and under them, set apart, the archive for older processors.
fn arch_block(
    cx: &Context<'_>,
    release: &Release,
    platform: Platform,
) -> Option<impl IntoView + use<>> {
    let lang = cx.lang;
    let files = release.for_platform(platform);
    if files.is_empty() {
        return None;
    }
    let (main, old): (Vec<_>, Vec<_>) = files.into_iter().partition(|f| f.kind != Kind::Fallback);
    let yours = cx.machine.platform == Some(platform);
    let mark = yours.then(|| view! { <span class="mine-tag">{words::dl_yours(lang)}</span> });
    let level = cpu_level(platform)
        .map(|level| view! { <span class="dl-level" translate="no">{level}</span> });
    let main_list = (!main.is_empty()).then(|| {
        let rows = main
            .into_iter()
            .map(|file| file_row(lang, file))
            .collect::<Vec<_>>();
        view! { <ul class="plat-files">{rows}</ul> }
    });
    let old_list = (!old.is_empty()).then(|| {
        let rows = old
            .into_iter()
            .map(|file| file_row(lang, file))
            .collect::<Vec<_>>();
        view! {
            <div class="plat-old">
                <p class="plat-old-name">{words::dl_v2_name(lang)}</p>
                <p class="plat-old-help">{words::dl_v2_help(lang)}</p>
                <ul class="plat-files">{rows}</ul>
            </div>
        }
    });
    Some(view! {
        <div class="arch" id=platform.slug() class:mine=yours>
            <h4 class="arch-name">
                <span>{arch(platform)}</span>
                {level}
                {mark}
            </h4>
            <p class="arch-sub">{platform_name(lang, platform).1}</p>
            {main_list}
            {old_list}
        </div>
    })
}

/// Every file of the release, a card per system, the visitor's first and
/// in gold; in it an architecture each, with its files, the archive for
/// older processors apart, and the steps of a first start.
fn platforms(cx: &Context<'_>, release: &Release) -> impl IntoView + use<> {
    let lang = cx.lang;
    let mut order = systems(lang);
    // the visitor's own system leads; the others keep the menu's order
    order.sort_by_key(|(_, _, _, platforms, _)| {
        cx.machine
            .platform
            .is_none_or(|mine| !platforms.contains(&mine))
    });
    let cards = order
        .into_iter()
        .map(|(name, mark, needs, platforms, steps)| {
            let yours = cx
                .machine
                .platform
                .is_some_and(|mine| platforms.contains(&mine));
            let blocks = platforms
                .into_iter()
                .filter_map(|platform| arch_block(cx, release, platform))
                .collect::<Vec<_>>();
            let body = if blocks.is_empty() {
                view! { <p class="plat-none">{words::plat_none(lang)}</p> }.into_any()
            } else {
                view! { <div class="archs">{blocks}</div> }.into_any()
            };
            let steps = steps
                .into_iter()
                .map(|step| view! { <li>{step}</li> })
                .collect::<Vec<_>>();
            view! {
                <li class="plat glass reveal" class:mine=yours>
                    <h3 class="plat-name">
                        {solid::mark(mark, "os")}
                        <span translate="no">{name}</span>
                    </h3>
                    <p class="plat-sub">{needs(lang)}</p>
                    {body}
                    <div class="howto">
                        <h4>{words::dl_howto_title(lang)}</h4>
                        <ol>{steps}</ol>
                    </div>
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
                    <a href=release.page.clone() target="_blank" rel="noopener noreferrer">{words::release_notes(lang)}</a>
                    <a href=format!("{REPOSITORY}/releases") target="_blank" rel="noopener noreferrer">{words::release_all(lang)}</a>
                </p>
                <ul class="platforms stagger">{cards}</ul>
                {soon(lang)}
            </div>
        </section>
    }
}

/// What has no download yet: said, so that nobody looks for it.
fn soon(lang: Lang) -> impl IntoView + use<> {
    view! {
        <div class="soon glass reveal">
            <h3>{words::dl_soon_title(lang)}</h3>
            <ul class="soon-list">
                <li>{icon(icons::SMARTPHONE)}<span translate="no">"Android"</span></li>
                <li>{icon(icons::SMARTPHONE)}<span translate="no">"iOS"</span></li>
                <li>{icon(icons::GLOBE)}<span>{words::dl_soon_web(lang)}</span></li>
            </ul>
            <p>{words::dl_soon_text(lang)}</p>
        </div>
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
