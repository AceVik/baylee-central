//! An account's status as its friends see it (`docs/push-socket.md`
//! §"Status"): what the person chose, what the client says it is doing, and
//! how a push socket's `presence` frame writes both.
//!
//! The status itself is fixed values only. The one thing a client may name
//! beside them is the table it sits at ([`Table`], `docs/push-socket.md`
//! §"Tables"), while its player shares it and it has no password: Central
//! checks it and passes it to the friends that are shown the account, in a
//! `table` frame of its own.
use crate::gateway::{shown_char, validate_origin};
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

/// The table a client says its player sits at, which Central passes on to
/// the friends that are shown the account (`docs/push-socket.md` §"Tables"):
/// the game gateway's origin, the game's id there, the table's name, its
/// seats and how many are taken. A client says it only while its player
/// shares the table (the gateway's `share_table`) and the table has no
/// password, only with [`Activity::Playing`] or [`Activity::Table`], and only
/// to a Central whose push socket announced tables in its `ready`. Central
/// keeps a table that passes [`Table::valid`] with the report, in memory, and
/// drops any other; the rest of the report is heard either way.
///
/// Nothing in it is a link anybody follows: a friend's client offers to join
/// only a table at the gateway it is signed in to itself, and never dials a
/// gateway because a friend named it.
///
/// Deliberately without `deny_unknown_fields`: a field a later version adds
/// is read past, in a report and in a frame alike, rather than costing the
/// table or the request around it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Table {
    /// The game gateway's canonical origin: `https://` and a host, or
    /// `http://` and a numeric loopback address with its port
    /// ([`validate_origin`]); at most [`Table::MAX_ORIGIN`] bytes.
    pub origin: String,
    /// The game's id at that gateway: 1 to [`Table::MAX_GAME`] of
    /// `A-Z a-z 0-9 - _`.
    pub game: String,
    /// The table's name, plain text, possibly empty: at most
    /// [`Table::MAX_NAME_CHARS`] characters, each one a [`shown_char`].
    pub name: String,
    /// Its seats, within [`Table::SEATS`].
    pub seats: u8,
    /// How many of them are taken, at most `seats`.
    pub taken: u8,
}

impl Table {
    /// The longest origin, in bytes.
    pub const MAX_ORIGIN: usize = 200;
    /// The longest game id, in bytes: a UUID's 36 and a little room.
    pub const MAX_GAME: usize = 40;
    /// The longest name, in characters (a Baylee gateway takes 60).
    pub const MAX_NAME_CHARS: usize = 64;
    /// The seats a table has.
    pub const SEATS: std::ops::RangeInclusive<u8> = 2..=8;

    /// Whether every field is within its bounds; Central passes on no other
    /// table, and a client shows none.
    #[must_use]
    pub fn valid(&self) -> bool {
        self.origin.len() <= Self::MAX_ORIGIN
            && (validate_origin(&self.origin, false).is_ok()
                || validate_origin(&self.origin, true).is_ok())
            && (1..=Self::MAX_GAME).contains(&self.game.len())
            && self
                .game
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
            // a character is at most four bytes: the count is only taken of
            // a name that can be short enough
            && self.name.len() <= 4 * Self::MAX_NAME_CHARS
            && self.name.chars().count() <= Self::MAX_NAME_CHARS
            && self.name.chars().all(shown_char)
            && Self::SEATS.contains(&self.seats)
            && self.taken <= self.seats
    }

    /// An optional table read leniently, for `#[serde(deserialize_with)]`:
    /// `null`, or anything that is no table (a field missing, a number past
    /// `u8`, a string where a number belongs), reads as none, so that it costs
    /// the request or frame around it nothing. [`Self::valid`] is still to be
    /// asked of what it reads.
    ///
    /// # Errors
    /// Only what the deserializer itself fails with, never the table's shape.
    pub fn lenient<'de, D>(deserializer: D) -> Result<Option<Self>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        #[allow(dead_code)] // the other shape is only read past
        enum Read {
            Table(Table),
            Other(serde::de::IgnoredAny),
        }
        Ok(match Read::deserialize(deserializer)? {
            Read::Table(table) => Some(table),
            Read::Other(_) => None,
        })
    }
}

