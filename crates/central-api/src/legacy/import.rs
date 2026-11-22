//! Shape and recovery bounds of the original operator export.

use super::{MAX_ARCHIVE_BYTES, MAX_SNAPSHOT_BYTES};
use serde::{Deserialize, de::IgnoredAny};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    preferences: serde_json::Map<String, serde_json::Value>,
    // Recovery retains the original bytes. Counting decks does not require materializing their
    // potentially large card arrays as a second in-memory tree.
    decks: Vec<IgnoredAny>,
}

/// Whether this original gateway snapshot can be imported and recovered by a moving client.
/// Large sources require at least one deck and preferences that fit in the original document budget.
#[must_use]
pub fn valid_import_snapshot(snapshot: &[u8]) -> bool {
    if snapshot.len() > MAX_ARCHIVE_BYTES {
        return false;
    }
    let Ok(source) = serde_json::from_slice::<Source>(snapshot) else {
        return false;
    };
    snapshot.len() <= MAX_SNAPSHOT_BYTES
        || (!source.decks.is_empty()
            && serde_json::to_vec(&source.preferences)
                .is_ok_and(|encoded| encoded.len() <= MAX_SNAPSHOT_BYTES))
}
