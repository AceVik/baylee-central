//! Pure-Rust `ElevenLabs` v4 narration synthesis for `/security`:
//! extracts spoken scripts directly from `crates/central-web/locales/*/security.ftl`,
//! calls `ElevenLabs` `model_id = "eleven_v4"` with timestamps, maps character-level
//! alignments to step cue windows, and writes `.opus` + `cues.json` under
//! `crates/central-web/static/audio/security/{lang}/`.
//!
//! Secrets handling: reads `ELEVENLABS_API_KEY` from the process environment only;
//! never accepts keys via CLI arguments or embeds/logs credentials.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::Duration;

use serde_json::{Value, json};
use ureq::Agent;

use crate::fail::{Fail, OrFail as _, Result};

/// The 11 supported languages matching `central-web`.
pub const LANGS: [&str; 11] = [
    "en", "de", "fr", "it", "es", "pt-br", "ru", "ja", "ko", "zh-hans", "zh-hant",
];

/// Curated warm, reassuring adult male voice (Brian, multilingual).
pub const DEFAULT_VOICE_ID: &str = "nPczCjzI2devNBz1zQrb";
/// `ElevenLabs` synthesis model identifier required for `/security` narration.
pub const MODEL_ID: &str = "eleven_v4";
const API_BASE: &str = "https://api.elevenlabs.io/v1";
const OUTPUT_FORMAT: &str = "opus_48000_64";
const MAX_RESPONSE_BYTES: u64 = 32 * 1024 * 1024;
const MAX_ERROR_BYTES: u64 = 4096;

/// CLI options for `cargo xtask security-audio`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Options {
    /// Selected language codes (`en`, `de`, …); empty means all 11 languages.
    pub langs: Vec<String>,
    /// Optional `ElevenLabs` voice ID override.
    pub voice_id: Option<String>,
    /// Extract scripts and report planned work without calling `ElevenLabs`.
    pub dry_run: bool,
    /// Overwrite existing `.opus` clips and cue manifests.
    pub force: bool,
}

/// Validates and appends comma-separated language codes.
///
/// # Errors
/// Returns an error if any code is unknown or the list is empty.
pub fn push_langs(out: &mut Vec<String>, raw: &str) -> Result<()> {
    let mut added = 0;
    for item in raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        if !LANGS.contains(&item) {
            return Err(Fail::new(format!("unknown language code: {item}")));
        }
        if !out.iter().any(|existing| existing == item) {
            out.push(item.to_owned());
        }
        added += 1;
    }
    if added == 0 {
        return Err(Fail::new("--lang requires at least one language code"));
    }
    Ok(())
}

/// Returns `true` when `id` is a safe `ElevenLabs` voice identifier token.
#[must_use]
pub fn valid_voice_id(id: &str) -> bool {
    (1..=64).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// Maps Baylee locale folder codes to `ElevenLabs` ISO 639-1 language codes.
#[must_use]
pub fn elevenlabs_language_code(lang: &str) -> &'static str {
    match lang {
        "de" => "de",
        "fr" => "fr",
        "it" => "it",
        "es" => "es",
        "pt-br" => "pt",
        "ru" => "ru",
        "ja" => "ja",
        "ko" => "ko",
        "zh-hans" | "zh-hant" => "zh",
        _ => "en",
    }
}

/// The five spoken audio sections on `/security`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    /// Warm spoken overview introducing the four flows.
    Intro,
    /// Flow 1: OPAQUE sign-in & registration.
    Signin,
    /// Flow 2: Browser memory handling & zeroing.
    Memory,
    /// Flow 3: End-to-end sealed chat envelopes.
    Chat,
    /// Flow 4: Client link pairing with short confirmation codes.
    Pairing,
}

impl Part {
    const ALL: [Self; 5] = [
        Self::Intro,
        Self::Signin,
        Self::Memory,
        Self::Chat,
        Self::Pairing,
    ];

