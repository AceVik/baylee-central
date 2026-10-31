//! Fixed Argon2 profile with explicit wiping of the caller-owned work buffer.

use opaque_ke::{
    argon2::{Algorithm, Argon2, Block, Params, Version},
    errors::InternalError,
    generic_array::{ArrayLength, GenericArray},
    ksf::Ksf,
};
use zeroize::{Zeroize, Zeroizing};

#[derive(Default)]
pub(super) struct FixedKsf;

fn algorithm() -> Result<Argon2<'static>, InternalError> {
    let parameters = Params::new(65_536, 3, 4, Some(64)).map_err(|_| InternalError::KsfError)?;
    Ok(Argon2::new(Algorithm::Argon2id, Version::V0x13, parameters))
}

// This guard also wipes the entire scratch buffer on error or unwinding; the
// legacy store's verification (`crate::legacy`) uses it too.
pub(crate) struct WorkBuffer<'a>(pub(crate) &'a mut [Block]);
impl Drop for WorkBuffer<'_> {
    fn drop(&mut self) {
        for block in &mut *self.0 {
            block.as_mut().zeroize();
        }
    }
}

impl Ksf for FixedKsf {
    fn hash<L: ArrayLength<u8>>(
        &self,
        input: GenericArray<u8, L>,
    ) -> Result<GenericArray<u8, L>, InternalError> {
        let input = Zeroizing::new(input);
        let mut output = Zeroizing::new(GenericArray::default());
        let algorithm = algorithm()?;
        let mut blocks = vec![Block::default(); algorithm.params().block_count()];
        let work = WorkBuffer(&mut blocks);
        // Same fixed salt as opaque-ke's Argon2 KSF integration; the OPRF output
        // is already account/password specific. No custom cryptographic algorithm.
        algorithm
            .hash_password_into_with_memory(&input, &[0; 16], &mut output, &mut *work.0)
            .map_err(|_| InternalError::KsfError)?;
        Ok((*output).clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opaque_ke::generic_array::typenum::U64;

    #[test]
    fn wiping_adapter_matches_upstream_profile() {
        let input = GenericArray::<u8, U64>::default();
        assert_eq!(
            FixedKsf.hash(input).unwrap(),
            algorithm().unwrap().hash(input).unwrap(),
        );
    }

    #[test]
    fn work_buffer_is_wiped_after_unwind() {
        let mut blocks = [Block::default(); 2];
        for block in &mut blocks {
            block.as_mut().fill(u64::MAX);
        }
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = WorkBuffer(&mut blocks);
            panic!("synthetic interruption");
        }));
        assert!(result.is_err());
        assert!(
            blocks
                .iter()
                .all(|block| block.as_ref().iter().all(|word| *word == 0))
        );
    }
}
