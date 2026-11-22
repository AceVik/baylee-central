//! Public ladders, session-owned matchmaking and verified host operations.
use super::{Error, IdentityService, Response, Surface, Token};
use central_api::ranked::{HostRequest, Request};

impl IdentityService {
    pub(super) async fn ranked(
        &self,
        request: Request,
        bearer: Option<&Token>,
        surface: Surface,
    ) -> Result<Response, Error> {
        let session = if request.is_public() {
            None
        } else {
            Some(surface.session(bearer)?)
        };
        Ok(Response::Ranked {
            response: self.store.ranked(session, request).await?,
        })
    }

    pub(super) async fn ranked_host(
        &self,
        request: HostRequest,
        bearer: Option<&Token>,
    ) -> Result<Response, Error> {
        let (gateway, origins) = self.gateway_authority(bearer).await?;
        let origin = origins.first().ok_or(Error::Rejected)?;
        Ok(Response::RankedHost {
            response: self.store.ranked_host(gateway, origin, request).await?,
        })
    }
}
