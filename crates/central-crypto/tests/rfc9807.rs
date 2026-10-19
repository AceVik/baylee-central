//! Published registration vector from RFC 9807, Appendix C.1.1.
//! <https://www.rfc-editor.org/rfc/rfc9807.html#appendix-C.1.1>
//! Hex values are public test data, never credentials. Identity KSF is TEST ONLY.
#![forbid(unsafe_code)]

use opaque_ke::{
    ClientRegistration, ClientRegistrationFinishParameters, RegistrationResponse, Ristretto255,
    TripleDh, ksf::Identity,
};
use rand::{CryptoRng, RngCore};

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
