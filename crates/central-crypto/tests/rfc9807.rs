//! Published registration and login vector from RFC 9807, Appendix C.1.1.
//! <https://www.rfc-editor.org/rfc/rfc9807.html#appendix-C.1.1>
//! Hex values are public test data, never credentials. Identity KSF is TEST ONLY.
#![forbid(unsafe_code)]

use opaque_ke::{
    ClientLogin, ClientLoginFinishParameters, ClientRegistration,
    ClientRegistrationFinishParameters, CredentialFinalization, CredentialRequest,
    CredentialResponse, Identifiers, RegistrationResponse, Ristretto255, ServerLogin,
    ServerLoginParameters, ServerRegistration, ServerSetup, TripleDh, generic_array::GenericArray,
    key_exchange::group::Group, ksf::Identity,
};
use rand::{CryptoRng, RngCore};

#[path = "rfc9807/vector.rs"]
mod vector;

struct ReferenceSuite;
impl opaque_ke::CipherSuite for ReferenceSuite {
    type OprfCs = Ristretto255;
    type KeyExchange = TripleDh<Ristretto255, sha2::Sha512>;
    type Ksf = Identity;
}

fn hex(value: &str) -> Vec<u8> {
    let value: String = value.split_whitespace().collect();
    assert_eq!(value.len() % 2, 0);
    (0..value.len())
        .step_by(2)
        .map(|offset| u8::from_str_radix(&value[offset..offset + 2], 16).unwrap())
        .collect()
}

// Deliberately deterministic, bounded reference-vector entropy; test binary only.
struct VectorRng(std::vec::IntoIter<u8>);
impl RngCore for VectorRng {
    fn next_u32(&mut self) -> u32 {
        let mut bytes = [0; 4];
        self.fill_bytes(&mut bytes);
        u32::from_le_bytes(bytes)
    }

