//! AI players (`docs/ai-players.md`): accounts an administrator creates as AIs,
//! which fill a lonely person's search and play each other in the carousel. A
//! person consents per search; every reply that names an AI to a client of
//! protocol 3 carries its badge.
use super::{Assignment, Format};
use serde::{Deserialize, Serialize};

/// Which AI opponents a search accepts. Absent on the wire it is
/// [`Self::None`], so a client built before AI players, which cannot show a
/// badge, is never seated against an AI.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum AiOpponents {
    /// People only ("Nur Menschen").
    #[default]
    None,
    /// House levels and the trained net: the person's moves stay on the
    /// operator's servers.
    Local,
    /// Also language models, whose providers receive the person's moves.
    Any,
}

impl AiOpponents {
    /// Whether the search accepts no AI at all.
    #[must_use]
    pub const fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    /// Stable storage key; `None` is stored as no row.
    #[must_use]
    pub const fn as_str(self) -> Option<&'static str> {
        match self {
            Self::None => None,
            Self::Local => Some("local"),
            Self::Any => Some("any"),
        }
    }
}

/// Whom a public board lists (protocol 3; absent on the wire it is
/// [`Self::Everyone`], as before).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum BoardPlayers {
    /// People and AI players.
    #[default]
    Everyone,
    /// People only.
    People,
    /// AI players only.
    Ai,
}

impl BoardPlayers {
    /// Whether the board lists everyone, which the wire leaves out.
    #[must_use]
    pub const fn is_everyone(&self) -> bool {
        matches!(self, Self::Everyone)
    }
}

/// What kind of mind an AI player is. A kind a later Central adds reads as
/// [`Self::Other`], so it costs a client nothing but the kind's own icon.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiKind {
    /// A house heuristic level.
    House,
    /// A language model.
    Model,
    /// The trained net.
    Net,
    /// A kind this build does not know.
    #[serde(other)]
    Other,
}

impl AiKind {
    /// Stable storage key; [`Self::Other`] is never stored.
    #[must_use]
    pub const fn as_str(self) -> Option<&'static str> {
        match self {
            Self::House => Some("house"),
            Self::Model => Some("model"),
            Self::Net => Some("net"),
            Self::Other => None,
        }
    }

    /// The kind a stored key names.
    #[must_use]
    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "house" => Some(Self::House),
            "model" => Some(Self::Model),
            "net" => Some(Self::Net),
            _ => None,
        }
    }

    /// Whether playing it costs nothing but the operator's CPU.
    #[must_use]
    pub const fn free(self) -> bool {
        matches!(self, Self::House | Self::Net)
    }
}

/// The label every client shows beside an AI account's name. Every ranked
/// AI is fair, so the badge says nothing of fairness. Only Central
/// sends it, so it reads past a field a later Central adds (no
/// `deny_unknown_fields`, like the push socket's table).
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct AiBadge {
    /// The kind of mind.
    pub kind: AiKind,
    /// The administrator's public label, such as "Haus-KI · Expert".
    pub label: String,
}

/// Profiles that see hidden cards. They never play Ranked: every ranked AI
/// is a fair remote seat (owner, 26.11.2026), so none is created with one.
pub const UNFAIR_PROFILES: [&str; 1] = ["house:seer"];

/// Whether `profile` may play Ranked: a valid key that is not unfair.
#[must_use]
pub fn ranked_profile(profile: &str) -> bool {
    valid_profile(profile) && !UNFAIR_PROFILES.contains(&profile)
}

/// Most characters in an AI's label.
pub const LABEL_CHARS: usize = 40;

/// Whether `label` may be an AI's label: 1 to [`LABEL_CHARS`] characters,
/// no control character, no surrounding space.
#[must_use]
pub fn valid_label(label: &str) -> bool {
    let count = label.chars().count();
    (1..=LABEL_CHARS).contains(&count)
        && label.trim() == label
        && !label.chars().any(char::is_control)
}

/// Whether `profile` may be a runner's key for a mind, such as `house:expert`.
#[must_use]
pub fn valid_profile(profile: &str) -> bool {
    let Some((family, level)) = profile.split_once(':') else {
        return false;
    };
    (1..=12).contains(&family.len())
        && family.bytes().all(|b| b.is_ascii_lowercase())
        && (1..=48).contains(&level.len())
        && level.bytes().all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'.' | b'-')
        })
}

/// One standing offer: the frozen deck an AI plays in a format.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Offer {
    /// Competitive format.
    pub format: Format,
    /// Commitment to the frozen playable deck, as in Join.
    pub deck: [u8; 32],
}

