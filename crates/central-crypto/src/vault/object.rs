//! Bounded personal objects encrypted with a purpose-separated account key.

use super::MasterKey;
use crate::{Context, CryptoError};
use chacha20poly1305::{
    XChaCha20Poly1305, XNonce,
    aead::{AeadInOut, KeyInit},
};
use hkdf::Hkdf;
use sha2::Sha512;
use zeroize::Zeroizing;

/// Maximum plaintext in one object; larger content must use an authenticated manifest.
pub const MAX_OBJECT_BYTES: usize = 262_144;
const DOMAIN: &[u8] = b"baylee-central/personal-object/v1\0";
const OVERHEAD: usize = 1 + 24 + 16;

/// Immutable object identity and the revision the caller expects to read or write.
/// The caller obtains the expected revision from its authenticated synchronization state.
#[derive(Clone, Copy)]
pub struct ObjectContext {
    account: Context,
    object: [u8; 16],
    revision: u64,
}

impl ObjectContext {
    /// Bind a database-generated object `UUIDv7` and a positive revision to an account.
    ///
    /// # Errors
    /// Rejects invalid identifiers or revision zero.
    pub fn new(account: Context, object: [u8; 16], revision: u64) -> Result<Self, CryptoError> {
        if object[6] >> 4 != 7 || object[8] >> 6 != 2 || revision == 0 {
            return Err(CryptoError::Invalid);
        }
        Ok(Self {
            account,
            object,
            revision,
        })
    }

    fn aad(self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(DOMAIN.len() + self.account.aad().len() + 24);
        bytes.extend_from_slice(DOMAIN);
        bytes.extend_from_slice(&self.account.aad());
        bytes.extend_from_slice(&self.object);
        bytes.extend_from_slice(&self.revision.to_be_bytes());
        bytes
    }
}

impl MasterKey {
    fn object_cipher(&self, context: ObjectContext) -> Result<XChaCha20Poly1305, CryptoError> {
        let mut key = Zeroizing::new([0; 32]);
        // Different from password wrappers, and different for every account/object/epoch.
        let mut info = context;
        info.revision = 0;
        Hkdf::<Sha512>::new(Some(DOMAIN), self.0.as_ref())
            .expand(&info.aad(), key.as_mut())
            .map_err(|_| CryptoError::Invalid)?;
        XChaCha20Poly1305::new_from_slice(key.as_ref()).map_err(|_| CryptoError::Invalid)
    }

    /// Encrypt an object with a new random 192-bit nonce; plaintext never enters a wire buffer.
    ///
    /// # Errors
    /// Rejects oversized input or unavailable randomness/encryption.
    pub fn seal_object(
        &self,
        context: ObjectContext,
        plaintext: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        if plaintext.len() > MAX_OBJECT_BYTES {
            return Err(CryptoError::Invalid);
        }
        let cipher = self.object_cipher(context)?;
        let mut nonce = [0; 24];
        getrandom::fill(&mut nonce).map_err(|_| CryptoError::RandomnessUnavailable)?;
        let mut body = Zeroizing::new(plaintext.to_vec());
        let tag = cipher
            .encrypt_inout_detached(
                &XNonce::from(nonce),
                &context.aad(),
                body.as_mut_slice().into(),
            )
            .map_err(|_| CryptoError::Invalid)?;
        let mut wire = Vec::with_capacity(plaintext.len() + OVERHEAD);
        wire.push(1);
        wire.extend_from_slice(&nonce);
        wire.extend_from_slice(&body);
        wire.extend_from_slice(&tag);
        Ok(wire)
    }

    /// Authenticate before returning zeroizing plaintext. Corruption is never an empty object.
    ///
    /// # Errors
    /// Rejects noncanonical framing, wrong context/key, tampering or oversized ciphertext.
    pub fn open_object(
        &self,
        context: ObjectContext,
        wire: &[u8],
    ) -> Result<Zeroizing<Vec<u8>>, CryptoError> {
        if !(OVERHEAD..=MAX_OBJECT_BYTES + OVERHEAD).contains(&wire.len()) || wire[0] != 1 {
            return Err(CryptoError::Invalid);
        }
        let cipher = self.object_cipher(context)?;
        let nonce = XNonce::try_from(&wire[1..25]).map_err(|_| CryptoError::Invalid)?;
        let split = wire.len() - 16;
        let tag =
            chacha20poly1305::Tag::try_from(&wire[split..]).map_err(|_| CryptoError::Invalid)?;
        let mut body = Zeroizing::new(wire[25..split].to_vec());
        cipher
            .decrypt_inout_detached(&nonce, &context.aad(), body.as_mut_slice().into(), &tag)
            .map_err(|_| CryptoError::Invalid)?;
        Ok(body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> ObjectContext {
        let mut id = [42; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        ObjectContext::new(Context::new([7; 32], id, 1).unwrap(), id, 1).unwrap()
    }

    #[test]
    fn objects_bind_identity_revision_and_every_ciphertext_byte() {
        let key = MasterKey::generate().unwrap();
        let context = context();
        let wire = key
            .seal_object(context, b"synthetic private settings")
            .unwrap();
        assert_eq!(
            &**key.open_object(context, &wire).unwrap(),
            b"synthetic private settings"
        );
        assert_ne!(
            wire,
            key.seal_object(context, b"synthetic private settings")
                .unwrap()
        );
        for index in 0..wire.len() {
            let mut changed = wire.clone();
            changed[index] ^= 1;
            assert!(key.open_object(context, &changed).is_err());
        }
        for changed in [
            ObjectContext {
                revision: 2,
                ..context
            },
            ObjectContext {
                object: [1; 16],
                ..context
            },
            ObjectContext {
                account: Context {
                    issuer: [8; 32],
                    ..context.account
                },
                ..context
            },
            ObjectContext {
                account: Context {
                    account: [8; 16],
                    ..context.account
                },
                ..context
            },
            ObjectContext {
                account: Context {
                    epoch: 2,
                    ..context.account
                },
                ..context
            },
        ] {
            assert!(key.open_object(changed, &wire).is_err());
        }
        assert!(
            MasterKey::generate()
                .unwrap()
                .open_object(context, &wire)
                .is_err()
        );
    }

    #[test]
    fn object_sizes_and_versions_are_bounded() {
        let key = MasterKey::generate().unwrap();
        for size in [0, 1, MAX_OBJECT_BYTES] {
            let body = vec![31; size];
            let wire = key.seal_object(context(), &body).unwrap();
            assert_eq!(*key.open_object(context(), &wire).unwrap(), body);
        }
        assert!(
            key.seal_object(context(), &vec![0; MAX_OBJECT_BYTES + 1])
                .is_err()
        );
        for size in [0, 24, OVERHEAD - 1, MAX_OBJECT_BYTES + OVERHEAD + 1] {
            assert!(key.open_object(context(), &vec![1; size]).is_err());
        }
        assert!(key.open_object(context(), &[2; OVERHEAD]).is_err());
    }
}
