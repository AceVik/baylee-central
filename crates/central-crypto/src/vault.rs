//! Versioned master-key wrappers (password and recovery code), bounded
//! encrypted personal objects, and what is derived one way from the master
//! key (its key check, a local vault's key).

mod derived;
mod object;
mod recovery;
pub use derived::{KeyCheck, LocalVaultKey};
pub use object::{MAX_OBJECT_BYTES, ObjectContext};
pub use recovery::RecoveryCode;

use crate::{Context, CryptoError};
use chacha20poly1305::{
    Tag, XChaCha20Poly1305, XNonce,
    aead::{AeadInOut, KeyInit},
};
use hkdf::Hkdf;
use sha2::Sha512;
use std::fmt;
use zeroize::Zeroizing;

/// Random 256-bit master key, kept client-side. Not clonable or serializable.
///
/// Secret ownership cannot be duplicated through the public API:
///
/// ```compile_fail,E0599
/// # #![forbid(unsafe_code)]
/// use central_crypto::vault::MasterKey;
/// fn duplicate(key: &MasterKey) -> MasterKey {
///     MasterKey::clone(key)
/// }
/// ```
pub struct MasterKey(Zeroizing<[u8; 32]>);

/// Password wrapping key derived ONLY from an OPAQUE client export key.
/// This type has no public raw-key or session-key constructor.
pub struct WrappingKey {
    key: Zeroizing<[u8; 32]>,
    context: Context,
}

/// Fixed 73-byte wire wrapper: version 1, 24-byte nonce and 48-byte ciphertext/tag.
#[derive(Clone, Eq, PartialEq)]
pub struct WrappedMasterKey([u8; 73]);

impl fmt::Debug for MasterKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("MasterKey([REDACTED])")
    }
}
impl fmt::Debug for WrappingKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("WrappingKey([REDACTED])")
    }
}
impl fmt::Debug for WrappedMasterKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("WrappedMasterKey([REDACTED])")
    }
}

impl MasterKey {
    /// Generate using operating-system cryptographic randomness.
    ///
    /// # Errors
    ///
    /// Fails rather than falling back to deterministic randomness.
    pub fn generate() -> Result<Self, CryptoError> {
        let mut key = Zeroizing::new([0; 32]);
        getrandom::fill(key.as_mut()).map_err(|_| CryptoError::RandomnessUnavailable)?;
        Ok(Self(key))
    }
}

impl WrappingKey {
    pub(crate) fn from_export(export: &[u8], context: Context) -> Result<Self, CryptoError> {
        let mut key = Zeroizing::new([0; 32]);
        Hkdf::<Sha512>::new(Some(b"baylee-central/export-kdf/v1"), export)
            .expand(&context.aad(), key.as_mut())
            .map_err(|_| CryptoError::Invalid)?;
        Ok(Self { key, context })
    }

    /// Encrypt the master key with a fresh random nonce and authenticated context.
    ///
    /// # Errors
    ///
    /// Fails on randomness failure or encryption failure.
    pub fn wrap(&self, master: &MasterKey) -> Result<WrappedMasterKey, CryptoError> {
        seal(&self.key, &self.context.aad(), master, random_nonce()?)
    }

    /// Authenticate and decrypt; never initialize a new key on failure.
    ///
    /// # Errors
    ///
    /// Rejects wrong keys, changed context, corruption and unsupported versions.
    pub fn unwrap(&self, wrapped: &WrappedMasterKey) -> Result<MasterKey, CryptoError> {
        open(&self.key, &self.context.aad(), wrapped)
    }
}

fn random_nonce() -> Result<[u8; 24], CryptoError> {
    let mut nonce = [0; 24];
    getrandom::fill(&mut nonce).map_err(|_| CryptoError::RandomnessUnavailable)?;
    Ok(nonce)
}

/// The version-1 wrapper of `master` under `key`: version, nonce, ciphertext, tag.
fn seal(
    key: &[u8; 32],
    aad: &[u8],
    master: &MasterKey,
    nonce: [u8; 24],
) -> Result<WrappedMasterKey, CryptoError> {
    let mut wire = [0; 73];
    wire[0] = 1;
    wire[1..25].copy_from_slice(&nonce);
    let cipher = XChaCha20Poly1305::new_from_slice(key).map_err(|_| CryptoError::Invalid)?;
    // Fixed buffers avoid a plaintext heap allocation and are wiped even if
    // encryption fails or unwinds. The public wire buffer receives ciphertext only.
    let mut ciphertext = Zeroizing::new([0; 32]);
    ciphertext.copy_from_slice(master.0.as_ref());
    let tag = cipher
        .encrypt_inout_detached(&XNonce::from(nonce), aad, ciphertext.as_mut().into())
        .map_err(|_| CryptoError::Invalid)?;
    wire[25..57].copy_from_slice(ciphertext.as_ref());
    wire[57..].copy_from_slice(&tag);
    Ok(WrappedMasterKey(wire))
}

