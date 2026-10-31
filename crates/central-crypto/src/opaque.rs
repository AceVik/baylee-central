//! Fixed-suite OPAQUE integration; server APIs are opt-in.
//!
//! This is an isolated protocol spike, not a network authentication service.
//! Registration tickets, rate limits, challenge expiry, account ownership, MFA,
//! atomic persistence and final session issuance must be enforced by the service.

use crate::{Context, CryptoError, vault::WrappingKey};
use opaque_ke::generic_array::typenum::Unsigned;
use opaque_ke::{
    ClientLogin, ClientLoginFinishParameters, ClientRegistration,
    ClientRegistrationFinishParameters, CredentialResponse, Identifiers, RegistrationResponse,
};
use rand::rngs::OsRng;
use secrecy::{ExposeSecret, SecretString};
use zeroize::Zeroize;

const TRANSCRIPT: &[u8] = b"baylee-central/opaque-3dh/ristretto255-sha512/argon2id-64m-t3-p4/v2";

// The fixed suite determines every wire size. Upstream parsers may accept prefixes.
fn exact_length<L: Unsigned>(bytes: &[u8]) -> Result<(), CryptoError> {
    if bytes.len() != L::USIZE {
        return Err(CryptoError::Invalid);
    }
    Ok(())
}

fn transcript(context: &Context) -> [u8; TRANSCRIPT.len() + 8] {
    let mut bytes = [0; TRANSCRIPT.len() + 8];
    bytes[..TRANSCRIPT.len()].copy_from_slice(TRANSCRIPT);
    bytes[TRANSCRIPT.len()..].copy_from_slice(&context.epoch.to_be_bytes());
    bytes
}

struct Suite;
impl opaque_ke::CipherSuite for Suite {
    type OprfCs = opaque_ke::Ristretto255;
    type KeyExchange = opaque_ke::TripleDh<opaque_ke::Ristretto255, sha2::Sha512>;
    type Ksf = stretch::FixedKsf;
}

pub(crate) mod stretch;

fn identifiers(context: &Context) -> Identifiers<'_> {
    Identifiers {
        client: Some(&context.account),
        server: Some(&context.issuer),
    }
}

fn password(value: String) -> Result<SecretString, CryptoError> {
    let value = SecretString::from(value);
    if value.expose_secret().is_empty() || value.expose_secret().len() > 1024 {
        return Err(CryptoError::Invalid);
    }
    // New-password strength policy belongs above the primitive; existing
    // accounts may require migration. Do not normalize UTF-8 passwords here.
    Ok(value)
}

/// Single-use client registration state. Password is retained only until finish/drop.
pub struct Enrollment {
    state: ClientRegistration<Suite>,
    password: SecretString,
    context: Context,
}

/// Client registration output. Upload is opaque; the wrapping key never goes to the server.
pub struct EnrollmentResult {
    /// Registration proof/envelope for the authenticated registration operation.
    pub upload: Vec<u8>,
    /// Client-only key for wrapping the account master key.
    pub wrapping_key: WrappingKey,
}

impl Enrollment {
    /// Start for the trusted issuer and allocated immutable account.
    ///
    /// # Errors
    ///
    /// Rejects invalid password length or cryptographic failure.
    pub fn start(password_value: String, context: Context) -> Result<(Self, Vec<u8>), CryptoError> {
        let password = password(password_value)?;
        let result =
            ClientRegistration::<Suite>::start(&mut OsRng, password.expose_secret().as_bytes())
                .map_err(|_| CryptoError::Invalid)?;
        Ok((
            Self {
                state: result.state,
                password,
                context,
            },
            result.message.serialize().to_vec(),
        ))
    }

    /// Finish once using a bounded server message and fixed client KDF profile.
    ///
    /// # Errors
    ///
    /// Rejects malformed/reflected messages or authentication/KDF failures.
    pub fn finish(self, response: &[u8]) -> Result<EnrollmentResult, CryptoError> {
        exact_length::<opaque_ke::RegistrationResponseLen<Suite>>(response)?;
        let response = RegistrationResponse::<Suite>::deserialize(response)
            .map_err(|_| CryptoError::Invalid)?;
        let ksf = stretch::FixedKsf;
        let mut result = self
            .state
            .finish(
                &mut OsRng,
                self.password.expose_secret().as_bytes(),
                response,
                ClientRegistrationFinishParameters::new(identifiers(&self.context), Some(&ksf)),
            )
            .map_err(|_| CryptoError::Invalid)?;
        let wrapping_key = WrappingKey::from_export(&result.export_key, self.context);
        result.export_key.zeroize();
        Ok(EnrollmentResult {
            upload: result.message.serialize().to_vec(),
            wrapping_key: wrapping_key?,
        })
    }
}

/// Single-use login state with a fixed expected issuer/account.
///
/// Finishing consumes the state even on failure; retry requires a new exchange:
///
/// ```compile_fail,E0382
/// # #![forbid(unsafe_code)]
/// use central_crypto::opaque::Login;
/// fn reuse(login: Login, response: &[u8]) {
///     let _ = login.finish(response);
///     let _ = login.finish(response);
/// }
/// ```
pub struct Login {
    state: ClientLogin<Suite>,
    password: SecretString,
    context: Context,
}

/// Client finish output. The service still has to verify finalization before
/// issuing a session; possession of this output alone is not a server session.
pub struct LoginResult {
    /// Proof to send to the pending server login operation.
    pub finalization: Vec<u8>,
    /// Client-only wrapping key. Never send this value in an HTTP response/request.
    pub wrapping_key: WrappingKey,
}

impl Login {
    /// Start a bounded native-client login attempt.
    ///
    /// # Errors
    ///
    /// Rejects invalid password length or cryptographic failure.
    pub fn start(password_value: String, context: Context) -> Result<(Self, Vec<u8>), CryptoError> {
        let password = password(password_value)?;
        let result = ClientLogin::<Suite>::start(&mut OsRng, password.expose_secret().as_bytes())
            .map_err(|_| CryptoError::Invalid)?;
        Ok((
            Self {
                state: result.state,
                password,
                context,
            },
            result.message.serialize().to_vec(),
        ))
    }

    /// Verify the server transcript and recover the client-only wrapping key.
    ///
    /// # Errors
    ///
    /// Rejects wrong password, changed identity/context and malformed server input.
    pub fn finish(self, response: &[u8]) -> Result<LoginResult, CryptoError> {
        exact_length::<opaque_ke::CredentialResponseLen<Suite>>(response)?;
        let response =
            CredentialResponse::<Suite>::deserialize(response).map_err(|_| CryptoError::Invalid)?;
        let ksf = stretch::FixedKsf;
        let mut result = self
            .state
            .finish(
                &mut OsRng,
                self.password.expose_secret().as_bytes(),
                response,
                ClientLoginFinishParameters::new(
                    Some(&transcript(&self.context)),
                    identifiers(&self.context),
                    Some(&ksf),
                ),
            )
            .map_err(|_| CryptoError::Invalid)?;
        let wrapping_key = WrappingKey::from_export(&result.export_key, self.context);
        result.export_key.zeroize();
        // The shared session key is known to the server and must never wrap the vault.
        result.session_key.zeroize();
        Ok(LoginResult {
            finalization: result.message.serialize().to_vec(),
            wrapping_key: wrapping_key?,
        })
    }
}

#[cfg(feature = "server")]
pub mod server;
