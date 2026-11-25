//! Bounded in-browser caching for chat card autocomplete, author-exact
//! Scryfall card printings, decoded card artwork warm-loading, and transient
//! in-memory `@User` public profiles.
//!
//! Card catalog entries persist in `localStorage` under
//! `baylee-card-cache-v1` (bounded to `128` entries / 7-day TTL) and are
//! strictly re-validated on read so Universes Beyond records never carry an
//! image UUID. User profile lookups stay strictly in memory (never written to
//! `localStorage`) with a 3-minute TTL.

use std::cell::RefCell;
use std::collections::BTreeMap;

use central_api::profile::PublicProfile;
use central_api::social::{CardPrinting, CardSuggestion};
use serde::{Deserialize, Serialize};

use super::wire;

const CARD_CACHE_KEY: &str = "baylee-card-cache-v1";
const MAX_CARD_CACHE_ENTRIES: usize = 128;
const MAX_SUGGEST_CACHE_ENTRIES: usize = 64;
const MAX_USER_CACHE_ENTRIES: usize = 64;
const MAX_CACHED_PRINTINGS: usize = 16;
#[cfg(target_arch = "wasm32")]
const MAX_WARM_IMAGES: usize = 48;

const CARD_CACHE_TTL_MS: f64 = 7.0 * 24.0 * 3600.0 * 1000.0;
const USER_CACHE_TTL_MS: f64 = 3.0 * 60.0 * 1000.0;

/// Normalized cache lookup key for a card catalog query.
#[must_use]
pub fn card_cache_key(
    name: &str,
    lang_tag: &str,
    set_filter: Option<&str>,
    collector_filter: Option<&str>,
    finish_filter: Option<&str>,
) -> String {
    format!(
        "{}|{}|{}|{}|{}",
        name.trim().to_ascii_lowercase(),
        lang_tag.trim().to_ascii_lowercase(),
        set_filter.unwrap_or("").trim().to_ascii_lowercase(),
        collector_filter.unwrap_or("").trim().to_ascii_lowercase(),
        finish_filter.unwrap_or("").trim().to_ascii_lowercase(),
    )
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CachedCardEntry {
    card: Option<CardSuggestion>,
    printings: Vec<CardPrinting>,
    saved_at_ms: f64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct PersistedCardStore {
    entries: BTreeMap<String, CachedCardEntry>,
}

#[derive(Clone, Debug)]
struct CachedSuggestEntry {
    items: Vec<CardSuggestion>,
    saved_at_ms: f64,
}

#[derive(Clone, Debug)]
struct CachedUserEntry {
    profile: Option<PublicProfile>,
    saved_at_ms: f64,
}

#[cfg(target_arch = "wasm32")]
thread_local! {
    static CARD_STORE: RefCell<Option<PersistedCardStore>> = const { RefCell::new(None) };
    static SUGGEST_CACHE: RefCell<BTreeMap<String, CachedSuggestEntry>> = const { RefCell::new(BTreeMap::new()) };
    static USER_CACHE: RefCell<BTreeMap<String, CachedUserEntry>> = const { RefCell::new(BTreeMap::new()) };
    static WARM_IMAGES: RefCell<Vec<(String, leptos::web_sys::HtmlImageElement)>> = const { RefCell::new(Vec::new()) };
}

#[cfg(not(target_arch = "wasm32"))]
thread_local! {
    static CARD_STORE: RefCell<Option<PersistedCardStore>> = const { RefCell::new(None) };
    static SUGGEST_CACHE: RefCell<BTreeMap<String, CachedSuggestEntry>> = const { RefCell::new(BTreeMap::new()) };
    static USER_CACHE: RefCell<BTreeMap<String, CachedUserEntry>> = const { RefCell::new(BTreeMap::new()) };
}

fn now_ms() -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::now()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0.0, |d| d.as_secs_f64() * 1000.0)
    }
}

