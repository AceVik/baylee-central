//! The pages, rendered on the server. Every page is complete HTML in the
//! first response: its words, its data and its links. Interactive parts are
//! islands (`crate::islands`); a page without one loads no WebAssembly.

use std::collections::BTreeMap;

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::data::{CardStatus, Gateway, Machine, Release, Repo, Section};
use crate::words::Lang;

pub mod icons;
mod pages;
mod shell;
mod strip;

/// A page of the site, by its path under the language prefix.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Page {
    /// `/{lang}/`.
    Home,
    /// `/{lang}/download`.
    Download,
    /// `/{lang}/gateways`.
    Gateways,
    /// `/{lang}/donate`: the monthly goal and the places to give.
    Donate,
    /// `/{lang}/faq`.
    Faq,
    /// `/{lang}/security`.
    Security,
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
    /// `/{lang}/signin`.
    SignIn,
    /// `/{lang}/register`.
    Register,
    /// `/{lang}/reset`: a new password with an administrator's reset code.
    Reset,
    /// `/{lang}/account…`: the signed-in area.
    Account(Section),
    /// `/{lang}/player/{handle}`: a public player profile.
    Player,
    /// `/{lang}/invite/{code}`: a personal friend invitation.
    Invite,
    /// Any path the site does not have (404).
    NotFound,
    /// A path the site had and removed for good (410).
    Gone,
    /// `/{lang}/offline`: what the service worker shows without a network.
    Offline,
}

impl Page {
    /// Every page with an address, in the sitemap's order.
    pub const LISTED: [Page; 11] = [
        Page::Home,
        Page::Download,
        Page::Gateways,
        Page::Faq,
        Page::Security,
        Page::Contact,
        Page::Donate,
        Page::Impressum,
        Page::Privacy,
        Page::Terms,
        Page::FanContent,
    ];

    /// The slugs of pages the site had and removed for good; each answers
    /// as [`Page::Gone`] in every language. `challenges`: Daily and Weekly
    /// tasks, the Tower and their rankings are the game client's to show.
    pub const REMOVED: [&'static str; 1] = ["challenges"];

    /// The path after the language prefix, without a leading slash; the
    /// home page's is empty.
    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Page::Home | Page::NotFound | Page::Gone => "",
            Page::Offline => "offline",
            Page::SignIn => "signin",
            Page::Register => "register",
            Page::Reset => "reset",
            Page::Account(section) => section.slug(),
            Page::Download => "download",
            Page::Gateways => "gateways",
            Page::Faq => "faq",
            Page::Security => "security",
            Page::Contact => "contact",
            Page::Donate => "donate",
            Page::Impressum => "impressum",
            Page::Privacy => "privacy",
            Page::Terms => "terms",
            Page::FanContent => "fan-content",
            Page::Player => "player",
            Page::Invite => "invite",
        }
    }

    /// Whether `slug` matches a public player profile path `player/{name}-{4..8 hex}`.
    fn is_player_slug(slug: &str) -> bool {
        let Some(handle) = slug.strip_prefix("player/") else {
            return false;
        };
        if handle.is_empty() || handle.contains('/') {
            return false;
        }
        let Some((name, tag)) = handle.rsplit_once('-') else {
            return false;
        };
        !name.is_empty()
            && (4..=8).contains(&tag.len())
            && tag.bytes().all(|b| b.is_ascii_hexdigit())
    }

    /// Whether `slug` matches a friend invitation path `invite/{code}`.
    fn is_invite_slug(slug: &str) -> bool {
        let Some(code) = slug.strip_prefix("invite/") else {
            return false;
        };
        !code.is_empty()
            && !code.contains('/')
            && code.len() <= 64
            && code
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    }

    /// The page at a slug.
    #[must_use]
    pub fn from_slug(slug: &str) -> Option<Page> {
        // one report of the moderation: its id is the island's to read
        if slug
            .strip_prefix("account/admin/feedback/")
            .is_some_and(|id| {
                id.len() == 32 && id.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
            })
        {
            return Some(Page::Account(Section::Report));
        }
        if Self::is_player_slug(slug) {
            return Some(Page::Player);
        }
        if Self::is_invite_slug(slug) {
            return Some(Page::Invite);
        }
        if Page::REMOVED.contains(&slug) {
            return Some(Page::Gone);
        }
        Page::LISTED
            .into_iter()
            .chain([Page::SignIn, Page::Register, Page::Reset, Page::Offline])
            .chain(Section::ROUTED.map(Page::Account))
            .find(|page| page.slug() == slug)
            .or_else(|| Section::account_page(slug).map(Page::Account))
    }

    /// Whether search engines may index it: not the Impressum (as before),
    /// not the forms and the signed-in area, not the error pages.
    #[must_use]
    pub const fn indexed(self) -> bool {
        !matches!(
            self,
            Page::Impressum
                | Page::NotFound
                | Page::Gone
                | Page::Offline
                | Page::SignIn
                | Page::Register
                | Page::Reset
                | Page::Account(_)
                | Page::Player
                | Page::Invite
        )
    }

    /// The page's absolute path in `lang`, `/de/download`; one account's
    /// page in the administration with the account's id.
    #[must_use]
    pub fn path(self, lang: Lang) -> String {
        match self {
            Page::Account(Section::AdminUser(id)) => format!(
                "/{}/{}/{}",
                lang.code(),
                self.slug(),
                crate::data::id_hex(&id)
            ),
            _ => format!("/{}/{}", lang.code(), self.slug()),
        }
    }

    /// The form that stands open at this address: signing in, registering
    /// and setting a new password are dialogs over any page, and their own
    /// addresses show the start page with that dialog open (`boot.js`).
    #[must_use]
    pub const fn form(self) -> Option<&'static str> {
        match self {
            Page::SignIn => Some("signin"),
            Page::Register => Some("register"),
            Page::Reset => Some("reset"),
            _ => None,
        }
    }

    /// Whether the page shows the start page: itself, or one of the three
    /// forms over it.
    #[must_use]
    pub const fn start(self) -> bool {
        matches!(self, Page::Home) || self.form().is_some()
    }

    /// Whether the server keeps the page once rendered: every page but one
    /// account's in the administration, whose address the visitor chooses
    /// (a kept page per id would let requests fill the table), and public
    /// player profiles.
    #[must_use]
    pub const fn kept(self) -> bool {
        !matches!(
            self,
            Page::Account(Section::AdminUser(_)) | Page::Player | Page::Invite
        )
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
    /// What this build is called, for the footer's last line.
    #[serde(default)]
    pub build: Build,
}

