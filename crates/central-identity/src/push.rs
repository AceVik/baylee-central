//! Sending a gateway what ended. Catch-up is what makes a revocation certain; this makes it
//! prompt. So a failure here loses nothing: the numbers stay in the log, and the gateway
//! reads them at its next heartbeat.
use super::{Error, IdentityService};
use central_api::{SignedToken, gateway::valid_invalidate_path};
use central_postgres::GatewayPushResult;

/// What became of one call to a gateway.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GatewayOutcome {
    /// `204`: taken, or taken before.
    Taken,
    /// Refused or answered with anything unexpected: asking again changes nothing.
    Refused,
    /// No answer, `429` or `5xx`: worth another bounded attempt.
    Retry,
}

/// One call to a gateway. The live implementation holds to verified HTTPS and public addresses,
/// with one operator-set exception for the official gateway's own host (`central-outbound`).
pub trait GatewayDelivery: Send + Sync {
    /// `POST` the message to this URL, built from the approved origin and registered path.
    fn deliver(&self, url: &str, message: &SignedToken) -> GatewayOutcome;
}

impl IdentityService {
    /// Send one due gateway its next page, if any is due. Returns whether one was, so that a
    /// caller can go on at once. The call blocks: run it off the async workers.
    ///
    /// # Errors
    /// Reports storage failure. A failed call is not an error: it counts as an attempt.
    pub async fn push_revocations<D: GatewayDelivery + 'static>(
        &self,
        delivery: std::sync::Arc<D>,
    ) -> Result<bool, Error> {
        let Some(push) = self.store.claim_gateway_push().await? else {
            return Ok(false);
        };
        // Numbers that aged out need no call; they are past every token they could name.
        if push.sids.is_empty() {
            self.store
                .finish_gateway_push(&push.origin, push.through, GatewayPushResult::Delivered)
                .await?;
            return Ok(true);
        }
        let result = if valid_invalidate_path(&push.path) {
            let message = self.revocation_message(&push.origin, push.from, &push.sids)?;
            let url = format!("{}{}", push.origin, push.path);
            let delivered =
                tokio::task::spawn_blocking(move || delivery.deliver(&url, &message)).await;
            match delivered {
                Ok(GatewayOutcome::Taken) => GatewayPushResult::Delivered,
                Ok(GatewayOutcome::Refused) => GatewayPushResult::Refused,
                Ok(GatewayOutcome::Retry) | Err(_) => GatewayPushResult::Retry,
            }
        } else {
            // The table's check holds this too; a path outside the rule is never called.
            GatewayPushResult::Refused
        };
        self.store
            .finish_gateway_push(&push.origin, push.through, result)
            .await?;
        Ok(true)
    }

    /// Forget revocations past their age. Returns how many went.
    ///
    /// # Errors
    /// Reports storage failure.
    pub async fn sweep_revocations(&self) -> Result<u64, Error> {
        self.store.sweep_revocations().await
    }
}
