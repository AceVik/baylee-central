use super::*;
use serde::Deserialize;
fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}
fn recipient() -> (Subscription, ece::EcKeyComponents) {
    // RFC 8291 Appendix A receiver, deliberately public test material.
    let secret = hex("ab5757a70dd4a53e553a6bbf71ffefea2874ec07a6b379e3c48f895a02dc33de");
    let public = hex(
        "042571b2becdfde360551aaf1ed0f4cd366c11cebe555f89bcb7b186a53339173168ece2ebe018597bd30479b86e3c8f8eced577ca59187e9246990db682008b0e",
    );
    let sub = Subscription {
        endpoint: "https://fcm.googleapis.com/fcm/send/synthetic".into(),
        p256dh: Base64UrlUnpadded::encode_string(&public),
        auth: Base64UrlUnpadded::encode_string(&[7; 16]),
        nonce: "0123456789abcdef0123456789abcdef".into(),
        lang: "de".into(),
    };
    (sub, ece::EcKeyComponents::new(secret, public))
}
#[test]
fn rfc8291_receiver_vector_decrypts_under_the_selected_native_provider() {
    let (_, keys) = recipient();
    let ciphertext = hex(
        "0c6bfaadad67958803092d454676f397000010004104fe33f4ab0dea71914db55823f73b54948f41306d920732dbb9a59a53286482200e597a7b7bc260ba1c227998580992e93973002f3012a28ae8f06bbb78e5ec0ff297de5b429bba7153d3a4ae0caa091fd425f3b4b5414add8ab37a19c1bbb05cf5cb5b2a2e0562d558635641ec52812c6c8ff42e95ccb86be7cd",
    );
    assert_eq!(
        ece::decrypt(&keys, &hex("05305932a1c7eabe13b6cec9fda48882"), &ciphertext).unwrap(),
        b"When I grow up, I want to be a watermelon"
    );
}
#[test]
fn payload_is_encrypted_and_vapid_is_bound_to_provider_origin() {
    #[derive(Deserialize)]
    struct Verified {
        aud: String,
        sub: String,
        exp: u64,
    }
    let key = Sender::generate_key().unwrap();
    let sender = Sender::new(&key, "https://baylee.app/contact".into()).unwrap();
    let (sub, keys) = recipient();
    let (header, body) = sender.prepare(&sub, Category::Chat).unwrap();
    let plaintext = ece::decrypt(&keys, &[7; 16], &body).unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&plaintext).unwrap(),
        serde_json::json!({"v":1,"kind":"chat","lang":"de","nonce":sub.nonce})
    );
    let token = header
        .strip_prefix("vapid t=")
        .unwrap()
        .split(", k=")
        .next()
        .unwrap();
    let mut validation = jsonwebtoken::Validation::new(Algorithm::ES256);
    validation.set_audience(&["https://fcm.googleapis.com"]);
    let decoded = jsonwebtoken::decode::<Verified>(
        token,
        &jsonwebtoken::DecodingKey::from_ec_der(
            &Base64UrlUnpadded::decode_vec(sender.public_key()).unwrap(),
        ),
        &validation,
    )
    .unwrap();
    assert_eq!(decoded.claims.aud, "https://fcm.googleapis.com");
    assert_eq!(decoded.claims.sub, "https://baylee.app/contact");
    assert!(
        decoded.claims.exp
            > SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
    );
    validation.set_audience(&["https://web.push.apple.com"]);
    assert!(
        jsonwebtoken::decode::<Verified>(
            token,
            &jsonwebtoken::DecodingKey::from_ec_der(
                &Base64UrlUnpadded::decode_vec(sender.public_key()).unwrap()
            ),
            &validation
        )
        .is_err()
    );
    let (_, second) = sender.prepare(&sub, Category::Chat).unwrap();
    assert_ne!(
        body, second,
        "each encryption has a fresh ephemeral key and salt"
    );
    let mut tampered = body;
    let end = tampered.len() - 1;
    tampered[end] ^= 1;
    assert!(ece::decrypt(&keys, &[7; 16], &tampered).is_err());
}
#[test]
fn malformed_points_lengths_and_wrong_vapid_curves_fail_before_network() {
    let key = Sender::generate_key().unwrap();
    let sender = Sender::new(&key, "https://baylee.app/contact".into()).unwrap();
    let (mut sub, _) = recipient();
    sub.p256dh = Base64UrlUnpadded::encode_string(&[4; 65]);
    assert!(sender.prepare(&sub, Category::Chat).is_err());
    let (mut sub, _) = recipient();
    sub.auth = "A".repeat(21);
    assert!(sender.prepare(&sub, Category::Chat).is_err());
    let group = EcGroup::from_curve_name(Nid::SECP384R1).unwrap();
    let key = PKey::from_ec_key(EcKey::generate(&group).unwrap())
        .unwrap()
        .private_key_to_pkcs8()
        .unwrap();
    assert!(Sender::new(&key, "https://baylee.app/contact".into()).is_err());
}
#[test]
fn endpoint_policy_refuses_ssrf_redirect_destinations_and_confusing_urls() {
    for value in [
        "http://fcm.googleapis.com/token",
        "https://127.0.0.1/token",
        "https://fcm.googleapis.com.evil.test/token",
        "https://fcm.googleapis.com@evil.test/token",
        "https://evil@fcm.googleapis.com/token",
        "https://fcm.googleapis.com:8443/token",
        "https://fcm.googleapis.com/token?next=https://127.0.0.1",
        "https://fcm.googleapis.com/token#private",
        "https://FCM.googleapis.com/token",
        "https://fcm.googleapis.com/",
    ] {
        assert!(endpoint(value).is_err(), "{value}");
    }
    assert!(endpoint("https://updates.push.services.mozilla.com/wpush/v2/synthetic").is_ok());
    assert!(endpoint("https://web.push.apple.com/synthetic").is_ok());
}