/// What a build of the site is called (`cargo xtask site`): the release it
/// descends from and the commit it was made of, as far as the build could
/// tell (a build outside a checkout knows neither).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Build {
    /// The digest of every asset's address, as the service worker carries
    /// it; empty where no build wrote the assets.
    #[serde(default)]
    pub id: String,
    /// The newest release tag the commit descends from, `v0.1.1-beta.9b`.
    #[serde(default)]
    pub version: Option<String>,
    /// How many commits it is past that tag.
    #[serde(default)]
    pub ahead: u32,
    /// The commit, forty hexadecimal digits.
    #[serde(default)]
    pub commit: Option<String>,
    /// The commit's day, `YYYY-MM-DD`.
    #[serde(default)]
    pub date: Option<String>,
}

impl Build {
    /// The commit where it is one: forty lowercase hexadecimal digits, so
    /// that nothing else is ever made part of an address.
    #[must_use]
    pub fn commit(&self) -> Option<&str> {
        self.commit
            .as_deref()
            .filter(|c| c.len() == 40 && c.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')))
    }

    /// The version as the footer writes it: the tag, and how far past it
    /// the build is (`v0.1.1-beta.9b+37`).
    #[must_use]
    pub fn version(&self) -> Option<String> {
        let tag = self.version.as_deref().filter(|tag| {
            !tag.is_empty()
                && tag.len() <= 48
                && tag
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_' | b'+'))
        })?;
        Some(if self.ahead == 0 {
            tag.to_owned()
        } else {
            format!("{tag}+{}", self.ahead)
        })
    }

    /// The commit's day where it is one (`YYYY-MM-DD`).
    #[must_use]
    pub fn date(&self) -> Option<&str> {
        self.date.as_deref().filter(|d| {
            d.len() == 10
                && d.bytes().enumerate().all(|(at, b)| match at {
                    4 | 7 => b == b'-',
                    _ => b.is_ascii_digit(),
                })
        })
    }
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
            .map_or_default(|asset| asset.integrity.clone())
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
    /// What the donation platforms say of the monthly goal.
    pub funding: Option<crate::data::Funding>,
    /// The newest release.
    pub release: Option<Release>,
    /// Its card status.
    pub cards: Option<CardStatus>,
    /// Central's directory, in its order (the origin Central calls official
    /// first, unmarked).
    pub gateways: Option<Vec<Gateway>>,
    /// The repository's state.
    pub repo: Repo,
    /// The accounts that can sign in, as Central counts them about once a
    /// minute (its public figures).
    pub accounts: Option<u64>,
    /// How often the site's pages were opened, where the deployment counts
    /// it (`central-web-server`'s `views`), as it stood at the last minute.
    pub views: Option<crate::data::Views>,
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