    /// File stem (`intro`, `signin`, `memory`, `chat`, `pairing`) and cue manifest key.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Intro => "intro",
            Self::Signin => "signin",
            Self::Memory => "memory",
            Self::Chat => "chat",
            Self::Pairing => "pairing",
        }
    }

    const fn ftl_prefix(self) -> Option<&'static str> {
        match self {
            Self::Intro => None,
            Self::Signin => Some("signin"),
            Self::Memory => Some("memory"),
            Self::Chat => Some("chat"),
            Self::Pairing => Some("pair"),
        }
    }
}

/// Character range in the synthesized input text belonging to one diagram step (`1..=5`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepSpan {
    /// Step number (`1..=5`).
    pub step: u8,
    /// Inclusive Unicode character index where this step's narration starts.
    pub char_start: usize,
    /// Exclusive Unicode character index where this step's narration ends.
    pub char_end: usize,
    /// Clean spoken text of this step (used for fuzzy alignment matching).
    pub text: String,
}

/// Constructed narration text plus step boundaries for a single audio clip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NarrationScript {
    /// Plain text passed to `ElevenLabs` TTS.
    pub text: String,
    /// Empty for `Intro`; 5 ordered step spans (`1..=5`) for diagram flows.
    pub steps: Vec<StepSpan>,
}

/// Character-level timestamp stream returned by `ElevenLabs` `/with-timestamps`.
#[derive(Debug, Clone, PartialEq)]
pub struct Alignment {
    /// Aligned characters (single glyphs or short normalized tokens).
    pub characters: Vec<String>,
    /// Start timestamp (seconds) per aligned item.
    pub starts: Vec<f64>,
    /// End timestamp (seconds) per aligned item.
    pub ends: Vec<f64>,
}

/// Cue window emitted into `cues.json` for diagram synchronization.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StepCue {
    /// Diagram step (`1..=5`).
    pub step: u8,
    /// Start second in the `.opus` stream.
    pub start: f64,
    /// End second in the `.opus` stream.
    pub end: f64,
}

