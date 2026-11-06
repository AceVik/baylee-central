use super::{Error, IdentityService, Request, Response, Token};
use central_api::gateway::{GatewayActivity, GatewayMetadata};
use std::time::{Duration, Instant};

/// How long the public figures' number of accounts is kept before it is
/// counted again: a minute, the pace at which the website reads it.
pub(super) const FIGURES_SECONDS: u64 = 60;

/// The number of accounts as last counted, and when.
#[derive(Default)]
pub(super) struct Counted(Option<(Instant, u64)>);

impl Counted {
    /// The kept number while it is younger than [`FIGURES_SECONDS`]. Once
    /// it is not, one caller is told to count (`None`) and the time moves
    /// on, so the others answer the earlier number until that count is
    /// stored: one count a window however many ask. A count that fails
    /// leaves the earlier number for one more window. Before the first
    /// count there is no number to answer, and every caller counts; the
    /// public budget bounds them.
    fn fresh(&mut self, now: Instant) -> Option<u64> {
        match self.0 {
            Some((at, accounts))
                if now.saturating_duration_since(at) < Duration::from_secs(FIGURES_SECONDS) =>
            {
                Some(accounts)
            }
            Some((_, accounts)) => {
                self.0 = Some((now, accounts));
                None
            }
            None => None,
        }
    }

    fn store(&mut self, now: Instant, accounts: u64) {
        self.0 = Some((now, accounts));
    }
}

impl IdentityService {
    /// The gateway behind `bearer` and the origins it answers for, canonical
    /// first: the official gateway's set for the bridge, a registered
    /// gateway's one origin otherwise.
    pub(super) async fn gateway_authority(
        &self,
        bearer: Option<&Token>,
    ) -> Result<(Option<[u8; 16]>, Vec<String>), Error> {
        if self.bridge(bearer).is_ok() {
            return Ok((None, self.official.clone()));
        }
        let (id, origin) = self
            .store
            .authenticate_gateway(bearer.ok_or(Error::Rejected)?)
            .await?;
        Ok((Some(id), vec![origin]))
    }

    /// Reserve an origin for the session's account. The bridge's origins are
    /// the official gateway's: no registered credential may redeem their
    /// tickets (docs/gateway-registry.md), so each is a conflict.
    async fn register_gateway(
        &self,
        origin: &str,
        metadata: &GatewayMetadata,
        bearer: Option<&Token>,
    ) -> Result<Response, Error> {
        if self.is_official(origin) {
            return Err(Error::Conflict);
        }
        let credential = Token::generate()?;
        let heartbeat = Token::generate()?;
        let gateway = self
            .store
            .register_gateway(
                bearer.ok_or(Error::Rejected)?,
                origin,
                metadata,
                &credential,
                &heartbeat,
                self.development,
            )
            .await?;
        Ok(Response::GatewayRegistered {
            gateway,
            credential,
            heartbeat,
        })
    }

    /// The bridge beats for the official gateway's row and no other; every
    /// other credential is a registered gateway's heartbeat token. A pulse
    /// is the same beat with the gateway's `activity`
    /// ([`Request::GatewayPulse`]): one path, so it has the heartbeat's
    /// credential, checks, limit and answer. The answer names the gateway's
    /// last revocation, so that it can tell whether it missed one.
    async fn heartbeat(
        &self,
        metadata: &GatewayMetadata,
        activity: Option<GatewayActivity>,
        bearer: Option<&Token>,
    ) -> Result<Response, Error> {
        let bridge = self.bridge(bearer).is_ok();
        let stream = match (bridge, activity) {
            (true, None) => {
                self.store
                    .official_heartbeat(&self.official, metadata, self.development)
                    .await?;
                self.official.first().ok_or(Error::Rejected)?.clone()
            }
            (true, Some(activity)) => {
                self.store
                    .official_pulse(&self.official, metadata, self.development, activity)
                    .await?;
                self.official.first().ok_or(Error::Rejected)?.clone()
            }
            (false, None) => {
                self.store
                    .gateway_heartbeat(bearer.ok_or(Error::Rejected)?, metadata)
                    .await?
            }
            (false, Some(activity)) => {
                self.store
                    .gateway_pulse(bearer.ok_or(Error::Rejected)?, metadata, activity)
                    .await?
            }
        };
        Ok(Response::GatewayHeartbeat {
            last_revocation: self.store.last_revocation(&stream).await?,
        })
    }

    /// A page of what this gateway must learn, named by its own credential: the stream is
    /// never chosen by the request.
    async fn revocations(&self, after: u64, bearer: Option<&Token>) -> Result<Response, Error> {
        let (_, origins) = self.gateway_authority(bearer).await?;
        let stream = origins.first().ok_or(Error::Rejected)?;
        let page = self.store.revocation_page(stream, after).await?;
        let token = if page.sids.is_empty() {
            None
        } else {
            Some(self.revocation_message(stream, page.from, &page.sids)?)
        };
        Ok(Response::GatewayRevocations {
            token,
            last: page.last,
            floor: page.floor,
        })
    }

