//! What the AI players' page's forms hold as typed, and the checks by
//! Central's own rules (`AiPlayerSettings::valid`, `AiSettings::valid`)
//! that a form passes before anything is sent.

use central_api::admin::{AdminAiPlayer, AiPlayerSettings, AiSettings};
use central_api::ranked::ai::{AiKind, MAX_CAROUSEL_GAMES, MAX_GAMES, MAX_GLOBAL_SEATS};

use crate::words::{self, Lang};

/// An AI player's fields as typed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Fields {
    pub(super) name: String,
    pub(super) tag: String,
    pub(super) kind: AiKind,
    pub(super) profile: String,
    pub(super) label: String,
    pub(super) max_games: String,
    pub(super) daily: String,
}

impl Fields {
    /// A new AI player's: its name begins with the prefix, its profile
    /// with the kind's family.
    pub(super) fn fresh() -> Self {
        Fields {
            name: "KI-".to_owned(),
            tag: String::new(),
            kind: AiKind::House,
            profile: "house:".to_owned(),
            label: String::new(),
            max_games: "1".to_owned(),
            daily: String::new(),
        }
    }

    /// An AI player's as it is now.
    pub(super) fn of(player: &AdminAiPlayer) -> Self {
        Fields {
            name: player
                .display_name
                .clone()
                .unwrap_or_else(|| player.settings.display_name.clone()),
            tag: String::new(),
            kind: player.settings.kind,
            profile: player.settings.profile.clone(),
            label: player.settings.label.clone(),
            max_games: player.settings.max_games.to_string(),
            daily: player
                .settings
                .daily_games
                .map_or_else(String::new, |games| games.to_string()),
        }
    }

    /// The settings typed, checked by Central's own rules; else the words
    /// for the first field that breaks one, and that field's id (`prefix`
    /// and its name).
    pub(super) fn settings(
        &self,
        lang: Lang,
    ) -> Result<AiPlayerSettings, (&'static str, &'static str)> {
        let name = self.name.trim().to_owned();
        if !central_api::profile::ai_name(&name) {
            return Err((words::ai_invalid_name(lang), "name"));
        }
        let label = self.label.trim().to_owned();
        if !central_api::ranked::ai::valid_label(&label) {
            return Err((words::ai_invalid_label(lang), "label"));
        }
        let profile = self.profile.trim().to_owned();
        if !central_api::ranked::ai::ranked_profile(&profile) {
            return Err((words::ai_invalid_profile(lang), "profile"));
        }
        let max_games = match self.max_games.trim().parse::<u8>() {
            Ok(games) if (1..=MAX_GAMES).contains(&games) => games,
            _ => return Err((words::ai_invalid_games(lang), "games")),
        };
        let daily = self.daily.trim();
        let daily_games = if daily.is_empty() {
            None
        } else {
            match daily.parse::<u32>() {
                Ok(games) if games <= 10_000 => Some(games),
                _ => return Err((words::ai_invalid_daily(lang), "daily")),
            }
        };
        let settings = AiPlayerSettings {
            kind: self.kind,
            profile,
            label,
            max_games,
            daily_games,
            display_name: name,
        };
        if !settings.valid() {
            // a language model needs a daily limit; every other rule is above
            return Err((words::ai_invalid_daily(lang), "daily"));
        }
        Ok(settings)
    }

    /// The tag typed: none to draw one, else one Central takes.
    pub(super) fn tag(&self) -> Result<Option<u32>, ()> {
        let text = self.tag.trim();
        if text.is_empty() {
            return Ok(None);
        }
        let hex = text.strip_prefix('#').unwrap_or(text);
        if hex.is_empty() || hex.len() > 8 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(());
        }
        let tag = u32::from_str_radix(hex, 16).map_err(|_| ())?;
        if central_api::profile::valid_tag(tag) {
            Ok(Some(tag))
        } else {
            Err(())
        }
    }
}

/// The settings for every AI as typed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct SettingsFields {
    /// Whether fill-in runs: carried as read, set by the operator's command
    /// for calibration (`docs/ai-players.md` §7).
    pub(super) fill_in: bool,
    pub(super) fill_in_after: String,
    pub(super) drain_minutes: String,
    pub(super) global_seats: String,
    pub(super) host_reserve: String,
    pub(super) carousel_games: String,
    pub(super) carousel_daily: String,
}

impl SettingsFields {
    pub(super) fn of(settings: &AiSettings) -> Self {
        SettingsFields {
            fill_in: settings.fill_in,
            fill_in_after: settings.fill_in_after.to_string(),
            drain_minutes: settings.drain_minutes.to_string(),
            global_seats: settings.global_seats.to_string(),
            host_reserve: settings.host_reserve.to_string(),
            carousel_games: settings.carousel_games.to_string(),
            carousel_daily: settings.carousel_daily.to_string(),
        }
    }

