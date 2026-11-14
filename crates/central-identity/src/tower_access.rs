//! Narrow identity delegation; the Tower's economy and execution live in Baylee.
use super::{Error, IdentityService, Response, Token};

impl IdentityService {
    /// Configure the sole trusted Tower destination, independent of gateway registration.
    /// Absent configuration disables issuance and introspection.
    /// # Errors
    /// Noncanonical origin or an insecure origin outside explicit local development.
    pub fn with_tower(mut self, origin: Option<&str>) -> Result<Self, Error> {
        if let Some(origin) = origin {
            central_api::gateway::validate_origin(origin, self.development)?;
            if origin == self.origin {
                return Err(Error::Rejected);
            }
        }
        self.tower_origin = origin.map(str::to_owned);
        Ok(self)
    }

    pub(super) async fn tower_token(&self, bearer: Option<&Token>) -> Result<Response, Error> {
        let origin = self.tower_origin.as_deref().ok_or(Error::Unavailable)?;
        let session = bearer.ok_or(Error::Rejected)?;
        let token = Token::generate()?;
        let expires_at = self
            .store
            .issue_tower_access(session, origin, &token)
            .await?;
        Ok(Response::TowerToken {
            grant: central_api::tower_access::Grant {
                origin: origin.into(),
                token,
                expires_at,
            },
        })
    }

    pub(super) async fn tower_identity(&self, bearer: Option<&Token>) -> Result<Response, Error> {
        let origin = self.tower_origin.as_deref().ok_or(Error::Unavailable)?;
        let token = bearer.ok_or(Error::Rejected)?;
        let identity = self
            .store
            .tower_identity(
                token,
                origin,
                self.terms.as_ref().map(|terms| &terms.version),
            )
            .await?;
        Ok(Response::TowerIdentity { identity })
    }
}
