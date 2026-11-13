//! Telling gateways what ended, and the keys that sign it.
//!
//! Live only: the delivery below reaches public HTTPS addresses, and the official gateway's
//! one compiled host at the address the operator set. A development service runs no sender.
use central_api::{OFFICIAL_ORIGIN, SignedToken};
use central_crypto::jwt::{PublicKey, SigningKey};
use central_identity::{GatewayDelivery, GatewayOutcome, IdentityService, TokenKeys};
use central_outbound::{Agent, Limits, Pin, agent};
use serde::Deserialize;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::sync::watch;
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

/// Senders at once. One gateway that hangs for its whole time limit holds up one of them.
const SENDERS: usize = 4;
/// How often a sender looks for work when it found none: the delay of a revocation at worst.
const IDLE: Duration = Duration::from_millis(250);
/// How long a sender rests after the store failed.
const UNAVAILABLE: Duration = Duration::from_secs(2);
/// How often old revocations are forgotten.
const SWEEP: Duration = Duration::from_secs(60);

/// The signing key's seed and the public keys listed beside it, from private configuration.
#[derive(Deserialize, Zeroize, ZeroizeOnDrop)]
#[serde(deny_unknown_fields)]
pub(super) struct TokenKeyConfig {
    /// The 32-byte seed of the key that signs.
    active: Vec<u8>,
    /// Other public keys to list: the next key before it signs, and earlier keys until
    /// what they signed has expired (`docs/operations.md` §"Rotating the signing key").
    #[serde(default)]
    published: Vec<Vec<u8>>,
}

impl TokenKeyConfig {
    /// The keys, or a refusal of the whole configuration.
    pub(super) fn keys(&self) -> Result<TokenKeys, ()> {
        let seed: [u8; 32] = self.active.as_slice().try_into().map_err(|_| ())?;
        let mut others = Vec::with_capacity(self.published.len());
        for public in &self.published {
            let bytes: [u8; 32] = public.as_slice().try_into().map_err(|_| ())?;
            others.push(PublicKey::from_bytes(&bytes).map_err(|_| ())?);
        }
        TokenKeys::new(SigningKey::from_seed(&Zeroizing::new(seed)), others).map_err(|_| ())
    }
}

/// Where the official gateway is reached when it runs beside this service: its own compiled
/// host, at the address the operator set. The call stays verified HTTPS to that name. Nothing
/// else reaches a private address, and a value that cannot be read stops the service rather
/// than silently leaving the official gateway to catch-up.
pub(super) fn official_pin(address: Option<&str>) -> Result<Option<Pin>, ()> {
    let Some(address) = address else {
        return Ok(None);
    };
    let host = OFFICIAL_ORIGIN.strip_prefix("https://").ok_or(())?;
    let address: SocketAddr = address.parse().map_err(|_| ())?;
    Pin::new(host, address).map(Some).map_err(|_| ())
}

/// What a gateway's answer means. Only `204` is taken; anything unexpected, a redirect
/// among them, is not asked again, and catch-up carries the numbers.
const fn classify(status: u16) -> GatewayOutcome {
    match status {
        204 => GatewayOutcome::Taken,
        429 | 500..=599 => GatewayOutcome::Retry,
        _ => GatewayOutcome::Refused,
    }
}

/// The one way Central calls a gateway. It reads the status and no more of the answer.
struct HttpsDelivery(Agent);

impl GatewayDelivery for HttpsDelivery {
    fn deliver(&self, url: &str, message: &SignedToken) -> GatewayOutcome {
        self.0
            .post(url)
            .header("Content-Type", "application/jwt")
            .send(message.expose())
            .map_or(GatewayOutcome::Retry, |response| {
                classify(response.status().as_u16())
            })
    }
}

async fn rest(duration: Duration, stop: &mut watch::Receiver<bool>) {
    tokio::select! {
        _ = stop.wait_for(|value| *value) => {}
        () = tokio::time::sleep(duration) => {}
    }
}

async fn sender(
    identity: Arc<IdentityService>,
    delivery: Arc<HttpsDelivery>,
    mut stop: watch::Receiver<bool>,
) {
    while !*stop.borrow() {
        match identity.push_revocations(delivery.clone()).await {
            Ok(true) => {}
            Ok(false) => rest(IDLE, &mut stop).await,
            Err(_) => {
                eprintln!("central: revocation delivery unavailable");
                rest(UNAVAILABLE, &mut stop).await;
            }
        }
    }
}

async fn sweeper(identity: Arc<IdentityService>, mut stop: watch::Receiver<bool>) {
    while !*stop.borrow() {
        if identity.sweep_revocations().await.is_err() {
            eprintln!("central: revocation sweep unavailable");
        }
        rest(SWEEP, &mut stop).await;
    }
}

/// Run the senders and the sweep until `stop`. A sender finishes the call it is in.
pub(super) async fn run(
    identity: Arc<IdentityService>,
    official: Option<Pin>,
    stop: watch::Receiver<bool>,
) {
    let delivery = Arc::new(HttpsDelivery(agent(Limits {
        port: None,
        total: Duration::from_secs(5),
        connect: Duration::from_secs(2),
        pin: official,
    })));
    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..SENDERS {
        tasks.spawn(sender(identity.clone(), delivery.clone(), stop.clone()));
    }
    tasks.spawn(sweeper(identity, stop));
    while tasks.join_next().await.is_some() {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_204_is_taken_and_only_a_wait_is_retried() {
        assert_eq!(classify(204), GatewayOutcome::Taken);
        for status in [429, 500, 502, 503, 599] {
            assert_eq!(classify(status), GatewayOutcome::Retry, "{status}");
        }
        for status in [200, 201, 301, 302, 307, 308, 400, 401, 403, 404, 426] {
            assert_eq!(classify(status), GatewayOutcome::Refused, "{status}");
        }
    }

    fn config(text: &str) -> Result<TokenKeyConfig, serde_json::Error> {
        serde_json::from_str(text)
    }

    #[test]
    fn the_official_address_is_optional_numeric_and_local() {
        assert!(official_pin(None).unwrap().is_none());
        assert!(official_pin(Some("127.0.0.1:443")).unwrap().is_some());
        for refused in [
            "gateway.baylee.app:443",
            "localhost:443",
            "127.0.0.1",
            "8.8.8.8:443",
            "169.254.169.254:80",
            "",
        ] {
            assert!(official_pin(Some(refused)).is_err(), "{refused}");
        }
    }

    #[test]
    fn a_key_configuration_is_refused_for_any_departure() {
        let seed = format!("{:?}", [7_u8; 32]);
        assert!(
            config(&format!(r#"{{"active":{seed}}}"#))
                .unwrap()
                .keys()
                .is_ok()
        );
        for text in [
            r#"{"active":[1,2,3]}"#.to_owned(),
            r#"{"active":[]}"#.to_owned(),
            format!(r#"{{"active":{seed},"published":[[1,2,3]]}}"#),
        ] {
            assert!(config(&text).unwrap().keys().is_err(), "{text}");
        }
        assert!(config(&format!(r#"{{"active":{seed},"extra":1}}"#)).is_err());
    }
}