    fn next_u64(&mut self) -> u64 {
        let mut bytes = [0; 8];
        self.fill_bytes(&mut bytes);
        u64::from_le_bytes(bytes)
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        for byte in dest {
            *byte = self.0.next().expect("RFC vector entropy exhausted");
        }
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand::Error> {
        self.fill_bytes(dest);
        Ok(())
    }
}
impl CryptoRng for VectorRng {}

#[test]
fn published_ristretto255_sha512_registration_vector() {
    let state = hex(
        "76cfbfe758db884bebb33582331ba9f159720ca8784a2a070a265d9c2d6abe01
         5059ff249eb1551b7ce4991f3336205bde44a105a032e747d21bf382e75f7a71",
    );
    let response = hex("7408a268083e03abc7097fc05b587834539065e86fb0c7
         b6342fcf5e01e5b019b2fe7af9f48cc502d016729d2fe25cdd433f2c4bc904660b2a3
         82c9b79df1a78");
    let mut rng = VectorRng(
        hex("ac13171b2f17bc2c74997f0fce1e1f35bec6b91fe2e12dbd323d23ba7a38dfec").into_iter(),
    );
    let result = ClientRegistration::<ReferenceSuite>::deserialize(&state)
        .unwrap()
        .finish(
            &mut rng,
            b"CorrectHorseBatteryStaple",
            RegistrationResponse::deserialize(&response).unwrap(),
            ClientRegistrationFinishParameters::default(),
        )
        .unwrap();
    assert_eq!(
        result.message.serialize().as_slice(),
        hex("76a845464c68a5d2f7e442436bb1424953b17d3e2e289ccb
             accafb57ac5c36751ac5844383c7708077dea41cbefe2fa15724f449e535dd7dd562e
             66f5ecfb95864eadddec9db5874959905117dad40a4524111849799281fefe3c51fa8
             2785c5ac13171b2f17bc2c74997f0fce1e1f35bec6b91fe2e12dbd323d23ba7a38dfe
             c634b0f5b96109c198a8027da51854c35bee90d1e1c781806d07d49b76de6a28b8d9e
             9b6c93b9f8b64d16dddd9c5bfb5fea48ee8fd2f75012a8b308605cdd8ba5"),
    );
    assert_eq!(
        result.export_key.as_slice(),
        hex("1ef15b4fa99e8a852412450ab78713aad30d21fa6966c9b8c9fb3262a
             970dc62950d4dd4ed62598229b1b72794fc0335199d9f7fcc6eaedde92cc04870e63f
             16"),
    );
    assert_eq!(rng.0.len(), 0);
}

fn login_state() -> ClientLogin<ReferenceSuite> {
    // Restore the published blinding scalar, KE1 and ephemeral seed. Production
    // uses fresh OS entropy; deterministic state injection is test-only.
    let mut state = hex(vector::BLIND_LOGIN);
    state.extend(hex(vector::KE1));
    let secret = Ristretto255::derive_scalar(GenericArray::clone_from_slice(&hex(
        vector::CLIENT_KEYSHARE_SEED,
    )))
    .unwrap();
    state.extend(Ristretto255::serialize_sk(&secret));
    state.extend(hex(vector::CLIENT_NONCE));
    ClientLogin::deserialize(&state).unwrap()
}

#[test]
fn published_ristretto255_sha512_login_vector() {
    let mut rng = VectorRng(Vec::new().into_iter());
    let client = login_state()
        .finish(
            &mut rng,
            b"CorrectHorseBatteryStaple",
            CredentialResponse::deserialize(&hex(vector::KE2)).unwrap(),
            ClientLoginFinishParameters::new(Some(b"OPAQUE-POC"), Identifiers::default(), None),
        )
        .unwrap();
    assert_eq!(client.message.serialize().as_slice(), hex(vector::KE3));
    assert_eq!(client.export_key.as_slice(), hex(vector::EXPORT_KEY));
    assert_eq!(client.session_key.as_slice(), hex(vector::SESSION_KEY));

    // A matching client implementation alone cannot prove server interoperability.
    // Reproduce KE2 and verify KE3 using the separately published server inputs.
    let mut setup = hex(vector::OPRF_SEED);
    setup.extend(hex(vector::SERVER_PRIVATE_KEY));
    // The dummy-record public key is irrelevant for this real-account vector.
    setup.extend(hex(vector::SERVER_PUBLIC_KEY));
    let server = ServerSetup::<ReferenceSuite>::deserialize(&setup).unwrap();
    let record = ServerRegistration::deserialize(&hex(vector::REGISTRATION_UPLOAD)).unwrap();
    // opaque-ke always generates a discarded 64-byte dummy masking key first.
    let mut entropy = vec![0; 64];
    entropy.extend(hex(vector::MASKING_NONCE));
    entropy.extend(hex(vector::SERVER_KEYSHARE_SEED));
    entropy.extend(hex(vector::SERVER_NONCE));
    let mut rng = VectorRng(entropy.into_iter());
    let parameters = ServerLoginParameters {
        context: Some(b"OPAQUE-POC"),
        identifiers: Identifiers::default(),
    };
    let server = ServerLogin::start(
        &mut rng,
        &server,
        Some(record),
        CredentialRequest::deserialize(&hex(vector::KE1)).unwrap(),
        b"1234",
        parameters.clone(),
    )
    .unwrap();
    assert_eq!(server.message.serialize().as_slice(), hex(vector::KE2));
    assert_eq!(rng.0.len(), 0);
    for index in 0..client.message.serialize().len() {
        let mut proof = client.message.serialize();
        proof[index] ^= 1;
        let restored =
            ServerLogin::<ReferenceSuite>::deserialize(&server.state.serialize()).unwrap();
        assert!(
            restored
                .finish(
                    CredentialFinalization::deserialize(&proof).unwrap(),
                    parameters.clone(),
                )
                .is_err(),
            "mutated finalization byte {index}"
        );
    }
    let server = server.state.finish(client.message, parameters).unwrap();
    assert_eq!(server.session_key.as_slice(), hex(vector::SESSION_KEY));
}

#[test]
fn published_login_rejects_each_changed_server_message_byte() {
    // Identity KSF makes exhaustive protocol mutation cheap while remaining
    // isolated from the private, fixed-Argon2 application suite.
    let response = hex(vector::KE2);
    for index in 0..response.len() {
        let mut changed = response.clone();
        changed[index] ^= 1;
        if let Ok(response) = CredentialResponse::deserialize(&changed) {
            assert!(
                login_state()
                    .finish(
                        &mut VectorRng(Vec::new().into_iter()),
                        b"CorrectHorseBatteryStaple",
                        response,
                        ClientLoginFinishParameters::new(
                            Some(b"OPAQUE-POC"),
                            Identifiers::default(),
                            None
                        ),
                    )
                    .is_err(),
                "mutated server message byte {index}"
            );
        }
    }
}
