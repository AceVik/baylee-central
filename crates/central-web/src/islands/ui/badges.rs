//! Ranked, Challenge, Tower, and Echoes badge showcase rendered with WebGL2
//! shaders (`badges.js`). Ranked badges always appear first.

use central_api::challenges::{BadgeKind as ChallengeKind, Tier};
use central_api::profile::PublicBadges;
use central_api::ranked::{AwardKind, League, Rank, Track};
use central_api::social::{RECRUITER_THRESHOLDS, SocialBadge, SocialBadgeKind};
use central_api::tower::BadgeKind as TowerKind;
use central_api::tower_awards::BadgeKind as EchoesKind;
use leptos::prelude::*;

use crate::words::{self, Lang};

use super::rank_avatar::rank_canvas;

/// Human-readable localized name for a ranked [`League`].
#[must_use]
pub fn league_name(lang: Lang, league: League) -> &'static str {
    match league {
        League::Unranked => words::rank_league_unranked(lang),
        League::Paper => words::rank_league_paper(lang),
        League::Wood => words::rank_league_wood(lang),
        League::Iron => words::rank_league_iron(lang),
        League::Bronze => words::rank_league_bronze(lang),
        League::Silver => words::rank_league_silver(lang),
        League::Gold => words::rank_league_gold(lang),
        League::Platinum => words::rank_league_platinum(lang),
        League::Diamond => words::rank_league_diamond(lang),
        League::Master => words::rank_league_master(lang),
        League::Grandmaster => words::rank_league_grandmaster(lang),
        League::Champion => words::rank_league_champion(lang),
    }
}

/// Formats a [`Rank`] as e.g. `"Gold II"` or `"Champion #1"`.
#[must_use]
pub fn rank_title(lang: Lang, rank: Rank) -> String {
    let base = league_name(lang, rank.league);
    if rank.league == League::Champion
        && let Some(place) = rank.champion_place
    {
        return format!("{base} #{place}");
    }
    if let Some(div) = rank.division {
        let roman = match div {
            1 => "I",
            2 => "II",
            3 => "III",
            4 => "IV",
            _ => "",
        };
        if !roman.is_empty() {
            return format!("{base} {roman}");
        }
    }
    base.to_owned()
}

fn tier_name(lang: Lang, tier: Option<Tier>) -> &'static str {
    match tier {
        None => words::badges_locked(lang),
        Some(Tier::Bronze) => words::badge_tier_bronze(lang),
        Some(Tier::Silver) => words::badge_tier_silver(lang),
        Some(Tier::Gold) => words::badge_tier_gold(lang),
        Some(Tier::Platinum) => words::badge_tier_platinum(lang),
        Some(Tier::Obsidian) => words::badge_tier_obsidian(lang),
    }
}

fn tier_num(tier: Option<Tier>) -> &'static str {
    match tier {
        None => "0",
        Some(Tier::Bronze) => "1",
        Some(Tier::Silver) => "2",
        Some(Tier::Gold) => "3",
        Some(Tier::Platinum) => "4",
        Some(Tier::Obsidian) => "5",
    }
}

fn tier_slug(tier: Option<Tier>) -> &'static str {
    match tier {
        None => "locked",
        Some(Tier::Bronze) => "bronze",
        Some(Tier::Silver) => "silver",
        Some(Tier::Gold) => "gold",
        Some(Tier::Platinum) => "platinum",
        Some(Tier::Obsidian) => "obsidian",
    }
}

const fn tier_rank(tier: Option<Tier>) -> u8 {
    match tier {
        None => 0,
        Some(Tier::Bronze) => 1,
        Some(Tier::Silver) => 2,
        Some(Tier::Gold) => 3,
        Some(Tier::Platinum) => 4,
        Some(Tier::Obsidian) => 5,
    }
}

fn format_name(format: central_api::ranked::Format) -> &'static str {
    match format {
        central_api::ranked::Format::Standard => "Standard",
        central_api::ranked::Format::Highlander => "Highlander",
        central_api::ranked::Format::Commander => "Commander",
    }
}

