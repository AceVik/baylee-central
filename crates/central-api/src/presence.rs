//! An account's status as its friends see it (`docs/push-socket.md`
//! §"Status"): what the person chose, what the client says it is doing, and
//! how a push socket's `presence` frame writes both.
//!
//! Only these fixed values exist: no game, table, room or gateway is ever
//! named, and a client cannot say more than one of them.
use serde::{Deserialize, Serialize};

/// What a person chooses to be shown as. Stored with the account.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Choice {
    /// Shown as online while a client is connected.
    #[default]
    Online,
    /// Shown as away while a client is connected.
    Afk,
    /// Never shown: friends see the account offline. Its own news still
    /// reaches its sockets.
    Offline,
}

impl Choice {
    /// The stored and spoken name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Online => "online",
            Self::Afk => "afk",
            Self::Offline => "offline",
        }
    }

    /// The choice named `name`; `None` for any other word.
    #[must_use]
    pub fn named(name: &str) -> Option<Self> {
        [Self::Online, Self::Afk, Self::Offline]
            .into_iter()
            .find(|choice| choice.name() == name)
    }
}

/// What a client says it is doing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Activity {
    /// Sitting in a game that is running, online or against the house AI.
    Playing,
    /// Sitting at a table whose game has not begun.
    Table,
}

/// One online friend as a `presence` frame shows it: away or not, and what
/// it is doing. Written as one letter of the frame's `states`, in the order
/// of its `online` tags.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shown {
    /// Away: chosen, or idle for ten minutes.
    pub away: bool,
    /// What it says it is doing.
    pub activity: Option<Activity>,
}

impl Shown {
    /// Online, doing nothing in particular: what a client reads for a letter
    /// it does not know, and for every friend of a frame whose `states` does
    /// not match its `online`.
    pub const ONLINE: Self = Self {
        away: false,
        activity: None,
    };

    /// Its letter: `o` online, `a` away, `g` playing, `t` at a table, and the
    /// capitals of the last two while away.
    #[must_use]
    pub const fn letter(self) -> char {
        match (self.away, self.activity) {
            (false, None) => 'o',
            (true, None) => 'a',
            (false, Some(Activity::Playing)) => 'g',
            (true, Some(Activity::Playing)) => 'G',
            (false, Some(Activity::Table)) => 't',
            (true, Some(Activity::Table)) => 'T',
        }
    }

    /// What `letter` says; `None` for a letter not (yet) known.
    #[must_use]
    pub const fn of_letter(letter: char) -> Option<Self> {
        let (away, activity) = match letter {
            'o' => (false, None),
            'a' => (true, None),
            'g' => (false, Some(Activity::Playing)),
            'G' => (true, Some(Activity::Playing)),
            't' => (false, Some(Activity::Table)),
            'T' => (true, Some(Activity::Table)),
            _ => return None,
        };
        Some(Self { away, activity })
    }

    /// The friends a frame's `online` names, with what its `states` says of
    /// each. A `states` that is missing or not exactly as long as `online`
    /// shows every friend as [`Self::ONLINE`]; a letter not known shows that
    /// friend so. So a later letter, or a frame of an older server, never
    /// breaks a reader: it only says less.
    #[must_use]
    pub fn read<'a>(online: &'a [String], states: Option<&str>) -> Vec<(&'a str, Self)> {
        let letters: Vec<char> = states
            .map(|states| states.chars().collect())
            .unwrap_or_default();
        let matched = letters.len() == online.len();
        online
            .iter()
            .enumerate()
            .map(|(index, tag)| {
                let shown = if matched {
                    Self::of_letter(letters[index]).unwrap_or(Self::ONLINE)
                } else {
                    Self::ONLINE
                };
                (tag.as_str(), shown)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_shown_state_has_its_own_letter_and_reads_back() {
        let all = [None, Some(Activity::Playing), Some(Activity::Table)]
            .into_iter()
            .flat_map(|activity| [false, true].map(|away| Shown { away, activity }))
            .collect::<Vec<_>>();
        let letters: std::collections::BTreeSet<char> = all.iter().map(|s| s.letter()).collect();
        assert_eq!(letters.len(), all.len());
        for shown in all {
            assert_eq!(Shown::of_letter(shown.letter()), Some(shown));
        }
        assert_eq!(Shown::of_letter('x'), None);
    }

    #[test]
    fn a_reader_falls_back_to_online_and_never_fails() {
        let online = vec!["002a".to_owned(), "1000".to_owned()];
        let read = |states| Shown::read(&online, states);
        assert_eq!(
            read(Some("at")),
            vec![
                (
                    "002a",
                    Shown {
                        away: true,
                        activity: None
                    }
                ),
                (
                    "1000",
                    Shown {
                        away: false,
                        activity: Some(Activity::Table)
                    }
                ),
            ]
        );
        // an older server's frame, a length that does not match, a letter not known
        for states in [None, Some("a"), Some("atg"), Some("")] {
            assert!(
                read(states)
                    .iter()
                    .all(|(_, shown)| *shown == Shown::ONLINE)
            );
        }
        assert_eq!(read(Some("x?"))[0].1, Shown::ONLINE);
        assert!(read(Some("ax"))[0].1.away);
    }

    #[test]
    fn choices_are_named_as_stored() {
        for choice in [Choice::Online, Choice::Afk, Choice::Offline] {
            assert_eq!(Choice::named(choice.name()), Some(choice));
            assert_eq!(
                serde_json::to_string(&choice).unwrap(),
                format!("\"{}\"", choice.name())
            );
        }
        assert_eq!(Choice::named("invisible"), None);
        assert_eq!(Choice::default(), Choice::Online);
    }
}
