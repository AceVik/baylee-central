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

#[cfg(test)]
mod tests {
    use super::*;

    fn account() -> [u8; 16] {
        let mut account = [3; 16];
        account[6] = 0x70;
        account[8] = 0x80;
        account
    }

    #[test]
    fn an_account_is_a_version_seven_uuid_and_nothing_near_it() {
        for octet in 0..=u8::MAX {
            let mut version = account();
            version[6] = octet;
            assert_eq!(
                Context::new([1; 32], version, 1).is_ok(),
                (0x70..=0x7f).contains(&octet),
                "version octet {octet:#x}"
            );
            let mut variant = account();
            variant[8] = octet;
            assert_eq!(
                Context::new([1; 32], variant, 1).is_ok(),
                (0x80..=0xbf).contains(&octet),
                "variant octet {octet:#x}"
            );
        }
        // One nonzero octet makes an issuer, at whichever end it stands.
        for index in [0, 31] {
            let mut issuer = [0; 32];
            issuer[index] = 1;
            assert!(Context::new(issuer, account(), 1).is_ok());
        }
        assert!(Context::new([0; 32], account(), 1).is_err());
        assert!(Context::new([1; 32], account(), 0).is_err());
        assert!(Context::new([1; 32], account(), u64::MAX).is_ok());
    }

    #[test]
    fn no_two_contexts_or_purposes_authenticate_the_same_bytes() {
        let base = Context::new([1; 32], account(), 1).unwrap();
        let mut seen = std::collections::BTreeSet::new();
        assert!(seen.insert(base.aad().to_vec()));
        assert!(seen.insert(base.recovery_aad().to_vec()));
        for bit in 0..8 {
            for octet in 0..32 {
                let mut issuer = base.issuer;
                issuer[octet] ^= 1 << bit;
                let other = Context { issuer, ..base };
                assert!(seen.insert(other.aad().to_vec()), "issuer {octet} {bit}");
                assert!(seen.insert(other.recovery_aad().to_vec()));
            }
            for octet in 0..16 {
                let mut account = base.account;
                account[octet] ^= 1 << bit;
                let other = Context { account, ..base };
                assert!(seen.insert(other.aad().to_vec()), "account {octet} {bit}");
                assert!(seen.insert(other.recovery_aad().to_vec()));
            }
        }
        for shift in 1..64 {
            let epoch = base.epoch ^ (1 << shift);
            let other = Context { epoch, ..base };
            assert!(seen.insert(other.aad().to_vec()), "epoch bit {shift}");
            assert!(seen.insert(other.recovery_aad().to_vec()));
        }
    }
}
