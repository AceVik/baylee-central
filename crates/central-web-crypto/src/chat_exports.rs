//! Opaque worker-owned chat identity. Serialized outputs contain public keys,
//! ciphertext, or explicitly requested display text; the one exception is
//! [`Chat::keep`], the identity's bytes for the Worker to seal under the
//! browser's own non-extractable key, which never leave the Worker.
use central_api::{Error, chat::AccountKeys};
use wasm_bindgen::prelude::{JsError, wasm_bindgen};

fn refused(error: Error) -> JsError {
    JsError::new(match error {
        Error::Conflict => "conflict",
        Error::Busy => "busy",
        Error::Unavailable => "unavailable",
        Error::Rejected | Error::TermsRequired | Error::UpdateRequired => "rejected",
    })
}

/// Chat cryptography retained only for the lifetime of the unlocked worker.
#[wasm_bindgen]
pub struct Chat(crate::flow::OpenChat);

/// Consume an authenticated sign-in and derive the account's chat identity.
///
/// # Errors
/// Rejects an invalid wrapper, identity or public tag.
#[wasm_bindgen]
pub fn chat_unlock(
    state: crate::exports::LoginKey,
    wrapped_master: &[u8],
    tag: u32,
) -> Result<Chat, JsError> {
    crate::flow::chat_unlock(state.state, wrapped_master, tag)
        .map(Chat)
        .map_err(|_| JsError::new("rejected"))
}

/// The chat identity again from the bytes [`Chat::keep`] gave, once the
/// Worker has opened them under the browser's key.
///
/// # Errors
/// Rejects bytes of another length or an invalid public tag.
#[wasm_bindgen]
pub fn chat_resume(kept: &[u8], tag: u32) -> Result<Chat, JsError> {
    crate::flow::chat_resume(kept, tag)
        .map(Chat)
        .map_err(|_| JsError::new("rejected"))
}

#[wasm_bindgen]
impl Chat {
    /// Stable public device fingerprint.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn device(&self) -> String {
        self.0.chat.device().to_owned()
    }

    /// The identity's secret bytes, for the Worker to seal at once under the
    /// browser's non-extractable key and wipe; never for the page.
    #[must_use]
    pub fn keep(&self) -> Vec<u8> {
        self.0.kept.to_vec()
    }

    /// Public signed registration as JSON bytes.
    ///
    /// # Errors
    /// Invalid binding or encoding.
    pub fn registration(&self) -> Result<Vec<u8>, JsError> {
        self.0
            .chat
            .registration()
            .and_then(|v| central_api::encode(&v))
            .map_err(refused)
    }

    /// Validate a peer's keys and return its safety-number review as JSON.
    ///
    /// # Errors
    /// Malformed, duplicated or substituted own-account keys.
    pub fn observe(&mut self, keys: &[u8]) -> Result<Vec<u8>, JsError> {
        let keys: AccountKeys = central_api::decode(keys).map_err(refused)?;
        self.0
            .chat
            .observe(keys)
            .and_then(|v| central_api::encode(&v))
            .map_err(refused)
    }

    /// Confirm exactly the public snapshot displayed to the person.
    ///
    /// # Errors
    /// Unknown peers, malformed or stale confirmations.
    pub fn confirm(&mut self, keys: &[u8]) -> Result<(), JsError> {
        self.0
            .chat
            .confirm(central_api::decode(keys).map_err(refused)?)
            .map_err(refused)
    }

    /// Seal one message with fresh randomness and a new correlation ID.
    ///
    /// # Errors
    /// Invalid text/membership, changed keys or randomness failure.
    pub fn seal(
        &self,
        conv: &str,
        text: String,
        emote: bool,
        members: &[u32],
        now: u64,
    ) -> Result<Vec<u8>, JsError> {
        self.0
            .chat
            .seal(conv, text, emote, members, now)
            .and_then(|v| central_api::encode(&v))
            .map_err(refused)
    }

    /// Authenticate and open a delivery for display.
    ///
    /// # Errors
    /// Malformed, forged, misaddressed or unconfirmed messages.
    pub fn open(&self, delivery: &[u8]) -> Result<Vec<u8>, JsError> {
        self.0
            .chat
            .open(&central_api::decode(delivery).map_err(refused)?)
            .and_then(|v| central_api::encode(&v))
            .map_err(refused)
    }

    /// Authenticated ciphertext for the browser's peer-key trust state.
    ///
    /// # Errors
    /// Randomness, encoding or encryption failure.
    pub fn save_trust(&self) -> Result<Vec<u8>, JsError> {
        self.0.chat.save_trust().map_err(refused)
    }

    /// Restore an authenticated account-bound trust state before reading new keys.
    ///
    /// # Errors
    /// Wrong account, corruption, duplicates or size overflow.
    pub fn restore_trust(&mut self, record: &[u8]) -> Result<(), JsError> {
        self.0.chat.restore_trust(record).map_err(refused)
    }
}
