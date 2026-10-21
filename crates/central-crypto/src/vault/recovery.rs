//! The recovery code handed out once at registration, and the second wrapper
//! of the master key it opens (`docs/crypto-protocol.md` §"Recovery code").
//!
//! 20 random bytes, written as 32 symbols of Crockford's base32 in eight
//! groups of four. Its wrapping key is HKDF-SHA-512 over those bytes; the code
//! carries 160 bits, so nothing stretches it.

use super::{MasterKey, WrappedMasterKey, open, random_nonce, seal};
use crate::{Context, CryptoError};
use hkdf::Hkdf;
use sha2::Sha512;
use std::fmt;
use zeroize::Zeroizing;

/// Crockford's base32: digits and capitals without I, L, O and U.
const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
const SALT: &[u8] = b"baylee-central/recovery/v1";
const BYTES: usize = 20;
const SYMBOLS: usize = 32;
/// Symbols per group in the written form.
const GROUP: usize = 4;
/// Written length: 32 symbols and seven hyphens.
const TEXT: usize = SYMBOLS + SYMBOLS / GROUP - 1;
/// Longest input [`RecoveryCode::parse`] reads, so a pasted page costs nothing.
const MAX_INPUT: usize = 128;

/// A recovery code: 160 random bits that open the recovery wrapper of an
/// account's master key. Not clonable or serializable; Debug is redacted.
pub struct RecoveryCode(Zeroizing<[u8; BYTES]>);

impl fmt::Debug for RecoveryCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RecoveryCode([REDACTED])")
    }
}

/// The symbol's five bits, reading `O` as 0 and `I` and `L` as 1 in either case.
fn value(symbol: u8) -> Option<u8> {
    let symbol = match symbol.to_ascii_uppercase() {
        b'O' => b'0',
        b'I' | b'L' => b'1',
        other => other,
    };
    ALPHABET
        .iter()
        .position(|&each| each == symbol)
        .and_then(|index| u8::try_from(index).ok())
}

impl RecoveryCode {
    /// A new code from operating-system randomness.
    ///
    /// # Errors
    ///
    /// Fails rather than falling back to deterministic randomness.
    pub fn generate() -> Result<Self, CryptoError> {
        let mut code = Zeroizing::new([0; BYTES]);
        getrandom::fill(code.as_mut()).map_err(|_| CryptoError::RandomnessUnavailable)?;
        Ok(Self(code))
    }

    /// Read a code as a person types it back: any case, `O` for 0, `I` or `L`
    /// for 1, hyphens and ASCII whitespace anywhere.
    ///
    /// # Errors
    ///
    /// Rejects input past 128 bytes, any other character, and anything but
    /// exactly 32 symbols.
    pub fn parse(text: &str) -> Result<Self, CryptoError> {
        if text.len() > MAX_INPUT {
            return Err(CryptoError::Invalid);
        }
        let mut symbols = Zeroizing::new([0_u8; SYMBOLS]);
        let mut count = 0;
        for byte in text.bytes() {
            if byte == b'-' || byte.is_ascii_whitespace() {
                continue;
            }
            let slot = symbols.get_mut(count).ok_or(CryptoError::Invalid)?;
            *slot = value(byte).ok_or(CryptoError::Invalid)?;
            count += 1;
        }
        if count != SYMBOLS {
            return Err(CryptoError::Invalid);
        }
        // Each eight symbols (40 bits) are five bytes, most significant first.
        let mut code = Zeroizing::new([0; BYTES]);
        for (bytes, group) in code
            .as_chunks_mut::<5>()
            .0
            .iter_mut()
            .zip(symbols.as_chunks::<8>().0)
        {
            let mut bits = Zeroizing::new(0_u64);
            for &symbol in group {
                *bits = (*bits << 5) | u64::from(symbol);
            }
            bytes.copy_from_slice(&bits.to_be_bytes()[3..]);
        }
        Ok(Self(code))
    }