/// Most offers at once: one for each format.
pub const MAX_OFFERS: usize = 3;

/// The most matches one AI plays at once, whatever its `max_games`: the
/// bound of [`AiPlayerSettings`](crate::admin::AiPlayerSettings) and of
/// [`Response::Work`]'s list, which an administrator's lower `max_games`
/// does not shorten while more are live.
pub const MAX_GAMES: u8 = 16;

/// The runner's operations, on a native session of an AI account.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Request {
    /// Replace the account's offers with these, at most one per format; an
    /// empty list withdraws them. Refused while the AI is switched off.
    Offer {
        /// The offers.
        offers: Vec<Offer>,
    },
    /// The account's AI settings and its live matches.
    Work,
    /// A match against another AI in this format, made now if every rule
    /// allows it (the carousel, `docs/ai-players.md` section 7): Central
    /// pairs the account's offer for the format with another AI's, only
    /// while no person waits in it and every cap has room.
    Carousel {
        /// Competitive format of one of the account's offers.
        format: Format,
    },
}

/// Why a carousel request made no match: no more than the runner needs to
/// know when asking again can help. A person's waiting search is one of the
/// reasons for [`Self::Later`] and is never named, so a runner session
/// learns nothing of people's searches.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Unseated {
    /// Closed to this AI until an administrator or the operator changes
    /// something: the AI is switched off or has a `daily_games` of 0, the
    /// carousel allows no match (`carousel_games` or `carousel_daily` 0,
    /// `global_seats` under two), or no season is open.
    Off,
    /// The account offers no deck for the format under a session with
    /// `drain_minutes` left before its cap: offer, under a newer session if
    /// need be, and ask again.
    Offer,
    /// A daily cap is reached, the AI's `daily_games` or today's carousel
    /// matches (`carousel_daily`): ask again after midnight UTC.
    Tomorrow,
    /// Not now: a person waits in the format, the AI or every partner has
    /// no free slot, the carousel's live matches, `global_seats` or the hosts'
    /// room are taken, or another request held a row. Ask again in a while.
    Later,
}

/// An AI account's own settings, as its administrator set them.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    /// The public badge.
    pub badge: AiBadge,
    /// The runner's key for the mind.
    pub profile: String,
    /// Whether Central seats it in new matches.
    pub enabled: bool,
    /// Matches at once.
    pub max_games: u8,
}

