//! Public gateway metadata and strict origin policy. Validation performs no network I/O.
use crate::{Endpoint, Error};
use serde::{Deserialize, Serialize};
use url::Url;

/// Public, bounded metadata. None of these fields grants ownership or administration.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GatewayMetadata {
    /// Human-readable gateway name, at most 80 UTF-8 bytes of [`shown_char`]s.
    pub name: String,
    /// Operator-reported software version, at most 64 ASCII bytes.
    pub version: String,
    /// Canonical website URL on the gateway's approved origin.
    pub website: String,
}

/// One approved public directory entry. Online means a recent authenticated heartbeat.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GatewayListing {
    /// Server-generated immutable gateway `UUIDv7`.
    pub id: [u8; 16],
    /// Canonical approved API origin.
    pub origin: String,
    /// Public operator metadata, never account data.
    pub metadata: GatewayMetadata,
    /// A heartbeat was accepted within the last 90 seconds; not an availability guarantee.
    pub online: bool,
    /// Optional live activity and table telemetry reported by the gateway.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stats: Option<GatewayStats>,
}

/// How much live activity a gateway chooses to publish to Central's directory.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GatewayPrivacy {
    /// Aggregate counts, public table settings and opted-in public player handles.
    Full,
    /// Aggregate player and game counts only; no individual table or player lists.
    CountsOnly,
    /// Liveness only; no player or game counts are shared.
    Private,
}

/// Where an online player currently stands on a gateway.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayerPresenceState {
    /// Connected and browsing in the lobby or deck builder.
    Lobby,
    /// Seated at a table waiting for the game to start.
    Table,
    /// Actively playing in a running game.
    InGame,
    /// Watching a running table as a spectator.
    Spectating,
}

/// Public presence entry for one opted-in player on a gateway; never a private login name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GatewayPlayerStat {
    /// Public display handle (e.g. `Name#1a2b`), at most 48 UTF-8 bytes of [`shown_char`]s.
    pub handle: String,
    /// Current activity state on the gateway.
    pub state: PlayerPresenceState,
    /// Optional bounded table or format summary (at most 80 UTF-8 bytes of [`shown_char`]s).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    /// Games completed on this gateway today.
    pub games_today: u16,
}

/// Public summary of one active or open table's settings on a gateway.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GatewayTableSetting {
    /// Format label (e.g. `Commander`, `Duell`, `Gemischt`), at most 40 UTF-8 bytes.
    pub format: String,
    /// Occupied seats at the table (`1..=8`).
    pub seats_occupied: u8,
    /// Total seats configured at the table (`2..=8`).
    pub seats_total: u8,
    /// Seats occupied by the house AI or a language model (`0..=8`).
    pub ai_seats: u8,
    /// Decision clock label (e.g. `Standard (60s)`, `Ohne Uhr`), at most 40 UTF-8 bytes.
    pub clock: String,
    /// Current turn number if the game is running, or `None` while arranging the table.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turn: Option<u16>,
    /// Number of spectators watching this table.
    pub spectators: u16,
    /// Whether this is a private room whose detailed settings are withheld.
    pub private_table: bool,
}

/// Bounded live telemetry reported by a gateway on heartbeat, respecting gateway privacy.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GatewayStats {
    /// Operator-chosen privacy level for this gateway's telemetry.
    pub privacy: GatewayPrivacy,
    /// Actively running games on this gateway.
    pub running_games: u32,
    /// Tables currently open in the lobby waiting for players.
    pub open_tables: u32,
    /// Total players currently connected to the gateway.
    pub players_online: u32,
    /// Connected players currently in the lobby.
    pub players_in_lobby: u32,
    /// Connected players seated at an open/preparing table.
    pub players_at_table: u32,
    /// Connected players actively in a running game.
    pub players_in_game: u32,
    /// Connected players watching a running game as spectators.
    pub spectators: u32,
    /// Active house-AI or language-model seats across tables.
    pub ai_opponents: u32,
    /// Games completed on this gateway in the last 24 hours.
    pub games_completed_today: u32,
    /// Players whose individual presence is hidden by player or gateway privacy settings.
    pub hidden_players: u32,
    /// Bounded list of public table settings (at most 32; empty unless `privacy` is `Full`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tables: Vec<GatewayTableSetting>,
    /// Bounded list of opted-in online players (at most 64; empty unless `privacy` is `Full`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub players: Vec<GatewayPlayerStat>,
}