    /// The settings typed, each in its range; else the id of the first
    /// field outside it.
    pub(super) fn settings(&self) -> Result<AiSettings, &'static str> {
        fn number<T: std::str::FromStr>(text: &str, id: &'static str) -> Result<T, &'static str> {
            text.trim().parse().map_err(|_| id)
        }
        let settings = AiSettings {
            fill_in: self.fill_in,
            fill_in_after: number(&self.fill_in_after, "ai-fill-in-after")?,
            drain_minutes: number(&self.drain_minutes, "ai-drain")?,
            global_seats: number(&self.global_seats, "ai-global-seats")?,
            host_reserve: number(&self.host_reserve, "ai-host-reserve")?,
            carousel_games: number(&self.carousel_games, "ai-carousel-games")?,
            carousel_daily: number(&self.carousel_daily, "ai-carousel-daily")?,
        };
        if settings.valid() {
            return Ok(settings);
        }
        Err(if !(30..=600).contains(&settings.fill_in_after) {
            "ai-fill-in-after"
        } else if !(30..=600).contains(&settings.drain_minutes) {
            "ai-drain"
        } else if settings.global_seats > MAX_GLOBAL_SEATS {
            "ai-global-seats"
        } else if settings.host_reserve > 64 {
            "ai-host-reserve"
        } else if settings.carousel_games > MAX_CAROUSEL_GAMES {
            "ai-carousel-games"
        } else {
            "ai-carousel-daily"
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn typed() -> Fields {
        Fields {
            name: "KI-Haus-Expert".into(),
            label: "Haus-KI · Expert".into(),
            profile: "house:expert".into(),
            ..Fields::fresh()
        }
    }

    /// A new AI player is checked by Central's rules before anything is
    /// sent, the first broken one said at its field.
    #[test]
    fn an_ai_player_is_checked_before_it_is_sent() {
        let lang = Lang::En;
        let settings = typed().settings(lang).expect("valid");
        assert_eq!(settings.kind, AiKind::House);
        assert_eq!(settings.max_games, 1);
        assert_eq!(settings.daily_games, None);
        assert!(settings.valid());
        let broken = |fields: Fields| fields.settings(lang).map(|_| ()).unwrap_err().1;
        assert_eq!(broken(Fields::fresh()), "name");
        assert_eq!(
            broken(Fields {
                name: "Haus-Expert".into(),
                ..typed()
            }),
            "name",
            "a name without the prefix"
        );
        assert_eq!(
            broken(Fields {
                label: String::new(),
                ..typed()
            }),
            "label"
        );
        assert_eq!(
            broken(Fields {
                profile: "house:seer".into(),
                ..typed()
            }),
            "profile",
            "a profile that sees hidden cards never plays Ranked"
        );
        for games in ["0", "17", "x", ""] {
            assert_eq!(
                broken(Fields {
                    max_games: games.into(),
                    ..typed()
                }),
                "games"
            );
        }
        assert_eq!(
            broken(Fields {
                daily: "10001".into(),
                ..typed()
            }),
            "daily"
        );
        // a language model needs a daily limit
        let model = Fields {
            kind: AiKind::Model,
            profile: "model:haiku".into(),
            ..typed()
        };
        assert_eq!(broken(model.clone()), "daily");
        let limited = Fields {
            daily: "40".into(),
            ..model
        };
        assert_eq!(limited.settings(lang).expect("valid").daily_games, Some(40));
    }

    /// A tag is empty (drawn) or one Central takes.
    #[test]
    fn a_tag_is_drawn_or_typed() {
        let tag = |text: &str| {
            Fields {
                tag: text.into(),
                ..typed()
            }
            .tag()
        };
        assert_eq!(tag(""), Ok(None));
        assert_eq!(tag(" #1a2b "), Ok(Some(0x1a2b)));
        for bad in ["#", "#0", "xyz", "#123456789", "#80000000"] {
            assert_eq!(tag(bad), Err(()), "{bad}");
        }
    }

    /// The settings for every AI are sent whole and in their ranges.
    #[test]
    fn the_settings_are_checked_in_their_ranges() {
        let held = AiSettings {
            fill_in: true,
            fill_in_after: 60,
            drain_minutes: 120,
            global_seats: 8,
            host_reserve: 2,
            carousel_games: 0,
            carousel_daily: 0,
        };
        let fields = SettingsFields::of(&held);
        assert_eq!(fields.settings(), Ok(held));
        let off = |change: fn(&mut SettingsFields)| {
            let mut fields = SettingsFields::of(&held);
            change(&mut fields);
            fields.settings()
        };
        assert_eq!(
            off(|f| f.fill_in_after = "29".into()),
            Err("ai-fill-in-after")
        );
        assert_eq!(off(|f| f.drain_minutes = "601".into()), Err("ai-drain"));
        assert_eq!(
            off(|f| f.global_seats = "33".into()),
            Err("ai-global-seats")
        );
        assert_eq!(
            off(|f| f.global_seats = "-1".into()),
            Err("ai-global-seats")
        );
        assert_eq!(
            off(|f| f.host_reserve = "65".into()),
            Err("ai-host-reserve")
        );
        assert_eq!(
            off(|f| f.carousel_games = "17".into()),
            Err("ai-carousel-games")
        );
        assert_eq!(
            off(|f| f.carousel_daily = "10001".into()),
            Err("ai-carousel-daily")
        );
        assert_eq!(
            off(|f| f.global_seats = "0".into()).map(|s| s.global_seats),
            Ok(0)
        );
        // The most: 32 seats, and the carousel alone fills them.
        assert_eq!(
            off(|f| {
                f.global_seats = "32".into();
                f.carousel_games = "16".into();
            })
            .map(|s| (s.global_seats, s.carousel_games)),
            Ok((32, 16))
        );
    }
}
