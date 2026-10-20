use super::{Error, IdentityService, Request, Response, Token};
use central_api::wire::TicketPurpose;

impl IdentityService {
    pub(super) async fn gateway_authority(
        &self,
        bearer: Option<&Token>,
    ) -> Result<(Option<[u8; 16]>, String), Error> {
        if self.bridge(bearer).is_ok() {
            return Ok((None, self.gateway.clone()));
        }
        let (id, origin) = self
            .store
            .authenticate_gateway(bearer.ok_or(Error::Rejected)?)
            .await?;
        Ok((Some(id), origin))
    }

    pub(super) async fn gateway_request(
        &self,
        request: Request,
        bearer: Option<&Token>,
    ) -> Result<Response, Error> {
        match request {
            Request::RegisterGateway { origin, metadata } => {
                let credential = Token::generate()?;
                let heartbeat = Token::generate()?;
                let gateway = self
                    .store
                    .register_gateway(
                        bearer.ok_or(Error::Rejected)?,
                        &origin,
                        &metadata,
                        &credential,
                        &heartbeat,
                        !self.legacy_discovery,
                    )
                    .await?;
                Ok(Response::GatewayRegistered {
                    gateway,
                    credential,
                    heartbeat,
                })
            }
            Request::GatewayDirectory { after } => Ok(Response::GatewayDirectory {
                gateways: self.store.gateway_directory(after).await?,
            }),
            Request::GatewayHeartbeat { metadata } => {
                self.store
                    .gateway_heartbeat(bearer.ok_or(Error::Rejected)?, &metadata)
                    .await?;
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
                let (gateway, origin) = self.gateway_authority(bearer).await?;
                let (identity, expires_at) = self
                    .store
                    .gateway_session(&lease, &origin, gateway, purpose)
                    .await?;
                Ok(Response::GatewaySession {
                    identity,
                    expires_at,
                })
            }
            Request::GatewayTicket { gateway, purpose } => {
                let session = bearer.ok_or(Error::Rejected)?;
                let ticket = Token::generate()?;
                if gateway == self.gateway && !matches!(purpose, TicketPurpose::Admin) {
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
            Request::RedeemGateway { ticket, purpose } => {
                let (gateway, origin) = self.gateway_authority(bearer).await?;
                let lease = matches!(purpose, TicketPurpose::Admit | TicketPurpose::Admin)
                    .then(Token::generate)
                    .transpose()?;
                let (identity, legacy_id, expires_at) = self
                    .store
                    .redeem_lease(&ticket, &origin, purpose, gateway, lease.as_ref())
                    .await?;
                Ok(Response::GatewayIdentity {
                    identity,
                    legacy_id,
                    lease,
                    expires_at,
                })
            }
            _ => Err(Error::Rejected),
        }
    }
}