    /// The written form, `XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX`, in capitals.
    #[must_use]
    pub fn to_text(&self) -> Zeroizing<String> {
        // Each five bytes (40 bits) are eight symbols, most significant first.
        let mut symbols = Zeroizing::new([0_u8; SYMBOLS]);
        for (group, bytes) in symbols
            .as_chunks_mut::<8>()
            .0
            .iter_mut()
            .zip(self.0.as_chunks::<5>().0)
        {
            let mut wide = Zeroizing::new([0; 8]);
            wide[3..].copy_from_slice(bytes);
            let bits = Zeroizing::new(u64::from_be_bytes(*wide));
            for (index, symbol) in group.iter_mut().enumerate() {
                let shift = 5 * (7 - index);
                *symbol = ALPHABET[usize::try_from((*bits >> shift) & 0x1f).unwrap_or(0)];
            }
        }
        // The capacity is the final length, so the buffer never moves.
        let mut text = Zeroizing::new(String::with_capacity(TEXT));
        for (index, &symbol) in symbols.iter().enumerate() {
            if index > 0 && index % GROUP == 0 {
                text.push('-');
            }
            text.push(char::from(symbol));
        }
        text
    }

    /// Wrap the account's master key under this code for `context`: the same
    /// 73-byte format as the password wrapper, under its own domain.
    ///
    /// # Errors
    ///
    /// Fails on randomness failure or encryption failure.
    pub fn wrap(
        &self,
        context: Context,
        master: &MasterKey,
    ) -> Result<WrappedMasterKey, CryptoError> {
        self.wrap_with_nonce(context, master, random_nonce()?)
    }

    /// Authenticate and decrypt the recovery wrapper; never a new key on failure.
    ///
    /// # Errors
    ///
    /// Rejects another code, another context, the password wrapper, corruption
    /// and unsupported versions.
    pub fn unwrap(
        &self,
        context: Context,
        wrapped: &WrappedMasterKey,
    ) -> Result<MasterKey, CryptoError> {
        open(&*self.key(context)?, &context.recovery_aad(), wrapped)
    }

    fn wrap_with_nonce(
        &self,
        context: Context,
        master: &MasterKey,
        nonce: [u8; 24],
    ) -> Result<WrappedMasterKey, CryptoError> {
        seal(&*self.key(context)?, &context.recovery_aad(), master, nonce)
    }

    /// HKDF-SHA-512: the code as input, [`SALT`], and the context's password
    /// wrapper data (domain, issuer, account, epoch) as info.
    fn key(&self, context: Context) -> Result<Zeroizing<[u8; 32]>, CryptoError> {
        let mut key = Zeroizing::new([0; 32]);
        Hkdf::<Sha512>::new(Some(SALT), self.0.as_ref())
            .expand(&context.aad(), key.as_mut())
            .map_err(|_| CryptoError::Invalid)?;
        Ok(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::WrappingKey;

    fn hex<const N: usize>(text: &str) -> [u8; N] {
        let mut bytes = [0; N];
        assert_eq!(text.len(), N * 2);
        for (index, byte) in bytes.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&text[index * 2..index * 2 + 2], 16).unwrap();
        }
        bytes
    }

    const ACCOUNT: &str = "0192d0c57b3e7a1f8c2d3e4f5a6b7c8d";
    /// `Endpoint::live().issuer()`: SHA-256 of `baylee-central/issuer/v1\0https://central.baylee.app`.
    const LIVE_ISSUER: &str = "04f2cff83011533f0a5fdea045398d66f3933a3ca42063c2ad242893d7384b3b";

    struct Vector {
        code: [u8; 20],
        context: Context,
        master: [u8; 32],
        nonce: [u8; 24],
        text: &'static str,
        kdf: &'static str,
        wire: &'static str,
    }

