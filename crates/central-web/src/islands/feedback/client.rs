//! The client's details of a report (its `client` part), read in the
//! browser (`docs/reports.md`: Central keeps the gzip as it came and never
//! unpacks it). The part is untrusted: a gzip of at most
//! `MAX_CLIENT_GZIP` bytes is inflated through the browser's own
//! `DecompressionStream` and stopped past [`MAX_CLIENT_JSON`] bytes of
//! output (`super::api::inflate`), decoded as strict UTF-8 and only then
//! parsed as JSON. Every reader here takes what it finds or nothing: a part
//! that is missing or of another shape reads as `None`, and the page says
//! so instead of breaking (Baylee `web/feedback/src/dump.ts`, the client's
//! `crates/baylee-client-core/src/bugreport.rs`).

use serde_json::{Map, Value};

/// The most a client part may inflate to: 16 MiB (`docs/reports.md`
/// pass 2): the client writes a few hundred KiB of JSON and a screenshot
/// of a few MiB as base64, and this stops a decompression bomb long before
/// it costs the tab.
pub const MAX_CLIENT_JSON: usize = 16 * 1024 * 1024;

/// A screenshot's largest side, in pixels, as its PNG header says it.
pub const MAX_SHOT_SIDE: u32 = 16_384;

/// The lines a log shows at most: the latest ones.
pub const LOG_LINES: usize = 500;

/// Why a client part cannot be shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unreadable {
    /// It inflates past [`MAX_CLIENT_JSON`].
    Large,
    /// It is no gzip, no UTF-8 or no JSON object.
    Shape,
}

/// The client part's bytes, inflated: strict UTF-8, a JSON object.
///
/// # Errors
/// [`Unreadable::Shape`] for anything else.
pub fn parse(bytes: &[u8]) -> Result<Map<String, Value>, Unreadable> {
    let text = std::str::from_utf8(bytes).map_err(|_| Unreadable::Shape)?;
    match serde_json::from_str::<Value>(text) {
        Ok(Value::Object(map)) => Ok(map),
        _ => Err(Unreadable::Shape),
    }
}

fn text(v: Option<&Value>, max: usize) -> Option<String> {
    v.and_then(Value::as_str)
        .filter(|s| s.chars().count() <= max)
        .map(str::to_owned)
}

fn num(v: Option<&Value>) -> Option<f64> {
    v.and_then(Value::as_f64).filter(|n| n.is_finite())
}

/// A whole number the client wrote, as text.
fn whole(v: Option<&Value>) -> Option<i64> {
    v.and_then(Value::as_i64)
}

fn part<'a>(client: &'a Map<String, Value>, key: &str) -> Option<&'a Map<String, Value>> {
    client.get(key).and_then(Value::as_object)
}

/// The client's build.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Build {
    /// Its version.
    pub version: Option<String>,
    /// Its commit.
    pub commit: Option<String>,
}

/// The `build` part.
#[must_use]
pub fn build(client: &Map<String, Value>) -> Option<Build> {
    let b = part(client, "build")?;
    Some(Build {
        version: text(b.get("version"), 128),
        commit: text(b.get("commit"), 64),
    })
}

/// A row of the system part.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemRow {
    /// The platform.
    Platform,
    /// How many processors.
    Cpus,
    /// The graphics adapter.
    Adapter,
    /// The graphics backend.
    Backend,
    /// The window's size.
    Window,
    /// Its scale.
    Scale,
    /// The language.
    Lang,
}

/// The system part's rows, in the order the client's form shows them.
#[must_use]
pub fn system(client: &Map<String, Value>) -> Option<Vec<(SystemRow, String)>> {
    let s = part(client, "system")?;
    let mut rows = Vec::new();
    let mut add = |row, value: Option<String>| {
        if let Some(value) = value.filter(|v| !v.is_empty()) {
            rows.push((row, value));
        }
    };
    add(SystemRow::Platform, text(s.get("platform"), 256));
    add(SystemRow::Cpus, whole(s.get("cpus")).map(|n| n.to_string()));
    add(SystemRow::Adapter, text(s.get("adapter"), 256));
    add(SystemRow::Backend, text(s.get("backend"), 64));
    let window = s.get("window").and_then(Value::as_array).and_then(|w| {
        match (w.len(), whole(w.first()), whole(w.get(1))) {
            (2, Some(x), Some(y)) => Some(format!("{x} × {y}")),
            _ => None,
        }
    });
    add(SystemRow::Window, window);
    add(
        SystemRow::Scale,
        num(s.get("scale")).map(|n| format!("{n}")),
    );
    add(SystemRow::Lang, text(s.get("lang"), 32));
    Some(rows)
}

