//! Every refusal has its own test, and each asserts the exact reason, so that none
//! passes because an earlier rule refused the token for another cause.
use super::*;
use zeroize::Zeroizing;

const CENTRAL: &str = "https://central.baylee.app";
const GATEWAY: &str = "https://gateway.example";
const OTHER: &str = "https://other.example";
const NOW: i64 = 1_800_000_000;
const SID: &str = "0123456789abcdef0123456789abcdef";
const JTI: &str = "fedcba9876543210fedcba9876543210";
const ACCOUNT: &str = "019a0000-0000-7000-8000-000000000001";

fn key(byte: u8) -> SigningKey {
    SigningKey::from_seed(&Zeroizing::new([byte; 32]))
}

fn keys(key: &SigningKey) -> KeySet {
    KeySet::new(vec![key.public().clone()]).unwrap()
}

fn gateway_claims() -> GatewayClaims {
    GatewayClaims {
        iss: CENTRAL.into(),
        aud: GATEWAY.into(),
        sub: ACCOUNT.into(),
        sid: SID.into(),
        jti: JTI.into(),
        iat: NOW,
        nbf: NOW,
        exp: NOW + GATEWAY_TOKEN_SECONDS,
        purpose: Purpose::Admit,
        name: Some("Ace".into()),
        tag: Some(7),
        legacy_id: None,
        terms: None,
    }
}

fn central_claims() -> CentralClaims {
    CentralClaims {
        iss: CENTRAL.into(),
        aud: CENTRAL.into(),
        sub: ACCOUNT.into(),
        sid: SID.repeat(2),
        jti: JTI.into(),
        iat: NOW,
        nbf: NOW,
        exp: NOW + CENTRAL_TOKEN_SECONDS,
    }
}

fn revocation_claims() -> RevocationClaims {
    RevocationClaims {
        iss: CENTRAL.into(),
        aud: GATEWAY.into(),
        jti: JTI.into(),
        iat: NOW,
        nbf: NOW,
        exp: NOW + REVOCATION_SECONDS,
        from: 4,
        seq: 4,
        sids: vec![SID.into()],
    }
}

const AT_GATEWAY: Expected<'static> = Expected {
    issuer: CENTRAL,
    audience: GATEWAY,
    now: NOW,
};

fn refusal<C: Claims>(token: &str, keys: &KeySet, expected: Expected<'_>) -> JwtError {
    verify::<C>(token, keys, expected).err().expect("refused")
}

/// A token signed by `key` with these exact header and payload bytes: what an attacker
/// holding the key, or a buggy issuer, could send.
fn forged(key: &SigningKey, header: &str, payload: &str) -> String {
    use ed25519_dalek::Signer as _;
    let signed = format!(
        "{}.{}",
        encode(header.as_bytes()),
        encode(payload.as_bytes())
    );
    let signature = key.raw().sign(signed.as_bytes());
    format!("{signed}.{}", encode(&signature.to_bytes()))
}

fn gateway_payload() -> String {
    serde_json::to_string(&gateway_claims()).unwrap()
}

#[test]
fn each_kind_round_trips() {
    let key = key(1);
    let set = keys(&key);
    let token = key.sign(&gateway_claims()).unwrap();
    let claims: GatewayClaims = verify(&token, &set, AT_GATEWAY).unwrap();
    assert_eq!(claims.sid, SID);
    assert_eq!(claims.name.as_deref(), Some("Ace"));
    assert_eq!(claims.account().unwrap()[0], 0x01);
    let token = key.sign(&revocation_claims()).unwrap();
    let claims: RevocationClaims = verify(&token, &set, AT_GATEWAY).unwrap();
    assert_eq!((claims.from, claims.seq), (4, 4));
    let token = key.sign(&central_claims()).unwrap();
    let at_central = Expected {
        issuer: CENTRAL,
        audience: CENTRAL,
        now: NOW,
    };
    let claims: CentralClaims = verify(&token, &set, at_central).unwrap();
    assert_eq!(claims.sid, SID.repeat(2));
}

#[test]
fn refuses_the_wrong_audience() {
    let key = key(1);
    let token = key.sign(&gateway_claims()).unwrap();
    let elsewhere = Expected {
        audience: OTHER,
        ..AT_GATEWAY
    };
    assert_eq!(
        refusal::<GatewayClaims>(&token, &keys(&key), elsewhere),
        JwtError::Audience
    );
    let token = key.sign(&revocation_claims()).unwrap();
    assert_eq!(
        refusal::<RevocationClaims>(&token, &keys(&key), elsewhere),
        JwtError::Audience
    );
}