    /// Central's public figures ([`Request::PublicFigures`]): the number of
    /// accounts, counted once in [`FIGURES_SECONDS`] however many ask
    /// ([`Counted`]), and one page of what online gateways report, read each
    /// time.
    async fn public_figures(&self, after: Option<[u8; 16]>) -> Result<Response, Error> {
        let gateways = self.store.gateway_figures(after).await?;
        let now = Instant::now();
        let kept = self
            .accounts
            .lock()
            .map_err(|_| Error::Unavailable)?
            .fresh(now);
        let accounts = if let Some(accounts) = kept {
            accounts
        } else {
            // The lock is not held across the count.
            let accounts = self.store.account_count().await?;
            self.accounts
                .lock()
                .map_err(|_| Error::Unavailable)?
                .store(now, accounts);
            accounts
        };
        Ok(Response::PublicFigures { accounts, gateways })
    }

    pub(super) async fn gateway_request(
        &self,
        request: Request,
        bearer: Option<&Token>,
    ) -> Result<Response, Error> {
        match request {
            Request::RegisterGateway { origin, metadata } => {
                self.register_gateway(&origin, &metadata, bearer).await
            }
            Request::GatewayDirectory { after } => Ok(Response::GatewayDirectory {
                gateways: self.store.gateway_directory(after).await?,
            }),
            Request::GatewayHeartbeat { metadata } => self.heartbeat(&metadata, None, bearer).await,
            Request::GatewayPulse { metadata, activity } => {
                self.heartbeat(&metadata, Some(activity), bearer).await
            }
            Request::PublicFigures { after } => self.public_figures(after).await,
            Request::SetGatewayAdmins {
                gateway,
                expected_revision,
                accounts,
            } => {
                let revision = self
                    .store
                    .set_gateway_admins(
                        bearer.ok_or(Error::Rejected)?,
                        gateway,
                        expected_revision,
                        &accounts,
                    )
                    .await?;
                Ok(Response::GatewayAdmins { revision })
            }
            Request::GatewayToken { gateway, purpose } => {
                self.gateway_token(&gateway, purpose, bearer).await
            }
            Request::RegisterInvalidate { path } => {
                let (_, origins) = self.gateway_authority(bearer).await?;
                let stream = origins.first().ok_or(Error::Rejected)?;
                self.store.register_invalidate(stream, &path).await?;
                Ok(Response::Done)
            }
            Request::GatewayRevocations { after } => self.revocations(after, bearer).await,
            Request::GatewayStatus => self.gateway_status(bearer).await,
            _ => Err(Error::Rejected),
        }
    }

    /// What Central knows about the gateway whose token this is
    /// (`docs/gateway-registry.md` §"Gateway tokens").
    async fn gateway_status(&self, bearer: Option<&Token>) -> Result<Response, Error> {
        let (gateway, origin, metadata, state, scopes, expires_at) = self
            .store
            .gateway_status(bearer.ok_or(Error::Rejected)?)
            .await?;
        Ok(Response::GatewayStatus {
            gateway,
            origin,
            metadata,
            state,
            scopes,
            expires_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{Counted, FIGURES_SECONDS};
    use std::time::{Duration, Instant};

    #[test]
    fn the_accounts_are_counted_once_a_minute_however_many_ask() {
        let start = Instant::now();
        let at = |seconds| start + Duration::from_secs(seconds);
        let mut counted = Counted::default();
        // Before the first count there is no number to answer.
        assert_eq!(counted.fresh(start), None);
        assert_eq!(counted.fresh(start), None);
        counted.store(start, 7);
        // Kept for the minute, whatever the table does meanwhile.
        for seconds in [0, 1, FIGURES_SECONDS - 1] {
            assert_eq!(counted.fresh(at(seconds)), Some(7));
        }
        // Then one caller counts and the others answer the earlier number,
        // for a whole minute more when that count never arrives.
        assert_eq!(counted.fresh(at(FIGURES_SECONDS)), None);
        assert_eq!(counted.fresh(at(FIGURES_SECONDS)), Some(7));
        assert_eq!(counted.fresh(at(2 * FIGURES_SECONDS - 1)), Some(7));
        assert_eq!(counted.fresh(at(2 * FIGURES_SECONDS)), None);
        counted.store(at(2 * FIGURES_SECONDS), 9);
        assert_eq!(counted.fresh(at(2 * FIGURES_SECONDS + 1)), Some(9));
        assert_eq!(counted.fresh(at(3 * FIGURES_SECONDS - 1)), Some(9));
        // A caller whose clock reading is the older one is not told to count.
        assert_eq!(counted.fresh(start), Some(9));
    }
}