fn entry_valid(entry: &CachedCardEntry, now: f64) -> bool {
    if !entry.saved_at_ms.is_finite() {
        return false;
    }
    let age = now - entry.saved_at_ms;
    if !(0.0..=CARD_CACHE_TTL_MS).contains(&age) {
        return false;
    }
    entry.card.as_ref().is_none_or(wire::card_suggestion_valid)
        && entry.printings.len() <= MAX_CACHED_PRINTINGS
        && entry
            .printings
            .iter()
            .all(|p| wire::card_printing_valid(p) && (!p.ub || p.scryfall_id.is_none()))
}

#[cfg(any(target_arch = "wasm32", test))]
fn parse_persisted_store(raw: &str, now: f64) -> PersistedCardStore {
    let Ok(mut parsed) = serde_json::from_str::<PersistedCardStore>(raw) else {
        return PersistedCardStore::default();
    };
    parsed.entries.retain(|_, entry| entry_valid(entry, now));
    while parsed.entries.len() > MAX_CARD_CACHE_ENTRIES {
        if !evict_oldest_card(&mut parsed.entries) {
            break;
        }
    }
    parsed
}

fn evict_oldest_card(map: &mut BTreeMap<String, CachedCardEntry>) -> bool {
    let oldest = map
        .iter()
        .min_by(|a, b| a.1.saved_at_ms.total_cmp(&b.1.saved_at_ms))
        .map(|(k, _)| k.clone());
    oldest.is_some_and(|k| map.remove(&k).is_some())
}

fn evict_oldest_suggest(map: &mut BTreeMap<String, CachedSuggestEntry>) {
    while map.len() >= MAX_SUGGEST_CACHE_ENTRIES {
        let Some(oldest) = map
            .iter()
            .min_by(|a, b| a.1.saved_at_ms.total_cmp(&b.1.saved_at_ms))
            .map(|(k, _)| k.clone())
        else {
            break;
        };
        map.remove(&oldest);
    }
}

fn evict_oldest_user(map: &mut BTreeMap<String, CachedUserEntry>) {
    while map.len() >= MAX_USER_CACHE_ENTRIES {
        let Some(oldest) = map
            .iter()
            .min_by(|a, b| a.1.saved_at_ms.total_cmp(&b.1.saved_at_ms))
            .map(|(k, _)| k.clone())
        else {
            break;
        };
        map.remove(&oldest);
    }
}

fn load_store(now: f64) -> PersistedCardStore {
    #[cfg(target_arch = "wasm32")]
    {
        let raw = leptos::web_sys::window()
            .and_then(|w| w.local_storage().ok().flatten())
            .and_then(|s| s.get_item(CARD_CACHE_KEY).ok().flatten());
        raw.map(|text| parse_persisted_store(&text, now))
            .unwrap_or_default()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (now, CARD_CACHE_KEY);
        PersistedCardStore::default()
    }
}