/// The window's picture: a PNG whose signature and size were checked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Screenshot {
    /// The PNG's bytes.
    pub png: Vec<u8>,
    /// Its width, from its own header.
    pub width: u32,
    /// Its height.
    pub height: u32,
}

const PNG: [u8; 8] = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];

/// The window's picture, when it is a PNG: its eight-byte signature, then
/// an `IHDR` whose sides are 1 to [`MAX_SHOT_SIDE`]. Anything else is no
/// picture.
#[must_use]
pub fn screenshot(client: &Map<String, Value>) -> Option<Screenshot> {
    let b64 = part(client, "screenshot")?.get("png_base64")?.as_str()?;
    let png = super::super::b64::decode_standard(b64)?;
    if png.len() < 24 || png[..8] != PNG || &png[12..16] != b"IHDR" {
        return None;
    }
    let side = |at: usize| u32::from_be_bytes([png[at], png[at + 1], png[at + 2], png[at + 3]]);
    let (width, height) = (side(16), side(20));
    let fits = |n: u32| (1..=MAX_SHOT_SIDE).contains(&n);
    (fits(width) && fits(height)).then_some(Screenshot { png, width, height })
}

/// One card the player's text names in brackets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CardRef {
    /// Its name.
    pub name: String,
    /// Where in the text, `[start, end)` in characters, brackets included.
    pub at: Option<(usize, usize)>,
    /// Its zone.
    pub zone: Option<String>,
    /// Its owner's seat.
    pub owner: Option<i64>,
}

/// One player the text names: a seat, never an account.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayerRef {
    /// The name as written.
    pub name: String,
    /// Where in the text.
    pub at: Option<(usize, usize)>,
    /// The seat.
    pub seat: Option<i64>,
}

/// The references the client resolved in the text.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Refs {
    /// Cards.
    pub cards: Vec<CardRef>,
    /// Players.
    pub players: Vec<PlayerRef>,
}

fn range(v: Option<&Value>) -> Option<(usize, usize)> {
    let pair = v?.as_array()?;
    if pair.len() != 2 {
        return None;
    }
    let at = |i: usize| pair.get(i)?.as_u64().and_then(|n| usize::try_from(n).ok());
    Some((at(0)?, at(1)?))
}

/// The first 200 objects of `v`, when it is an array.
fn objects(v: Option<&Value>) -> Vec<&Map<String, Value>> {
    v.and_then(Value::as_array)
        .map(|a| a.iter().take(200).filter_map(Value::as_object).collect())
        .unwrap_or_default()
}

/// The references, at most 200 of each; `None` when the text names none.
#[must_use]
pub fn refs(client: &Map<String, Value>) -> Option<Refs> {
    let r = part(client, "refs")?;
    let refs = Refs {
        cards: objects(r.get("cards"))
            .into_iter()
            .map(|c| CardRef {
                name: text(c.get("text"), 200).unwrap_or_else(|| "?".into()),
                at: range(c.get("at")),
                zone: text(c.get("zone"), 32),
                owner: whole(c.get("owner")),
            })
            .collect(),
        players: objects(r.get("players"))
            .into_iter()
            .map(|p| PlayerRef {
                name: text(p.get("text"), 200).unwrap_or_else(|| "?".into()),
                at: range(p.get("at")),
                seat: whole(p.get("seat")),
            })
            .collect(),
    };
    (refs.cards.len() + refs.players.len() > 0).then_some(refs)
}

/// A piece of the player's text: as written, or a reference in its place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Piece {
    /// Text as written.
    Text(String),
    /// The card at this index of `Refs::cards`.
    Card(usize),
    /// The player at this index of `Refs::players`.
    Player(usize),
}

