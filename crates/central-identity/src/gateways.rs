use super::{Error, IdentityService, Request, Response, Token};
use central_api::{gateway::GatewayMetadata, wire::TicketPurpose};

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
    /// other credential is a registered gateway's heartbeat token.
    async fn heartbeat(
        &self,
        metadata: &GatewayMetadata,
        bearer: Option<&Token>,
    ) -> Result<(), Error> {
        if self.bridge(bearer).is_ok() {
            return self
                .store
                .official_heartbeat(&self.official, metadata, self.development)
                .await;
        }
        self.store
            .gateway_heartbeat(bearer.ok_or(Error::Rejected)?, metadata)
            .await
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
            Request::GatewayHeartbeat { metadata } => {
                self.heartbeat(&metadata, bearer).await?;
                Ok(Response::Done)
            }
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
            Request::CheckGatewayAdmin { account } => {
                let (gateway, _) = self
                    .store
                    .authenticate_gateway(bearer.ok_or(Error::Rejected)?)
                    .await?;
                Ok(Response::GatewayAdmin {
                    allowed: self.store.gateway_admin(gateway, account).await?,
                })
            }
            Request::GatewaySession { lease, purpose } => {
                let (gateway, origins) = self.gateway_authority(bearer).await?;
                let (identity, expires_at) = self
                    .store
                    .gateway_session(&lease, &origins, gateway, purpose)
                    .await?;
                Ok(Response::GatewaySession {
                    identity,
                    expires_at,
                })
            }
            Request::GatewayProfile { lease, purpose } => {
                let (gateway, origins) = self.gateway_authority(bearer).await?;
                let profile = self
                    .store
                    .gateway_profile(&lease, &origins, gateway, purpose)
                    .await?;
                Ok(Response::GatewayProfile {
                    identity: profile.identity,
                    display_name: profile.display_name,
                    tag: profile.tag,
                })
            }
            Request::GatewayTicket { gateway, purpose } => {
                let session = bearer.ok_or(Error::Rejected)?;
                let ticket = Token::generate()?;
                // a ticket for any of the official origins is the bridge's,
                // bound to the origin the client asked for
                if self.is_official(&gateway) && !matches!(purpose, TicketPurpose::Admin) {
                    self.store
                        .gateway_ticket(session, &gateway, &ticket, purpose)
                        .await?;
                } else {
                    self.store
                        .registered_ticket(session, &gateway, &ticket, purpose)
                        .await?;
                }
                Ok(Response::GatewayTicket { ticket })
            }
            Request::GatewayStatus => self.gateway_status(bearer).await,
            Request::RedeemGateway { ticket, purpose } => {
                self.redeem(&ticket, purpose, bearer).await
            }
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

    /// A gateway redeems its player's ticket for an identity, and a lease for
    /// admission and administration.
    async fn redeem(
        &self,
        ticket: &Token,
        purpose: TicketPurpose,
        bearer: Option<&Token>,
    ) -> Result<Response, Error> {
        let (gateway, origins) = self.gateway_authority(bearer).await?;
        // a sign-in is one of the uses a token's owner sees; a
        // credential pair or the bridge is no token and is left alone
        if gateway.is_some()
            && let Some(bearer) = bearer
        {
            self.store.touch_gateway_token(bearer).await?;
        }
        let lease = matches!(purpose, TicketPurpose::Admit | TicketPurpose::Admin)
            .then(Token::generate)
            .transpose()?;
        let (identity, legacy_id, expires_at) = self
            .store
            .redeem_lease(ticket, &origins, purpose, gateway, lease.as_ref())
            .await?;
        Ok(Response::GatewayIdentity {
            identity,
            legacy_id,
            lease,
            expires_at,
        })
    }
}