/// Where a registered gateway stands with Central's administrators. Only an
/// approved gateway is listed. A pending or approved one signs players in
/// with a gateway token (the token makes its owner answerable and can be
/// revoked); a credential pair still needs the approval. A disabled one does
/// neither.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GatewayState {
    /// Registered, not yet approved: not listed, signs players in by token.
    Pending,
    /// Approved and enabled.
    Approved,
    /// Approved once, or refused, and disabled now.
    Disabled,
}

/// Validate a canonical live HTTPS or explicitly selected loopback development origin.
///
/// # Errors
/// Rejects credentials, paths, aliases, queries, fragments and noncanonical spellings.
pub fn validate_origin(value: &str, development: bool) -> Result<(), Error> {
    if value.len() > 256 {
        return Err(Error::Rejected);
    }
    if development {
        return (Endpoint::development(value)?.origin() == value)
            .then_some(())
            .ok_or(Error::Rejected);
    }
    let url = Url::parse(value).map_err(|_| Error::Rejected)?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
        || url.origin().ascii_serialization() != value
    {
        return Err(Error::Rejected);
    }
    Ok(())
}

/// Whether `c` may stand in a name another person reads: no control
/// character, and none of the invisible characters that change how the text
/// around them is drawn (direction marks, embeddings, overrides and
/// isolates, invisible separators and spaces, the byte order mark), which
/// could make a gateway's or a token's name read as something else. The
/// zero-width joiner and non-joiner stay: emoji sequences and some scripts
/// need them.
#[must_use]
pub const fn shown_char(c: char) -> bool {
    !c.is_control()
        && !matches!(
            c,
            '\u{061C}'
                | '\u{200B}'
                | '\u{200E}'
                | '\u{200F}'
                | '\u{202A}'..='\u{202E}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{2060}'..='\u{2064}'
                | '\u{2066}'..='\u{2069}'
                | '\u{FEFF}'
        )
}

impl GatewayMetadata {
    /// Validate metadata without dereferencing the website or accepting another origin.
    ///
    /// # Errors
    /// Rejects unsafe, oversized or misleadingly noncanonical values.
    pub fn validate(&self, origin: &str) -> Result<(), Error> {
        let website = Url::parse(&self.website).map_err(|_| Error::Rejected)?;
        if self.name.is_empty()
            || self.name.len() > 80
            || self.name.trim() != self.name
            || !self.name.chars().all(shown_char)
            || self.version.is_empty()
            || self.version.len() > 64
            || !self
                .version
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b".-_+".contains(&b))
            || self.website.len() > 512
            || website.origin().ascii_serialization() != origin
            || !website.username().is_empty()
            || website.password().is_some()
            || website.query().is_some()
            || website.fragment().is_some()
            || website.as_str() != self.website
        {
            return Err(Error::Rejected);
        }
        Ok(())
    }
}

fn valid_text(value: &str, max_bytes: usize) -> bool {
    !value.is_empty()
        && value.len() <= max_bytes
        && value.trim() == value
        && value.chars().all(shown_char)
}