#[test]
fn refuses_an_audience_that_is_a_list() {
    let key = key(1);
    let header = format!(
        r#"{{"alg":"EdDSA","typ":"baylee-gateway+jwt","kid":"{}"}}"#,
        key.public().kid()
    );
    let payload = gateway_payload().replace(
        &format!(r#""aud":"{GATEWAY}""#),
        &format!(r#""aud":["{GATEWAY}"]"#),
    );
    assert_eq!(
        refusal::<GatewayClaims>(&forged(&key, &header, &payload), &keys(&key), AT_GATEWAY),
        JwtError::Claims
    );
}

#[test]
fn refuses_the_wrong_issuer() {
    let key = key(1);
    let token = key.sign(&gateway_claims()).unwrap();
    let elsewhere = Expected {
        issuer: OTHER,
        ..AT_GATEWAY
    };
    assert_eq!(
        refusal::<GatewayClaims>(&token, &keys(&key), elsewhere),
        JwtError::Issuer
    );
}

#[test]
fn refuses_an_expired_token_after_the_tolerance_only() {
    let key = key(1);
    let token = key.sign(&gateway_claims()).unwrap();
    let exp = NOW + GATEWAY_TOKEN_SECONDS;
    let at = |now| Expected { now, ..AT_GATEWAY };
    assert!(verify::<GatewayClaims>(&token, &keys(&key), at(exp + LEEWAY_SECONDS - 1)).is_ok());
    assert_eq!(
        refusal::<GatewayClaims>(&token, &keys(&key), at(exp + LEEWAY_SECONDS)),
        JwtError::Expired
    );
}

#[test]
fn refuses_a_token_not_yet_valid_past_the_tolerance_only() {
    let key = key(1);
    let token = key.sign(&gateway_claims()).unwrap();
    let at = |now| Expected { now, ..AT_GATEWAY };
    assert!(verify::<GatewayClaims>(&token, &keys(&key), at(NOW - LEEWAY_SECONDS)).is_ok());
    assert_eq!(
        refusal::<GatewayClaims>(&token, &keys(&key), at(NOW - LEEWAY_SECONDS - 1)),
        JwtError::NotYetValid
    );
}

#[test]
fn refuses_every_other_algorithm_whatever_the_signature() {
    let key = key(1);
    let kid = key.public().kid().to_owned();
    for alg in ["none", "HS256", "RS256", "ES256", "eddsa", "EdDSA "] {
        let header = format!(r#"{{"alg":"{alg}","typ":"baylee-gateway+jwt","kid":"{kid}"}}"#);
        assert_eq!(
            refusal::<GatewayClaims>(
                &forged(&key, &header, &gateway_payload()),
                &keys(&key),
                AT_GATEWAY
            ),
            JwtError::Algorithm,
            "{alg}"
        );
    }
    // `alg: none` with an empty signature, the classic form.
    let header =
        encode(format!(r#"{{"alg":"none","typ":"baylee-gateway+jwt","kid":"{kid}"}}"#).as_bytes());
    let token = format!("{header}.{}.", encode(gateway_payload().as_bytes()));
    assert_eq!(
        refusal::<GatewayClaims>(&token, &keys(&key), AT_GATEWAY),
        JwtError::Algorithm
    );
}

#[test]
fn refuses_a_header_with_anything_more_or_in_another_order() {
    let key = key(1);
    let kid = key.public().kid().to_owned();
    for (header, why) in [
        (
            format!(r#"{{"alg":"EdDSA","typ":"baylee-gateway+jwt","kid":"{kid}","jwk":{{}}}}"#),
            JwtError::Malformed,
        ),
        (
            format!(r#"{{"alg":"EdDSA","typ":"baylee-gateway+jwt","kid":"{kid}","crit":["x"]}}"#),
            JwtError::Malformed,
        ),
        (
            format!(r#"{{"typ":"baylee-gateway+jwt","alg":"EdDSA","kid":"{kid}"}}"#),
            JwtError::Algorithm,
        ),
        (
            format!(r#"{{ "alg":"EdDSA","typ":"baylee-gateway+jwt","kid":"{kid}"}}"#),
            JwtError::Algorithm,
        ),
    ] {
        assert_eq!(
            refusal::<GatewayClaims>(
                &forged(&key, &header, &gateway_payload()),
                &keys(&key),
                AT_GATEWAY
            ),
            why,
            "{header}"
        );
    }
}

#[test]
fn refuses_an_unknown_kid() {
    let signer = key(1);
    let token = signer.sign(&gateway_claims()).unwrap();
    assert_eq!(
        refusal::<GatewayClaims>(&token, &keys(&key(2)), AT_GATEWAY),
        JwtError::UnknownKey
    );
}

#[test]
fn refuses_a_key_named_by_another_kid() {
    // The token claims the trusted key's `kid` but was signed by another key.
    let trusted = key(1);
    let attacker = key(2);
    let header = format!(
        r#"{{"alg":"EdDSA","typ":"baylee-gateway+jwt","kid":"{}"}}"#,
        trusted.public().kid()
    );
    assert_eq!(
        refusal::<GatewayClaims>(
            &forged(&attacker, &header, &gateway_payload()),
            &keys(&trusted),
            AT_GATEWAY
        ),
        JwtError::Signature
    );
}

#[test]
fn refuses_a_changed_payload_or_signature() {
    let key = key(1);
    let token = key.sign(&gateway_claims()).unwrap();
    let parts: Vec<&str> = token.split('.').collect();
    let mut claims = gateway_claims();
    claims.sub = "019a0000-0000-7000-8000-000000000002".into();
    let other = encode(serde_json::to_string(&claims).unwrap().as_bytes());
    let swapped = format!("{}.{other}.{}", parts[0], parts[2]);
    assert_eq!(
        refusal::<GatewayClaims>(&swapped, &keys(&key), AT_GATEWAY),
        JwtError::Signature
    );
    let zero = encode(&[0; 64]);
    let blank = format!("{}.{}.{zero}", parts[0], parts[1]);
    assert_eq!(
        refusal::<GatewayClaims>(&blank, &keys(&key), AT_GATEWAY),
        JwtError::Signature
    );
}

#[test]
fn refuses_one_kind_read_as_another() {
    let key = key(1);
    let set = keys(&key);
    let gateway = key.sign(&gateway_claims()).unwrap();
    let revocation = key.sign(&revocation_claims()).unwrap();
    let central = key.sign(&central_claims()).unwrap();
    let at_central = Expected {
        issuer: CENTRAL,
        audience: CENTRAL,
        now: NOW,
    };
    // A gateway token offered at Central, and Central's token offered at a gateway.
    assert_eq!(
        refusal::<CentralClaims>(&gateway, &set, at_central),
        JwtError::Kind
    );
    assert_eq!(
        refusal::<GatewayClaims>(&central, &set, AT_GATEWAY),
        JwtError::Kind
    );
    assert_eq!(
        refusal::<GatewayClaims>(&revocation, &set, AT_GATEWAY),
        JwtError::Kind
    );
    assert_eq!(
        refusal::<RevocationClaims>(&gateway, &set, AT_GATEWAY),
        JwtError::Kind
    );
}

#[test]
fn refuses_a_gateway_token_relabelled_as_central() {
    // Even under a valid signature the claims of another kind do not parse.
    let key = key(1);
    let header = format!(
        r#"{{"alg":"EdDSA","typ":"baylee-central+jwt","kid":"{}"}}"#,
        key.public().kid()
    );
    let at_central = Expected {
        issuer: CENTRAL,
        audience: GATEWAY,
        now: NOW,
    };
    assert_eq!(
        refusal::<CentralClaims>(
            &forged(&key, &header, &gateway_payload()),
            &keys(&key),
            at_central
        ),
        JwtError::Claims
    );
}

#[test]
fn refuses_unknown_missing_and_duplicate_claims() {
    let key = key(1);
    let header = format!(
        r#"{{"alg":"EdDSA","typ":"baylee-gateway+jwt","kid":"{}"}}"#,
        key.public().kid()
    );
    let payload = gateway_payload();
    let body = payload.strip_suffix('}').unwrap();
    for changed in [
        format!(r#"{body},"admin":true}}"#),
        format!(r#"{body},"sid":"{SID}"}}"#),
        payload.replace(r#""exp":"#, r#""expires":"#),
        payload.replace(&format!(r#""nbf":{NOW},"#), ""),
        payload.replace(&format!(r#""iss":"{CENTRAL}","#), ""),
        payload.replace(r#""purpose":"admit""#, r#""purpose":"export""#),
    ] {
        assert_eq!(
            refusal::<GatewayClaims>(&forged(&key, &header, &changed), &keys(&key), AT_GATEWAY),
            JwtError::Claims,
            "{changed}"
        );
    }
}

#[test]
fn refuses_a_lifetime_longer_than_its_kind() {
    let key = key(1);
    let header = format!(
        r#"{{"alg":"EdDSA","typ":"baylee-gateway+jwt","kid":"{}"}}"#,
        key.public().kid()
    );
    let mut claims = gateway_claims();
    claims.exp = NOW + GATEWAY_TOKEN_SECONDS + 1;
    let payload = serde_json::to_string(&claims).unwrap();
    assert_eq!(
        refusal::<GatewayClaims>(&forged(&key, &header, &payload), &keys(&key), AT_GATEWAY),
        JwtError::Claims
    );
    let mut claims = gateway_claims();
    claims.nbf = claims.exp;
    let payload = serde_json::to_string(&claims).unwrap();
    assert_eq!(
        refusal::<GatewayClaims>(&forged(&key, &header, &payload), &keys(&key), AT_GATEWAY),
        JwtError::Claims
    );
}

#[test]
fn refuses_malformed_input_before_it_is_read() {
    let key = key(1);
    let token = key.sign(&gateway_claims()).unwrap();
    let padded = format!("{}=", token.as_str());
    let spaced = format!(" {}", token.as_str());
    let four = format!("{}.AAAA", token.as_str());
    let long = "a".repeat(Kind::Gateway.max_bytes() + 1);
    for text in [
        "",
        ".",
        "..",
        "a.b",
        padded.as_str(),
        spaced.as_str(),
        four.as_str(),
        long.as_str(),
    ] {
        assert_eq!(
            refusal::<GatewayClaims>(text, &keys(&key), AT_GATEWAY),
            JwtError::Malformed,
            "{text}"
        );
    }
}

#[test]
fn refuses_a_noncanonical_signature_encoding() {
    let key = key(1);
    let token = key.sign(&gateway_claims()).unwrap();
    // 64 bytes leave four unused bits in the last character: setting one writes the
    // same bytes under a second spelling, which must not verify.
    let (head, last) = token.rsplit_once('.').unwrap();
    let mut signature = last.to_owned();
    let end = signature.pop().unwrap();
    let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let index = alphabet.iter().position(|b| char::from(*b) == end).unwrap();
    signature.push(char::from(alphabet[index | 1]));
    assert_ne!(signature, last);
    assert_eq!(
        refusal::<GatewayClaims>(&format!("{head}.{signature}"), &keys(&key), AT_GATEWAY),
        JwtError::Malformed
    );
}

#[test]
fn signing_refuses_what_a_verifier_would() {
    let key = key(1);
    let mut claims = gateway_claims();
    claims.sid = "not a sid".into();
    assert_eq!(key.sign(&claims).err(), Some(JwtError::Claims));
    let mut claims = gateway_claims();
    claims.name = Some("n".repeat(Kind::Gateway.max_bytes()));
    assert_eq!(key.sign(&claims).err(), Some(JwtError::Claims));
    let mut claims = revocation_claims();
    claims.sids.push(SID.into());
    assert_eq!(key.sign(&claims).err(), Some(JwtError::Claims));
    let mut claims = revocation_claims();
    claims.from = 0;
    claims.seq = 0;
    assert_eq!(key.sign(&claims).err(), Some(JwtError::Claims));
}

#[test]
fn a_full_page_of_sids_fits_a_message() {
    let key = key(1);
    let mut claims = revocation_claims();
    claims.from = 1;
    claims.seq = MAX_SIDS as u64;
    claims.sids = vec![SID.into(); MAX_SIDS];
    let token = key.sign(&claims).unwrap();
    assert!(verify::<RevocationClaims>(&token, &keys(&key), AT_GATEWAY).is_ok());
    claims.seq += 1;
    claims.sids.push(SID.into());
    assert_eq!(key.sign(&claims).err(), Some(JwtError::Claims));
}

#[test]
fn a_replayed_message_is_refused_and_a_retry_is_not() {
    let mut cache = ReplayCache::new(8);
    let message = revocation_claims();
    cache.take(&message, NOW).unwrap();
    assert_eq!(cache.take(&message, NOW), Err(JwtError::Replayed));
    // A retry of the same numbers is signed afresh, under a new `jti`.
    let mut retry = revocation_claims();
    retry.jti = "00000000000000000000000000000001".into();
    cache.take(&retry, NOW).unwrap();
}

#[test]
fn the_replay_cache_refuses_rather_than_forgets_a_live_message() {
    let mut cache = ReplayCache::new(1);
    cache.take(&revocation_claims(), NOW).unwrap();
    let mut second = revocation_claims();
    second.jti = "00000000000000000000000000000001".into();
    assert_eq!(cache.take(&second, NOW), Err(JwtError::Full));
    // Past the message's last valid second the entry may go: it can no longer verify.
    let later = NOW + REVOCATION_SECONDS + LEEWAY_SECONDS;
    cache.take(&second, later).unwrap();
    assert_eq!(cache.take(&second, later), Err(JwtError::Replayed));
}

#[test]
fn rotation_keeps_old_tokens_valid_while_their_key_is_listed() {
    let old = key(1);
    let new = key(2);
    let token = old.sign(&gateway_claims()).unwrap();
    let both = KeySet::new(vec![new.public().clone(), old.public().clone()]).unwrap();
    assert!(verify::<GatewayClaims>(&token, &both, AT_GATEWAY).is_ok());
    assert_eq!(
        refusal::<GatewayClaims>(&token, &keys(&new), AT_GATEWAY),
        JwtError::UnknownKey
    );
}

#[test]
fn a_key_set_round_trips_through_its_document() {
    let set = KeySet::new(vec![key(1).public().clone(), key(2).public().clone()]).unwrap();
    let read = KeySet::from_jwks(&set.to_jwks().unwrap()).unwrap();
    assert_eq!(read.keys().len(), 2);
    assert!(read.find(key(2).public().kid()).is_some());
}

#[test]
fn a_document_is_refused_for_any_departure() {
    let good = String::from_utf8(keys(&key(1)).to_jwks().unwrap()).unwrap();
    let kid = key(1).public().kid().to_owned();
    let mut other = kid.clone();
    other.replace_range(0..1, if other.starts_with('A') { "B" } else { "A" });
    let long = " ".repeat(MAX_JWKS_BYTES + 1);
    for document in [
        r#"{"keys":[]}"#.to_owned(),
        "not json".to_owned(),
        long,
        good.replace("OKP", "EC"),
        good.replace("Ed25519", "Ed448"),
        good.replace("EdDSA", "ES256"),
        good.replace(r#""use":"sig""#, r#""use":"enc""#),
        good.replace(&kid, &other),
        good.replace(r#""kty""#, r#""d":"AAAA","kty""#),
    ] {
        assert_eq!(
            KeySet::from_jwks(document.as_bytes()).err(),
            Some(JwtError::Malformed),
            "{document}"
        );
    }
    let nine: Vec<_> = (1..=9).map(|byte| key(byte).public().clone()).collect();
    assert!(KeySet::new(nine).is_err());
    assert!(KeySet::new(vec![key(1).public().clone(), key(1).public().clone()]).is_err());
}

#[test]
fn a_small_order_key_is_refused() {
    let mut identity = [0; 32];
    identity[0] = 1;
    assert_eq!(
        PublicKey::from_bytes(&identity).err(),
        Some(JwtError::Malformed)
    );
}

#[test]
fn rfc_8037_vectors_hold() {
    use base64ct::{Base64UrlUnpadded, Encoding as _};
    use ed25519_dalek::Signer as _;
    // RFC 8037 appendix A.1 to A.4.
    let d = Base64UrlUnpadded::decode_vec("nWGxne_9WmC6hEr0kuwsxERJxWl7MmkZcDusAxyuf2A").unwrap();
    let seed: [u8; 32] = d.try_into().unwrap();
    let key = SigningKey::from_seed(&Zeroizing::new(seed));
    assert_eq!(
        key.public().kid(),
        "kPrK_qmxVWaYVA9wwBF6Iuo3vVzz7TxHCTwXBygrS4k"
    );
    assert_eq!(
        encode(&key.public().to_bytes()),
        "11qYAYKxCrfVS_7TyWQHOg7hcvPapiMlrwIaaPcHURo"
    );
    let signature = key
        .raw()
        .sign(b"eyJhbGciOiJFZERTQSJ9.RXhhbXBsZSBvZiBFZDI1NTE5IHNpZ25pbmc");
    assert_eq!(
        encode(&signature.to_bytes()),
        "hgyY0il_MGCjP0JzlnLWG1PPOt7-09PGcvMg3AIbQR6dWbhijcNR4ki4iylGjg5BhVsPt9g7sVvpAr_MuM0KAg"
    );
}

#[test]
fn nothing_prints_a_key_or_a_token() {
    let key = key(1);
    let printed = format!("{key:?} {:?}", key.public());
    assert!(printed.contains(key.public().kid()));
    assert!(!printed.contains(&encode(&[1; 32])));
    assert_eq!(JwtError::Signature.to_string(), "token rejected");
}
