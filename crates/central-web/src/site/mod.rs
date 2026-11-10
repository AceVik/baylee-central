//! The pages, rendered on the server. Every page is complete HTML in the
//! first response: its words, its data and its links. Interactive parts are
//! islands (`crate::islands`); a page without one loads no WebAssembly.

use std::collections::BTreeMap;

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::data::{CardStatus, Gateway, Machine, Release, Repo};
use crate::words::Lang;

mod format;
pub mod icons;
mod markdown;
mod pages;
mod shell;

pub use markdown::render as markdown_blocks;

/// A page of the site, by its path under the language prefix.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Page {
    /// `/{lang}/`.
    Home,
    /// `/{lang}/download`.
    Download,
    /// `/{lang}/gateways`.
    Gateways,
    /// `/{lang}/faq`.
    Faq,
    /// `/{lang}/contact`.
    Contact,
    /// `/{lang}/impressum`.
    Impressum,
    /// `/{lang}/privacy`.
    Privacy,
    /// `/{lang}/terms`.
    Terms,
    /// `/{lang}/fan-content`.
    FanContent,
    /// Any path the site does not have (404).
    NotFound,
    /// A path the site had and removed for good (410).
    Gone,
}

impl Page {
    /// Every page with an address, in the sitemap's order.
    pub const LISTED: [Page; 9] = [
        Page::Home,
        Page::Download,
        Page::Gateways,
        Page::Faq,
        Page::Contact,
        Page::Impressum,
        Page::Privacy,
        Page::Terms,
        Page::FanContent,
    ];

    /// The path after the language prefix, without a leading slash; the
    /// home page's is empty.
    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Page::Home | Page::NotFound | Page::Gone => "",
            Page::Download => "download",
            Page::Gateways => "gateways",
            Page::Faq => "faq",
            Page::Contact => "contact",
            Page::Impressum => "impressum",
            Page::Privacy => "privacy",
            Page::Terms => "terms",
            Page::FanContent => "fan-content",
        }
    }

    /// The page at a slug.
    #[must_use]
    pub fn from_slug(slug: &str) -> Option<Page> {
        Page::LISTED.into_iter().find(|page| page.slug() == slug)
    }

    /// Whether search engines may index it (the Impressum is not, as before).
    #[must_use]
    pub const fn indexed(self) -> bool {
        !matches!(self, Page::Impressum | Page::NotFound | Page::Gone)
    }

    /// The page's absolute path in `lang`, `/de/download`.
    #[must_use]
    pub fn path(self, lang: Lang) -> String {
        format!("/{}/{}", lang.code(), self.slug())
    }
}

/// One asset as the build wrote it: its hashed address and its
/// Subresource Integrity value.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Asset {
    /// `/assets/site.3f2a….css`.
    pub url: String,
    /// `sha384-…`.
    pub integrity: String,
}

/// The build's asset manifest (`cargo xtask site`): logical name to asset.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Assets {
    /// By logical name, `site.css`, `img/world-1200.avif`.
    pub files: BTreeMap<String, Asset>,
}

impl Assets {
    /// The address of `name`, or the unhashed `/assets/<name>` when the
    /// build did not write it (the tests check that every name exists).
    #[must_use]
    pub fn url(&self, name: &str) -> String {
        self.files
            .get(name)
            .map_or_else(|| format!("/assets/{name}"), |asset| asset.url.clone())
    }

    /// The integrity value of `name`, empty when unknown.
    #[must_use]
    pub fn integrity(&self, name: &str) -> String {
        self.files
            .get(name)
            .map(|asset| asset.integrity.clone())
            .unwrap_or_default()
    }

    /// Whether the build wrote `name`.
    #[must_use]
    pub fn has(&self, name: &str) -> bool {
        self.files.contains_key(name)
    }
}

/// The public data the server read, each part absent until read once.
#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    /// The newest release.
    pub release: Option<Release>,
    /// Its card status.
    pub cards: Option<CardStatus>,
    /// Central's directory, official first.
    pub gateways: Option<Vec<Gateway>>,
    /// The repository's state.
    pub repo: Repo,
}

/// What a page needs besides its own words.
pub struct Context<'a> {
    /// The page's language.
    pub lang: Lang,
    /// The site's origin, `https://baylee.app`, for canonical addresses.
    pub origin: &'a str,
    /// The build's assets.
    pub assets: &'a Assets,
    /// The public data.
    pub data: &'a Snapshot,
    /// The visitor's machine, from the request.
    pub machine: Machine,
    /// A search-console verification value from the deployment, if any.
    pub verification: Option<&'a str>,
}

/// Renders `page` to a complete HTML document.
#[must_use]
pub fn render(page: Page, cx: &Context<'_>) -> String {
    let owner = Owner::new();
    let body = owner.with(|| pages::document(page, cx).to_html());
    format!("<!doctype html>{body}")
}
