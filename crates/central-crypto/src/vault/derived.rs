//! What is derived one way from the master key, each value for one purpose
//! (`docs/crypto-protocol.md` §"Derived from the master key"): HKDF-SHA-512
//! with the master key as input, a salt of its own per purpose and the
//! account context as info, as the object keys are derived. The master key
//! itself still has no export; each purpose is a method that answers its
//! value and nothing else.

use super::MasterKey;
use crate::{Context, CryptoError};
use hkdf::Hkdf;
use sha2::Sha512;
use std::fmt;
use zeroize::Zeroizing;

const CHECK_SALT: &[u8] = b"baylee-central/key-check/v1";
const CHECK_DOMAIN: &[u8] = b"baylee-central/key-check/v1\0";
const VAULT_SALT: &[u8] = b"baylee-central/local-vault/v1";
const VAULT_DOMAIN: &[u8] = b"baylee-central/local-vault/v1\0";

/// The account's key check: 32 bytes Central stores so that a proof made
/// with it shows the master key's holder (pairing's approval), while the
/// value reveals nothing of the master key. It changes with the epoch.
#[derive(Clone, Eq, PartialEq)]
pub struct KeyCheck([u8; 32]);

impl fmt::Debug for KeyCheck {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("KeyCheck([REDACTED])")
    }
}

impl KeyCheck {
    /// Reads a stored or received check: exactly 32 bytes.
    ///
    /// # Errors
    ///
    /// Rejects any other length.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CryptoError> {
        Ok(Self(bytes.try_into().map_err(|_| CryptoError::Invalid)?))
    }

    /// The bytes Central stores.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// The key a client wraps one local chat vault's data key under (Baylee's
/// `password.vault`, version 2), for the vault whose 32-byte scope it was
/// derived for. Never stored and never sent; wiped when dropped.
pub struct LocalVaultKey(Zeroizing<[u8; 32]>);

impl fmt::Debug for LocalVaultKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("LocalVaultKey([REDACTED])")
    }
}

impl LocalVaultKey {
    /// The key itself, for the client's local cipher; still wiped on drop.
    #[must_use]
    pub fn into_bytes(self) -> Zeroizing<[u8; 32]> {
        self.0
    }
}

impl MasterKey {
    /// The account's key check for `context` (its issuer, account and epoch).
    ///
    /// # Errors
    ///
    /// Only if the KDF refuses the fixed output length, which it never does.
    pub fn key_check(&self, context: Context) -> Result<KeyCheck, CryptoError> {
        let mut check = [0; 32];
        Hkdf::<Sha512>::new(Some(CHECK_SALT), self.0.as_ref())
            .expand(&context.bound::<{ 28 + 56 }>(CHECK_DOMAIN), &mut check)
            .map_err(|_| CryptoError::Invalid)?;
        Ok(KeyCheck(check))
    }