/// Parses top-level `key = value` Fluent messages matching `central-web/build.rs`.
///
/// # Errors
/// Returns an error if a top-level entry is malformed or duplicated.
pub fn parse_ftl_messages(text: &str) -> Result<BTreeMap<String, String>> {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = BTreeMap::new();
    let mut at = 0;
    while at < lines.len() {
        let line = lines[at];
        if line.trim().is_empty() || line.starts_with('#') {
            at += 1;
            continue;
        }
        let Some((id_raw, first_raw)) = line.split_once('=') else {
            return Err(Fail::new(format!(
                "security.ftl line {}: missing `=`",
                at + 1
            )));
        };
        let id = id_raw.trim();
        let mut block = vec![first_raw.trim().to_owned()];
        at += 1;
        while at < lines.len() {
            let next = lines[at];
            if next.starts_with(' ') || next.starts_with('}') {
                let trimmed = next.trim();
                if !trimmed.is_empty() {
                    block.push(trimmed.to_owned());
                }
                at += 1;
            } else if next.trim().is_empty() {
                let mut probe = at;
                while probe < lines.len() && lines[probe].trim().is_empty() {
                    probe += 1;
                }
                if probe < lines.len()
                    && (lines[probe].starts_with(' ') || lines[probe].starts_with('}'))
                {
                    at = probe;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        let value = block
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        if out.insert(id.to_owned(), value).is_some() {
            return Err(Fail::new(format!("duplicate Fluent message `{id}`")));
        }
    }
    Ok(out)
}

/// Removes simple lowercase XML/HTML markup tags (`<b>`, `<code>`, `<contact>`, …)
/// and normalizes inline whitespace for crisp speech synthesis.
#[must_use]
pub fn clean_spoken_text(raw: &str) -> String {
    let mut no_tags = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(open) = rest.find('<') {
        no_tags.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        if let Some(close) = after.find('>') {
            let inside = &after[..close];
            let name = inside.strip_prefix('/').unwrap_or(inside);
            if !name.is_empty() && name.bytes().all(|b| b.is_ascii_lowercase()) {
                rest = &after[close + 1..];
                continue;
            }
        }
        no_tags.push('<');
        rest = after;
    }
    no_tags.push_str(rest);

    let unescaped = no_tags
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'");
    unescaped.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn require_msg<'a>(
    messages: &'a BTreeMap<String, String>,
    key: &str,
    lang: &str,
) -> Result<&'a str> {
    messages
        .get(key)
        .map(String::as_str)
        .ok_or_else(|| Fail::new(format!("{lang}/security.ftl is missing `{key}`")))
}

fn ensure_sentence_end(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.ends_with(['.', '!', '?', '。', '！', '？']) {
        trimmed.to_owned()
    } else {
        format!("{trimmed}.")
    }
}

/// Builds the full narration script and per-step character spans for `part`.
///
/// # Errors
/// Returns an error if required Fluent keys are absent or empty.
pub fn build_narration(
    messages: &BTreeMap<String, String>,
    lang: &str,
    part: Part,
) -> Result<NarrationScript> {
    let Some(prefix) = part.ftl_prefix() else {
        let intro = clean_spoken_text(require_msg(messages, "sec-audio-intro-script", lang)?);
        if intro.is_empty() {
            return Err(Fail::new(format!("{lang}: empty intro script")));
        }
        return Ok(NarrationScript {
            text: intro,
            steps: Vec::new(),
        });
    };

    let title = clean_spoken_text(require_msg(messages, &format!("sec-{prefix}-title"), lang)?);
    let lead = clean_spoken_text(require_msg(messages, &format!("sec-{prefix}-lead"), lang)?);
    let step_tpl = require_msg(messages, "sec-step", lang)?;

    let mut text = format!(
        "{} {}",
        ensure_sentence_end(&title),
        ensure_sentence_end(&lead)
    );
    let mut steps = Vec::with_capacity(5);
    for n in 1_u8..=5_u8 {
        let label = clean_spoken_text(&step_tpl.replace("{ $n }", &n.to_string()));
        let t_key = format!("sec-{prefix}-t{n}");
        let s_key = format!("sec-{prefix}-s{n}");
        let step_title =
            ensure_sentence_end(&clean_spoken_text(require_msg(messages, &t_key, lang)?));
        let step_body =
            ensure_sentence_end(&clean_spoken_text(require_msg(messages, &s_key, lang)?));
        let step_text = format!("{label}: {step_title} {step_body}");

        text.push_str("\n\n");
        let char_start = text.chars().count();
        text.push_str(&step_text);
        let char_end = text.chars().count();

        steps.push(StepSpan {
            step: n,
            char_start,
            char_end,
            text: step_text,
        });
    }

    Ok(NarrationScript { text, steps })
}

fn round_ms(seconds: f64) -> f64 {
    if !seconds.is_finite() || seconds <= 0.0 {
        return 0.0;
    }
    (seconds * 1000.0).round() / 1000.0
}

fn proportional_time(char_pos: usize, total_chars: usize, duration: f64) -> f64 {
    if total_chars == 0 || char_pos == 0 {
        return 0.0;
    }
    let pos = u32::try_from(char_pos).unwrap_or(u32::MAX);
    let total = u32::try_from(total_chars).unwrap_or(u32::MAX).max(1);
    (f64::from(pos) / f64::from(total)) * duration
}

fn compact_alnum(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_whitespace() && !c.is_ascii_punctuation())
        .flat_map(char::to_lowercase)
        .collect()
}

fn find_anchor_time(
    step_text: &str,
    align_stream: &[(char, usize)],
    alignment: &Alignment,
    cursor: &mut usize,
) -> Option<f64> {
    let needle: Vec<char> = compact_alnum(step_text).chars().take(10).collect();
    if needle.len() < 3 {
        return None;
    }
    let max_start = align_stream.len().saturating_sub(needle.len());
    for start in *cursor..=max_start {
        if align_stream[start..]
            .iter()
            .zip(&needle)
            .all(|((ch, _), want)| ch == want)
        {
            *cursor = start + needle.len();
            let raw_idx = align_stream[start].1;
            return alignment.starts.get(raw_idx).copied();
        }
    }
    None
}

/// Converts `ElevenLabs` character alignment timestamps into seamless `[start, end]`
/// step cue windows and returns `(total_duration_seconds, step_cues)`.
#[must_use]
pub fn extract_step_cues(script: &NarrationScript, alignment: &Alignment) -> (f64, Vec<StepCue>) {
    let len = alignment
        .characters
        .len()
        .min(alignment.starts.len())
        .min(alignment.ends.len());
    let total_duration = alignment.ends[..len]
        .iter()
        .copied()
        .filter(|t| t.is_finite() && *t > 0.0)
        .fold(0.1_f64, f64::max);

    if script.steps.is_empty() {
        return (round_ms(total_duration), Vec::new());
    }

    let input_non_ws: Vec<char> = script.text.chars().filter(|c| !c.is_whitespace()).collect();
    let align_non_ws: Vec<(char, usize)> = alignment.characters[..len]
        .iter()
        .enumerate()
        .filter_map(|(idx, s)| {
            let ch = s.chars().find(|c| !c.is_whitespace())?;
            Some((ch, idx))
        })
        .collect();

    // Build map from `script.text` char offset -> `input_non_ws` index.
    let mut non_ws_before = Vec::with_capacity(script.text.chars().count() + 1);
    let mut count = 0_usize;
    for ch in script.text.chars() {
        non_ws_before.push(count);
        if !ch.is_whitespace() {
            count += 1;
        }
    }
    non_ws_before.push(count);
    let total_chars = script.text.chars().count();

    let exact_len_match = input_non_ws.len() == align_non_ws.len() && !align_non_ws.is_empty();
    let align_compact: Vec<(char, usize)> = alignment.characters[..len]
        .iter()
        .enumerate()
        .flat_map(|(idx, s)| {
            s.chars()
                .filter(|c| !c.is_whitespace() && !c.is_ascii_punctuation())
                .flat_map(char::to_lowercase)
                .map(move |c| (c, idx))
        })
        .collect();
    let mut fuzzy_cursor = 0_usize;
    let mut starts: Vec<f64> = Vec::with_capacity(script.steps.len());

    for span in &script.steps {
        let mut resolved = None;
        if exact_len_match
            && let Some(&nws_idx) = non_ws_before.get(span.char_start)
            && let Some(&(_, align_idx)) = align_non_ws.get(nws_idx)
        {
            resolved = alignment.starts.get(align_idx).copied();
        }
        if resolved.is_none() {
            resolved = find_anchor_time(&span.text, &align_compact, alignment, &mut fuzzy_cursor);
        }
        let fallback = proportional_time(span.char_start, total_chars, total_duration);
        let mut start_sec = resolved
            .filter(|t| t.is_finite() && *t >= 0.0)
            .unwrap_or(fallback);
        if let Some(&prev) = starts.last() {
            start_sec = start_sec.max(prev);
        }
        start_sec = start_sec.min(total_duration);
        starts.push(start_sec);
    }

    let mut cues = Vec::with_capacity(script.steps.len());
    for (idx, span) in script.steps.iter().enumerate() {
        let start = round_ms(starts[idx]);
        let next = starts.get(idx + 1).copied().unwrap_or(total_duration);
        let end = round_ms(next.max(starts[idx]));
        cues.push(StepCue {
            step: span.step,
            start,
            end,
        });
    }
    (round_ms(total_duration), cues)
}

/// Decodes standard RFC 4648 base64 (ignoring ASCII whitespace).
///
/// # Errors
/// Returns an error if the payload length or symbols are invalid base64.
pub fn decode_base64(input: &str) -> Result<Vec<u8>> {
    const fn val(b: u8) -> Option<u8> {
        match b {
            b'A'..=b'Z' => Some(b - b'A'),
            b'a'..=b'z' => Some(b - b'a' + 26),
            b'0'..=b'9' => Some(b - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }

    let clean: Vec<u8> = input.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    if clean.is_empty() || !clean.len().is_multiple_of(4) {
        return Err(Fail::new("invalid base64 length in audio payload"));
    }

    let mut out = Vec::with_capacity((clean.len() / 4) * 3);
    let last_chunk = (clean.len() / 4).saturating_sub(1);
    let (chunks, _) = clean.as_chunks::<4>();
    for (offset, chunk) in chunks.iter().enumerate() {
        let pad = usize::from(chunk[2] == b'=') + usize::from(chunk[3] == b'=');
        if pad > 0 && offset != last_chunk {
            return Err(Fail::new("misplaced padding in base64 audio payload"));
        }
        if chunk[2] == b'=' && chunk[3] != b'=' {
            return Err(Fail::new("malformed padding in base64 audio payload"));
        }
        let a = val(chunk[0]).or_fail("invalid base64 character")?;
        let b = val(chunk[1]).or_fail("invalid base64 character")?;
        out.push((a << 2) | (b >> 4));
        if chunk[2] != b'=' {
            let c = val(chunk[2]).or_fail("invalid base64 character")?;
            out.push((b << 4) | (c >> 2));
            if chunk[3] != b'=' {
                let d = val(chunk[3]).or_fail("invalid base64 character")?;
                out.push((c << 6) | d);
            }
        }
    }
    Ok(out)
}

fn build_agent() -> Agent {
    let config = Agent::config_builder()
        .https_only(true)
        .max_redirects(0)
        .timeout_connect(Some(Duration::from_secs(10)))
        .timeout_global(Some(Duration::from_secs(90)))
        .http_status_as_error(false)
        .user_agent("baylee-central-xtask")
        .build();
    Agent::new_with_config(config)
}

fn parse_alignment(json_val: &Value) -> Result<Alignment> {
    let block = json_val
        .get("alignment")
        .filter(|v| v.is_object())
        .or_else(|| json_val.get("normalized_alignment"))
        .or_fail("ElevenLabs response missing character alignment")?;

    let chars = block["characters"]
        .as_array()
        .or_fail("alignment missing characters array")?
        .iter()
        .map(|v| v.as_str().unwrap_or("").to_owned())
        .collect::<Vec<_>>();
    let starts = block["character_start_times_seconds"]
        .as_array()
        .or_fail("alignment missing start times")?
        .iter()
        .map(|v| v.as_f64().unwrap_or(0.0))
        .collect::<Vec<_>>();
    let ends = block["character_end_times_seconds"]
        .as_array()
        .or_fail("alignment missing end times")?
        .iter()
        .map(|v| v.as_f64().unwrap_or(0.0))
        .collect::<Vec<_>>();

    if chars.is_empty() || chars.len() != starts.len() || chars.len() != ends.len() {
        return Err(Fail::new(
            "ElevenLabs alignment arrays have inconsistent lengths",
        ));
    }
    Ok(Alignment {
        characters: chars,
        starts,
        ends,
    })
}

fn lookup_fallback_voice_id(agent: &Agent, api_key: &str) -> Result<String> {
    let url = format!("{API_BASE}/voices");
    let mut response = agent
        .get(&url)
        .header("xi-api-key", api_key)
        .header("Accept", "application/json")
        .call()
        .map_err(|err| Fail::new(format!("ElevenLabs voice lookup failed: {err}")))?;
    let status = response.status().as_u16();
    if status != 200 {
        return Err(Fail::new(format!(
            "ElevenLabs /v1/voices returned HTTP {status}"
        )));
    }
    let body = response
        .body_mut()
        .with_config()
        .limit(MAX_RESPONSE_BYTES)
        .read_to_vec()
        .map_err(|err| Fail::new(format!("failed reading /v1/voices response: {err}")))?;
    let parsed: Value = serde_json::from_slice(&body)
        .map_err(|err| Fail::new(format!("invalid /v1/voices JSON: {err}")))?;
    let voices = parsed["voices"]
        .as_array()
        .or_fail("ElevenLabs /v1/voices missing voices array")?;

    let preferred = voices
        .iter()
        .find(|voice| {
            let male = voice["labels"]["gender"].as_str() == Some("male");
            let id = voice["voice_id"].as_str().is_some_and(valid_voice_id);
            male && id
        })
        .or_else(|| {
            voices
                .iter()
                .find(|voice| voice["voice_id"].as_str().is_some_and(valid_voice_id))
        })
        .and_then(|voice| voice["voice_id"].as_str())
        .or_fail("no usable voice ID returned by ElevenLabs /v1/voices")?;
    Ok(preferred.to_owned())
}

fn synthesize_with_voice(
    agent: &Agent,
    api_key: &str,
    voice_id: &str,
    lang: &str,
    text: &str,
) -> std::result::Result<(Vec<u8>, Alignment), (u16, String)> {
    let url = format!(
        "{API_BASE}/text-to-speech/{voice_id}/with-timestamps?output_format={OUTPUT_FORMAT}"
    );
    let payload = json!({
        "text": text,
        "model_id": MODEL_ID,
        "language_code": elevenlabs_language_code(lang),
        "voice_settings": {
            "stability": 0.55,
            "similarity_boost": 0.75
        }
    });
    let encoded = serde_json::to_vec(&payload)
        .map_err(|err| (0, format!("failed encoding TTS request: {err}")))?;

    let mut response = agent
        .post(&url)
        .header("xi-api-key", api_key)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .send(encoded.as_slice())
        .map_err(|err| (0, format!("ElevenLabs request failed: {err}")))?;

    let status = response.status().as_u16();
    if status != 200 {
        let err_bytes = response
            .body_mut()
            .with_config()
            .limit(MAX_ERROR_BYTES)
            .read_to_vec()
            .unwrap_or_default();
        let detail = String::from_utf8_lossy(&err_bytes);
        let detail = detail.trim();
        return Err((
            status,
            if detail.is_empty() {
                format!("ElevenLabs returned HTTP {status} for `{lang}`")
            } else {
                format!("ElevenLabs returned HTTP {status} for `{lang}`: {detail}")
            },
        ));
    }

    let raw = response
        .body_mut()
        .with_config()
        .limit(MAX_RESPONSE_BYTES)
        .read_to_vec()
        .map_err(|err| (status, format!("failed reading TTS response: {err}")))?;
    let json_val: Value = serde_json::from_slice(&raw)
        .map_err(|err| (status, format!("invalid TTS JSON response: {err}")))?;
    let b64 = json_val["audio_base64"].as_str().ok_or_else(|| {
        (
            status,
            "ElevenLabs response missing audio_base64".to_owned(),
        )
    })?;
    let audio = decode_base64(b64).map_err(|err| (status, err.to_string()))?;
    if audio.is_empty() {
        return Err((status, "ElevenLabs returned empty audio bytes".to_owned()));
    }
    if !audio.starts_with(b"OggS") {
        let mut head = String::new();
        for byte in audio.iter().take(4) {
            let _ = std::fmt::Write::write_fmt(&mut head, format_args!("{byte:02x}"));
        }
        return Err((
            status,
            format!("ElevenLabs did not return an Ogg/Opus stream (head bytes: {head})"),
        ));
    }
    let alignment = parse_alignment(&json_val).map_err(|err| (status, err.to_string()))?;
    Ok((audio, alignment))
}

fn load_existing_cues(path: &Path) -> BTreeMap<String, Value> {
    let Ok(text) = fs::read_to_string(path) else {
        return BTreeMap::new();
    };
    let Ok(val) = serde_json::from_str::<Value>(&text) else {
        return BTreeMap::new();
    };
    let Some(parts) = val.get("parts").and_then(Value::as_object) else {
        return BTreeMap::new();
    };
    parts.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
}

struct SynthContext<'a> {
    web: &'a Path,
    agent: &'a Agent,
    api_key: &'a str,
    allow_fallback: bool,
    force: bool,
}

fn synthesize_language(
    ctx: &SynthContext<'_>,
    lang: &str,
    parts: &[(Part, NarrationScript)],
    voice_id: &mut String,
) -> Result<()> {
    let out_dir = ctx.web.join("static/audio/security").join(lang);
    fs::create_dir_all(&out_dir)
        .or_fail("could not create static/audio/security language folder")?;
    let cues_path = out_dir.join("cues.json");
    let mut parts_map = if ctx.force {
        BTreeMap::new()
    } else {
        load_existing_cues(&cues_path)
    };

    for (part, script) in parts {
        let audio_name = format!("{}.opus", part.key());
        let audio_path = out_dir.join(&audio_name);
        if !ctx.force && audio_path.is_file() && parts_map.contains_key(part.key()) {
            println!("Skipping existing {lang}/{audio_name}");
            continue;
        }

        let synthesized =
            match synthesize_with_voice(ctx.agent, ctx.api_key, voice_id, lang, &script.text) {
                Ok(ok) => ok,
                Err((404, _)) if ctx.allow_fallback => {
                    *voice_id = lookup_fallback_voice_id(ctx.agent, ctx.api_key)?;
                    synthesize_with_voice(ctx.agent, ctx.api_key, voice_id, lang, &script.text)
                        .map_err(|(_, msg)| Fail::new(msg))?
                }
                Err((_, msg)) => return Err(Fail::new(msg)),
            };

        let (audio_bytes, alignment) = synthesized;
        let (duration, step_cues) = extract_step_cues(script, &alignment);
        fs::write(&audio_path, audio_bytes).or_fail("could not write synthesized Opus file")?;

        let steps_json: Vec<Value> = step_cues
            .iter()
            .map(|cue| {
                json!({
                    "step": cue.step,
                    "start": cue.start,
                    "end": cue.end,
                })
            })
            .collect();
        parts_map.insert(
            part.key().to_owned(),
            json!({
                "src": format!("/static/audio/security/{lang}/{audio_name}"),
                "duration": duration,
                "steps": steps_json,
            }),
        );
        println!(
            "Synthesized {lang}/{audio_name} ({duration:.2}s, {} step cues)",
            step_cues.len()
        );
    }

    let manifest = json!({
        "available": parts_map.len() == Part::ALL.len(),
        "lang": lang,
        "voiceId": voice_id.as_str(),
        "modelId": MODEL_ID,
        "parts": parts_map,
    });
    fs::write(&cues_path, format!("{manifest:#}\n")).or_fail("could not write cues.json manifest")
}

/// Runs `cargo xtask security-audio`.
///
/// # Errors
/// Fails when Fluent scripts are malformed, `ELEVENLABS_API_KEY` is missing on a live
/// synthesis run, or `ElevenLabs` API requests fail.
pub fn run(root: &Path, options: &Options) -> Result<()> {
    let web = root.join("crates/central-web");
    let langs: Vec<String> = if options.langs.is_empty() {
        LANGS.iter().map(|s| (*s).to_owned()).collect()
    } else {
        options.langs.clone()
    };

    let mut scripts_by_lang = Vec::with_capacity(langs.len());
    for lang in &langs {
        let ftl_path = web.join("locales").join(lang).join("security.ftl");
        let ftl_text = fs::read_to_string(&ftl_path)
            .or_fail("security.ftl could not be read for selected language")?;
        let messages = parse_ftl_messages(&ftl_text)?;
        let mut parts = Vec::with_capacity(Part::ALL.len());
        for part in Part::ALL {
            let script = build_narration(&messages, lang, part)?;
            parts.push((part, script));
        }
        scripts_by_lang.push((lang.clone(), parts));
    }

    let mut voice_id = options
        .voice_id
        .clone()
        .unwrap_or_else(|| DEFAULT_VOICE_ID.to_owned());

    if options.dry_run {
        for (lang, parts) in &scripts_by_lang {
            for (part, script) in parts {
                println!(
                    "[dry-run] {lang}/{}.opus: {} chars, {} steps (model={MODEL_ID}, voice={voice_id})",
                    part.key(),
                    script.text.chars().count(),
                    script.steps.len()
                );
            }
        }
        return Ok(());
    }

    let api_key = std::env::var("ELEVENLABS_API_KEY")
        .ok()
        .map(|v| v.trim().to_owned())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| {
            Fail::new("ELEVENLABS_API_KEY environment variable is required (not passed on CLI)")
        })?;
    let agent = build_agent();
    let ctx = SynthContext {
        web: &web,
        agent: &agent,
        api_key: &api_key,
        allow_fallback: options.voice_id.is_none(),
        force: options.force,
    };

    for (lang, parts) in &scripts_by_lang {
        synthesize_language(&ctx, lang, parts, &mut voice_id)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_spoken_text_strips_tags_and_entities() {
        let raw = "<b>When you register,</b> use <code>baylee://pair</code> &amp; <contact>tell us</contact>.";
        assert_eq!(
            clean_spoken_text(raw),
            "When you register, use baylee://pair & tell us."
        );
    }

    #[test]
    fn all_eleven_locales_build_valid_narration_scripts() {
        let root = crate::workspace::root().expect("workspace root");
        for lang in LANGS {
            let path = root
                .join("crates/central-web/locales")
                .join(lang)
                .join("security.ftl");
            let ftl = fs::read_to_string(&path).expect("security.ftl exists");
            let messages = parse_ftl_messages(&ftl).expect("valid security.ftl");
            for part in Part::ALL {
                let script = build_narration(&messages, lang, part).expect("narration script");
                assert!(!script.text.is_empty(), "{lang}/{}", part.key());
                assert!(!script.text.contains("<b>"), "{lang}/{}", part.key());
                assert!(!script.text.contains("<code>"), "{lang}/{}", part.key());
                assert!(!script.text.contains("{ $"), "{lang}/{}", part.key());
                if part == Part::Intro {
                    assert_eq!(script.steps, [] as [StepSpan; 0]);
                } else {
                    assert_eq!(script.steps.len(), 5, "{lang}/{}", part.key());
                    let mut prev_end = 0;
                    for (idx, span) in script.steps.iter().enumerate() {
                        assert_eq!(usize::from(span.step), idx + 1);
                        assert!(span.char_start >= prev_end);
                        assert!(span.char_end > span.char_start);
                        prev_end = span.char_end;
                    }
                }
            }
        }
    }

    #[test]
    fn decode_base64_handles_padding_and_rejects_malformed() {
        assert_eq!(decode_base64("Zg==").unwrap(), b"f");
        assert_eq!(decode_base64("Zm8=").unwrap(), b"fo");
        assert_eq!(decode_base64("Zm9v").unwrap(), b"foo");
        assert_eq!(decode_base64("Zm9vYmFy\n").unwrap(), b"foobar");
        assert!(decode_base64("").is_err());
        assert!(decode_base64("abc").is_err());
        assert!(decode_base64("ab=d").is_err());
        assert!(decode_base64("!!!!").is_err());
    }

    #[test]
    fn extract_step_cues_maps_exact_and_fuzzy_character_timings() {
        let mut messages = BTreeMap::new();
        messages.insert("sec-step".to_owned(), "Step { $n }".to_owned());
        messages.insert("sec-signin-title".to_owned(), "Sign in".to_owned());
        messages.insert("sec-signin-lead".to_owned(), "Quick intro".to_owned());
        for n in 1..=5 {
            messages.insert(format!("sec-signin-t{n}"), format!("Title {n}"));
            messages.insert(format!("sec-signin-s{n}"), format!("Summary number {n}"));
        }

        let script = build_narration(&messages, "en", Part::Signin).expect("script");
        let mut chars = Vec::new();
        let mut starts = Vec::new();
        let mut ends = Vec::new();
        let mut clock = 0.0_f64;
        for ch in script.text.chars() {
            chars.push(ch.to_string());
            starts.push(clock);
            clock += 0.1;
            ends.push(clock);
        }
        let alignment = Alignment {
            characters: chars,
            starts,
            ends,
        };

        let (duration, cues) = extract_step_cues(&script, &alignment);
        assert!(duration > 5.0);
        assert_eq!(cues.len(), 5);
        for (idx, cue) in cues.iter().enumerate() {
            assert_eq!(usize::from(cue.step), idx + 1);
            assert!(cue.start > 1.5);
            assert!(cue.end >= cue.start);
            if let Some(next) = cues.get(idx + 1) {
                assert!((cue.end - next.start).abs() < 0.002);
            } else {
                assert!((cue.end - duration).abs() < 0.002);
            }
        }
    }
}
