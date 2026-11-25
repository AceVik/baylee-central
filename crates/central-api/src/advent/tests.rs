//! An image is taken only as the metadata that was approved for it.
use super::*;
use sha2::{Digest as _, Sha256};

fn meta(bytes: &[u8]) -> AssetMeta {
    AssetMeta {
        mat_framing: None,
        tier: Finish::Standard,
        digest: Sha256::digest(bytes).into(),
        size: u32::try_from(bytes.len()).unwrap(),
        mime: Mime::Webp,
        width: 1,
        height: 4096,
    }
}

#[test]
fn bytes_are_accepted_only_whole_and_unchanged() {
    let bytes: Vec<u8> = (0..64).collect();
    let meta = meta(&bytes);
    assert!(meta.verify(&bytes).is_ok());
    for index in 0..bytes.len() {
        let mut changed = bytes.clone();
        changed[index] ^= 1;
        assert!(meta.verify(&changed).is_err(), "octet {index}");
    }
    assert!(meta.verify(&bytes[..63]).is_err());
    assert!(meta.verify(&[&bytes[..], &[0]].concat()).is_err());
    assert!(meta.verify(&[]).is_err());
    for octet in 0..32 {
        let mut other = meta;
        other.digest[octet] ^= 0x80;
        assert!(other.verify(&bytes).is_err(), "digest octet {octet}");
    }
    // The size is checked on its own, beside the digest of the same bytes.
    for size in [63, 65] {
        assert!(AssetMeta { size, ..meta }.verify(&bytes).is_err());
    }
    // Matching bytes do not excuse metadata outside its bounds.
    assert!(AssetMeta { width: 0, ..meta }.verify(&bytes).is_err());
}

#[test]
fn every_dimension_and_the_size_are_bounded_at_both_ends() {
    let base = meta(b"x");
    for (size, width, height, accepted) in [
        (1, 1, 1, true),
        (MAX_ART_BYTES, 4096, 4096, true),
        (0, 1, 1, false),
        (MAX_ART_BYTES + 1, 1, 1, false),
        (u32::MAX, 1, 1, false),
        (1, 0, 1, false),
        (1, 1, 0, false),
        (1, 4097, 1, false),
        (1, 1, 4097, false),
    ] {
        let meta = AssetMeta {
            size,
            width,
            height,
            ..base
        };
        assert_eq!(
            meta.validate().is_ok(),
            accepted,
            "{size} {width} x {height}"
        );
    }
    let framed = AssetMeta {
        mat_framing: Some(framing::MatFraming {
            duel: framing::Framing {
                focus: [framing::EDGE + 1, 0],
                safe: None,
            },
            ..framing::MatFraming::default()
        }),
        ..base
    };
    assert!(framed.validate().is_err());
}

#[test]
fn an_asset_is_a_webp_image_and_nothing_a_browser_would_run() {
    let wire = serde_json::to_value(meta(b"x")).unwrap();
    assert!(serde_json::from_value::<AssetMeta>(wire.clone()).is_ok());
    for mime in [
        "image/svg+xml",
        "text/html",
        "image/WEBP",
        "image/webp;x=1",
        "",
    ] {
        let mut other = wire.clone();
        other["mime"] = mime.into();
        assert!(
            serde_json::from_value::<AssetMeta>(other).is_err(),
            "{mime}"
        );
    }
    let mut extra = wire;
    extra["url"] = "https://elsewhere.example/".into();
    assert!(serde_json::from_value::<AssetMeta>(extra).is_err());
}
