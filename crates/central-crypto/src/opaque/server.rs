//! Server-side OPAQUE primitive only. No password or vault key API is exposed.

use super::{Suite, exact_length, identifiers, transcript};
use crate::{Context, CryptoError};
use opaque_ke::{
    CredentialFinalization, CredentialRequest, RegistrationRequest, RegistrationUpload,
    ServerLogin, ServerLoginParameters, ServerRegistration, ServerSetup,
};
use rand::rngs::OsRng;
use zeroize::Zeroize;

/// OPAQUE deployment secrets. No Debug/Clone/serialization API in this spike.
pub struct ServerKeys(ServerSetup<Suite>);

/// Stored OPAQUE credential record, not a plaintext password or vault key.
pub struct Record(ServerRegistration<Suite>);

/// Single-use login challenge; the enclosing service must enforce expiry,
/// account/session binding, bounded pending attempts and atomic consumption.
pub struct Attempt {
    state: ServerLogin<Suite>,
    context: Context,
}

impl ServerKeys {
    /// Create new deployment keys using the operating system RNG.
    /// These ephemeral spike keys must not replace real persisted server keys.
    #[must_use]
    pub fn generate() -> Self {
        Self(ServerSetup::new(&mut OsRng))
    }

    /// Return an enrollment response. Caller must own a valid registration ticket.
    ///
    /// # Errors
    ///
    /// Rejects invalid request encoding/group elements.
    pub fn enrollment_response(
        &self,
        context: Context,
        request: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        exact_length::<opaque_ke::RegistrationRequestLen<Suite>>(request)?;
        let request =
            RegistrationRequest::<Suite>::deserialize(request).map_err(|_| CryptoError::Invalid)?;
        let result = ServerRegistration::<Suite>::start(&self.0, request, &context.account)
            .map_err(|_| CryptoError::Invalid)?;
        Ok(result.message.serialize().to_vec())
    }

    /// Parse an enrollment upload; the caller must atomically bind it to the
    /// authorized registration operation. This method is NOT authorization.
    ///
    /// # Errors
    ///
    /// Rejects malformed or oversized registration uploads.
    pub fn enrollment_record(upload: &[u8]) -> Result<Record, CryptoError> {
        exact_length::<opaque_ke::RegistrationUploadLen<Suite>>(upload)?;
        let message =
            RegistrationUpload::<Suite>::deserialize(upload).map_err(|_| CryptoError::Invalid)?;
        Ok(Record(ServerRegistration::finish(message)))
    }

    /// Start a login with a real record or the library's fake-record path.
    /// HTTP status, lookup timing and rate limits still need enumeration defense.
    ///
    /// # Errors
    ///
    /// Rejects malformed client messages or protocol failure.
    pub fn start_login(
        &self,
        context: Context,
        record: Option<&Record>,
        request: &[u8],
    ) -> Result<(Attempt, Vec<u8>), CryptoError> {
        exact_length::<opaque_ke::CredentialRequestLen<Suite>>(request)?;
        let request =
            CredentialRequest::<Suite>::deserialize(request).map_err(|_| CryptoError::Invalid)?;
        let transcript = transcript(&context);
        let parameters = ServerLoginParameters {
            context: Some(&transcript),
            identifiers: identifiers(&context),
        };
        let result = ServerLogin::<Suite>::start(
            &mut OsRng,
            &self.0,
            record.map(|record| record.0.clone()),
            request,
            &context.account,
            parameters,
        )
        .map_err(|_| CryptoError::Invalid)?;
        Ok((
            Attempt {
                state: result.state,
                context,
            },
            result.message.serialize().to_vec(),
        ))
    }
}

impl Attempt {
    /// Authenticate the final message once. Only after this succeeds may the
    /// service consider session issuance, subject to account/MFA/rate-limit rules.
    ///
    /// # Errors
    ///
    /// Rejects invalid or mismatched proof. No success for an incomplete exchange.
    pub fn finish(self, proof: &[u8]) -> Result<(), CryptoError> {
        exact_length::<opaque_ke::CredentialFinalizationLen<Suite>>(proof)?;
        let message = CredentialFinalization::<Suite>::deserialize(proof)
            .map_err(|_| CryptoError::Invalid)?;
        let mut result = self
            .state
            .finish(
                message,
                ServerLoginParameters {
                    context: Some(&transcript(&self.context)),
                    identifiers: identifiers(&self.context),
                },
            )
            .map_err(|_| CryptoError::Invalid)?;
        result.session_key.zeroize();
        Ok(())
    }
}