fn save_store(store: &PersistedCardStore) {
    #[cfg(target_arch = "wasm32")]
    {
        let Some(storage) =
            leptos::web_sys::window().and_then(|w| w.local_storage().ok().flatten())
        else {
            return;
        };
        if let Ok(json) = serde_json::to_string(store) {
            // Silently ignore quota exceeded errors; in-memory cache stays intact.
            let _ = storage.set_item(CARD_CACHE_KEY, &json);
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = store;
    }
}

fn ensure_card_store<R>(now: f64, f: impl FnOnce(&mut PersistedCardStore) -> R) -> R {
    CARD_STORE.with(|slot| {
        let mut guard = slot.borrow_mut();
        let store = guard.get_or_insert_with(|| load_store(now));
        f(store)
    })
}

/// Returns a cached `(CardSuggestion, Vec<CardPrinting>)` result when fresh and valid.
#[must_use]
pub fn get_card_printings(key: &str) -> Option<(Option<CardSuggestion>, Vec<CardPrinting>)> {
    let now = now_ms();
    ensure_card_store(now, |store| {
        let hit = store.entries.get(key).cloned();
        match hit {
            Some(entry) if entry_valid(&entry, now) => Some((entry.card, entry.printings)),
            Some(_) => {
                store.entries.remove(key);
                save_store(store);
                None
            }
            None => None,
        }
    })
}

/// Stores validated `(CardSuggestion, Vec<CardPrinting>)` in memory and `localStorage`
/// (bounded to `128` entries and at most `16` printings per entry).
pub fn put_card_printings(
    key: String,
    card: Option<CardSuggestion>,
    mut printings: Vec<CardPrinting>,
) {
    if !card.as_ref().is_none_or(wire::card_suggestion_valid) {
        return;
    }
    printings.retain(|p| wire::card_printing_valid(p) && (!p.ub || p.scryfall_id.is_none()));
    printings.truncate(MAX_CACHED_PRINTINGS);
    if card.is_none() && printings.is_empty() {
        return;
    }
    let now = now_ms();
    ensure_card_store(now, |store| {
        while store.entries.len() >= MAX_CARD_CACHE_ENTRIES && !store.entries.contains_key(&key) {
            if !evict_oldest_card(&mut store.entries) {
                break;
            }
        }
        store.entries.insert(
            key,
            CachedCardEntry {
                card,
                printings,
                saved_at_ms: now,
            },
        );
        save_store(store);
    });
}

/// Returns cached autocomplete card suggestions when present in memory.
#[must_use]
pub fn get_card_suggestions(key: &str) -> Option<Vec<CardSuggestion>> {
    let now = now_ms();
    SUGGEST_CACHE.with(|cell| {
        let mut map = cell.borrow_mut();
        match map.get(key) {
            Some(entry)
                if entry.saved_at_ms.is_finite()
                    && (0.0..=CARD_CACHE_TTL_MS).contains(&(now - entry.saved_at_ms)) =>
            {
                Some(entry.items.clone())
            }
            Some(_) => {
                map.remove(key);
                None
            }
            None => None,
        }
    })
}

/// Caches autocomplete card suggestions in memory (LRU bounded to `64` entries).
pub fn put_card_suggestions(key: String, items: &[CardSuggestion]) {
    if !items.iter().all(wire::card_suggestion_valid) {
        return;
    }
    let now = now_ms();
    SUGGEST_CACHE.with(|cell| {
        let mut map = cell.borrow_mut();
        if !map.contains_key(&key) {
            evict_oldest_suggest(&mut map);
        }
        map.insert(
            key,
            CachedSuggestEntry {
                items: items.to_vec(),
                saved_at_ms: now,
            },
        );
    });
}

/// Result of looking up an `@User` profile in the bounded in-memory cache.
#[derive(Clone, Debug)]
pub enum CachedUserProfile {
    /// Fresh cached public profile.
    Found(Box<PublicProfile>),
    /// Fresh negative cache hit (user missing or private).
    Missing,
}

/// Returns a cached in-memory `@User` public profile when fresh (3-minute TTL).
#[must_use]
pub fn get_user_profile(handle_lower: &str) -> Option<CachedUserProfile> {
    let now = now_ms();
    USER_CACHE.with(|cell| {
        let mut map = cell.borrow_mut();
        match map.get(handle_lower) {
            Some(entry)
                if entry.saved_at_ms.is_finite()
                    && (0.0..=USER_CACHE_TTL_MS).contains(&(now - entry.saved_at_ms)) =>
            {
                Some(match entry.profile.clone() {
                    Some(profile) => CachedUserProfile::Found(Box::new(profile)),
                    None => CachedUserProfile::Missing,
                })
            }
            Some(_) => {
                map.remove(handle_lower);
                None
            }
            None => None,
        }
    })
}

/// Stores an `@User` public profile strictly in memory (never `localStorage`).
pub fn put_user_profile(handle_lower: String, profile: Option<PublicProfile>) {
    let now = now_ms();
    USER_CACHE.with(|cell| {
        let mut map = cell.borrow_mut();
        if !map.contains_key(&handle_lower) {
            evict_oldest_user(&mut map);
        }
        map.insert(
            handle_lower,
            CachedUserEntry {
                profile,
                saved_at_ms: now,
            },
        );
    });
}

/// Preloads and decodes an allowed `cards.scryfall.io` image URL in memory so
/// repeated hovers swap instantaneously. Callers MUST ensure card previews are
/// enabled and `!printing.ub`.
pub fn warm_card_image(url: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        if !url.starts_with("https://cards.scryfall.io/") {
            return;
        }
        WARM_IMAGES.with(|cell| {
            let mut list = cell.borrow_mut();
            if list.iter().any(|(existing, _)| existing == url) {
                return;
            }
            let Ok(img) = leptos::web_sys::HtmlImageElement::new() else {
                return;
            };
            img.set_referrer_policy("no-referrer");
            img.set_decoding("async");
            img.set_src(url);
            if list.len() >= MAX_WARM_IMAGES {
                list.remove(0);
            }
            list.push((url.to_owned(), img));
        });
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = url;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use central_api::social::FINISH_FOIL;

    fn sample_printing(ub: bool, scryfall_id: Option<&str>) -> CardPrinting {
        CardPrinting {
            scryfall_id: scryfall_id.map(str::to_owned),
            set_code: "2x2".into(),
            set_name: "Double Masters 2022".into(),
            collector_number: "117".into(),
            lang: "de".into(),
            finishes: FINISH_FOIL,
            ub,
        }
    }

    #[test]
    fn card_cache_enforces_ttl_ub_and_lru_bounds() {
        let now = 10_000_000_000.0;
        let fresh = sample_printing(false, Some("9c3f2eb1-3e79-4d7c-bf96-d920f973b122"));
        let bad_ub = sample_printing(true, Some("9c3f2eb1-3e79-4d7c-bf96-d920f973b122"));
        let mut entries = BTreeMap::new();
        entries.insert(
            "valid".into(),
            CachedCardEntry {
                card: None,
                printings: vec![fresh.clone()],
                saved_at_ms: now - 1000.0,
            },
        );
        entries.insert(
            "expired".into(),
            CachedCardEntry {
                card: None,
                printings: vec![fresh.clone()],
                saved_at_ms: now - CARD_CACHE_TTL_MS - 1.0,
            },
        );
        entries.insert(
            "ub-leak".into(),
            CachedCardEntry {
                card: None,
                printings: vec![bad_ub],
                saved_at_ms: now - 500.0,
            },
        );
        let raw = serde_json::to_string(&PersistedCardStore { entries }).expect("serialize");
        let parsed = parse_persisted_store(&raw, now);
        assert!(parsed.entries.contains_key("valid"));
        assert!(!parsed.entries.contains_key("expired"));
        assert!(!parsed.entries.contains_key("ub-leak"));

        let mut many = BTreeMap::new();
        for i in 0..=MAX_CARD_CACHE_ENTRIES + 5 {
            many.insert(
                format!("k{i:03}"),
                CachedCardEntry {
                    card: None,
                    printings: vec![fresh.clone()],
                    saved_at_ms: now - f64::from(u32::try_from(i).unwrap_or(0)),
                },
            );
        }
        let raw_many =
            serde_json::to_string(&PersistedCardStore { entries: many }).expect("serialize many");
        let bounded = parse_persisted_store(&raw_many, now);
        assert_eq!(bounded.entries.len(), MAX_CARD_CACHE_ENTRIES);
        assert!(bounded.entries.contains_key("k000"));
    }

    #[test]
    fn suggestion_and_user_memory_caches_round_trip() {
        let key = card_cache_key("Blitzschlag", "de", Some("2x2"), Some("117"), Some("foil"));
        assert_eq!(key, "blitzschlag|de|2x2|117|foil");
        let s = CardSuggestion {
            name: "Blitzschlag".into(),
            en_name: "Lightning Bolt".into(),
            matched_lang: "de".into(),
            preview: Some(sample_printing(false, None)),
        };
        put_card_suggestions(key.clone(), std::slice::from_ref(&s));
        assert_eq!(get_card_suggestions(&key), Some(vec![s]));

        put_user_profile("lunaduel#82dd".into(), None);
        assert!(matches!(
            get_user_profile("lunaduel#82dd"),
            Some(CachedUserProfile::Missing)
        ));
    }
}