    /// Computed apart from this crate by `tests/recovery-vectors.py`: Python's
    /// base32 in Crockford's alphabet, pyca/cryptography's HKDF and
    /// libsodium's XChaCha20-Poly1305.
    fn vectors() -> [Vector; 2] {
        let mut ascending = [0; 32];
        for (index, byte) in ascending.iter_mut().enumerate() {
            *byte = u8::try_from(index).unwrap();
        }
        let mut nonce = [0; 24];
        for (index, byte) in nonce.iter_mut().enumerate() {
            *byte = 0xa0 + u8::try_from(index).unwrap();
        }
        [
            Vector {
                code: hex("f00dfacecafebeef0123456789abcdeffedcba98"),
                context: Context::new(hex(LIVE_ISSUER), hex(ACCOUNT), 1).unwrap(),
                master: ascending,
                nonce,
                text: "Y06Z-NKPA-ZTZE-Y093-8NKR-KAYD-XZZD-SEMR",
                kdf: "384e1caeb0c444ccb00bb3d86463a4de867d9f88f6cdb128a6a30b0c0c0797aa",
                wire: "01a0a1a2a3a4a5a6a7a8a9aaabacadaeafb0b1b2b3b4b5b6b77ab04983e613\
                       7478bca88a223df6f2b02c1e8d6ab2a3e712a1313c782ff47c5fd27f109320a0\
                       de0ae59117a0d389192b",
            },
            Vector {
                code: [0; 20],
                context: Context::new([1; 32], hex(ACCOUNT), u64::MAX).unwrap(),
                master: [0xff; 32],
                nonce: [0; 24],
                text: "0000-0000-0000-0000-0000-0000-0000-0000",
                kdf: "da19f1b88fe9b0733336adb233ae30a6190ab978a7974be3824a9382ce00bc59",
                wire: "01000000000000000000000000000000000000000000000000a3dfecfd5e1d38\
                       90c45cae6a8b15bbe4915ccf6bfb61d0a019d79166e82c25ff00298ca3312fc1\
                       a293d398c3e6be8d36",
            },
        ]
    }

    #[test]
    fn known_answers_for_text_key_and_wrapper() {
        for vector in vectors() {
            let code = RecoveryCode(Zeroizing::new(vector.code));
            assert_eq!(code.to_text().as_str(), vector.text);
            assert_eq!(*code.key(vector.context).unwrap(), hex::<32>(vector.kdf));
            let master = MasterKey(Zeroizing::new(vector.master));
            let wrapped = code
                .wrap_with_nonce(vector.context, &master, vector.nonce)
                .unwrap();
            assert_eq!(wrapped.as_bytes(), &hex::<73>(vector.wire));
            // and back from the written form alone
            let parsed = RecoveryCode::parse(vector.text).unwrap();
            let opened = parsed.unwrap(vector.context, &wrapped).unwrap();
            assert_eq!(opened.0.as_ref(), &vector.master);
        }
        let ones = RecoveryCode(Zeroizing::new([0xff; 20]));
        assert_eq!(
            ones.to_text().as_str(),
            "ZZZZ-ZZZZ-ZZZZ-ZZZZ-ZZZZ-ZZZZ-ZZZZ-ZZZZ"
        );
    }

    #[test]
    fn typed_back_in_any_case_with_confusable_letters_and_spacing() {
        let expected = vectors()[0].code;
        for text in [
            "Y06Z-NKPA-ZTZE-Y093-8NKR-KAYD-XZZD-SEMR",
            "y06z-nkpa-ztze-y093-8nkr-kayd-xzzd-semr",
            "YO6Z NKPA ZTZE YO93 8NKR KAYD XZZD SEMR\n",
            "y06znkpaztzey0938nkrkaydxzzdsemr",
            "  Y06Z-NKPA-ZTZE-Y093--8NKR-KAYD-XZZD-SEMR  ",
            "Y-0-6-Z-N-K-P-A-Z-T-Z-E-Y-0-9-3-8-N-K-R-K-A-Y-D-X-Z-Z-D-S-E-M-R",
        ] {
            assert_eq!(*RecoveryCode::parse(text).unwrap().0, expected, "{text}");
        }
        let ones = RecoveryCode::parse("1111-iiii-LLLL-llll-IiLl-1i1L-0000-oOoO").unwrap();
        assert_eq!(
            ones.to_text().as_str(),
            "1111-1111-1111-1111-1111-1111-0000-0000"
        );
    }