fn insignia_card(
    lang: Lang,
    title: &'static str,
    motif: &'static str,
    tier: Option<Tier>,
    count: u64,
    next: Option<u64>,
) -> AnyView {
    let t_label = tier_name(lang, tier);
    let progress = match next {
        Some(target) => format!("{count} / {target}"),
        None => count.to_string(),
    };
    let locked = if tier.is_none() {
        "badge-card is-locked"
    } else {
        "badge-card"
    };
    let bar = match next {
        Some(target) if target > 0 => Some((
            "badge-bar",
            count.min(target).to_string(),
            target.to_string(),
        )),
        None if tier.is_some() => Some(("badge-bar is-max", "1".to_owned(), "1".to_owned())),
        _ => None,
    }
    .map(|(bar_class, val, max)| {
        view! {
            <progress class=bar_class value=val max=max aria-hidden="true"></progress>
        }
    });
    view! {
        <li class=locked data-tier=tier_slug(tier) title=format!("{title} — {t_label}")>
            <div class="badge-art">
                <canvas
                    class="rank-badge-canvas"
                    width="104"
                    height="104"
                    aria-hidden="true"
                    data-rank-kind="insignia"
                    data-rank-motif=motif
                    data-rank-tier=tier_num(tier)
                ></canvas>
            </div>
            <div class="badge-meta">
                <strong class="badge-title">{title}</strong>
                <span class="badge-sub">{t_label}</span>
                {bar}
                <span class="badge-progress" translate="no">{progress}</span>
            </div>
        </li>
    }
    .into_any()
}

fn ranked_cards_for(lang: Lang, badges: &PublicBadges) -> Vec<AnyView> {
    let mut cards: Vec<AnyView> = Vec::new();
    let mut standings = badges.ranked_standings.clone();
    standings.sort_by_key(|s| {
        (
            u8::from(s.track != Track::Player),
            std::cmp::Reverse(s.rank.league),
            s.rank.division.unwrap_or(99),
            std::cmp::Reverse(s.elo),
        )
    });
    for s in &standings {
        let deck = s.track == Track::Deck;
        let label = rank_title(lang, s.rank);
        let fmt_label = format_name(s.format);
        let track_label = if deck {
            words::badges_deck_track(lang)
        } else {
            words::badges_player_track(lang)
        };
        let elo_line = format!("{} · {} Elo · {}/{}", track_label, s.elo, s.wins, s.games);
        let title_attr = format!("{fmt_label} — {label}");
        cards.push(
            view! {
                <li class="badge-card badge-ranked" title=title_attr>
                    <div class="badge-art">
                        {rank_canvas("badge", s.rank, deck, 112)}
                    </div>
                    <div class="badge-meta">
                        <strong class="badge-title">{label}</strong>
                        <span class="badge-sub" translate="no">{fmt_label}</span>
                        <span class="badge-progress">{elo_line}</span>
                    </div>
                </li>
            }
            .into_any(),
        );
    }

    for award in &badges.ranked_awards {
        let deck = award.track == Track::Deck;
        let label = rank_title(lang, award.rank);
        let fmt_label = format_name(award.format);
        let kind_label = match award.kind {
            AwardKind::Peak => words::badges_peak_award(lang),
            AwardKind::SeasonFinish => words::badges_season_award(lang),
        };
        let title_attr = format!("{kind_label} — {fmt_label}: {label}");
        cards.push(
            view! {
                <li class="badge-card badge-ranked" title=title_attr>
                    <div class="badge-art">
                        {rank_canvas("badge", award.rank, deck, 112)}
                    </div>
                    <div class="badge-meta">
                        <strong class="badge-title">{label}</strong>
                        <span class="badge-sub">{kind_label}</span>
                        <span class="badge-progress" translate="no">{fmt_label}</span>
                    </div>
                </li>
            }
            .into_any(),
        );
    }
    cards
}

struct TrophyEntry {
    title: &'static str,
    motif: &'static str,
    tier: Option<Tier>,
    count: u64,
    next: Option<u64>,
}

