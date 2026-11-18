//! Permanent facts cross the boundary; Tower gameplay never enters Central.
use super::{Error, IdentityService, Response, Surface, Token};
use central_api::tower_awards::Delivery;

impl IdentityService {
    pub(super) async fn tower_award(
        &self,
        delivery: &Delivery,
        bearer: Option<&Token>,
    ) -> Result<Response, Error> {
        let origin = self.tower_origin.as_deref().ok_or(Error::Unavailable)?;
        let credential = bearer.ok_or(Error::Rejected)?;
        Ok(Response::TowerAward {
            receipt: self
                .store
                .accept_tower_award(credential, origin, delivery)
                .await?,
        })
    }

    pub(super) async fn tower_progress(
        &self,
        bearer: Option<&Token>,
        surface: Surface,
    ) -> Result<Response, Error> {
        Ok(Response::TowerProgress {
            progress: self
                .store
                .tower_award_progress(surface.session(bearer)?)
                .await?,
        })
    }

    pub(super) async fn tower_award_export(
        &self,
        after: Option<[u8; 16]>,
        bearer: Option<&Token>,
        surface: Surface,
    ) -> Result<Response, Error> {
        Ok(Response::TowerAwardExport {
            export: self
                .store
                .export_tower_awards(surface.session(bearer)?, after)
                .await?,
        })
    }
}
