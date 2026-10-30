use super::*;
use core::convert::Infallible;
use getrandom::rand_core::{TryCryptoRng, TryRng};
use hpke::aead::AeadCtxR;

fn hex(text: &str) -> Vec<u8> {
    let text: String = text.split_whitespace().collect();
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
        .collect()
}

fn array(text: &str) -> [u8; 32] {
    hex(text).try_into().unwrap()
}

/// Hands out exactly the bytes it was made with, and refuses to be asked
/// for more: the RFC's ephemeral input, for its vectors. Test code only.
struct Fixed(Vec<u8>);

impl TryRng for Fixed {
    type Error = Infallible;
    fn try_next_u32(&mut self) -> Result<u32, Infallible> {
        unreachable!("HPKE draws key material as bytes")
    }
    fn try_next_u64(&mut self) -> Result<u64, Infallible> {
        unreachable!("HPKE draws key material as bytes")
    }
    fn try_fill_bytes(&mut self, dst: &mut [u8]) -> Result<(), Infallible> {
        assert!(
            dst.len() <= self.0.len(),
            "asked for more than the vector holds"
        );
        let rest = self.0.split_off(dst.len());
        dst.copy_from_slice(&self.0);
        self.0 = rest;
        Ok(())
    }
}

impl TryCryptoRng for Fixed {}

/// X25519's clamping (RFC 7748 §5), which `hpke` 0.15 applies when it
/// serializes a private key; the RFC's vectors print the unclamped bytes.
fn clamped(mut key: [u8; 32]) -> [u8; 32] {
    key[0] &= 0b1111_1000;
    key[31] &= 0b0111_1111;
    key[31] |= 0b0100_0000;
    key
}