fn trophy_entries(lang: Lang, badges: &PublicBadges) -> Vec<TrophyEntry> {
    let mut entries = Vec::with_capacity(
        badges.challenge_badges.len()
            + badges.echoes_badges.len()
            + badges.tower_badges.len()
            + badges.social_badges.len(),
    );
    for b in &badges.social_badges {
        let (title, motif) = match b.kind {
            SocialBadgeKind::Recruiter => (words::badge_recruiter(lang), "recruiter"),
        };
        entries.push(TrophyEntry {
            title,
            motif,
            tier: b.tier,
            count: u64::from(b.count),
            next: b.next.map(u64::from),
        });
    }
    for b in &badges.challenge_badges {
        let (title, motif) = match b.kind {
            ChallengeKind::Daily => (words::badge_daily(lang), "daily"),
            ChallengeKind::Weekly => (words::badge_weekly(lang), "weekly"),
            ChallengeKind::Tactician => (words::badge_tactician(lang), "tactician"),
            ChallengeKind::FirstTry => (words::badge_first_try(lang), "first_try"),
            ChallengeKind::Regular => (words::badge_regular(lang), "regular"),
        };
        entries.push(TrophyEntry {
            title,
            motif,
            tier: b.tier,
            count: u64::from(b.count),
            next: b.next.map(u64::from),
        });
    }
    for b in &badges.echoes_badges {
        let (title, motif) = match b.kind {
            EchoesKind::Victories => (words::badge_echoes_victories(lang), "echoes_victories"),
            EchoesKind::Ascents => (words::badge_echoes_ascents(lang), "echoes_ascents"),
            EchoesKind::Unbroken => (words::badge_echoes_unbroken(lang), "echoes_unbroken"),
        };
        entries.push(TrophyEntry {
            title,
            motif,
            tier: b.tier,
            count: b.count,
            next: b.next,
        });
    }
    for b in &badges.tower_badges {
        let (title, motif) = match b.kind {
            TowerKind::Pathfinder => (words::badge_tower_pathfinder(lang), "tower_pathfinder"),
            TowerKind::Summit => (words::badge_tower_summit(lang), "tower_summit"),
            TowerKind::Unbroken => (words::badge_tower_unbroken(lang), "tower_unbroken"),
        };
        entries.push(TrophyEntry {
            title,
            motif,
            tier: b.tier,
            count: u64::from(b.count),
            next: b.next.map(u64::from),
        });
    }
    entries.sort_by_key(|entry| std::cmp::Reverse(tier_rank(entry.tier)));
    entries
}

/// Full badge & ranked achievement showcase (`Ranked` always rendered first).
pub fn badge_wall(lang: Lang, badges: &PublicBadges) -> AnyView {
    // 1. Ranked section FIRST (player track standings, then awards, then deck standings).
    let ranked_cards = ranked_cards_for(lang, badges);

    // 2. Permanent insignia groups (Challenges, Echoes, Tower), earned tiers first.
    let trophies = trophy_entries(lang, badges);
    if ranked_cards.is_empty() && trophies.is_empty() {
        return view! { <p class="meta">{words::badges_empty(lang)}</p> }.into_any();
    }

    let ranked_group = (!ranked_cards.is_empty()).then(|| {
        let season_note = badges.ranked_season.as_ref().map(|s| s.name.clone());
        view! {
            <div class="badge-group">
                <div class="badge-group-head">
                    <h3>{words::badges_tab_ranked(lang)}</h3>
                    {season_note.map(|n| view! { <span class="badge-season-pill">{n}</span> })}
                </div>
                <ul class="badge-grid">{ranked_cards}</ul>
            </div>
        }
    });

    let total_trophies = trophies.len();
    let earned_trophies = trophies.iter().filter(|t| t.tier.is_some()).count();
    let trophy_cards: Vec<AnyView> = trophies
        .into_iter()
        .map(|t| insignia_card(lang, t.title, t.motif, t.tier, t.count, t.next))
        .collect();
    let trophies_group = (!trophy_cards.is_empty()).then(|| {
        view! {
            <div class="badge-group">
                <div class="badge-group-head">
                    <h3>{words::badges_tab_challenges(lang)}</h3>
                    <span class="badge-season-pill" translate="no">
                        {format!("{earned_trophies} / {total_trophies}")}
                    </span>
                </div>
                <ul class="badge-grid">{trophy_cards}</ul>
            </div>
        }
    });

    view! {
        <div class="badge-wall">
            {ranked_group}
            {trophies_group}
        </div>
    }
    .into_any()
}

