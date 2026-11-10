//! The download page: the file for this machine first, then every
//! platform's files with their checksums and signatures, then what a first
//! start needs. Links go straight to GitHub's files: a download is a
//! navigation the visitor chooses, not a request the page makes.

use leptos::prelude::*;
use serde_json::{Value, json};

use super::Built;
use crate::data::{Kind, Platform, ReleaseFile};
use crate::format;
use crate::rich::{icon, plain, rich};
use crate::site::icons;
use crate::site::shell::{Meta, REPOSITORY};
use crate::site::{Context, Page};
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

/// The primary download button for the visitor's machine, or a link to
/// the download page when there is no file to name.
pub fn best_link(cx: &Context<'_>) -> impl IntoView + use<> {
    let lang = cx.lang;
    let best = cx
        .data
        .release
        .as_ref()
        .zip(cx.machine.platform)
        .and_then(|(release, platform)| {
            release.best(platform).map(|file| (platform, file.clone()))
        });
    match best {
        Some((platform, file)) => {
            let (system, _) = platform_name(lang, platform);
            view! {
                <a class="button primary" href=file.url.clone() rel="noopener noreferrer">
                    {icon(icons::DOWNLOAD)}
                    <span>{words::dl_for(lang, system)}</span>
                </a>
            }
            .into_any()
        }
        None => view! {
            <a class="button primary" href=Page::Download.path(lang)>
                {icon(icons::DOWNLOAD)}
                <span>{words::nav_download(lang)}</span>
            </a>
        }
        .into_any(),
    }
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

fn file_row(lang: Lang, file: &ReleaseFile) -> impl IntoView + use<> {
    let checksum = file.checksum.clone().map(|url| {
        view! { <a href=url rel="noopener noreferrer">{words::dl_checksum(lang)}</a> }
    });
    let signature = file.signature.clone().map(|url| {
        view! { <a href=url rel="noopener noreferrer">{words::dl_signature(lang)}</a> }
    });
    let help = (file.kind == Kind::Fallback)
        .then(|| view! { <p class="hint">{words::dl_fallback_help(lang)}</p> });
    view! {
        <li class="file">
            <a class="file-link" href=file.url.clone() rel="noopener noreferrer">
                {icon(if file.kind == Kind::Installer { icons::HARD_DRIVE_DOWNLOAD } else { icons::FILE_ARCHIVE })}
                <span class="file-kind">{kind_name(lang, file.kind)}</span>
                <code class="file-name" translate="no">{file.name.clone()}</code>
            </a>
            <span class="file-meta num">{words::dl_size(lang, &format::megabytes(lang, file.size))}</span>
            <span class="file-extra">{checksum}{signature}</span>
            {help}
        </li>
    }
}

/// What a first start needs.
fn install(lang: Lang) -> impl IntoView + use<> {
    view! {
        <section class="wrap prose-block" aria-labelledby="install-title">
            <h2 id="install-title">{words::install_title(lang)}</h2>
            <div class="cards-grid">
                <article class="card">
                    <h3>{words::install_first_title(lang)}</h3>
                    <p>{plain(words::install_first(lang))}</p>
                </article>
                <article class="card">
                    <h3>{words::install_check_title(lang)}</h3>
                    <p>{plain(words::install_check(lang))}</p>
                </article>
                <article class="card">
                    <h3>{words::install_needs_title(lang)}</h3>
                    <ul>
                        <li>{plain(words::install_windows(lang))}</li>
                        <li>{plain(words::install_mac(lang))}</li>
                        <li>{plain(words::install_linux(lang))}</li>
                        <li>{plain(words::install_cpu(lang))}</li>
                    </ul>
                </article>
            </div>
        </section>
    }
}

/// The release's files, or why there are none.
fn release_view(cx: &Context<'_>, note: Option<&'static str>) -> AnyView {
    let lang = cx.lang;
    match &cx.data.release {
        Some(release) => {
            let missing = cx
                .machine
                .platform
                .filter(|platform| release.best(*platform).is_none())
                .map(|_| view! { <p class="notice">{rich(words::dl_missing(lang), releases_link)}</p> });
            let platforms = Platform::ALL
                .into_iter()
                .map(|platform| {
                    let (name, sub) = platform_name(lang, platform);
                    let yours = cx.machine.platform == Some(platform);
                    let files = release.for_platform(platform);
                    let list = if files.is_empty() {
                        view! { <p class="hint">{words::plat_none(lang)}</p> }.into_any()
                    } else {
                        let rows = files
                            .into_iter()
                            .map(|file| file_row(lang, file))
                            .collect_view();
                        view! { <ul class="files">{rows}</ul> }.into_any()
                    };
                    let mark = yours.then(
                        || view! { <span class="pill accent">{words::dl_yours(lang)}</span> },
                    );
                    view! {
                        <article class="card platform" id=platform.slug() class:yours=yours>
                            <h3>{name}{mark}</h3>
                            <p class="meta">{sub}</p>
                            {list}
                        </article>
                    }
                })
                .collect_view();
            let prerelease = release
                .prerelease
                .then(|| view! { <span class="pill">{words::dl_prerelease(lang)}</span> });
            view! {
                <div class="release-head">
                    <p class="meta">
                        <code translate="no">{words::dl_release(lang, &release.tag, &format::date(lang, &release.date))}</code>
                        {prerelease}
                    </p>
                    <div class="actions">
                        {best_link(cx)}
                        <a class="button quiet" href=release.page.clone() rel="noopener noreferrer">{icon(icons::EXTERNAL_LINK)}{words::dl_notes(lang)}</a>
                    </div>
                    {note.map(|note| view! { <p class="hint">{note}</p> })}
                    {missing}
                </div>
                <h2 class="visually-hidden">{words::dl_all(lang)}</h2>
                <div class="platforms">{platforms}</div>
            }
            .into_any()
        }
        None => {
            view! { <p class="notice">{rich(words::dl_none(lang), releases_link)}</p> }.into_any()
        }
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
    let note = match (cx.machine.mobile, cx.machine.platform, cx.machine.sure) {
        (true, _, _) => Some(words::dl_mobile(lang)),
        (false, None, _) => Some(words::dl_unknown(lang)),
        (false, Some(Platform::MacArm64), false) => Some(words::dl_mac_unsure(lang)),
        (false, Some(Platform::WindowsX64), false) => Some(words::dl_windows_unsure(lang)),
        (false, Some(Platform::LinuxX64), false) => Some(words::dl_linux_unsure(lang)),
        _ => None,
    };
    let body = release_view(cx, note);
    let body = view! {
        <section class="page-head wrap">
            <p class="eyebrow">{words::dl_eyebrow(lang)}</p>
            <h1>{words::dl_title(lang)}</h1>
            <p class="lead">{words::dl_lead(lang)}</p>
        </section>
        <section class="wrap" aria-label=words::dl_all(lang)>{body}</section>
        {install(lang)}
    }
    .into_any();
    (meta, body)
}
