//! Bounded opaque private payloads on the native transport.
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use std::{fmt, marker::PhantomData};
use zeroize::Zeroizing;

/// A bounded body encoded as canonical lowercase hex on the JSON transport.
#[derive(Clone, PartialEq, Eq)]
pub struct PrivateBytes<const LIMIT: usize>(Zeroizing<Vec<u8>>);

impl<const LIMIT: usize> PrivateBytes<LIMIT> {
    pub(crate) fn serialize_slice<S: Serializer>(
        slice: &[u8],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        if slice.is_empty() || slice.len() > LIMIT {
            return Err(serde::ser::Error::custom("invalid private payload length"));
        }
        let text = Zeroizing::new(crate::picture::hex(slice));
        serializer.serialize_str(&text)
    }

    pub(crate) fn into_vec(mut self) -> Vec<u8> {
        std::mem::take(&mut *self.0)
    }

    /// Construct a non-empty, size-checked payload.
    ///
    /// # Errors
    /// Rejects an empty or oversized body.
    pub fn new(bytes: Vec<u8>) -> Result<Self, crate::Error> {
        let bytes = Zeroizing::new(bytes);
        if bytes.is_empty() || bytes.len() > LIMIT {
            return Err(crate::Error::Rejected);
        }
        Ok(Self(bytes))
    }

    /// Borrow the exact committed bytes without copying.
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl<const LIMIT: usize> fmt::Debug for PrivateBytes<LIMIT> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PrivateBytes([REDACTED])")
    }
}

impl<const LIMIT: usize> Serialize for PrivateBytes<LIMIT> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        Self::serialize_slice(self.as_slice(), serializer)
    }
}

impl<'de, const LIMIT: usize> Deserialize<'de> for PrivateBytes<LIMIT> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Hex<const N: usize>(PhantomData<()>);
        impl<const N: usize> de::Visitor<'_> for Hex<N> {
            type Value = PrivateBytes<N>;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a bounded private payload")
            }

            fn visit_str<E: de::Error>(self, text: &str) -> Result<Self::Value, E> {
                let len = text.len();
                if len == 0 || !len.is_multiple_of(2) || len / 2 > N {
                    return Err(E::custom("invalid private payload length"));
                }
                let nibble = |byte| match byte {
                    b'0'..=b'9' => Ok(byte - b'0'),
                    b'a'..=b'f' => Ok(byte - b'a' + 10),
                    _ => Err(E::custom("invalid private payload encoding")),
                };
                let mut bytes = Zeroizing::new(Vec::with_capacity(len / 2));
                for pair in text.as_bytes().as_chunks::<2>().0 {
                    bytes.push(nibble(pair[0])? << 4 | nibble(pair[1])?);
                }
                Ok(PrivateBytes(bytes))
            }
        }
        deserializer.deserialize_str(Hex::<LIMIT>(PhantomData))
    }
}