fn hero_standing_item(lang: Lang, s: &central_api::ranked::Standing) -> AnyView {
    let deck = s.track == Track::Deck;
    let title = rank_title(lang, s.rank);
    let fmt = format_name(s.format);
    let elo = format!("{} Elo", s.elo);
    let record = format!("{}/{}", s.wins, s.games);
    view! {
        <li class="preview-ranked-item">
            <div class="preview-ranked-art">
                {rank_canvas("badge", s.rank, deck, 72)}
            </div>
            <strong class="preview-ranked-title">{title}</strong>
            <span class="preview-ranked-format" translate="no">{fmt}</span>
            <span class="preview-ranked-elo" translate="no">{elo}</span>
            <span class="preview-ranked-record" translate="no">{record}</span>
        </li>
    }
    .into_any()
}

fn hero_award_item(lang: Lang, award: &central_api::ranked::Award) -> AnyView {
    let deck = award.track == Track::Deck;
    let title = rank_title(lang, award.rank);
    let fmt = format_name(award.format);
    let kind = match award.kind {
        AwardKind::Peak => words::badges_peak_award(lang),
        AwardKind::SeasonFinish => words::badges_season_award(lang),
    };
    view! {
        <li class="preview-ranked-item">
            <div class="preview-ranked-art">
                {rank_canvas("badge", award.rank, deck, 72)}
            </div>
            <strong class="preview-ranked-title">{title}</strong>
            <span class="preview-ranked-format" translate="no">{fmt}</span>
            <span class="preview-ranked-elo">{kind}</span>
        </li>
    }
    .into_any()
}

fn ranked_hero_items(lang: Lang, badges: &PublicBadges) -> (Vec<AnyView>, usize) {
    let mut standings = badges.ranked_standings.clone();
    standings.sort_by_key(|s| {
        (
            u8::from(s.track != Track::Player),
            std::cmp::Reverse(s.rank.league),
            s.rank.division.unwrap_or(99),
            std::cmp::Reverse(s.elo),
        )
    });
    let total_ranked = standings.len() + badges.ranked_awards.len();
    let mut items: Vec<AnyView> = standings
        .iter()
        .take(3)
        .map(|s| hero_standing_item(lang, s))
        .collect();
    if items.len() < 3 {
        let need = 3 - items.len();
        items.extend(
            badges
                .ranked_awards
                .iter()
                .take(need)
                .map(|a| hero_award_item(lang, a)),
        );
    }
    (items, total_ranked)
}

fn earned_trophy_items(lang: Lang, badges: &PublicBadges) -> Vec<AnyView> {
    trophy_entries(lang, badges)
        .into_iter()
        .filter(|entry| entry.tier.is_some())
        .take(6)
        .map(|entry| {
            let label = format!("{} — {}", entry.title, tier_name(lang, entry.tier));
            let kind = "insignia";
            let width = "36";
            view! {
                <li class="preview-trophy-item" data-tier=tier_slug(entry.tier) title=label.clone() aria-label=label>
                    <canvas
                        class="rank-badge-canvas"
                        width=width
                        height=width
                        aria-hidden="true"
                        data-rank-kind=kind
                        data-rank-motif=entry.motif
                        data-rank-tier=tier_num(entry.tier)
                    ></canvas>
                </li>
            }
            .into_any()
        })
        .collect()
}

