//! Challenge authority stays at Central; only the configured official bridge publishes/results.
use super::{Error, IdentityService, Response, Surface, Token};
use central_api::challenges::{HostRequest, Request};

impl IdentityService {
    pub(super) async fn challenges(
        &self,
        request: Request,
        bearer: Option<&Token>,
        surface: Surface,
    ) -> Result<Response, Error> {
        let session = bearer.map(|_| surface.session(bearer)).transpose()?;
        Ok(Response::Challenges {
            response: self
                .store
                .challenges(session, request, self.canonical())
                .await?,
        })
    }

    pub(super) async fn challenge_host(
        &self,
        request: HostRequest,
        bearer: Option<&Token>,
    ) -> Result<Response, Error> {
        self.bridge(bearer)?;
        Ok(Response::ChallengeHost {
            response: self
                .store
                .official_challenges(request, self.canonical())
                .await?,
        })
    }
}
