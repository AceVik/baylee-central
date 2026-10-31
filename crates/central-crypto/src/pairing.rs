//! The pairing that brings a signed-in page's master key to Baylee's client
//! (Baylee `docs/proposals/deep-links.md` §"Protocol specification", owner's
//! decision of 30.10.2026; `docs/crypto-protocol.md` §"Pairing").
//!
//! The page makes a one-time secret `S` that only the page and the link
//! carry. The client makes an X25519 key pair and a verifier, and claims the
//! pairing with its public key, the verifier's challenge and a MAC under `S`.
//! The page checks that MAC, seals the master key to the client's key with
//! HPKE (RFC 9180, PSK mode with `S` as the pre-shared key and the handle as
//! its ID, DHKEM(X25519, HKDF-SHA256), HKDF-SHA256, ChaCha20-Poly1305,
//! through the `hpke` crate), bound to the issuer, account, epoch, handle and
//! expiry, and proves to Central with its key check that it holds the master
//! key. Since Central never sees `S`, a seal Central (or anyone writing its
//! database) made to the client's key does not open: only the page that made
//! the link can bring a key. Every value here is fixed-size; every comparison
//! of a received MAC or proof is constant-time.

use crate::vault::{KeyCheck, MasterKey};
use crate::{Context, CryptoError};
use getrandom::{SysRng, rand_core::UnwrapErr};
use hmac::{Hmac, Mac};
use hpke::aead::ChaCha20Poly1305;
use hpke::kdf::HkdfSha256;
use hpke::kem::X25519HkdfSha256;
use hpke::{Deserializable, Kem, OpModeR, OpModeS, PskBundle, Serializable};
use sha2::{Digest, Sha256};
use std::fmt;
use zeroize::Zeroizing;

type PrivateKey = <X25519HkdfSha256 as Kem>::PrivateKey;
type PublicKey = <X25519HkdfSha256 as Kem>::PublicKey;
type EncappedKey = <X25519HkdfSha256 as Kem>::EncappedKey;

const CLAIM_DOMAIN: &[u8] = b"baylee-central/pair-claim/v1\0";
const CODE_DOMAIN: &[u8] = b"baylee-central/pair-code/v1\0";
const SEAL_DOMAIN: &[u8] = b"baylee-central/pair-seal/v1\0";
const APPROVE_DOMAIN: &[u8] = b"baylee-central/pair-approve/v1\0";

/// A pairing's handle, a client's public key, a verifier, a challenge, a
/// MAC and a proof are each 32 bytes.
pub const LEN: usize = 32;
/// The sealed master key: HPKE's encapsulated key (32) and the ciphertext of
/// the 32-byte key with its 16-byte tag.
pub const SEALED_LEN: usize = 32 + 32 + 16;

/// The page's one-time secret `S`, carried by the link and never sent to
/// Central. Wiped when dropped; never printed.
pub struct PairingSecret(Zeroizing<[u8; LEN]>);

impl fmt::Debug for PairingSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PairingSecret([REDACTED])")
    }
}

impl PairingSecret {
    /// A fresh secret from the operating system's randomness.
    ///
    /// # Errors
    /// No randomness; never a predictable fallback.
    pub fn generate() -> Result<Self, CryptoError> {
        let mut secret = Zeroizing::new([0; LEN]);
        getrandom::fill(secret.as_mut()).map_err(|_| CryptoError::RandomnessUnavailable)?;
        Ok(Self(secret))
    }

    /// The secret read from a link: exactly 32 bytes.
    ///
    /// # Errors
    /// Any other length.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CryptoError> {
        let bytes: [u8; LEN] = bytes.try_into().map_err(|_| CryptoError::Invalid)?;
        Ok(Self(Zeroizing::new(bytes)))
    }

    /// The bytes the page puts into the link.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; LEN] {
        &self.0
    }

    fn mac(&self) -> Hmac<Sha256> {
        <Hmac<Sha256> as Mac>::new_from_slice(self.0.as_ref())
            .expect("HMAC takes a key of any length")
    }

    /// HPKE's pre-shared key: `S` itself, with the handle as its ID (RFC 9180
    /// §5.1.2: 32 random bytes meet its entropy floor).
    fn psk<'a>(&'a self, handle: &'a [u8; LEN]) -> Result<PskBundle<'a>, CryptoError> {
        PskBundle::new(self.0.as_ref(), handle).map_err(|_| CryptoError::Invalid)
    }
}

/// What a claim's MAC covers, in order.
fn claim_message(
    mac: &mut Hmac<Sha256>,
    issuer: &[u8; 32],
    handle: &[u8; LEN],
    public_key: &[u8; LEN],
    challenge: &[u8; LEN],
) {
    mac.update(CLAIM_DOMAIN);
    mac.update(issuer);
    mac.update(handle);
    mac.update(public_key);
    mac.update(challenge);
}

