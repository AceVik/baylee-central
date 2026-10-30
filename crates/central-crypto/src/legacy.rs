//! The official gateway's own password hashes, for the legacy store's sign-in
//! alone (`docs/native-login.md` §"Legacy login"): Argon2id, version 0x13, with
//! that gateway's fixed parameters, over a caller-owned work buffer wiped as
//! the OPAQUE stretch's is. There are no other parameters, so a stored hash
//! cannot choose what one verification costs. Server side only (`server`).
//!
//! This derives; it does not compare. The caller compares the output with the
//! stored one in constant time and drops both. Argon2's own intermediate
//! hashes (H0, the final block) stay on the worker's stack, as they do for
//! the OPAQUE stretch: argon2's `zeroize` feature would wipe them and is not
//! in the approved inventory yet.

use crate::{CryptoError, opaque::stretch::WorkBuffer};
use opaque_ke::argon2::{Algorithm, Argon2, Block, Params, Version};
use zeroize::Zeroizing;

/// The gateway's salt length: `RECOMMENDED_SALT_LEN` of its `password-hash`.
pub const SALT_BYTES: usize = 16;
/// The gateway's output length: argon2's default.
pub const HASH_BYTES: usize = 32;
/// The longest password derived, in bytes; the gateway took 128 characters.
pub const MAX_PASSWORD_BYTES: usize = 1024;

/// The gateway's parameters, argon2's defaults when it hashed: 19 MiB, two
/// passes, one lane.
fn algorithm() -> Result<Argon2<'static>, CryptoError> {
    let parameters =
        Params::new(19_456, 2, 1, Some(HASH_BYTES)).map_err(|_| CryptoError::Invalid)?;
    Ok(Argon2::new(Algorithm::Argon2id, Version::V0x13, parameters))
}

/// The output the gateway stored for `password` under `salt`. It never fails
/// for a wrong password: the comparison tells that.
///
/// # Errors
/// An empty password or one over [`MAX_PASSWORD_BYTES`], refused before any
/// work.
pub fn derive(
    password: &[u8],
    salt: &[u8; SALT_BYTES],
) -> Result<Zeroizing<[u8; HASH_BYTES]>, CryptoError> {
    if !(1..=MAX_PASSWORD_BYTES).contains(&password.len()) {
        return Err(CryptoError::Invalid);
    }
    let algorithm = algorithm()?;
    let mut output = Zeroizing::new([0; HASH_BYTES]);
    let mut blocks = vec![Block::default(); algorithm.params().block_count()];
    let work = WorkBuffer(&mut blocks);
    algorithm
        .hash_password_into_with_memory(password, salt, output.as_mut(), &mut *work.0)
        .map_err(|_| CryptoError::Invalid)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Written by the official gateway's own code (argon2 0.6,
    /// `Argon2::default()`), for a synthetic password:
    /// `$argon2id$v=19$m=19456,t=2,p=1$wittOwBy6Zxq/y0DGjZasA$fBlUSay3ZkJb5bDps9DdGXWgriHX9cjNYnp4ZXs80go`.
    const SALT: [u8; SALT_BYTES] = [
        194, 43, 109, 59, 0, 114, 233, 156, 106, 255, 45, 3, 26, 54, 90, 176,
    ];
    const HASH: [u8; HASH_BYTES] = [
        124, 25, 84, 73, 172, 183, 102, 66, 91, 229, 176, 233, 179, 208, 221, 25, 117, 160, 174,
        33, 215, 245, 200, 205, 98, 122, 120, 101, 123, 60, 210, 10,
    ];

    #[test]
    fn the_gateways_own_hash_is_reproduced() {
        assert_eq!(*derive(b"synthetic legacy password", &SALT).unwrap(), HASH);
    }

    #[test]
    fn another_password_or_salt_gives_another_output() {
        assert_ne!(*derive(b"synthetic legacy passwore", &SALT).unwrap(), HASH);
        let mut salt = SALT;
        salt[15] ^= 1;
        assert_ne!(*derive(b"synthetic legacy password", &salt).unwrap(), HASH);
    }

    #[test]
    fn an_empty_or_overlong_password_is_refused_before_any_work() {
        assert_eq!(derive(b"", &SALT).err(), Some(CryptoError::Invalid));
        let long = vec![b'x'; MAX_PASSWORD_BYTES + 1];
        assert_eq!(derive(&long, &SALT).err(), Some(CryptoError::Invalid));
        assert!(derive(&long[..MAX_PASSWORD_BYTES], &SALT).is_ok());
    }

    #[test]
    fn the_parameters_are_the_gateways() {
        let parameters = algorithm().unwrap();
        let parameters = parameters.params();
        assert_eq!(
            (
                parameters.m_cost(),
                parameters.t_cost(),
                parameters.p_cost(),
                parameters.output_len()
            ),
            (19_456, 2, 1, Some(HASH_BYTES))
        );
    }
}
