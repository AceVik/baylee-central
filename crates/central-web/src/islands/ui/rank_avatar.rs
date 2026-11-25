//! Reusable avatar disc paired with the player's highest current-season
//! ranked border, rendered via `badges.js` (WebGL2 shader + 2D fallback) and
//! matching Baylee's exact tier cutout scale ratios.

use std::collections::BTreeMap;

use central_api::picture::PictureKey;
use central_api::ranked::{League, Rank};
use leptos::prelude::*;

use super::super::central;
use super::super::chat::model::{hue, initials};
use super::super::picture;
use super::super::session;

/// Pixel size presets for ranked avatar discs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AvatarSize {
    /// Medium avatar (`64px`, player preview overlay).
    Medium,
    /// Large avatar (`96px`, overview / profile / public player).
    Large,
}

impl AvatarSize {
    const fn px(self) -> u32 {
        match self {
            Self::Medium => 64,
            Self::Large => 96,
        }
    }

    const fn class(self) -> &'static str {
        match self {
            Self::Medium => "rank-avatar rank-avatar-md",
            Self::Large => "rank-avatar rank-avatar-lg",
        }
    }
}

/// Reactive map of peer tags to their highest current-season player rank,
/// shared via Leptos context (`provide` + `read`) just like [`picture::Keys`].
#[derive(Clone, Copy)]
pub struct Ranks(RwSignal<BTreeMap<u32, Option<Rank>>>);

impl Ranks {
    /// Provides an empty [`Ranks`] map in the current reactive scope.
    pub fn provide() -> Self {
        let ranks = Self(RwSignal::new(BTreeMap::new()));
        provide_context(ranks);
        ranks
    }

    /// Queries Baylee Central for the highest current-season ranks of `tags`.
    pub fn read(self, mut tags: Vec<u32>) {
        tags.sort_unstable();
        tags.dedup();
        tags.truncate(central_api::picture::MAX_PICTURE_TAGS);
        if tags.is_empty() {
            return;
        }
        leptos::task::spawn_local(async move {
            if let Ok(list) = central::social_ranks(tags).await {
                self.0.update(|held| {
                    for entry in list {
                        held.insert(entry.tag, Some(entry.rank));
                    }
                });
            }
        });
    }
}

/// Reads a peer's cached highest current-season rank from context, if loaded.
#[must_use]
pub fn rank_of(tag: u32) -> Option<Rank> {
    use_context::<Ranks>().and_then(|ranks| ranks.0.with(|held| held.get(&tag).copied().flatten()))
}

/// CSS cutout scaling class matching `Baylee`'s `RankArt::portrait_scale()`.
#[must_use]
pub const fn rank_scale_class(rank: Option<&Rank>) -> &'static str {
    let Some(rank) = rank else {
        return "";
    };
    match rank.league {
        League::Paper | League::Wood => "rank-scale-p",
        League::Iron | League::Bronze | League::Silver => "rank-scale-i",
        League::Gold | League::Platinum | League::Diamond => "rank-scale-g",
        League::Master | League::Grandmaster | League::Champion => "rank-scale-m",
        League::Unranked => "rank-scale-u",
    }
}

/// Overlay `<canvas>` driven by `badges.js` for a ranked frame or badge.
pub fn rank_canvas(kind: &'static str, rank: Rank, deck: bool, px: u32) -> AnyView {
    let (tier, div, podium) = session::rank_parts(rank);
    let track = if deck { "1" } else { "0" };
    let cls = if kind == "frame" {
        "rank-frame-overlay"
    } else {
        "rank-badge-canvas"
    };
    view! {
        <canvas
            class=cls
            width=px
            height=px
            aria-hidden="true"
            data-rank-kind=kind
            data-rank-tier=tier.to_string()
            data-rank-div=div.to_string()
            data-rank-podium=podium.to_string()
            data-rank-track=track
        ></canvas>
    }
    .into_any()
}

/// Avatar disc (`picture` or monogram initials) framed with `rank`'s shader
/// border.
pub fn rank_avatar(
    display_name: String,
    seed: &str,
    picture_key: Option<(PictureKey, bool)>,
    rank: Option<Rank>,
    size: AvatarSize,
) -> AnyView {
    let px = size.px();
    let scale = rank_scale_class(rank.as_ref());
    let outer_class = if scale.is_empty() {
        size.class().to_owned()
    } else {
        format!("{} has-rank {scale}", size.class())
    };
    let plate_content: AnyView = match picture_key {
        Some((key, moving)) => picture::face(&key, moving, display_name).into_any(),
        None => {
            view! { <span class="rank-avatar-initials">{initials(&display_name)}</span> }.into_any()
        }
    };
    let overlay = rank.map(|r| rank_canvas("frame", r, false, px));
    view! {
        <span class=outer_class aria-hidden="true">
            <span class="rank-avatar-plate" data-hue=hue(seed).to_string()>
                {plate_content}
            </span>
            {overlay}
        </span>
    }
    .into_any()
}