/// The answers to [`Request`].
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Response {
    /// The offers stand until this Unix second, unless renewed.
    Offered {
        /// Expiry; none when the list was empty.
        until: Option<i64>,
    },
    /// The account's settings and live matches.
    Work {
        /// Its settings.
        settings: Settings,
        /// Every live match (reserved or playing), at most [`MAX_GAMES`]: more
        /// than `max_games` while an administrator has just lowered it.
        assignments: Vec<Assignment>,
    },
    /// The carousel made a match now: the account's seat in it, which
    /// [`Self::Work`] lists from now on too. The opponent is an AI.
    Seated {
        /// The seat, as a person's assignment.
        assignment: Assignment,
    },
    /// The carousel made no match now.
    Unseated {
        /// When asking again can help.
        reason: Unseated,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_absent_choice_is_no_ai_and_a_later_kind_is_other() {
        assert_eq!(AiOpponents::default(), AiOpponents::None);
        assert_eq!(AiOpponents::None.as_str(), None);
        let read: AiKind = serde_json::from_str("\"seer\"").unwrap();
        assert_eq!(read, AiKind::Other);
        let badge: AiBadge =
            serde_json::from_str(r#"{"kind":"house","label":"Haus-KI · Expert","icon":"x"}"#)
                .unwrap();
        assert_eq!(badge.kind, AiKind::House);
        for kind in [AiKind::House, AiKind::Model, AiKind::Net] {
            assert_eq!(AiKind::from_key(kind.as_str().unwrap()), Some(kind));
        }
        assert!(AiKind::House.free() && AiKind::Net.free() && !AiKind::Model.free());
    }

    /// Today's shapes as a build of protocol 2 decodes them: a caller of 2
    /// must read every answer protocol 3 changed.
    #[expect(
        dead_code,
        reason = "frozen copies are decoded to prove the shape, never read"
    )]
    mod v2 {
        use crate::ranked::{Format, Id, Rank, Standing};
        use serde::Deserialize;

        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        pub struct Row {
            pub account: Id,
            pub name: Option<String>,
            pub tag: Option<u32>,
            pub standing: Standing,
        }

        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        pub struct Assignment {
            pub match_id: Id,
            pub host_id: Id,
            pub origin: String,
            pub format: Format,
            pub account: Id,
            pub seat: u8,
            pub deck: [u8; 32],
            pub game: Option<Id>,
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "snake_case", deny_unknown_fields)]
        pub enum QueueState {
            Idle,
            Waiting {
                key: Id,
                format: Format,
                queued_at: i64,
            },
            Assigned {
                key: Id,
                assignment: Assignment,
            },
            Finished {
                deck: crate::ranked::DeckResult,
                key: Id,
                match_id: Id,
                format: Format,
                outcome: crate::ranked::Outcome,
                seat: u8,
                elo: i32,
                delta: i32,
                before: Rank,
                after: Rank,
            },
        }

        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        pub struct PublicProfile {
            pub display_name: String,
            pub tag: u32,
            pub picture: Option<crate::profile::PublicPicture>,
            pub badges: crate::profile::PublicBadges,
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "snake_case", deny_unknown_fields)]
        pub enum Seat {
            Participants {
                match_id: Id,
                seats: [Option<String>; 2],
            },
        }
    }

    fn badge() -> AiBadge {
        AiBadge {
            kind: AiKind::House,
            label: "Haus-KI · Expert".into(),
        }
    }

    fn rank() -> crate::ranked::Rank {
        crate::ranked::Rank {
            league: crate::ranked::League::Unranked,
            division: None,
            champion_place: None,
        }
    }

    fn assignment() -> Assignment {
        Assignment {
            match_id: [1; 16],
            host_id: [2; 16],
            origin: "https://baylee.app".into(),
            format: Format::Standard,
            account: [3; 16],
            seat: 1,
            deck: [4; 32],
            game: None,
            opponent_ai: Some(badge()),
        }
    }

    fn public_profile() -> crate::wire::Response {
        crate::wire::Response::PublicProfile {
            profile: Box::new(crate::profile::PublicProfile {
                display_name: "KI-Haus-Expert".into(),
                tag: 2,
                picture: None,
                badges: crate::profile::PublicBadges {
                    ranked_season: None,
                    ranked_standings: vec![],
                    ranked_awards: vec![],
                    best_rank: None,
                    challenge_badges: vec![],
                    tower_badges: vec![],
                    echoes_badges: vec![],
                    social_badges: vec![],
                },
                ai: Some(badge()),
            }),
        }
    }

    /// Every reply protocol 3 changed, each carrying what 3 added.
    fn changed_replies() -> Vec<crate::wire::Response> {
        use crate::ranked::{self as r, replica::SeatResponse};
        use crate::wire::Response as W;
        let standing = r::Standing {
            career: r::Record::default(),
            placement_games: 0,
            format: Format::Standard,
            track: r::Track::Player,
            deck: None,
            elo: 1500,
            games: 0,
            wins: 0,
            losses: 0,
            draws: 0,
            rank: rank(),
        };
        let season = r::Season {
            id: [9; 16],
            name: "S1".into(),
            starts: 0,
            ends: None,
        };
        let deck = r::DeckResult {
            before: rank(),
            after: rank(),
            elo: 1500,
            delta: 0,
            placement_games: 1,
        };
        let queue = |state| W::Ranked {
            response: r::Response::Queue { state },
        };
        vec![
            W::Ranked {
                response: r::Response::Board {
                    season: season.clone(),
                    entries: vec![r::Row {
                        account: [5; 16],
                        name: Some("KI-Haus-Expert".into()),
                        tag: Some(1),
                        standing: standing.clone(),
                        ai: Some(badge()),
                    }],
                    next: None,
                },
            },
            W::Ranked {
                response: r::Response::Profile {
                    season,
                    account: [5; 16],
                    name: None,
                    tag: None,
                    formats: vec![standing],
                    awards: vec![],
                    ai: Some(badge()),
                },
            },
            queue(r::QueueState::Waiting {
                key: [6; 16],
                format: Format::Standard,
                queued_at: 10,
                ai_fill_in_at: Some(70),
            }),
            queue(r::QueueState::Assigned {
                key: [6; 16],
                assignment: assignment(),
            }),
            queue(r::QueueState::Finished {
                deck,
                key: [6; 16],
                match_id: [1; 16],
                format: Format::Standard,
                outcome: r::Outcome::Draw,
                seat: 0,
                elo: 1500,
                delta: 0,
                before: rank(),
                after: rank(),
                opponent_ai: Some(badge()),
            }),
            W::Ranked {
                response: r::Response::Replica {
                    response: SeatResponse::Participants {
                        match_id: [1; 16],
                        seats: [Some("Ann#0001".into()), Some("KI-Haus-Expert#0002".into())],
                        ai: [None, Some(badge())],
                    },
                },
            },
            public_profile(),
        ]
    }

    /// Down a protocol 2 path, every field 3 added is gone and the reply
    /// decodes with the old types; down a protocol 3 path the badge stays.
    #[test]
    fn a_caller_of_protocol_2_reads_every_changed_reply() {
        for reply in changed_replies() {
            let full = String::from_utf8(crate::encode(&reply).unwrap()).unwrap();
            assert!(
                full.contains("Haus-KI") || full.contains("ai_fill_in_at"),
                "{full}"
            );
            let mut old = reply;
            old.omit_ai();
            let text = String::from_utf8(crate::encode(&old).unwrap()).unwrap();
            for added in ["\"ai\"", "opponent_ai", "ai_fill_in_at", "Haus-KI"] {
                assert!(!text.contains(added), "{added} in {text}");
            }
            let value: serde_json::Value = serde_json::from_str(&text).unwrap();
            if value["result"] == "ranked" {
                let ranked = &value["response"];
                if let Some(board) = ranked.get("board") {
                    for row in board["entries"].as_array().unwrap() {
                        serde_json::from_value::<v2::Row>(row.clone()).unwrap();
                    }
                }
                if let Some(queue) = ranked.get("queue") {
                    serde_json::from_value::<v2::QueueState>(queue["state"].clone()).unwrap();
                }
                if let Some(replica) = ranked.get("replica") {
                    serde_json::from_value::<v2::Seat>(replica["response"].clone()).unwrap();
                }
                if let Some(profile) = ranked.get("profile") {
                    assert!(profile.get("ai").is_none());
                }
            } else {
                assert_eq!(value["result"], "public_profile");
                let profile = value["profile"].clone();
                serde_json::from_value::<v2::PublicProfile>(profile)
                    .unwrap_or_else(|e| panic!("{e}: {text}"));
            }
        }
    }

    /// The carousel asks for a format alone and answers a seat or one of
    /// four reasons; nothing else reads as either.
    #[test]
    fn the_carousel_asks_for_a_format_and_answers_a_seat_or_a_reason() {
        let request: Request =
            serde_json::from_str(r#"{"carousel":{"format":"highlander"}}"#).unwrap();
        assert_eq!(
            request,
            Request::Carousel {
                format: Format::Highlander
            }
        );
        for wrong in [
            r#"{"carousel":{}}"#,
            r#"{"carousel":{"format":"standard","partner":"x"}}"#,
            r#"{"carousel":{"format":"vintage"}}"#,
        ] {
            assert!(serde_json::from_str::<Request>(wrong).is_err(), "{wrong}");
        }
        let reasons = [
            Unseated::Off,
            Unseated::Offer,
            Unseated::Tomorrow,
            Unseated::Later,
        ];
        let replies = reasons
            .map(|reason| Response::Unseated { reason })
            .into_iter()
            .chain([Response::Seated {
                assignment: assignment(),
            }]);
        for reply in replies {
            let text = serde_json::to_string(&reply).unwrap();
            assert_eq!(serde_json::from_str::<Response>(&text).unwrap(), reply);
        }
        assert_eq!(
            serde_json::to_string(&Response::Unseated {
                reason: Unseated::Later
            })
            .unwrap(),
            r#"{"unseated":{"reason":"later"}}"#
        );
        // A reason this build does not know is refused, never read as another.
        for wrong in [
            r#"{"unseated":{"reason":"people"}}"#,
            r#"{"unseated":{"reason":"later","who":1}}"#,
        ] {
            assert!(serde_json::from_str::<Response>(wrong).is_err(), "{wrong}");
        }
    }

    #[test]
    fn labels_and_profiles_keep_their_bounds() {
        assert!(valid_label("Haus-KI · Expert"));
        assert!(valid_label(&"ü".repeat(LABEL_CHARS)));
        for wrong in [
            "",
            " Haus",
            "Haus ",
            "a\u{7}b",
            &"x".repeat(LABEL_CHARS + 1),
        ] {
            assert!(!valid_label(wrong), "{wrong:?}");
        }
        assert!(valid_profile("house:expert"));
        assert!(valid_profile("model:haiku-4.5_high"));
        assert!(ranked_profile("house:expert"));
        assert!(valid_profile("house:seer") && !ranked_profile("house:seer"));
        for wrong in [
            "house",
            ":expert",
            "house:",
            "House:expert",
            "house:Expert",
            "a:b c",
        ] {
            assert!(!valid_profile(wrong), "{wrong:?}");
        }
    }
}
