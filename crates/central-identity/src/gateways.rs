use super::{Error, IdentityService, Request, Response, Token};
use central_api::gateway::GatewayMetadata;

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
    /// other credential is a registered gateway's heartbeat token. The answer names the
    /// gateway's last revocation, so that it can tell whether it missed one.
    async fn heartbeat(
        &self,
        metadata: &GatewayMetadata,
        bearer: Option<&Token>,
    ) -> Result<Response, Error> {
        let stream = if self.bridge(bearer).is_ok() {
            self.store
                .official_heartbeat(&self.official, metadata, self.development)
                .await?;
            self.official.first().ok_or(Error::Rejected)?.clone()
        } else {
            self.store
                .gateway_heartbeat(bearer.ok_or(Error::Rejected)?, metadata)
                .await?
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
            Request::GatewayHeartbeat { metadata } => self.heartbeat(&metadata, bearer).await,
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