/// A push socket's `table` frame (`docs/push-socket.md` §"Tables"):
/// `{"event":"table","tag":"002a","table":{…}}` for the friend `tag` (written
/// as a `presence` frame's `online` writes it) and the table it shares, or
/// `{"event":"table","tag":"002a"}` once it shares none. Serialized, never
/// formatted by hand: a name may hold quotes and backslashes.
#[must_use]
pub fn table_frame(tag: u32, table: Option<&Table>) -> String {
    #[derive(Serialize)]
    struct Frame<'a> {
        event: &'static str,
        tag: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        table: Option<&'a Table>,
    }
    serde_json::to_string(&Frame {
        event: "table",
        tag: format!("{tag:04x}"),
        table,
    })
    // a struct of strings and small numbers always serializes
    .unwrap_or_default()
}

/// The longest `table` frame [`table_frame`] writes for a table within its
/// bounds: the widest tag, every field at its longest, the origin's every
/// byte escaped (it is ASCII), and four bytes for each character of the name
/// (a character that is escaped takes two).
pub const LONGEST_TABLE_FRAME: usize = r#"{"event":"table","tag":"7fffffff","table":{"origin":"","game":"","name":"","seats":255,"taken":255}}"#
    .len()
    + 2 * Table::MAX_ORIGIN
    + Table::MAX_GAME
    + 4 * Table::MAX_NAME_CHARS;

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

    fn table() -> Table {
        Table {
            origin: "https://gateway.example".into(),
            game: "01937d4e-7b1c-7000-8000-000000000001".into(),
            name: "Friday draft".into(),
            seats: 4,
            taken: 2,
        }
    }

    fn with(change: impl FnOnce(&mut Table)) -> Table {
        let mut table = table();
        change(&mut table);
        table
    }

    #[test]
    fn a_table_within_its_bounds_is_valid() {
        let longest_host = format!("https://{}.example", "a".repeat(63));
        let at_most = |value: &str| value.len() <= Table::MAX_ORIGIN;
        for table in [
            table(),
            with(|t| t.origin = "http://127.0.0.1:28766".into()),
            with(|t| t.origin = "http://[::1]:8080".into()),
            with(|t| t.origin = "https://gateway.example:8443".into()),
            with(|t| t.origin.clone_from(&longest_host)),
            with(|t| t.game = "a".into()),
            with(|t| t.game = "A-z_0".repeat(8)),
            with(|t| t.name = String::new()),
            with(|t| t.name = "ü".repeat(Table::MAX_NAME_CHARS)),
            // joiners stay, as in every name another person reads
            with(|t| t.name = "👩\u{200D}👩\u{200D}👦 \"night\" \\ <b>".into()),
            with(|t| (t.seats, t.taken) = (2, 0)),
            with(|t| (t.seats, t.taken) = (8, 8)),
        ] {
            assert!(at_most(&table.origin));
            assert!(table.valid(), "{table:?}");
        }
    }

    #[test]
    fn a_table_out_of_any_bound_is_not() {
        let long_origin = format!("https://{}.example", "a.".repeat(100));
        assert!(long_origin.len() > Table::MAX_ORIGIN);
        for table in [
            // origins: not https, a remote http, a name for loopback, no port,
            // a path, a query, credentials, a trailing dot, not canonical,
            // another scheme, too long
            with(|t| t.origin = "http://gateway.example".into()),
            with(|t| t.origin = "http://192.168.1.2:8080".into()),
            with(|t| t.origin = "http://localhost:28766".into()),
            with(|t| t.origin = "http://127.0.0.1".into()),
            with(|t| t.origin = "https://gateway.example/".into()),
            with(|t| t.origin = "https://gateway.example/lobby".into()),
            with(|t| t.origin = "https://gateway.example?x=1".into()),
            with(|t| t.origin = "https://user@gateway.example".into()),
            with(|t| t.origin = "https://gateway.example.".into()),
            with(|t| t.origin = "https://Gateway.example".into()),
            with(|t| t.origin = "https://gateway.example:443".into()),
            with(|t| t.origin = "wss://gateway.example".into()),
            with(|t| t.origin = "javascript:alert(1)".into()),
            with(|t| t.origin = String::new()),
            with(|t| t.origin.clone_from(&long_origin)),
            // game ids: empty, too long, outside the alphabet
            with(|t| t.game = String::new()),
            with(|t| t.game = "a".repeat(Table::MAX_GAME + 1)),
            with(|t| t.game = "../admin".into()),
            with(|t| t.game = "a b".into()),
            with(|t| t.game = "a/b".into()),
            with(|t| t.game = "a%2f".into()),
            with(|t| t.game = "ä".into()),
            // names: too long, a control character, a line break, direction
            // controls, invisible characters
            with(|t| t.name = "a".repeat(Table::MAX_NAME_CHARS + 1)),
            with(|t| t.name = "ü".repeat(Table::MAX_NAME_CHARS + 1)),
            with(|t| t.name = "a\nb".into()),
            with(|t| t.name = "a\u{0}".into()),
            with(|t| t.name = "a\u{7f}".into()),
            with(|t| t.name = "a\u{85}".into()),
            with(|t| t.name = "a\u{2028}b".into()),
            with(|t| t.name = "\u{202E}gnp.exe".into()),
            with(|t| t.name = "a\u{2066}b".into()),
            with(|t| t.name = "a\u{200B}b".into()),
            with(|t| t.name = "\u{FEFF}a".into()),
            // seats
            with(|t| (t.seats, t.taken) = (1, 1)),
            with(|t| (t.seats, t.taken) = (0, 0)),
            with(|t| (t.seats, t.taken) = (9, 2)),
            with(|t| (t.seats, t.taken) = (255, 2)),
            with(|t| (t.seats, t.taken) = (4, 5)),
        ] {
            assert!(!table.valid(), "{table:?}");
        }
    }

    #[test]
    fn a_table_is_read_leniently_and_a_later_field_read_past() {
        #[derive(Deserialize)]
        struct Holder {
            #[serde(default, deserialize_with = "Table::lenient")]
            table: Option<Table>,
        }
        let read = |json: &str| serde_json::from_str::<Holder>(json).unwrap().table;
        assert_eq!(read("{}"), None);
        assert_eq!(read(r#"{"table":null}"#), None);
        assert_eq!(read(r#"{"table":[1,2]}"#), None);
        assert_eq!(read(r#"{"table":{"seats":4}}"#), None);
        let written = serde_json::to_string(&table()).unwrap();
        assert_eq!(read(&format!(r#"{{"table":{written}}}"#)), Some(table()));
        let later = written.replacen('{', r#"{"format":"draft","#, 1);
        assert_eq!(read(&format!(r#"{{"table":{later}}}"#)), Some(table()));
    }

    /// What a reader of the frame takes from it: the shape a client parses,
    /// lenient in the table as a client must be.
    #[derive(Deserialize)]
    struct Heard {
        event: String,
        tag: String,
        #[serde(default, deserialize_with = "Table::lenient")]
        table: Option<Table>,
    }

    #[test]
    fn a_table_frame_reads_back_and_quotes_its_name() {
        let shared = with(|t| t.name = "\"},\"event\":\"revoked\\".into());
        assert!(shared.valid());
        let frame = table_frame(0x2a, Some(&shared));
        let heard: Heard = serde_json::from_str(&frame).unwrap();
        assert_eq!(
            (heard.event.as_str(), heard.tag.as_str()),
            ("table", "002a")
        );
        assert_eq!(heard.table, Some(shared));
        assert_eq!(
            table_frame(0x7fff_ffff, None),
            r#"{"event":"table","tag":"7fffffff"}"#
        );
        let cleared: Heard = serde_json::from_str(&table_frame(0x1000, None)).unwrap();
        assert_eq!((cleared.tag.as_str(), cleared.table), ("1000", None));
    }

    #[test]
    fn the_longest_table_frame_is_within_its_bound() {
        // ASCII escaped where it can be, four-byte characters where not
        let label = "a".repeat(60) + ".";
        let longest = Table {
            // 198 bytes, in labels a DNS name may have
            origin: format!("https://{}example", label.repeat(3)),
            game: "a".repeat(Table::MAX_GAME),
            name: "\"\\".repeat(Table::MAX_NAME_CHARS / 2),
            seats: 8,
            taken: 8,
        };
        let widest = with(|t| t.name = "𝔅".repeat(Table::MAX_NAME_CHARS));
        assert_eq!(longest.origin.len(), 198);
        assert!(longest.valid() && widest.valid());
        for table in [longest, widest] {
            assert!(table_frame(0x7fff_ffff, Some(&table)).len() <= LONGEST_TABLE_FRAME);
        }
    }
}
