//! Terms of use: a document version, its texts and the digest an acceptance
//! names (`docs/terms.md`). Text is untrusted display data: render only the
//! Markdown subset of `legal/README.md`, never HTML. Nothing here is secret,
//! so these types print.
use crate::Error;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// The version that is never in force.
pub const DRAFT: &str = "draft";
/// Most bytes of a version string.
pub const MAX_VERSION_BYTES: usize = 64;
/// Most languages one version carries.
pub const MAX_TEXTS: usize = 4;
/// Most UTF-8 bytes of one language's text.
pub const MAX_TEXT_BYTES: usize = 65_536;
/// Most bytes of a text's `updated` date.
pub const MAX_UPDATED_BYTES: usize = 32;

/// One language of a document version.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TermsText {
    /// Primary language tag: two or three lowercase ASCII letters.
    pub lang: String,
    /// The date the text names as its last change, if it names one.
    pub updated: Option<String>,
    /// The text, in the Markdown subset; no control characters but newline and tab.
    pub markdown: String,
}

/// A version and its digest: what an acceptance names.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TermsVersion {
    /// Printable ASCII, no whitespace, never [`DRAFT`].
    pub version: String,
    /// [`digest`] of the version's texts.
    pub digest: [u8; 32],
}

/// A document version in force, with every language it carries.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TermsDocument {
    /// Version and digest.
    pub version: TermsVersion,
    /// Its languages, in ascending tag order.
    pub texts: Vec<TermsText>,
}

/// Whose terms an acceptance is for.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TermsSubject {
    /// Baylee Central's own terms.
    Central,
    /// A gateway's terms, by its canonical origin.
    Gateway {
        /// The gateway's canonical origin, as its tickets name it.
        origin: String,
    },
}

/// A version string a published document may carry.
#[must_use]
pub fn valid_version(version: &str) -> bool {
    (1..=MAX_VERSION_BYTES).contains(&version.len())
        && version.bytes().all(|b| b.is_ascii_graphic())
        && version != DRAFT
}

fn valid_lang(lang: &str) -> bool {
    (2..=3).contains(&lang.len()) && lang.bytes().all(|b| b.is_ascii_lowercase())
}

fn valid_markdown(markdown: &str) -> bool {
    (1..=MAX_TEXT_BYTES).contains(&markdown.len())
        && markdown
            .chars()
            .all(|c| c == '\n' || c == '\t' || !c.is_control())
}

fn valid_updated(updated: Option<&str>) -> bool {
    updated.is_none_or(|date| {
        (1..=MAX_UPDATED_BYTES).contains(&date.len()) && date.bytes().all(|b| b.is_ascii_graphic())
    })
}

/// Check a version's texts: one to [`MAX_TEXTS`] languages in strictly
/// ascending tag order, German or English among them, each text bounded.
///
/// # Errors
/// Rejects any text outside these bounds.
pub fn validate(texts: &[TermsText]) -> Result<(), Error> {
    let ordered = texts.windows(2).all(|pair| pair[0].lang < pair[1].lang);
    let known = texts
        .iter()
        .any(|text| text.lang == "de" || text.lang == "en");
    if !(1..=MAX_TEXTS).contains(&texts.len())
        || !ordered
        || !known
        || !texts.iter().all(|text| {
            valid_lang(&text.lang)
                && valid_markdown(&text.markdown)
                && valid_updated(text.updated.as_deref())
        })
    {
        return Err(Error::Rejected);
    }
    Ok(())
}

fn field(hash: &mut Sha256, bytes: &[u8]) {
    hash.update((bytes.len() as u64).to_be_bytes());
    hash.update(bytes);
}

/// The digest of a version's texts: SHA-256 over a domain label and, per
/// text in the given order, its tag, its date (empty when none) and its
/// Markdown, each preceded by its length. Call [`validate`] first.
#[must_use]
pub fn digest(texts: &[TermsText]) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(b"baylee-terms-v1\0");
    for text in texts {
        field(&mut hash, text.lang.as_bytes());
        field(&mut hash, text.updated.as_deref().unwrap_or("").as_bytes());
        field(&mut hash, text.markdown.as_bytes());
    }
    hash.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(lang: &str, markdown: &str) -> TermsText {
        TermsText {
            lang: lang.into(),
            updated: Some("2026-10-20".into()),
            markdown: markdown.into(),
        }
    }

    #[test]
    fn texts_are_bounded_ordered_and_name_german_or_english() {
        assert!(validate(&[text("de", "# A"), text("en", "# A")]).is_ok());
        assert!(validate(&[text("en", "# A")]).is_ok());
        for bad in [
            vec![],
            vec![text("en", "# A"), text("de", "# A")],
            vec![text("en", "# A"), text("en", "# B")],
            vec![text("fr", "# A")],
            vec![text("EN", "# A")],
            vec![text("eng-", "# A")],
            vec![text("en", "")],
            vec![text("en", "a\u{0}b")],
            vec![text("en", "a\rb")],
            vec![text("en", &"a".repeat(MAX_TEXT_BYTES + 1))],
            vec![
                text("de", "a"),
                text("en", "a"),
                text("es", "a"),
                text("fr", "a"),
                text("it", "a"),
            ],
        ] {
            assert!(validate(&bad).is_err());
        }
        let mut dated = text("en", "# A");
        dated.updated = Some("2026 10 20".into());
        assert!(validate(&[dated]).is_err());
        assert!(validate(&[text("en", "tabs\tand\nnewlines")]).is_ok());
    }

    #[test]
    fn versions_are_printable_bounded_and_never_draft() {
        assert!(valid_version("2026-10-1"));
        for bad in ["", "draft", "a b", "ä", &"v".repeat(MAX_VERSION_BYTES + 1)] {
            assert!(!valid_version(bad), "{bad}");
        }
    }

    #[test]
    fn the_digest_separates_every_field_and_follows_every_byte() {
        let base = [text("de", "# A"), text("en", "# B")];
        let same = digest(&base);
        assert_eq!(same, digest(&base.clone()));
        let mut moved = base.clone();
        moved[0].markdown = "# A#".into();
        moved[1].markdown = " B".into();
        assert_ne!(same, digest(&moved));
        let mut undated = base.clone();
        undated[0].updated = None;
        assert_ne!(same, digest(&undated));
        let mut empty = base.clone();
        empty[0].updated = Some(String::new());
        assert_eq!(
            digest(&undated),
            digest(&empty),
            "no date hashes as an empty one"
        );
        assert_ne!(same, digest(&base[..1]));
    }

    #[test]
    fn the_largest_document_fits_the_wire() {
        let markdown = "\"".repeat(MAX_TEXT_BYTES);
        let texts: Vec<TermsText> = ["de", "en", "es", "fr"]
            .iter()
            .map(|lang| TermsText {
                lang: (*lang).into(),
                updated: Some("u".repeat(MAX_UPDATED_BYTES)),
                markdown: markdown.clone(),
            })
            .collect();
        validate(&texts).unwrap();
        let response = crate::wire::Response::Terms {
            terms: Some(TermsDocument {
                version: TermsVersion {
                    version: "v".repeat(MAX_VERSION_BYTES),
                    digest: digest(&texts),
                },
                texts: texts.clone(),
            }),
        };
        let bytes = crate::encode(&response).unwrap();
        assert!(crate::decode::<crate::wire::Response>(&bytes).is_ok());
        let request = crate::wire::Request::PublishGatewayTerms {
            version: "v".repeat(MAX_VERSION_BYTES),
            texts,
        };
        assert!(crate::encode(&request).is_ok());
    }
}
