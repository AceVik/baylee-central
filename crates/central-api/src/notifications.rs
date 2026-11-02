//! Browser-owned Web Push subscriptions and independently selectable categories.
use crate::Error;
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Fixed categories; private content never appears in a push payload.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    /// New encrypted chat delivery.
    Chat,
    /// A new incoming contact request.
    Contacts,
    /// A newly deployed version.
    Updates,
    /// A newly filed feedback report; administrators only.
    Feedback,
    /// A gateway awaiting directory approval; administrators only.
    Gateways,
}
impl Category {
    /// The stable bit in a persisted preference mask.
    #[must_use]
    pub const fn bit(self) -> i32 {
        match self {
            Self::Chat => 1,
            Self::Contacts => 2,
            Self::Updates => 4,
            Self::Feedback => 8,
            Self::Gateways => 16,
        }
    }
    /// Whether current administrator authority is required.
    #[must_use]
    pub const fn admin(self) -> bool {
        matches!(self, Self::Feedback | Self::Gateways)
    }
}

/// A browser-generated subscription. Its URL and authentication material are secrets.
#[derive(Clone, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
#[serde(deny_unknown_fields)]
pub struct Subscription {
    /// HTTPS capability URL, restricted to supported browser push services.
    pub endpoint: String,
    /// Uncompressed SEC1 P-256 key, unpadded base64url.
    pub p256dh: String,
    /// Sixteen-byte authentication secret, unpadded base64url.
    pub auth: String,
    /// Language of generic notification text.
    pub lang: String,
    /// Random installation generation; prevents delayed pushes crossing account switches.
    pub nonce: String,
}
impl Subscription {
    /// Validate bounded, canonical wire fields before storage or networking.
    /// # Errors
    /// Rejects unsupported endpoints, malformed keys and unsupported languages.
    pub fn validate(&self) -> Result<(), Error> {
        endpoint(&self.endpoint)?;
        let b64 = |s: &str, n| {
            s.len() == n
                && s.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        };
        if !b64(&self.p256dh, 87)
            || !b64(&self.auth, 22)
            || !matches!(self.lang.as_str(), "en" | "de")
            || self.nonce.len() != 32
            || !self
                .nonce
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(Error::Rejected);
        }
        Ok(())
    }
}
/// Validate the exact allowed HTTPS hosts. No user-supplied arbitrary HTTP destination.
/// # Errors
/// Rejects credentials, ports, fragments, queries, non-ASCII and unsupported providers.
pub fn endpoint(value: &str) -> Result<url::Url, Error> {
    if value.len() > 2048 || !value.is_ascii() {
        return Err(Error::Rejected);
    }
    let u = url::Url::parse(value).map_err(|_| Error::Rejected)?;
    let allowed = matches!(
        u.host_str(),
        Some("fcm.googleapis.com" | "updates.push.services.mozilla.com" | "web.push.apple.com")
    );
    if !allowed
        || u.scheme() != "https"
        || !u.username().is_empty()
        || u.password().is_some()
        || u.port().is_some()
        || u.query().is_some()
        || u.fragment().is_some()
        || u.as_str() != value
        || u.path().len() < 2
    {
        return Err(Error::Rejected);
    }
    Ok(u)
}
/// Cookie-authenticated notification management; never exposed to gateway credentials.
#[derive(Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
pub enum NotificationRequest {
    /// Read account preferences and optionally check this installation's subscription.
    Read {
        /// This browser's current endpoint, if it has one.
        endpoint: Option<String>,
    },
    /// Change one category without overwriting another tab's choices.
    Set {
        /// Category to change.
        category: Category,
        /// Requested state.
        enabled: bool,
    },
    /// Bind a browser subscription to the authenticated account.
    Subscribe {
        /// Browser-generated subscription, never an arbitrary webhook.
        subscription: Subscription,
    },
    /// Remove only this account's subscription, idempotently.
    Unsubscribe {
        /// This installation's endpoint.
        endpoint: String,
    },
}
/// Current preferences. A null public key means push delivery is not configured.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NotificationState {
    /// Bit mask of enabled categories, filtered by current authority.
    pub enabled: u8,
    /// Public VAPID key; never private signing material.
    pub public_key: Option<String>,
    /// Whether the supplied endpoint belongs to this account's current credential.
    pub subscribed: bool,
}