/// Whether `mac` is the claim's MAC under `secret` (the page's check),
/// compared in constant time.
#[must_use]
pub fn claim_ok(
    secret: &PairingSecret,
    issuer: &[u8; 32],
    handle: &[u8; LEN],
    public_key: &[u8; LEN],
    challenge: &[u8; LEN],
    mac: &[u8],
) -> bool {
    let mut check = secret.mac();
    claim_message(&mut check, issuer, handle, public_key, challenge);
    check.verify_slice(mac).is_ok()
}

/// The six characters the client and the page both show, so the player can
/// see at a glance that the page is pairing their own client: Crockford
/// base32 of the first 30 bits of SHA-256 over the domain, the handle and the
/// client's public key, as `XXX-XXX`.
#[must_use]
pub fn comparison_code(handle: &[u8; LEN], public_key: &[u8; LEN]) -> String {
    const CROCKFORD: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
    let digest = Sha256::new()
        .chain_update(CODE_DOMAIN)
        .chain_update(handle)
        .chain_update(public_key)
        .finalize();
    let bits = u32::from_be_bytes([digest[0], digest[1], digest[2], digest[3]]) >> 2;
    let mut code = String::with_capacity(7);
    for i in 0..6 {
        if i == 3 {
            code.push('-');
        }
        let symbol = (bits >> (25 - 5 * i)) & 0x1f;
        code.push(char::from(CROCKFORD[symbol as usize]));
    }
    code
}

/// What a seal is bound to besides the client's key: the account's context
/// (issuer, account, epoch), the pairing's handle and its expiry.
#[derive(Clone, Copy, Debug)]
pub struct SealBinding {
    /// The account the master key belongs to, at its epoch.
    pub context: Context,
    /// The pairing's handle.
    pub handle: [u8; LEN],
    /// The pairing's expiry, Unix seconds (not the session's).
    pub expires_at: i64,
}

impl SealBinding {
    /// HPKE's `info`: the domain, issuer, account, epoch (eight big-endian
    /// bytes), handle and expiry (eight big-endian bytes).
    fn info(&self) -> [u8; 28 + 56 + LEN + 8] {
        let bound = self.context.bound::<{ 28 + 56 }>(SEAL_DOMAIN);
        let mut info = [0; 28 + 56 + LEN + 8];
        info[..bound.len()].copy_from_slice(&bound);
        info[bound.len()..bound.len() + LEN].copy_from_slice(&self.handle);
        info[bound.len() + LEN..].copy_from_slice(&self.expires_at.to_be_bytes());
        info
    }
}

/// The client's half of a pairing: its X25519 key pair and its verifier.
/// Lives only in memory, for one pairing; wiped when dropped.
pub struct ClientPairing {
    private: PrivateKey,
    public: [u8; LEN],
    verifier: Zeroizing<[u8; LEN]>,
}

impl fmt::Debug for ClientPairing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ClientPairing([REDACTED])")
    }
}

impl ClientPairing {
    /// A fresh key pair and verifier from the operating system's randomness.
    ///
    /// # Errors
    /// No randomness; never a predictable fallback.
    pub fn generate() -> Result<Self, CryptoError> {
        let mut ikm = Zeroizing::new([0; LEN]);
        let mut verifier = Zeroizing::new([0; LEN]);
        getrandom::fill(ikm.as_mut()).map_err(|_| CryptoError::RandomnessUnavailable)?;
        getrandom::fill(verifier.as_mut()).map_err(|_| CryptoError::RandomnessUnavailable)?;
        let (private, public) = X25519HkdfSha256::derive_keypair(ikm.as_ref());
        let public: [u8; LEN] = public.to_bytes().into();
        Ok(Self {
            private,
            public,
            verifier,
        })
    }

    /// The public key the claim sends.
    #[must_use]
    pub const fn public_key(&self) -> &[u8; LEN] {
        &self.public
    }

    /// SHA-256 of the verifier, which the claim sends.
    #[must_use]
    pub fn challenge(&self) -> [u8; LEN] {
        challenge_of(&self.verifier)
    }

    /// The verifier itself, sent once, in the redeem.
    #[must_use]
    pub fn verifier(&self) -> &[u8; LEN] {
        &self.verifier
    }

    /// The claim's MAC under the link's secret.
    #[must_use]
    pub fn claim_mac(
        &self,
        secret: &PairingSecret,
        issuer: &[u8; 32],
        handle: &[u8; LEN],
    ) -> [u8; LEN] {
        let mut mac = secret.mac();
        claim_message(&mut mac, issuer, handle, &self.public, &self.challenge());
        mac.finalize().into_bytes().into()
    }

