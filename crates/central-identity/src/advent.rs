//! Advent operations distinguish native account sessions from table hosts.
use super::{Error, IdentityService, Response, Surface, Token};
use central_api::advent as api;

impl IdentityService {
    pub(super) async fn advent(
        &self,
        request: api::Request,
        bearer: Option<&Token>,
        surface: Surface,
    ) -> Result<Response, Error> {
        if surface != Surface::Native {
            return Err(Error::Rejected);
        }
        let session = surface.session(bearer)?;
        let response = match request {
            api::Request::Table { request: table } if self.is_official(table.origin()) => {
                self.store
                    .advent_official_table(session, table, &self.official)
                    .await?
            }
            request => self.store.advent(session, request).await?,
        };
        Ok(Response::Advent { response })
    }
    pub(super) async fn advent_host(
        &self,
        request: api::table::HostRequest,
        bearer: Option<&Token>,
    ) -> Result<Response, Error> {
        let actor = if self.bridge(bearer).is_ok() {
            central_postgres::RankedHostCredential::Official(
                self.official.first().ok_or(Error::Rejected)?,
            )
        } else {
            central_postgres::RankedHostCredential::Registered(bearer.ok_or(Error::Rejected)?)
        };
        Ok(Response::AdventHost {
            response: self.store.advent_table_host(actor, request).await?,
        })
    }
}
