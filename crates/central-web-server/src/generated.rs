//! Documents the server writes itself: `robots.txt`, `sitemap.xml`, the web
//! app manifests and the TDM reservation.

use std::fmt::Write as _;

use central_web::site::{Assets, Page, Snapshot};
use central_web::words::{self, Lang};

/// Crawlers that collect training data, shut out entirely (`LICENSE.md`
/// § 4: text and data mining is reserved); search engines stay welcome.
const AI_CRAWLERS: [&str; 10] = [
    "GPTBot",
    "ClaudeBot",
    "anthropic-ai",
    "Google-Extended",
    "Applebot-Extended",
    "CCBot",
    "Bytespider",
    "meta-externalagent",
    "cohere-ai",
    "Amazonbot",
];

/// `robots.txt`.
#[must_use]
pub fn robots(origin: &str) -> String {
    let mut out = String::new();
    for bot in AI_CRAWLERS {
        let _ = writeln!(out, "User-agent: {bot}\nDisallow: /\n");
    }
    out.push_str("User-agent: *\n");
    for lang in Lang::ALL {
        let _ = writeln!(out, "Disallow: /{}/impressum", lang.code());
        let _ = writeln!(out, "Disallow: /{}/account", lang.code());
    }
    out.push_str("Disallow: /site/\nDisallow: /api/\n\n");
    let _ = writeln!(out, "Sitemap: {origin}/sitemap.xml");
    out
}

/// The date a page's content last changed, `YYYY-MM-DD`, where known.
fn changed(page: Page, data: &Snapshot) -> Option<String> {
    match page {
        Page::Download => data.release.as_ref().map(|release| release.date.clone()),
        Page::Home => {
            let release = data.release.as_ref().map(|release| release.date.clone());
            let cards = data.cards.as_ref().map(|cards| cards.date.clone());
            release
                .into_iter()
                .chain(cards)
                .filter(|date| date.len() == 10)
                .max()
        }
        Page::Privacy => Some("2026-11-10".to_owned()),
        _ => None,
    }
    .filter(|date| date.len() == 10)
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// `sitemap.xml`: every indexed page in every language, each naming its
/// translations, with the date its content last changed where known.
#[must_use]
pub fn sitemap(origin: &str, data: &Snapshot) -> String {
    let mut out = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\" xmlns:xhtml=\"http://www.w3.org/1999/xhtml\">\n",
    );
    for page in Page::LISTED.into_iter().filter(|page| page.indexed()) {
        for lang in Lang::ALL {
            let _ = writeln!(
                out,
                "  <url>\n    <loc>{}</loc>",
                escape(&format!("{origin}{}", page.path(lang)))
            );
            for other in Lang::ALL {
                let href = escape(&format!("{origin}{}", page.path(other)));
                let _ = writeln!(
                    out,
                    "    <xhtml:link rel=\"alternate\" hreflang=\"{}\" href=\"{href}\"/>",
                    other.code()
                );
            }
            let href = escape(&format!("{origin}{}", page.path(Lang::En)));
            let _ = writeln!(
                out,
                "    <xhtml:link rel=\"alternate\" hreflang=\"x-default\" href=\"{href}\"/>"
            );
            if let Some(date) = changed(page, data) {
                let _ = writeln!(out, "    <lastmod>{}</lastmod>", escape(&date));
            }
            out.push_str("  </url>\n");
        }
    }
    out.push_str("</urlset>\n");
    out
}

/// The web app manifest in `lang`: the same `id` as before, so an
/// installed app stays the same app; it opens the account in its language.
#[must_use]
pub fn manifest(lang: Lang, assets: &Assets) -> String {
    serde_json::json!({
        "id": "/",
        "name": "Baylee Central",
        "short_name": "Baylee",
        "description": words::desc_home(lang),
        "lang": lang.code(),
        "dir": "ltr",
        "start_url": format!("/{}/account", lang.code()),
        "scope": "/",
        "display": "standalone",
        "background_color": "#0f1b2d",
        "theme_color": "#0f1b2d",
        "icons": [
            {"src": assets.url("img/icon-192.webp"), "sizes": "192x192", "type": "image/webp", "purpose": "any"},
            {"src": assets.url("img/icon-512.webp"), "sizes": "512x512", "type": "image/webp", "purpose": "any"},
            {"src": assets.url("img/icon-maskable-512.webp"), "sizes": "512x512", "type": "image/webp", "purpose": "maskable"}
        ]
    })
    .to_string()
}

/// `/.well-known/tdmrep.json` (W3C TDMRep): mining reserved site-wide.
pub const TDMREP: &str = "[{\"location\":\"/\",\"tdm-reservation\":1}]\n";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn robots_and_sitemap() {
        let robots = robots("https://baylee.app");
        assert!(robots.contains("User-agent: GPTBot\nDisallow: /\n"));
        assert!(robots.contains("Disallow: /de/impressum"));
        assert!(robots.ends_with("Sitemap: https://baylee.app/sitemap.xml\n"));
        let sitemap = sitemap("https://baylee.app", &Snapshot::default());
        assert!(sitemap.contains("<loc>https://baylee.app/de/download</loc>"));
        assert!(sitemap.contains("hreflang=\"x-default\" href=\"https://baylee.app/en/faq\""));
        assert!(
            !sitemap.contains("impressum"),
            "the Impressum is not indexed"
        );
        assert!(sitemap.contains("<lastmod>2026-11-10</lastmod>"));
    }
}