/// Authenticate and decrypt a version-1 wrapper under `key`.
fn open(key: &[u8; 32], aad: &[u8], wrapped: &WrappedMasterKey) -> Result<MasterKey, CryptoError> {
    if wrapped.0[0] != 1 {
        return Err(CryptoError::Invalid);
    }
    let nonce = XNonce::try_from(&wrapped.0[1..25]).map_err(|_| CryptoError::Invalid)?;
    let cipher = XChaCha20Poly1305::new_from_slice(key).map_err(|_| CryptoError::Invalid)?;
    let mut master = Zeroizing::new([0; 32]);
    master.copy_from_slice(&wrapped.0[25..57]);
    let tag = Tag::try_from(&wrapped.0[57..]).map_err(|_| CryptoError::Invalid)?;
    cipher
        .decrypt_inout_detached(&nonce, aad, master.as_mut().into(), &tag)
        .map_err(|_| CryptoError::Invalid)?;
    Ok(MasterKey(master))
}

impl WrappedMasterKey {
    /// Parse exactly one supported record; allocation is bounded by the fixed size.
    ///
    /// # Errors
    ///
    /// Rejects truncated, oversized or unknown-version input.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CryptoError> {
        let wire: [u8; 73] = bytes.try_into().map_err(|_| CryptoError::Invalid)?;
        if wire[0] != 1 {
            return Err(CryptoError::Invalid);
        }
        Ok(Self(wire))
    }

    /// Ciphertext only, suitable for authenticated account storage.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 73] {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chacha20poly1305::aead::{Aead, Payload};

    fn context() -> Context {
        let mut account = [2; 16];
        account[6] = 0x70;
        account[8] = 0x80;
        Context::new([1; 32], account, 1).unwrap()
    }

    #[test]
    fn every_byte_is_authenticated_and_wrapping_is_randomized() {
        let key = WrappingKey::from_export(&[42; 64], context()).unwrap();
        let master = MasterKey::generate().unwrap();
        let wrapped = key.wrap(&master).unwrap();
        assert_eq!(key.unwrap(&wrapped).unwrap().0.as_ref(), master.0.as_ref());
        assert_ne!(wrapped, key.wrap(&master).unwrap());
        for index in 0..73 {
            for bit in 0..8 {
                let mut changed = wrapped.0;
                changed[index] ^= 1 << bit;
                if let Ok(parsed) = WrappedMasterKey::from_bytes(&changed) {
                    assert!(
                        key.unwrap(&parsed).is_err(),
                        "tampering at byte {index}, bit {bit}"
                    );
                }
            }
        }
    }

    #[test]
    fn fixed_buffers_preserve_the_version_one_wire_format() {
        let key = WrappingKey::from_export(&[42; 64], context()).unwrap();
        let master = MasterKey(Zeroizing::new([91; 32]));
        let wrapped = key.wrap(&master).unwrap();
        // Compare the new detached operation with the upstream allocating AEAD
        // interface, including the tag location consumed by existing wrappers.
        let cipher = XChaCha20Poly1305::new_from_slice(key.key.as_ref()).unwrap();
        let nonce = XNonce::try_from(&wrapped.0[1..25]).unwrap();
        let reference = cipher
            .encrypt(
                &nonce,
                Payload {
                    msg: master.0.as_ref(),
                    aad: &key.context.aad(),
                },
            )
            .unwrap();
        assert_eq!(&wrapped.0[25..], reference);
        assert_eq!(key.unwrap(&wrapped).unwrap().0.as_ref(), master.0.as_ref());
    }

    #[test]
    fn different_account_issuer_epoch_or_password_key_cannot_open() {
        let original = context();
        let master = MasterKey::generate().unwrap();
        let wrapped = WrappingKey::from_export(&[42; 64], original)
            .unwrap()
            .wrap(&master)
            .unwrap();
        let mut contexts = [original; 3];
        contexts[0].issuer[0] ^= 1;
        contexts[1].account[0] ^= 1;
        contexts[2].epoch += 1;
        for context in contexts {
            assert!(
                WrappingKey::from_export(&[42; 64], context)
                    .unwrap()
                    .unwrap(&wrapped)
                    .is_err()
            );
        }
        assert!(
            WrappingKey::from_export(&[43; 64], original)
                .unwrap()
                .unwrap(&wrapped)
                .is_err()
        );
    }

    #[test]
    fn malformed_sizes_fail_and_secret_debug_is_redacted() {
        for size in [0, 1, 24, 72, 74, 1024] {
            assert!(WrappedMasterKey::from_bytes(&vec![1; size]).is_err());
        }
        assert!(Context::new([0; 32], context().account, 1).is_err());
        assert!(Context::new([1; 32], [0; 16], 1).is_err());
        assert!(Context::new([1; 32], context().account, 0).is_err());
        let master = MasterKey(Zeroizing::new([42; 32]));
        assert_eq!(format!("{master:?}"), "MasterKey([REDACTED])");
        let key = WrappingKey::from_export(&[42; 64], context()).unwrap();
        assert_eq!(format!("{key:?}"), "WrappingKey([REDACTED])");
    }
}
