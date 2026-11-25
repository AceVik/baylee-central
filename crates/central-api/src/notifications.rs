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

#[cfg(test)]
mod tests {
    use super::*;

    type Change = (&'static str, fn(&mut Subscription));

    fn subscription() -> Subscription {
        Subscription {
            endpoint: "https://web.push.apple.com/synthetic".into(),
            p256dh: "B".repeat(87),
            auth: "-_".repeat(11),
            lang: "de".into(),
            nonce: "0123456789abcdef".repeat(2),
        }
    }

    #[test]
    fn every_subscription_field_is_checked_on_its_own() {
        assert!(subscription().validate().is_ok());
        let refused: [Change; 16] = [
            ("endpoint", |s| s.endpoint = "https://127.0.0.1/x".into()),
            ("short key", |s| s.p256dh.truncate(86)),
            ("long key", |s| s.p256dh.push('A')),
            ("padded key", |s| s.p256dh.replace_range(86.., "=")),
            ("standard alphabet", |s| s.p256dh.replace_range(..1, "+")),
            ("slash", |s| s.p256dh.replace_range(..1, "/")),
            ("key with a multibyte letter", |s| {
                s.p256dh.replace_range(..2, "ä");
            }),
            ("short secret", |s| s.auth.truncate(21)),
            ("long secret", |s| s.auth.push('A')),
            ("secret with a break", |s| s.auth.replace_range(..1, "\n")),
            ("unknown language", |s| s.lang = "fr".into()),
            ("cased language", |s| s.lang = "EN".into()),
            ("no language", |s| s.lang.clear()),
            ("short nonce", |s| s.nonce.truncate(31)),
            ("long nonce", |s| s.nonce.push('0')),
            ("uppercase nonce", |s| s.nonce.replace_range(..1, "A")),
        ];
        for (why, change) in refused {
            let mut subscription = subscription();
            change(&mut subscription);
            assert!(subscription.validate().is_err(), "{why}");
        }
        let mut subscription = subscription();
        subscription.nonce.replace_range(31.., "g");
        assert!(subscription.validate().is_err());
    }

    #[test]
    fn the_endpoint_length_limit_is_exact() {
        let of = |length: usize| {
            let base = "https://fcm.googleapis.com/";
            format!("{base}{}", "a".repeat(length - base.len()))
        };
        assert!(endpoint(&of(2048)).is_ok());
        assert!(endpoint(&of(2049)).is_err());
    }

    #[test]
    fn an_endpoint_reads_as_it_is_fetched() {
        for value in [
            // Every one of these parses; some to an allowed host.
            "https://fcm.googleapis.com/a/../b",
            "https://fcm.googleapis.com/a/%2e%2e/b",
            "https://fcm.googleapis.com\\@evil.test/token",
            "https:fcm.googleapis.com/token",
            "https:///fcm.googleapis.com/token",
            "HTTPS://fcm.googleapis.com/token",
            "https://fcm.googleapis.com:443/token",
            "https://fcm.googleapis.com./token",
            "https://fcm.googleapis.com%2eevil.test/token",
            "https://fcm.googleapis.com/to\tken",
            "https://fcm.googleapis.com/token\n",
            " https://fcm.googleapis.com/token",
            "https://fcm.googleapis.com/to ken",
            "https://fcm.googleapis.com/token?",
            "https://fcm.googleapis.com/token#",
            "https://:secret@fcm.googleapis.com/token",
            "https://fcm.googleapis.com/tökén",
            "https://ｆcm.googleapis.com/token",
            "https://sub.web.push.apple.com/token",
            "https://push.apple.com/token",
            "file:///fcm.googleapis.com/token",
            "",
        ] {
            assert!(endpoint(value).is_err(), "{value:?}");
        }
        let accepted = "https://fcm.googleapis.com/fcm/send/abc:DEF-_1";
        assert_eq!(endpoint(accepted).unwrap().as_str(), accepted);
    }

    #[test]
    fn categories_never_share_a_preference_bit() {
        let all = [
            Category::Chat,
            Category::Contacts,
            Category::Updates,
            Category::Feedback,
            Category::Gateways,
        ];
        let mask = all.iter().fold(0, |mask, category| {
            assert_eq!(category.bit().count_ones(), 1);
            assert_eq!(mask & category.bit(), 0, "{category:?}");
            mask | category.bit()
        });
        // `NotificationState::enabled` carries the whole mask in one octet.
        assert!(u8::try_from(mask).is_ok());
    }

    #[test]
    fn a_subscription_or_command_with_an_extra_field_is_refused() {
        let body = serde_json::to_value(subscription()).unwrap();
        assert!(serde_json::from_value::<Subscription>(body.clone()).is_ok());
        for field in ["endpoint", "p256dh", "auth", "lang", "nonce"] {
            let mut missing = body.clone();
            missing.as_object_mut().unwrap().remove(field);
            assert!(
                serde_json::from_value::<Subscription>(missing).is_err(),
                "{field}"
            );
        }
        let mut extra = body;
        extra["webhook"] = "https://evil.test/".into();
        assert!(serde_json::from_value::<Subscription>(extra).is_err());
        for command in [
            r#"{"command":"set","category":"chat","enabled":true,"account":1}"#,
            r#"{"command":"set","category":"everything","enabled":true}"#,
            r#"{"command":"set","category":"chat","enabled":1}"#,
            r#"{"command":"send","endpoint":"https://fcm.googleapis.com/x"}"#,
            r#"{"command":"unsubscribe"}"#,
        ] {
            assert!(
                crate::decode::<NotificationRequest>(command.as_bytes()).is_err(),
                "{command}"
            );
        }
    }
}