    /// Opens what the page sealed to this key under the link's secret: the
    /// master key, or a refusal for anything sealed to another key, under
    /// another secret or binding, or changed.
    ///
    /// # Errors
    /// A wrong size, key, secret, binding or any changed byte.
    pub fn open(
        &self,
        secret: &PairingSecret,
        sealed: &[u8],
        binding: &SealBinding,
    ) -> Result<MasterKey, CryptoError> {
        let sealed: &[u8; SEALED_LEN] = sealed.try_into().map_err(|_| CryptoError::Invalid)?;
        let encapped = EncappedKey::from_bytes(&sealed[..LEN]).map_err(|_| CryptoError::Invalid)?;
        let plain = Zeroizing::new(
            hpke::single_shot_open::<ChaCha20Poly1305, HkdfSha256, X25519HkdfSha256>(
                &OpModeR::Psk(secret.psk(&binding.handle)?),
                &self.private,
                &encapped,
                &binding.info(),
                &sealed[LEN..],
                &[],
            )
            .map_err(|_| CryptoError::Invalid)?,
        );
        MasterKey::from_slice(&plain)
    }
}

/// `SHA-256(verifier)`: what Central compares a redeem's verifier with.
#[must_use]
pub fn challenge_of(verifier: &[u8; LEN]) -> [u8; LEN] {
    Sha256::digest(verifier).into()
}

/// The approval's proof: HMAC-SHA-256 under the key check over the
/// approval (handle, client's key, challenge, and SHA-256 of the sealed key).
#[must_use]
pub fn approval_proof(
    check: &KeyCheck,
    handle: &[u8; LEN],
    public_key: &[u8; LEN],
    challenge: &[u8; LEN],
    sealed: &[u8; SEALED_LEN],
) -> [u8; LEN] {
    approval_mac(check, handle, public_key, challenge, sealed)
        .finalize()
        .into_bytes()
        .into()
}

/// Whether `proof` is the approval's proof under the stored key check
/// (Central's check), compared in constant time.
#[must_use]
pub fn approval_ok(
    check: &KeyCheck,
    handle: &[u8; LEN],
    public_key: &[u8; LEN],
    challenge: &[u8; LEN],
    sealed: &[u8; SEALED_LEN],
    proof: &[u8],
) -> bool {
    approval_mac(check, handle, public_key, challenge, sealed)
        .verify_slice(proof)
        .is_ok()
}

fn approval_mac(
    check: &KeyCheck,
    handle: &[u8; LEN],
    public_key: &[u8; LEN],
    challenge: &[u8; LEN],
    sealed: &[u8; SEALED_LEN],
) -> Hmac<Sha256> {
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(check.as_bytes())
        .expect("HMAC takes a key of any length");
    mac.update(APPROVE_DOMAIN);
    mac.update(handle);
    mac.update(public_key);
    mac.update(challenge);
    mac.update(&Sha256::digest(sealed));
    mac
}

impl MasterKey {
    /// The master key sealed to a client's public key for a pairing (the
    /// page's step), under the link's secret, with a fresh ephemeral key from
    /// the operating system.
    ///
    /// # Errors
    /// A public key that is not one (a wrong size, or one HPKE refuses, such
    /// as a low-order point giving an all-zero shared secret).
    ///
    /// # Panics
    /// If the operating system's randomness fails, as `opaque-ke` already
    /// does (`docs/crypto-protocol.md` §"Evidence and remaining gates").
    pub fn seal_for_pairing(
        &self,
        secret: &PairingSecret,
        public_key: &[u8],
        binding: &SealBinding,
    ) -> Result<[u8; SEALED_LEN], CryptoError> {
        let public = PublicKey::from_bytes(public_key).map_err(|_| CryptoError::Invalid)?;
        let (encapped, ciphertext) =
            hpke::single_shot_seal_with_rng::<ChaCha20Poly1305, HkdfSha256, X25519HkdfSha256>(
                &OpModeS::Psk(secret.psk(&binding.handle)?),
                &public,
                &binding.info(),
                self.expose(),
                &[],
                &mut UnwrapErr(SysRng),
            )
            .map_err(|_| CryptoError::Invalid)?;
        let mut sealed = [0; SEALED_LEN];
        sealed[..LEN].copy_from_slice(&encapped.to_bytes());
        if ciphertext.len() != SEALED_LEN - LEN {
            return Err(CryptoError::Invalid);
        }
        sealed[LEN..].copy_from_slice(&ciphertext);
        Ok(sealed)
    }
}

#[cfg(test)]
mod tests;