impl GatewayStats {
    /// Validate bounded gateway telemetry and enforce the gateway's chosen privacy level.
    ///
    /// # Errors
    /// Rejects oversized lists, control/bidi characters, impossible seat counts, or
    /// detailed table/player entries when `privacy` is not [`GatewayPrivacy::Full`].
    pub fn validate(&self) -> Result<(), Error> {
        if self.tables.len() > 32 || self.players.len() > 64 {
            return Err(Error::Rejected);
        }
        if self.privacy != GatewayPrivacy::Full
            && (!self.tables.is_empty() || !self.players.is_empty())
        {
            return Err(Error::Rejected);
        }
        for t in &self.tables {
            if !valid_text(&t.format, 40)
                || !valid_text(&t.clock, 40)
                || !(2..=8).contains(&t.seats_total)
                || t.seats_occupied > t.seats_total
                || t.ai_seats > t.seats_total
            {
                return Err(Error::Rejected);
            }
        }
        for p in &self.players {
            if !valid_text(&p.handle, 48)
                || p.context.as_deref().is_some_and(|c| !valid_text(c, 80))
            {
                return Err(Error::Rejected);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn origins_and_public_metadata_cannot_redirect_or_smuggle_credentials() {
        for origin in [
            "http://gateway.invalid",
            "https://u@gateway.invalid",
            "https://gateway.invalid/",
            "https://GATEWAY.invalid",
            "https://gateway.invalid/path",
        ] {
            assert!(validate_origin(origin, false).is_err());
        }
        validate_origin("https://gateway.invalid", false).unwrap();
        validate_origin("http://127.0.0.1:9876", true).unwrap();
        assert!(validate_origin("https://gateway.invalid", true).is_err());
        let mut metadata = GatewayMetadata {
            name: "Synthetic".into(),
            version: "1.2.3-test".into(),
            website: "https://gateway.invalid/status".into(),
        };
        metadata.validate("https://gateway.invalid").unwrap();
        for website in [
            "https://foreign.invalid/",
            "https://u@gateway.invalid/",
            "https://gateway.invalid/?next=elsewhere",
        ] {
            metadata.website = website.into();
            assert!(metadata.validate("https://gateway.invalid").is_err());
        }
    }

    #[test]
    fn a_name_cannot_hide_how_it_reads() {
        let named = |name: &str| GatewayMetadata {
            name: name.into(),
            version: "1.2.3-test".into(),
            website: "https://gateway.invalid/".into(),
        };
        // a right-to-left override, an isolate, a mark, an invisible space,
        // a line separator, a byte order mark, a control character
        for name in [
            "Official\u{202E}lacif",
            "Table \u{2067}x\u{2069}",
            "Table\u{200F}",
            "Bay\u{200B}lee",
            "Two\u{2028}lines",
            "\u{FEFF}Table",
            "Tab\u{0007}le",
        ] {
            assert!(
                named(name).validate("https://gateway.invalid").is_err(),
                "{name:?}"
            );
            assert!(crate::owner::token_name(name).is_err(), "{name:?}");
        }
        // other scripts, accents and an emoji sequence with its joiners stay
        for name in [
            "Spieltisch Köln",
            "東京テーブル",
            "طاولة",
            "Family 👨\u{200D}👩\u{200D}👧",
        ] {
            named(name).validate("https://gateway.invalid").unwrap();
            crate::owner::token_name(name).unwrap();
        }
    }

    #[test]
    fn gateway_stats_respect_privacy_and_bounds() {
        let mut stats = GatewayStats {
            privacy: GatewayPrivacy::Full,
            running_games: 3,
            open_tables: 2,
            players_online: 14,
            players_in_lobby: 3,
            players_at_table: 3,
            players_in_game: 6,
            spectators: 2,
            ai_opponents: 2,
            games_completed_today: 18,
            hidden_players: 4,
            tables: vec![GatewayTableSetting {
                format: "Commander".into(),
                seats_occupied: 4,
                seats_total: 4,
                ai_seats: 1,
                clock: "Standard (60s)".into(),
                turn: Some(7),
                spectators: 2,
                private_table: false,
            }],
            players: vec![GatewayPlayerStat {
                handle: "Maren#4f12".into(),
                state: PlayerPresenceState::InGame,
                context: Some("Tisch 1 · Commander (Zug 7)".into()),
                games_today: 3,
            }],
        };
        stats.validate().unwrap();

        let mut counts_only = stats.clone();
        counts_only.privacy = GatewayPrivacy::CountsOnly;
        assert!(counts_only.validate().is_err());
        counts_only.tables.clear();
        counts_only.players.clear();
        counts_only.validate().unwrap();

        stats.tables[0].seats_occupied = 5;
        assert!(stats.validate().is_err());
    }
}
