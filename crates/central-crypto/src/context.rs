const WRAP_DOMAIN: &[u8] = b"baylee-central/master-wrap/v1\0";
const RECOVERY_DOMAIN: &[u8] = b"baylee-central/recovery-wrap/v1\0";

/// Fixed-width domain context supplied by the trusted client configuration and
/// authenticated account mapping. These bytes do not authenticate themselves.
/// The issuer is an agreed 32-byte deployment identifier, not a gateway-provided URL.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Context {
    pub(crate) issuer: [u8; 32],
    pub(crate) account: [u8; 16],
    pub(crate) epoch: u64,
}

impl Context {
    /// Bind an externally established issuer and immutable `UUIDv7` account to a
    /// nonzero vault epoch. This function does not generate or authorize IDs.
    ///
    /// # Errors
    ///
    /// Rejects a zero issuer, non-UUIDv7 account or zero epoch.
    pub fn new(
        issuer: [u8; 32],
        account: [u8; 16],
        epoch: u64,
    ) -> Result<Self, crate::CryptoError> {
        if issuer == [0; 32] || account[6] >> 4 != 7 || account[8] >> 6 != 2 || epoch == 0 {
            return Err(crate::CryptoError::Invalid);
        }
        Ok(Self {
            issuer,
            account,
            epoch,
        })
    }

    /// The password wrapper's associated data, and the info of both wrapping
    /// keys' derivations: the domain, issuer, account and big-endian epoch.
    pub(crate) fn aad(self) -> [u8; WRAP_DOMAIN.len() + 56] {
        self.bound(WRAP_DOMAIN)
    }

    /// The recovery wrapper's associated data: the same fields under its own domain.
    pub(crate) fn recovery_aad(self) -> [u8; RECOVERY_DOMAIN.len() + 56] {
        self.bound(RECOVERY_DOMAIN)
    }

    /// `domain`, then the issuer, account and epoch; `N` is `domain`'s length plus 56.
    pub(crate) fn bound<const N: usize>(self, domain: &[u8]) -> [u8; N] {
        let mut result = [0; N];
        let (label, fields) = result.split_at_mut(domain.len());
        label.copy_from_slice(domain);
        fields[..32].copy_from_slice(&self.issuer);
        fields[32..48].copy_from_slice(&self.account);
        fields[48..].copy_from_slice(&self.epoch.to_be_bytes());
        result
    }
}