/// One of RFC 9180 Appendix A.2's setups for this crate's suite,
/// DHKEM(X25519, HKDF-SHA256), HKDF-SHA256, ChaCha20-Poly1305: the key pairs'
/// inputs and the RFC's outputs, the pre-shared key and its ID (mode 1) or
/// none (mode 0), the ciphertexts at six sequence numbers and three exports.
struct Appendix {
    ikm_r: &'static str,
    pk_r: &'static str,
    sk_r: &'static str,
    ikm_e: &'static str,
    pk_e: &'static str,
    sk_e: &'static str,
    psk: Option<(&'static str, &'static str)>,
    ciphertexts: [(u64, &'static str); 6],
    exports: [&'static str; 3],
}

/// A.2.1, base mode.
const A_2_1: Appendix = Appendix {
    ikm_r: "1ac01f181fdf9f352797655161c58b75c656a6cc2716dcb66372da835542e1df",
    pk_r: "4310ee97d88cc1f088a5576c77ab0cf5c3ac797f3d95139c6c84b5429c59662a",
    sk_r: "8057991eef8f1f1af18f4a9491d16a1ce333f695d4db8e38da75975c4478e0fb",
    ikm_e: "909a9b35d3dc4713a5e72a4da274b55d3d3821a37e5d099e74a647db583a904b",
    pk_e: "1afa08d3dec047a643885163f1180476fa7ddb54c6a8029ea33f95796bf2ac4a",
    sk_e: "f4ec9b33b792c372c1d2c2063507b684ef925b8c75a42dbcbf57d63ccd381600",
    psk: None,
    ciphertexts: [
        (
            0,
            "1c5250d8034ec2b784ba2cfd69dbdb8af406cfe3ff938e131f0def8c8b60b4db 21993c62ce81883d2dd1b51a28",
        ),
        (
            1,
            "6b53c051e4199c518de79594e1c4ab18b96f081549d45ce015be002090bb119e 85285337cc95ba5f59992dc98c",
        ),
        (
            2,
            "71146bd6795ccc9c49ce25dda112a48f202ad220559502cef1f34271e0cb4b02 b4f10ecac6f48c32f878fae86b",
        ),
        (
            4,
            "63357a2aa291f5a4e5f27db6baa2af8cf77427c7c1a909e0b37214dd47db122b b153495ff0b02e9e54a50dbe16",
        ),
        (
            255,
            "18ab939d63ddec9f6ac2b60d61d36a7375d2070c9b683861110757062c52b888 0a5f6b3936da9cd6c23ef2a95c",
        ),
        (
            256,
            "7a4a13e9ef23978e2c520fd4d2e757514ae160cd0cd05e556ef692370ca53076 214c0c40d4c728d6ed9e727a5b",
        ),
    ],
    exports: [
        "4bbd6243b8bb54cec311fac9df81841b6fd61f56538a775e7c80a9f40160606e",
        "8c1df14732580e5501b00f82b10a1647b40713191b7c1240ac80e2b68808ba69",
        "5acb09211139c43b3090489a9da433e8a30ee7188ba8b0a9a1ccf0c229283e53",
    ],
};

/// A.2.2, PSK mode: the mode the pairing's seal uses.
const A_2_2: Appendix = Appendix {
    ikm_r: "26b923eade72941c8a85b09986cdfa3f1296852261adedc52d58d2930269812b",
    pk_r: "13640af826b722fc04feaa4de2f28fbd5ecc03623b317834e7ff4120dbe73062",
    sk_r: "77d114e0212be51cb1d76fa99dd41cfd4d0166b08caa09074430a6c59ef17879",
    ikm_e: "35706a0b09fb26fb45c39c2f5079c709c7cf98e43afa973f14d88ece7e29c2e3",
    pk_e: "2261299c3f40a9afc133b969a97f05e95be2c514e54f3de26cbe5644ac735b04",
    sk_e: "0c35fdf49df7aa01cd330049332c40411ebba36e0c718ebc3edf5845795f6321",
    psk: Some((
        "0247fd33b913760fa1fa51e1892d9f307fbe65eb171e8132c2af18555a738b82",
        "456e6e796e20447572696e206172616e204d6f726961",
    )),
    ciphertexts: [
        (
            0,
            "4a177f9c0d6f15cfdf533fb65bf84aecdc6ab16b8b85b4cf65a370e07fc1d78d 28fb073214525276f4a89608ff",
        ),
        (
            1,
            "5c3cabae2f0b3e124d8d864c116fd8f20f3f56fda988c3573b40b09997fd6c76 9e77c8eda6cda4f947f5b704a8",
        ),
        (
            2,
            "14958900b44bdae9cbe5a528bf933c5c990dbb8e282e6e495adf8205d19da9eb 270e3a6f1e0613ab7e757962a4",
        ),
        (
            4,
            "c2a7bc09ddb853cf2effb6e8d058e346f7fe0fb3476528c80db6b698415c5f8c 50b68a9a355609e96d2117f8d3",
        ),
        (
            255,
            "2414d0788e4bc39a59a26d7bd5d78e111c317d44c37bd5a4c2a1235f2ddc2085 c487d406490e75210c958724a7",
        ),
        (
            256,
            "c567ae1c3f0f75abe1dd9e4532b422600ed4a6e5b9484dafb1e43ab9f5fd662b 28c00e2e81d3cde955dae7e218",
        ),
    ],
    exports: [
        "813c1bfc516c99076ae0f466671f0ba5ff244a41699f7b2417e4c59d46d39f40",
        "2745cf3d5bb65c333658732954ee7af49eb895ce77f8022873a62a13c94cb4e1",
        "ad40e3ae14f21c99bfdebc20ae14ab86f4ca2dc9a4799d200f43a25f99fa78ae",
    ],
};

/// The setup's key pairs from their inputs: the recipient's, which the test
/// goes on with, and the sender's ephemeral one, both checked.
fn appendix_keys(
    appendix: &Appendix,
) -> (
    <X25519HkdfSha256 as Kem>::PrivateKey,
    <X25519HkdfSha256 as Kem>::PublicKey,
) {
    let (sk_r, pk_r) = X25519HkdfSha256::derive_keypair(&hex(appendix.ikm_r));
    assert_eq!(pk_r.to_bytes().to_vec(), hex(appendix.pk_r));
    assert_eq!(
        <[u8; 32]>::from(sk_r.to_bytes()),
        clamped(array(appendix.sk_r))
    );
    let (sk_e, pk_e) = X25519HkdfSha256::derive_keypair(&hex(appendix.ikm_e));
    assert_eq!(pk_e.to_bytes().to_vec(), hex(appendix.pk_e));
    assert_eq!(
        <[u8; 32]>::from(sk_e.to_bytes()),
        clamped(array(appendix.sk_e))
    );
    (sk_r, pk_r)
}

/// A whole setup through the crate's public API, so a later `hpke` cannot
/// change the suite or the mode silently.
fn the_appendix(appendix: &Appendix) {
    let info = hex("4f6465206f6e2061204772656369616e2055726e");
    let (sk_r, pk_r) = appendix_keys(appendix);
    let psk = appendix.psk.map(|(psk, id)| (hex(psk), hex(id)));
    let bundle = psk
        .as_ref()
        .map(|(psk, id)| PskBundle::new(psk, id).unwrap());
    let (mode_s, mode_r) = bundle.map_or((OpModeS::Base, OpModeR::Base), |bundle| {
        (OpModeS::Psk(bundle), OpModeR::Psk(bundle))
    });
    // The sender's ephemeral key comes from the RFC's ikmE.
    let mut rng = Fixed(hex(appendix.ikm_e));
    let (enc, mut sender) =
        hpke::setup_sender_with_rng::<ChaCha20Poly1305, HkdfSha256, X25519HkdfSha256>(
            &mode_s, &pk_r, &info, &mut rng,
        )
        .unwrap();
    assert!(rng.0.is_empty(), "the whole ikmE and nothing more");
    assert_eq!(enc.to_bytes().to_vec(), hex(appendix.pk_e));
    let mut receiver: AeadCtxR<ChaCha20Poly1305, HkdfSha256, X25519HkdfSha256> =
        hpke::setup_receiver(&mode_r, &sk_r, &enc, &info).unwrap();
    let pt = hex("4265617574792069732074727574682c20747275746820626561757479");
    // Every sequence number up to 256 in order: the vectors' nonces are
    // base_nonce XOR the sequence number, so skipping one would move them.
    for sequence in 0..=256_u64 {
        let aad = format!("Count-{sequence}");
        let ct = sender.seal(&pt, aad.as_bytes()).unwrap();
        if let Some((_, vector)) = appendix.ciphertexts.iter().find(|(at, _)| *at == sequence) {
            assert_eq!(ct, hex(vector), "sequence number {sequence}");
        }
        assert_eq!(
            receiver.open(&ct, aad.as_bytes()).unwrap(),
            pt,
            "{sequence}"
        );
    }
    for (context, value) in ["", "00", "54657374436f6e74657874"]
        .into_iter()
        .zip(appendix.exports)
    {
        let mut out = [0; 32];
        sender.export(&hex(context), &mut out).unwrap();
        assert_eq!(out.to_vec(), hex(value), "exporter context {context:?}");
        let mut out = [0; 32];
        receiver.export(&hex(context), &mut out).unwrap();
        assert_eq!(out.to_vec(), hex(value), "receiver's export {context:?}");
    }
}

/// RFC 9180 Appendix A.2.1, base mode, the whole of it.
#[test]
fn rfc_9180_appendix_a_2_1() {
    the_appendix(&A_2_1);
}

/// RFC 9180 Appendix A.2.2, PSK mode, the whole of it: the pairing's mode.
#[test]
fn rfc_9180_appendix_a_2_2() {
    the_appendix(&A_2_2);
}

fn account() -> [u8; 16] {
    hex("0192d0c57b3e7a1f8c2d3e4f5a6b7c8d").try_into().unwrap()
}

fn live() -> [u8; 32] {
    array("04f2cff83011533f0a5fdea045398d66f3933a3ca42063c2ad242893d7384b3b")
}

fn binding() -> SealBinding {
    SealBinding {
        context: Context::new(live(), account(), 3).unwrap(),
        handle: [0x30; LEN],
        expires_at: 1_793_000_000,
    }
}

fn master() -> MasterKey {
    MasterKey::from_slice(&[0x42; 32]).unwrap()
}

/// The link's secret the page sealed under.
fn secret() -> PairingSecret {
    PairingSecret::from_bytes(&[0x5a; LEN]).unwrap()
}

#[test]
fn the_client_opens_what_the_page_sealed_to_it() {
    let client = ClientPairing::generate().unwrap();
    let sealed = master()
        .seal_for_pairing(&secret(), client.public_key(), &binding())
        .unwrap();
    let opened = client.open(&secret(), &sealed, &binding()).unwrap();
    assert_eq!(opened.expose(), master().expose());
    // Two seals of one key differ: a fresh ephemeral key each time.
    let again = master()
        .seal_for_pairing(&secret(), client.public_key(), &binding())
        .unwrap();
    assert_ne!(sealed, again);
}

#[test]
fn every_byte_of_the_seal_counts() {
    let client = ClientPairing::generate().unwrap();
    let sealed = master()
        .seal_for_pairing(&secret(), client.public_key(), &binding())
        .unwrap();
    for at in 0..SEALED_LEN {
        let mut damaged = sealed;
        damaged[at] ^= 1;
        assert!(
            client.open(&secret(), &damaged, &binding()).is_err(),
            "byte {at}"
        );
    }
    for len in [0, SEALED_LEN - 1, SEALED_LEN + 1] {
        assert_eq!(
            client
                .open(&secret(), &vec![0; len], &binding())
                .unwrap_err(),
            CryptoError::Invalid
        );
    }
}

/// The seal is bound to everything the spec names: another key, account,
/// issuer, epoch, handle or expiry opens nothing.
#[test]
fn a_seal_opens_only_under_its_own_binding_and_key() {
    let client = ClientPairing::generate().unwrap();
    let sealed = master()
        .seal_for_pairing(&secret(), client.public_key(), &binding())
        .unwrap();
    let mut other_account = account();
    other_account[15] ^= 1;
    let mut others = [binding(); 5];
    others[0].context = Context::new([1; 32], account(), 3).unwrap();
    others[1].context = Context::new(live(), other_account, 3).unwrap();
    others[2].context = Context::new(live(), account(), 4).unwrap();
    others[3].handle[0] ^= 1;
    others[4].expires_at += 1;
    for (n, other) in others.iter().enumerate() {
        assert!(
            client.open(&secret(), &sealed, other).is_err(),
            "binding {n}"
        );
    }
    let stranger = ClientPairing::generate().unwrap();
    assert!(
        stranger.open(&secret(), &sealed, &binding()).is_err(),
        "sealed to another key"
    );
}

/// Only the link's secret opens the seal: another one does not, and neither
/// does a seal made without it by someone who knows everything else Central
/// knows (the client's key, the binding, the handle that is the PSK's ID),
/// in base mode or under a guess.
#[test]
fn a_seal_opens_only_under_the_links_secret() {
    let client = ClientPairing::generate().unwrap();
    let sealed = master()
        .seal_for_pairing(&secret(), client.public_key(), &binding())
        .unwrap();
    let other = PairingSecret::from_bytes(&[0x5b; LEN]).unwrap();
    assert!(
        client.open(&other, &sealed, &binding()).is_err(),
        "another S"
    );

    let substitute = MasterKey::from_slice(&[0x66; 32]).unwrap();
    let public = PublicKey::from_bytes(client.public_key()).unwrap();
    let (encapped, ciphertext) =
        hpke::single_shot_seal_with_rng::<ChaCha20Poly1305, HkdfSha256, X25519HkdfSha256>(
            &OpModeS::Base,
            &public,
            &binding().info(),
            substitute.expose(),
            &[],
            &mut UnwrapErr(SysRng),
        )
        .unwrap();
    let mut planted = [0; SEALED_LEN];
    planted[..LEN].copy_from_slice(&encapped.to_bytes());
    planted[LEN..].copy_from_slice(&ciphertext);
    assert!(
        client.open(&secret(), &planted, &binding()).is_err(),
        "sealed in base mode, as Central could"
    );
    for guess in [[0; LEN], binding().handle] {
        let guessed = PairingSecret::from_bytes(&guess).unwrap();
        let planted = substitute
            .seal_for_pairing(&guessed, client.public_key(), &binding())
            .unwrap();
        assert!(
            client.open(&secret(), &planted, &binding()).is_err(),
            "sealed under a guessed S"
        );
        assert!(
            client.open(&guessed, &planted, &binding()).is_ok(),
            "the guess itself would open it: the secret is what decides"
        );
    }
}

/// A low-order public key gives an all-zero shared secret, which RFC 9180
/// §7.1.4 says to refuse; so does a key of the wrong size.
#[test]
fn a_public_key_that_is_not_one_is_refused() {
    for public in [vec![0; 32], vec![1; 31], vec![1; 33]] {
        assert!(
            master()
                .seal_for_pairing(&secret(), &public, &binding())
                .is_err(),
            "{public:?}"
        );
    }
}

fn fixed_client() -> ClientPairing {
    let (private, _) = X25519HkdfSha256::derive_keypair(&[1; 32]);
    ClientPairing {
        private,
        public: core::array::from_fn(|i| 0x50 + u8::try_from(i).unwrap()),
        verifier: Zeroizing::new(core::array::from_fn(|i| 0x70 + u8::try_from(i).unwrap())),
    }
}

fn counting(from: u8) -> [u8; 32] {
    core::array::from_fn(|i| from + u8::try_from(i).unwrap())
}

/// `tests/pairing-vectors.py`: the standard library's HMAC and SHA-256.
#[test]
fn the_macs_and_the_code_match_an_independent_implementation() {
    let secret = PairingSecret::from_bytes(&counting(0x10)).unwrap();
    let handle = counting(0x30);
    let client = fixed_client();
    let check = KeyCheck::from_bytes(&counting(0x90)).unwrap();
    let sealed: [u8; SEALED_LEN] = core::array::from_fn(|i| 0xb0 + u8::try_from(i).unwrap());
    assert_eq!(
        client.challenge(),
        array("ee070b29400115ee2b23ede5e46a7da719eae1d85af97bd76d55c0c3042b108a")
    );
    let mac = client.claim_mac(&secret, &live(), &handle);
    assert_eq!(
        mac,
        array("2dc8810cb9d7c32e21cec478495548a0aacbee82a4ce404f46c56cbc9b135db2")
    );
    let proof = approval_proof(
        &check,
        &handle,
        client.public_key(),
        &client.challenge(),
        &sealed,
    );
    assert_eq!(
        proof,
        array("3fba8288a3f8e2c19b8d6d567bb034cf9cf59807a1b815fb89f9b7fabaf0acdd")
    );
    assert_eq!(comparison_code(&handle, client.public_key()), "7YM-V6J");
}

#[test]
fn a_claim_checks_only_with_its_own_secret_and_fields() {
    let secret = PairingSecret::from_bytes(&counting(0x10)).unwrap();
    let handle = counting(0x30);
    let client = fixed_client();
    let mac = client.claim_mac(&secret, &live(), &handle);
    let (public, challenge) = (*client.public_key(), client.challenge());
    assert!(claim_ok(
        &secret,
        &live(),
        &handle,
        &public,
        &challenge,
        &mac
    ));
    let other = PairingSecret::from_bytes(&counting(0x11)).unwrap();
    assert!(
        !claim_ok(&other, &live(), &handle, &public, &challenge, &mac),
        "another S"
    );
    let mut changed = handle;
    changed[0] ^= 1;
    assert!(
        !claim_ok(&secret, &live(), &changed, &public, &challenge, &mac),
        "handle"
    );
    let mut changed = public;
    changed[0] ^= 1;
    assert!(
        !claim_ok(&secret, &live(), &handle, &changed, &challenge, &mac),
        "key"
    );
    let mut changed = challenge;
    changed[0] ^= 1;
    assert!(
        !claim_ok(&secret, &live(), &handle, &public, &changed, &mac),
        "challenge"
    );
    assert!(
        !claim_ok(&secret, &[1; 32], &handle, &public, &challenge, &mac),
        "issuer"
    );
    assert!(
        !claim_ok(&secret, &live(), &handle, &public, &challenge, &mac[..31]),
        "short"
    );
}

#[test]
fn an_approval_checks_only_with_its_own_key_check_and_seal() {
    let check = KeyCheck::from_bytes(&counting(0x90)).unwrap();
    let handle = counting(0x30);
    let client = fixed_client();
    let (public, challenge) = (*client.public_key(), client.challenge());
    let sealed: [u8; SEALED_LEN] = [7; SEALED_LEN];
    let proof = approval_proof(&check, &handle, &public, &challenge, &sealed);
    assert!(approval_ok(
        &check, &handle, &public, &challenge, &sealed, &proof
    ));
    let other = KeyCheck::from_bytes(&counting(0x91)).unwrap();
    assert!(
        !approval_ok(&other, &handle, &public, &challenge, &sealed, &proof),
        "another account's check"
    );
    let mut swapped = sealed;
    swapped[79] ^= 1;
    assert!(
        !approval_ok(&check, &handle, &public, &challenge, &swapped, &proof),
        "another seal"
    );
    let mut changed = public;
    changed[0] ^= 1;
    assert!(
        !approval_ok(&check, &handle, &changed, &challenge, &sealed, &proof),
        "another key"
    );
    assert!(
        !approval_ok(&check, &handle, &public, &challenge, &sealed, &proof[1..]),
        "short"
    );
}

#[test]
fn a_redeem_matches_only_its_own_verifier() {
    let client = ClientPairing::generate().unwrap();
    assert_eq!(challenge_of(client.verifier()), client.challenge());
    let mut other = *client.verifier();
    other[0] ^= 1;
    assert_ne!(challenge_of(&other), client.challenge());
}

#[test]
fn the_code_is_six_crockford_symbols_and_moves_with_its_inputs() {
    let code = comparison_code(&[1; LEN], &[2; LEN]);
    assert_eq!(code.len(), 7);
    assert_eq!(code.as_bytes()[3], b'-');
    assert!(
        code.chars()
            .filter(|c| *c != '-')
            .all(|c| "0123456789ABCDEFGHJKMNPQRSTVWXYZ".contains(c))
    );
    assert_ne!(code, comparison_code(&[1; LEN], &[3; LEN]));
    assert_ne!(code, comparison_code(&[4; LEN], &[2; LEN]));
}

#[test]
fn no_secret_prints() {
    assert_eq!(
        format!("{:?}", PairingSecret::from_bytes(&[9; 32]).unwrap()),
        "PairingSecret([REDACTED])"
    );
    assert_eq!(
        format!("{:?}", ClientPairing::generate().unwrap()),
        "ClientPairing([REDACTED])"
    );
}