/// Compact showcase used inside `@User` preview overlays: puts up to three
/// current-season or award Ranked Abzeichen front and center, followed by a
/// compact row of earned trophy insignias.
pub fn preview_badges_showcase(lang: Lang, badges: &PublicBadges) -> AnyView {
    let (hero_items, total_ranked) = ranked_hero_items(lang, badges);
    let extra_ranked = total_ranked.saturating_sub(hero_items.len());
    let ranked_section = (!hero_items.is_empty()).then(|| {
        let season_note = badges.ranked_season.as_ref().map(|s| s.name.clone());
        view! {
            <section class="preview-ranked-hero">
                <div class="preview-ranked-head">
                    <span class="preview-ranked-kicker">{words::chat_user_ranked(lang)}</span>
                    {season_note.map(|n| view! { <span class="badge-season-pill">{n}</span> })}
                    {(extra_ranked > 0).then(|| view! {
                        <span class="pill" translate="no">{format!("+{extra_ranked}")}</span>
                    })}
                </div>
                <ul class="preview-ranked-grid">{hero_items}</ul>
            </section>
        }
    });

    let earned_trophies = earned_trophy_items(lang, badges);
    let trophies_strip = (!earned_trophies.is_empty()).then(|| {
        view! { <ul class="preview-trophies">{earned_trophies}</ul> }
    });

    if ranked_section.is_none() && trophies_strip.is_none() {
        return view! { <p class="meta">{words::badges_empty(lang)}</p> }.into_any();
    }

    view! {
        <div class="preview-badges-wrap">
            {ranked_section}
            {trophies_strip}
        </div>
    }
    .into_any()
}

const fn next_tier_for(tier: Option<Tier>) -> Option<Tier> {
    match tier {
        None => Some(Tier::Bronze),
        Some(Tier::Bronze) => Some(Tier::Silver),
        Some(Tier::Silver) => Some(Tier::Gold),
        Some(Tier::Gold) => Some(Tier::Platinum),
        Some(Tier::Platinum) => Some(Tier::Obsidian),
        Some(Tier::Obsidian) => None,
    }
}

/// Ingress-inspired tiered medal banner shown inside the Invite Friends modal.
pub fn recruiter_medal_banner(lang: Lang, badge: &SocialBadge) -> AnyView {
    let tier = badge.tier;
    let tier_label = tier_name(lang, tier);
    let count_str = badge.count.to_string();
    let status_text = match (badge.next, next_tier_for(tier)) {
        (Some(target), Some(upcoming)) => words::invite_badge_next(
            lang,
            &count_str,
            &target.to_string(),
            tier_name(lang, Some(upcoming)),
        ),
        _ => words::invite_badge_max(lang, &count_str),
    };
    let bar = match badge.next {
        Some(target) if target > 0 => (
            "badge-bar invite-medal-bar",
            badge.count.min(target).to_string(),
            target.to_string(),
        ),
        _ => (
            "badge-bar invite-medal-bar is-max",
            "1".to_owned(),
            "1".to_owned(),
        ),
    };
    let tiers = [
        Tier::Bronze,
        Tier::Silver,
        Tier::Gold,
        Tier::Platinum,
        Tier::Obsidian,
    ];
    let pips: Vec<AnyView> = RECRUITER_THRESHOLDS
        .into_iter()
        .zip(tiers)
        .map(|(threshold, step_tier)| {
            let earned = badge.count >= threshold;
            let current = tier == Some(step_tier);
            let pip_label = format!("{}: {threshold}", tier_name(lang, Some(step_tier)));
            view! {
                <span
                    class="invite-medal-pip"
                    data-tier=tier_slug(Some(step_tier))
                    data-earned=earned.then_some("true")
                    data-current=current.then_some("true")
                    title=pip_label.clone()
                    aria-label=pip_label
                    translate="no"
                >
                    {threshold}
                </span>
            }
            .into_any()
        })
        .collect();

    view! {
        <section
            class="invite-medal-banner"
            data-tier=tier_slug(tier)
            aria-label=words::invite_badge_title(lang)
        >
            <div class="invite-medal-art">
                <canvas
                    class="rank-badge-canvas"
                    width="84"
                    height="84"
                    aria-hidden="true"
                    data-rank-kind="insignia"
                    data-rank-motif="recruiter"
                    data-rank-tier=tier_num(tier)
                ></canvas>
            </div>
            <div class="invite-medal-body">
                <div class="invite-medal-head">
                    <strong class="invite-medal-title">{words::invite_badge_title(lang)}</strong>
                    <span class="invite-medal-tier">{tier_label}</span>
                </div>
                <progress class=bar.0 value=bar.1 max=bar.2 aria-hidden="true"></progress>
                <div class="invite-medal-foot">
                    <span class="invite-medal-sub">{status_text}</span>
                    <div class="invite-medal-pips">{pips}</div>
                </div>
            </div>
        </section>
    }
    .into_any()
}