    #[test]
    fn anything_but_32_symbols_of_the_alphabet_is_refused() {
        let good = "Y06Z-NKPA-ZTZE-Y093-8NKR-KAYD-XZZD-SEMR";
        for text in [
            "",
            "Y06Z-NKPA-ZTZE-Y093-8NKR-KAYD-XZZD-SEM",
            "Y06Z-NKPA-ZTZE-Y093-8NKR-KAYD-XZZD-SEMRR",
            "U06Z-NKPA-ZTZE-Y093-8NKR-KAYD-XZZD-SEMR",
            "Y06Z_NKPA-ZTZE-Y093-8NKR-KAYD-XZZD-SEMR",
            "Y06Z-NKPA-ZTZE-Y093-8NKR-KAYD-XZZD-SEM\u{212a}",
            "Y06Z-NKPA-ZTZE-Y093-8NKR-KAYD-XZZD-SEM\u{0}",
            "Y06Z.NKPA.ZTZE.Y093.8NKR.KAYD.XZZD.SEMR",
            "Ｙ06Z-NKPA-ZTZE-Y093-8NKR-KAYD-XZZD-SEMR",
        ] {
            assert!(RecoveryCode::parse(text).is_err(), "{text:?}");
        }
        let padded = format!("{good}{}", " ".repeat(MAX_INPUT - good.len()));
        assert!(RecoveryCode::parse(&padded).is_ok());
        assert!(RecoveryCode::parse(&format!("{padded} ")).is_err());
    }

    #[test]
    fn generated_codes_differ_and_survive_their_written_form() {
        let first = RecoveryCode::generate().unwrap();
        let second = RecoveryCode::generate().unwrap();
        assert_ne!(*first.0, *second.0);
        for code in [first, second] {
            let text = code.to_text();
            assert_eq!(text.len(), TEXT);
            assert_eq!(*RecoveryCode::parse(&text).unwrap().0, *code.0);
        }
    }

    #[test]
    fn every_bit_context_and_the_password_domain_are_authenticated() {
        let vector = &vectors()[0];
        let code = RecoveryCode(Zeroizing::new(vector.code));
        let master = MasterKey::generate().unwrap();
        let wrapped = code.wrap(vector.context, &master).unwrap();
        assert_ne!(wrapped, code.wrap(vector.context, &master).unwrap());
        for index in 0..73 {
            for bit in 0..8 {
                let mut changed = *wrapped.as_bytes();
                changed[index] ^= 1 << bit;
                if let Ok(parsed) = WrappedMasterKey::from_bytes(&changed) {
                    assert!(
                        code.unwrap(vector.context, &parsed).is_err(),
                        "{index}/{bit}"
                    );
                }
            }
        }
        let original = vector.context;
        let mut contexts = [original; 3];
        contexts[0].issuer[0] ^= 1;
        contexts[1].account[15] ^= 1;
        contexts[2].epoch += 1;
        for context in contexts {
            assert!(code.unwrap(context, &wrapped).is_err());
        }
        let mut other = vector.code;
        other[19] ^= 1;
        assert!(
            RecoveryCode(Zeroizing::new(other))
                .unwrap(original, &wrapped)
                .is_err()
        );
        // The same key under the password wrapper's domain opens neither way.
        let key = code.key(original).unwrap();
        let as_password = seal(&key, &original.aad(), &master, [7; 24]).unwrap();
        assert!(code.unwrap(original, &as_password).is_err());
        assert!(open(&key, &original.aad(), &wrapped).is_err());
        // Nor does a password wrapping key, whose derivation differs.
        let password = WrappingKey::from_export(&vector.code, original).unwrap();
        assert!(password.unwrap(&wrapped).is_err());
        assert!(
            code.unwrap(original, &password.wrap(&master).unwrap())
                .is_err()
        );
    }

    #[test]
    fn debug_is_redacted() {
        let code = RecoveryCode(Zeroizing::new(vectors()[0].code));
        assert_eq!(format!("{code:?}"), "RecoveryCode([REDACTED])");
    }
}