/// The player's words, each `[Card]` and `[@Player]` the client resolved
/// taken out as a piece in its place; a reference whose range does not say
/// exactly what the text says there is left as text.
#[must_use]
pub fn pieces(text: &str, refs: &Refs) -> Vec<Piece> {
    let chars: Vec<char> = text.chars().collect();
    let spans = |at: Option<(usize, usize)>, want: &str| {
        let (from, to) = at?;
        (from < to && to <= chars.len() && chars[from..to].iter().collect::<String>() == want)
            .then_some((from, to))
    };
    let mut found: Vec<(usize, usize, Piece)> = Vec::new();
    for (i, card) in refs.cards.iter().enumerate() {
        if let Some((from, to)) = spans(card.at, &format!("[{}]", card.name)) {
            found.push((from, to, Piece::Card(i)));
        }
    }
    for (i, player) in refs.players.iter().enumerate() {
        if let Some((from, to)) = spans(player.at, &format!("[@{}]", player.name)) {
            found.push((from, to, Piece::Player(i)));
        }
    }
    found.sort_by_key(|(from, ..)| *from);
    let mut out = Vec::new();
    let mut cursor = 0;
    for (from, to, piece) in found {
        if from < cursor {
            continue;
        }
        if from > cursor {
            out.push(Piece::Text(chars[cursor..from].iter().collect()));
        }
        out.push(piece);
        cursor = to;
    }
    if cursor < chars.len() {
        out.push(Piece::Text(chars[cursor..].iter().collect()));
    }
    out
}

/// A crash as the client saw it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Crash {
    /// Its backtrace.
    pub backtrace: Option<String>,
}

/// The crash part's backtrace.
#[must_use]
pub fn crash(client: &Map<String, Value>) -> Option<Crash> {
    let c = part(client, "crash")?;
    Some(Crash {
        backtrace: text(c.get("backtrace"), 200_000),
    })
}

/// One line of the game's log.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogLine {
    /// The turn.
    pub turn: Option<i64>,
    /// The words.
    pub text: String,
}

/// The game's log: who sat where, and the latest lines.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Log {
    /// Seat, name and whether the computer played it.
    pub roster: Vec<(Option<i64>, String, bool)>,
    /// The latest [`LOG_LINES`] lines.
    pub lines: Vec<LogLine>,
}

