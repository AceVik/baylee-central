//! Clean-URL public player showcase island (`/{lang}/player/{Name}-{tag}`).

use central_api::profile::{PublicProfile, tag_text};
use central_api::ranked::Track;
use leptos::prelude::*;

use crate::words::{self, Lang};

#[cfg(target_arch = "wasm32")]
use super::central;
use super::ui::badges::{badge_wall, rank_title};
use super::ui::copy_handle::handle_with_copy;
use super::ui::rank_avatar::{AvatarSize, rank_avatar};

#[cfg_attr(all(not(target_arch = "wasm32"), not(test)), allow(dead_code))]
fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(h), Some(l)) = (
                char::from(bytes[i + 1]).to_digit(16),
                char::from(bytes[i + 2]).to_digit(16),
            )
        {
            let byte = u8::try_from((h << 4) | l).unwrap_or(0);
            out.push(byte);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| input.to_owned())
}

/// Converts a clean URL slug (`Name-0001` or `Name%20Two-0001`) into
/// `Name#0001` suitable for `Request::PublicProfile`.
#[cfg_attr(all(not(target_arch = "wasm32"), not(test)), allow(dead_code))]
#[must_use]
pub fn handle_from_path(pathname: &str) -> Option<String> {
    let (_, slug) = pathname.rsplit_once("/player/")?;
    let slug = slug.trim_end_matches('/');
    let (raw_name, raw_tag) = slug.rsplit_once('-')?;
    if !(4..=8).contains(&raw_tag.len()) || !raw_tag.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let name = percent_decode(raw_name);
    if name.is_empty() {
        return None;
    }
    Some(format!("{name}#{}", raw_tag.to_ascii_lowercase()))
}

fn player_card(lang: Lang, profile: PublicProfile) -> AnyView {
    let tag_hex = tag_text(profile.tag);
    let pic = profile.picture.as_ref().map(|p| (p.key, p.moving));
    let seed = format!("{}#{tag_hex}", profile.display_name);
    let career_games: u32 = profile
        .badges
        .ranked_standings
        .iter()
        .filter(|s| s.track == Track::Player)
        .map(|s| s.career.games)
        .sum();
    let career_wins: u32 = profile
        .badges
        .ranked_standings
        .iter()
        .filter(|s| s.track == Track::Player)
        .map(|s| s.career.wins)
        .sum();
    let unlocked = profile.badges.ranked_standings.len()
        + profile.badges.ranked_awards.len()
        + profile
            .badges
            .challenge_badges
            .iter()
            .filter(|b| b.tier.is_some())
            .count()
        + profile
            .badges
            .tower_badges
            .iter()
            .filter(|b| b.tier.is_some())
            .count()
        + profile
            .badges
            .echoes_badges
            .iter()
            .filter(|b| b.tier.is_some())
            .count()
        + profile
            .badges
            .social_badges
            .iter()
            .filter(|b| b.tier.is_some())
            .count();
    let best_rank = profile.badges.best_rank.map_or_else(
        || words::rank_league_unranked(lang).to_owned(),
        |r| rank_title(lang, r),
    );
    let wall = badge_wall(lang, &profile.badges);

    view! {
        <div class="stack">
            <section class="card player-hero">
                <div class="player-hero-head">
                    {rank_avatar(
                        profile.display_name.clone(),
                        &seed,
                        pic,
                        profile.badges.best_rank,
                        AvatarSize::Large,
                    )}
                    <div class="player-hero-meta">
                        <h1><bdi>{profile.display_name.clone()}</bdi></h1>
                        {handle_with_copy(lang, profile.display_name, tag_hex, false)}
                    </div>
                </div>
                <dl class="overview-stats">
                    <div>
                        <dt>{words::player_best_rank(lang)}</dt>
                        <dd>{best_rank}</dd>
                    </div>
                    <div>
                        <dt>{words::player_career_games(lang)}</dt>
                        <dd translate="no">{career_games.to_string()}</dd>
                    </div>
                    <div>
                        <dt>{words::player_career_wins(lang)}</dt>
                        <dd translate="no">{career_wins.to_string()}</dd>
                    </div>
                    <div>
                        <dt>{words::player_badges_count(lang)}</dt>
                        <dd translate="no">{unlocked.to_string()}</dd>
                    </div>
                </dl>
            </section>
            <section class="card">
                <h2>{words::badges_title(lang)}</h2>
                {wall}
            </section>
        </div>
    }
    .into_any()
}

/// Public player profile showcase mounted on `/{lang}/player/{Name}-{tag}`.
#[island]
pub fn PlayerProfile(lang: Lang) -> impl IntoView {
    let state = RwSignal::new(None::<Option<PublicProfile>>);

    #[cfg(target_arch = "wasm32")]
    Effect::new(move |_| {
        let path = web_sys::window()
            .and_then(|w| w.location().pathname().ok())
            .unwrap_or_default();
        let Some(handle) = handle_from_path(&path) else {
            state.set(Some(None));
            return;
        };
        leptos::task::spawn_local(async move {
            let loaded = central::public_profile(&handle).await.ok();
            state.set(Some(loaded));
        });
    });

    view! {
        <div class="stack">
            {move || match state.get() {
                None => view! {
                    <section class="card">
                        <p class="meta">{words::account_loading(lang)}</p>
                    </section>
                }.into_any(),
                Some(None) => view! {
                    <section class="card">
                        <h1>{words::player_private_title(lang)}</h1>
                        <p class="meta">{words::player_private_lead(lang)}</p>
                    </section>
                }.into_any(),
                Some(Some(profile)) => player_card(lang, profile),
            }}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_clean_player_handle_from_url_path() {
        assert_eq!(
            handle_from_path("/de/player/Synthetic-0001"),
            Some("Synthetic#0001".to_owned())
        );
        assert_eq!(
            handle_from_path("/en/player/Player%20One-ABCD/"),
            Some("Player One#abcd".to_owned())
        );
        assert_eq!(handle_from_path("/de/player/NoTag"), None);
    }
}
