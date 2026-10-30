//! An account's own sessions and gateways on the website (`docs/web-accounts.md`
//! §"The account area"): on the browser listener only, with the cookie's
//! session, checked again by the store in the transaction that acts. Minting
//! a token needs a password sign-in less than 30 minutes old.
use super::{Error, IdentityService, Response, Token};
use central_api::owner::{OwnerAnswer, OwnerRequest};

impl IdentityService {
    pub(super) async fn owner_request(
        &self,
        request: OwnerRequest,
        cookie: &Token,
    ) -> Result<Response, Error> {
        let answer = match request {
            OwnerRequest::Sessions => OwnerAnswer::Sessions {
                sessions: self.store.owner_sessions(cookie).await?,
            },
            OwnerRequest::EndClientSession => {
                self.store.end_client_session(cookie).await?;
                return Ok(Response::Done);
            }
            OwnerRequest::Gateways => OwnerAnswer::Gateways {
                gateways: self.store.owner_gateways(cookie).await?,
            },
            OwnerRequest::AddGateway { origin, name } => {
                // the official gateway's origin is the bridge's alone
                if origin == self.gateway {
                    return Err(Error::Conflict);
                }
                OwnerAnswer::GatewayAdded {
                    gateway: self
                        .store
                        .add_gateway(cookie, &origin, &name, self.development)
                        .await?,
                }
            }
            OwnerRequest::CreateToken {
                gateway,
                name,
                scopes,
                days,
            } => {
                let token = Token::generate()?;
                let (id, expires_at) = self
                    .store
                    .create_gateway_token(cookie, gateway, &name, scopes, days, &token)
                    .await?;
                OwnerAnswer::TokenCreated {
                    id,
                    token,
                    expires_at,
                }
            }
            OwnerRequest::RevokeToken { token } => {
                self.store.revoke_own_gateway_token(cookie, token).await?;
                return Ok(Response::Done);
            }
        };
        Ok(Response::Owner { answer })
    }
}