    /// The key of the local vault whose scope is `scope`, for `context`.
    ///
    /// # Errors
    ///
    /// Only if the KDF refuses the fixed output length, which it never does.
    pub fn local_vault_key(
        &self,
        context: Context,
        scope: &[u8; 32],
    ) -> Result<LocalVaultKey, CryptoError> {
        let bound = context.bound::<{ 30 + 56 }>(VAULT_DOMAIN);
        let mut info = [0; 30 + 56 + 32];
        info[..bound.len()].copy_from_slice(&bound);
        info[bound.len()..].copy_from_slice(scope);
        let mut key = Zeroizing::new([0; 32]);
        Hkdf::<Sha512>::new(Some(VAULT_SALT), self.0.as_ref())
            .expand(&info, key.as_mut())
            .map_err(|_| CryptoError::Invalid)?;
        Ok(LocalVaultKey(key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(text: &str) -> Vec<u8> {
        (0..text.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
            .collect()
    }

    fn account() -> [u8; 16] {
        hex("0192d0c57b3e7a1f8c2d3e4f5a6b7c8d").try_into().unwrap()
    }

    fn live() -> [u8; 32] {
        hex("04f2cff83011533f0a5fdea045398d66f3933a3ca42063c2ad242893d7384b3b")
            .try_into()
            .unwrap()
    }

    fn master(byte: u8) -> MasterKey {
        let mut key = Zeroizing::new([byte; 32]);
        if byte == 0 {
            for (i, b) in key.iter_mut().enumerate() {
                *b = u8::try_from(i).unwrap();
            }
        }
        MasterKey(key)
    }

    #[test]
    fn the_domains_are_as_long_as_their_buffers_say() {
        assert_eq!(CHECK_DOMAIN.len(), 28);
        assert_eq!(VAULT_DOMAIN.len(), 30);
    }

    /// `tests/derived-vectors.py`: RFC 5869 HKDF over the standard library's
    /// HMAC, nothing of this crate's stack.
    #[test]
    fn the_derivations_match_an_independent_implementation() {
        let scope: [u8; 32] = (0x40..0x60).collect::<Vec<u8>>().try_into().unwrap();
        let first = Context::new(live(), account(), 1).unwrap();
        let key = master(0);
        assert_eq!(
            key.key_check(first).unwrap().as_bytes().to_vec(),
            hex("d4101f7292453b4c0a88af53b78562ac9b25a8ca2549b66f0023602a08cb8d0d")
        );
        assert_eq!(
            key.local_vault_key(first, &scope)
                .unwrap()
                .into_bytes()
                .to_vec(),
            hex("a9e00f8ed6e070d0fb218e7383ca9568e70cc1cbf5402754ef2f4005f090b5f4")
        );
        let last = Context::new([1; 32], account(), u64::MAX).unwrap();
        let key = master(0xff);
        assert_eq!(
            key.key_check(last).unwrap().as_bytes().to_vec(),
            hex("473feff2f3ef22f7d1bc916fa08dee4d4ee6a9de61885e3f49c55c7d25435a15")
        );
        assert_eq!(
            key.local_vault_key(last, &[0; 32])
                .unwrap()
                .into_bytes()
                .to_vec(),
            hex("ba72dfc13d159976c3b90db4e7010ba4e5aa466d39c1737afab2a1d0ea89074e")
        );
    }

    /// Every input moves every output, and no two purposes share a value.
    #[test]
    fn each_input_and_each_purpose_gives_another_value() {
        let base = Context::new(live(), account(), 1).unwrap();
        let mut other_account = account();
        other_account[15] ^= 1;
        let contexts = [
            base,
            Context::new([1; 32], account(), 1).unwrap(),
            Context::new(live(), other_account, 1).unwrap(),
            Context::new(live(), account(), 2).unwrap(),
        ];
        let mut seen = std::collections::BTreeSet::new();
        for key in [master(0), master(1)] {
            for context in contexts {
                for scope in [[0; 32], [1; 32]] {
                    assert!(
                        seen.insert(
                            key.local_vault_key(context, &scope)
                                .unwrap()
                                .into_bytes()
                                .to_vec()
                        )
                    );
                }
                assert!(seen.insert(key.key_check(context).unwrap().as_bytes().to_vec()));
            }
        }
        assert_eq!(seen.len(), 2 * 4 * 3);
    }

    #[test]
    fn a_check_is_exactly_thirty_two_bytes() {
        assert!(KeyCheck::from_bytes(&[7; 32]).is_ok());
        for len in [0, 31, 33, 64] {
            assert_eq!(
                KeyCheck::from_bytes(&vec![7; len]).unwrap_err(),
                CryptoError::Invalid
            );
        }
    }

    #[test]
    fn neither_value_prints() {
        let context = Context::new(live(), account(), 1).unwrap();
        let key = master(0);
        let check = key.key_check(context).unwrap();
        let vault = key.local_vault_key(context, &[0; 32]).unwrap();
        assert_eq!(format!("{check:?}"), "KeyCheck([REDACTED])");
        assert_eq!(format!("{vault:?}"), "LocalVaultKey([REDACTED])");
    }
}
