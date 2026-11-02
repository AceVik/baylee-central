//! Authentication, replay bytes, key changes and trust isolation shared by WASM/native.
#![forbid(unsafe_code)]
use central_api::{
    Error,
    chat::{AccountKeys, Delivery, DeviceKey},
    social::Person,
};
use central_client::chat::Chat;
use central_crypto::chat::DeviceSecrets;

const NOW: u64 = 1_793_600_000_000;
fn identity(seed: u8, tag: u32) -> Chat {
    Chat::new(DeviceSecrets::from_bytes(&[seed; 96]), tag).unwrap()
}
fn keys(chat: &Chat, tag: u32) -> AccountKeys {
    let p = chat.registration().unwrap();
    AccountKeys {
        tag: format!("{tag:04x}"),
        handle: format!("Player#{tag:04x}"),
        devices: vec![DeviceKey {
            id: chat.device().into(),
            ik: p.ik,
            dk: p.dk,
            dsig: p.dsig,
            label: p.label,
            added_at: NOW,
        }],
    }
}

#[test]
fn two_independent_installations_open_the_same_signed_messages() {
    let mut alice = identity(10, 7);
    let mut bob = identity(20, 9);
    alice.observe(keys(&bob, 9)).unwrap();
    bob.observe(keys(&alice, 7)).unwrap();
    let sealed = alice
        .seal(
            "dm:0007:0009",
            "<img src=x> hello 🦋".into(),
            false,
            &[7, 9],
            NOW,
        )
        .unwrap();
    let a = keys(&alice, 7).devices.remove(0);
    let mut delivered = Delivery {
        seq: 1,
        sender: Person {
            tag: 7,
            display_name: "Alice".into(),
        },
        device: a,
        sealed,
        at: NOW,
    };
    delivered.sealed.wraps.retain(|w| w.device == bob.device());
    let opened = bob.open(&delivered).unwrap();
    assert_eq!(opened.text, "<img src=x> hello 🦋");
    let trust = bob.save_trust().unwrap();
    let mut second = identity(20, 9);
    second.restore_trust(&trust).unwrap();
    assert_eq!(second.open(&delivered).unwrap().id, opened.id);
    let mut foreign = identity(21, 9);
    assert!(foreign.restore_trust(&trust).is_err());
    assert!(foreign.open(&delivered).is_err());
    delivered.sender.tag = 8;
    assert!(bob.open(&delivered).is_err());
    delivered.sender.tag = 7;
    delivered.sealed.wraps[0].device = alice.device().into();
    assert!(bob.open(&delivered).is_err());
}

#[test]
fn changed_keys_need_exact_confirmation_even_after_restart() {
    let mut alice = identity(10, 7);
    let bob = identity(20, 9);
    let original = keys(&bob, 9);
    let first = alice.observe(original.clone()).unwrap();
    assert!(!first.changed);
    let saved = alice.save_trust().unwrap();
    let replacement = keys(&identity(30, 9), 9);
    assert!(alice.observe(replacement.clone()).unwrap().changed);
    assert!(matches!(
        alice.seal("dm:0007:0009", "test".into(), false, &[7, 9], NOW),
        Err(Error::Conflict)
    ));
    assert!(alice.confirm(original).is_err());
    let mut restarted = identity(10, 7);
    restarted.restore_trust(&saved).unwrap();
    assert!(restarted.observe(replacement.clone()).unwrap().changed);
    restarted.confirm(replacement).unwrap();
    assert!(
        restarted
            .seal("dm:0007:0009", "test".into(), false, &[7, 9], NOW)
            .is_ok()
    );
}

#[test]
fn directories_cannot_add_own_keys_or_duplicate_recipients() {
    let mut alice = identity(10, 7);
    let own = keys(&alice, 7);
    alice.observe(own.clone()).unwrap();
    assert!(alice.observe(keys(&identity(11, 7), 7)).is_err());
    let mut extra = own;
    extra
        .devices
        .push(keys(&identity(11, 7), 7).devices.remove(0));
    assert!(alice.observe(extra).is_err());
    let bob = identity(20, 9);
    let mut duplicate = keys(&bob, 9);
    duplicate.devices.push(duplicate.devices[0].clone());
    assert!(alice.observe(duplicate).is_err());
    alice.observe(keys(&bob, 9)).unwrap();
    for (conv, recipients) in [
        ("dm:0009:0007", vec![7, 9]),
        ("dm:0007:0009", vec![7, 9, 9]),
        ("dm:0007:0009", vec![7, 8]),
        ("dm:0008:0009", vec![8, 9]),
    ] {
        assert!(
            alice
                .seal(conv, "text".into(), false, &recipients, NOW)
                .is_err()
        );
    }
}

#[test]
fn text_bounds_and_randomized_message_ids_hold() {
    let mut alice = identity(10, 7);
    alice.observe(keys(&identity(20, 9), 9)).unwrap();
    for text in [
        " ".into(),
        "a".repeat(1001),
        "bad\u{202e}direction".into(),
        "bad\0control".into(),
    ] {
        assert!(alice.seal("dm:0007:0009", text, false, &[9], NOW).is_err());
    }
    let a = alice
        .seal("dm:0007:0009", "same text".into(), false, &[9], NOW)
        .unwrap();
    let b = alice
        .seal("dm:0007:0009", "same text".into(), false, &[9], NOW)
        .unwrap();
    assert_ne!(a.hdr, b.hdr);
    assert_ne!(a.ct, b.ct);
    let mut damaged = alice.save_trust().unwrap();
    damaged[0] ^= 1;
    assert!(alice.restore_trust(&damaged).is_err());
    // Failed restoration leaves the existing accepted key intact.
    assert!(
        alice
            .seal("dm:0007:0009", "still trusted".into(), false, &[9], NOW)
            .is_ok()
    );
}

#[test]
fn a_key_change_barrier_survives_directory_rollback_and_restart() {
    let mut alice = identity(10, 7);
    let bob = keys(&identity(20, 9), 9);
    alice.observe(bob.clone()).unwrap();
    assert!(alice.observe(keys(&identity(30, 9), 9)).unwrap().changed);
    assert!(alice.observe(bob.clone()).unwrap().changed);
    let saved = alice.save_trust().unwrap();
    let mut restored = identity(10, 7);
    restored.restore_trust(&saved).unwrap();
    assert!(restored.observe(bob.clone()).unwrap().changed);
    assert!(matches!(
        restored.seal("dm:0007:0009", "blocked".into(), false, &[9], NOW),
        Err(Error::Conflict)
    ));
    restored.confirm(bob).unwrap();
    assert!(
        restored
            .seal("dm:0007:0009", "confirmed".into(), false, &[9], NOW)
            .is_ok()
    );
}
