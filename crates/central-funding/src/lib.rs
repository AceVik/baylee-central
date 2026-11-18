//! What the donation platforms tell Baylee Central, read strictly
//! (`docs/operations.md` §"Donations"): Liberapay's public figures, GitHub
//! Sponsors' answer to the sponsored account's token, the European Central
//! Bank's reference rates, and Ko-fi's deliveries to Central's webhook.
//!
//! This crate asks nobody and stores nothing. Every reader takes a bounded
//! text the caller fetched or was sent and returns numbers, or nothing: no
//! text of an answer becomes a figure, and nothing that names a giver or a
//! recipient is ever deserialised. Amounts are decimal text read by one
//! rule ([`amount::hundredths`]); no amount passes through a float.
#![forbid(unsafe_code)]

pub mod amount;
pub mod github;
pub mod kofi;
pub mod liberapay;
pub mod rates;

pub use amount::Currency;
pub use rates::Rates;

/// How a platform's transaction is remembered for de-duplication: the SHA-256 of the
/// platform's name and its own id, never the id itself. A reader of the store cannot
/// look a donation up at the platform by it.
#[must_use]
pub fn reference(platform: &str, id: &str) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(platform.as_bytes());
    hasher.update([0]);
    hasher.update(id.as_bytes());
    hasher.finalize().into()
}

/// A platform's own id of a transaction or an activity: 1 to `most` characters of
/// letters, digits and `-_=`; anything else is no id.
fn plain_id(id: &str, most: usize) -> bool {
    (1..=most).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'='))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reference_is_the_platform_s_and_never_the_id() {
        let kofi = reference("kofi", "00000000-1111-2222-3333-444444444444");
        assert_ne!(
            kofi,
            reference("github", "00000000-1111-2222-3333-444444444444")
        );
        assert_ne!(
            kofi,
            reference("kofi", "00000000-1111-2222-3333-444444444445")
        );
        // the separator: no platform and id run into another pair
        assert_ne!(reference("ko", "fi1"), reference("kof", "i1"));
        assert_eq!(
            kofi,
            reference("kofi", "00000000-1111-2222-3333-444444444444")
        );
    }

    #[test]
    fn an_id_is_plain_and_bounded() {
        assert!(plain_id("SA_kwDOABCD12", 100));
        assert!(plain_id("00000000-1111-2222-3333-444444444444", 64));
        for id in ["", "a b", "a/b", "<x>", "é", "a\u{0}"] {
            assert!(!plain_id(id, 64), "{id:?}");
        }
        assert!(!plain_id(&"a".repeat(65), 64));
    }
}