/// The `log` part.
#[must_use]
pub fn log(client: &Map<String, Value>) -> Option<Log> {
    let l = part(client, "log")?;
    let roster = l
        .get("roster")
        .and_then(Value::as_array)
        .map(|r| {
            r.iter()
                .take(16)
                .filter_map(Value::as_object)
                .map(|p| {
                    (
                        whole(p.get("seat")),
                        text(p.get("name"), 64).unwrap_or_else(|| "?".into()),
                        p.get("is_ai") == Some(&Value::Bool(true)),
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    let all: Vec<&Map<String, Value>> = l
        .get("lines")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_object).collect())
        .unwrap_or_default();
    let lines = all[all.len().saturating_sub(LOG_LINES)..]
        .iter()
        .map(|x| LogLine {
            turn: whole(x.get("turn")),
            text: text(x.get("text"), 2_000).unwrap_or_default(),
        })
        .collect();
    Some(Log { roster, lines })
}

/// The name of an enum serde writes externally tagged: a string, or
/// `{"Name": …}`.
#[must_use]
pub fn variant(v: Option<&Value>) -> Option<String> {
    match v? {
        Value::String(s) => (s.chars().count() <= 64).then(|| s.clone()),
        Value::Object(o) if o.len() == 1 => {
            o.keys().next().filter(|k| k.chars().count() <= 64).cloned()
        }
        _ => None,
    }
}

/// Where in the game the report was written, in a few facts.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Game {
    /// When, as the client wrote it.
    pub when: Option<String>,
    /// The turn.
    pub turn: Option<i64>,
    /// The reporter's seat.
    pub seat: Option<i64>,
    /// The seat whose turn it was.
    pub active: Option<i64>,
    /// How many players.
    pub players: Option<usize>,
    /// The question the engine was asking, by its variant's name.
    pub pending: Option<String>,
}

/// The `game` part.
#[must_use]
pub fn game(client: &Map<String, Value>) -> Option<Game> {
    let g = part(client, "game")?;
    let empty = Map::new();
    let table = g.get("table").and_then(Value::as_object).unwrap_or(&empty);
    let view = g.get("view").and_then(Value::as_object).unwrap_or(&empty);
    Some(Game {
        when: text(table.get("when"), 128),
        turn: whole(view.get("turn")),
        seat: whole(table.get("seat")),
        active: whole(view.get("active")),
        players: view.get("players").and_then(Value::as_array).map(Vec::len),
        pending: variant(g.get("pending")),
    })
}

/// `ChooseAttackers` as `Choose attackers`.
#[must_use]
pub fn spaced(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    let mut previous: Option<char> = None;
    for c in name.chars() {
        if c.is_ascii_uppercase() && previous.is_some_and(|p| p.is_ascii_lowercase()) {
            out.push(' ');
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
        previous = Some(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn object(v: Value) -> Map<String, Value> {
        match v {
            Value::Object(map) => map,
            _ => Map::new(),
        }
    }

    #[test]
    fn only_a_json_object_in_utf8_is_a_client_part() {
        assert!(parse(br#"{"build":{}}"#).is_ok());
        assert_eq!(parse(b"[1,2]"), Err(Unreadable::Shape));
        assert_eq!(parse(b"\xff\xfe"), Err(Unreadable::Shape));
        assert_eq!(parse(b"{"), Err(Unreadable::Shape));
    }

    #[test]
    fn the_facts_are_read_or_left_out() {
        let c = object(json!({
            "build": {"version": "0.1.1-beta.9b", "commit": "3f9a1c7e21"},
            "system": {"platform": "Linux", "cpus": 8, "adapter": "<b>GPU</b>", "window": [1280, 720], "scale": 1.5, "lang": "de"},
            "game": {"table": {"when": "Combat", "seat": 1}, "view": {"turn": 4, "active": 2, "players": [{}, {}]}, "pending": {"ChooseAttackers": {}}},
            "crash": {"backtrace": "at main"},
            "log": {"roster": [{"seat": 1, "name": "Ada", "is_ai": false}, {"seat": 2, "name": "Bot", "is_ai": true}],
                    "lines": [{"turn": 1, "text": "Ada plays a land"}]},
            "settings": {"volume": 3},
        }));
        assert_eq!(
            build(&c),
            Some(Build {
                version: Some("0.1.1-beta.9b".into()),
                commit: Some("3f9a1c7e21".into())
            })
        );
        let rows = system(&c).expect("rows");
        assert_eq!(rows[0], (SystemRow::Platform, "Linux".into()));
        assert_eq!(rows[1], (SystemRow::Cpus, "8".into()));
        assert_eq!(
            rows[2],
            (SystemRow::Adapter, "<b>GPU</b>".into()),
            "as text"
        );
        assert!(rows.contains(&(SystemRow::Window, "1280 × 720".into())));
        assert!(rows.contains(&(SystemRow::Scale, "1.5".into())));
        let g = game(&c).expect("game");
        assert_eq!(g.pending.as_deref(), Some("ChooseAttackers"));
        assert_eq!(
            (g.turn, g.seat, g.active, g.players),
            (Some(4), Some(1), Some(2), Some(2))
        );
        assert_eq!(spaced("ChooseAttackers"), "Choose attackers");
        assert_eq!(
            crash(&c).and_then(|c| c.backtrace).as_deref(),
            Some("at main")
        );
        let l = log(&c).expect("log");
        assert_eq!(l.roster[1], (Some(2), "Bot".into(), true));
        assert_eq!(l.lines[0].text, "Ada plays a land");
        // another shape is no fact, never a failure
        let odd = object(json!({"build": 3, "system": [], "log": {"lines": "x"}}));
        assert_eq!(build(&odd), None);
        assert_eq!(system(&odd), None);
        assert_eq!(log(&odd).map(|l| l.lines.len()), Some(0));
    }

    #[test]
    fn a_screenshot_is_a_checked_png() {
        let mut png = PNG.to_vec();
        png.extend([0, 0, 0, 13]);
        png.extend(b"IHDR");
        png.extend(640_u32.to_be_bytes());
        png.extend(480_u32.to_be_bytes());
        png.extend([8, 6, 0, 0, 0]);
        let standard = super::super::super::b64::encode(&png);
        let c = object(json!({"screenshot": {"png_base64": standard}}));
        let shot = screenshot(&c).expect("a picture");
        assert_eq!((shot.width, shot.height), (640, 480));
        let html = object(json!({"screenshot": {"png_base64": "PHNjcmlwdD4="}}));
        assert_eq!(screenshot(&html), None);
    }

    #[test]
    fn references_take_their_place_only_where_the_text_says_them() {
        let text = "My [Llanowar Elves] hit [@Ada] for [3]";
        let c = object(json!({"refs": {
            "cards": [
                {"text": "Llanowar Elves", "at": [3, 19], "zone": "battlefield", "owner": 1},
                {"text": "Forest", "at": [33, 36]},
            ],
            "players": [{"text": "Ada", "at": [24, 30], "seat": 2}],
        }}));
        let r = refs(&c).expect("refs");
        assert_eq!(
            pieces(text, &r),
            vec![
                Piece::Text("My ".into()),
                Piece::Card(0),
                Piece::Text(" hit ".into()),
                Piece::Player(0),
                Piece::Text(" for [3]".into()),
            ]
        );
        assert_eq!(refs(&object(json!({"refs": {"cards": []}}))), None);
    }
}
