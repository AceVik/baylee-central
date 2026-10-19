use super::{Discovery, Error, IdentityService, Request, Response, Token};

impl IdentityService {
    pub(super) async fn dispatch(
        &self,
        request: Request,
        bearer: Option<&Token>,
    ) -> Result<Response, Error> {
        match request {
            Request::Discover { username } => {
                let discovery = if let Some(identity) = self.store.discover(&username).await? {
                    Discovery::Central { identity }
                } else if self.legacy_discovery {
                    Discovery::Legacy
                } else {
                    return Err(Error::Rejected);
                };
                Ok(Response::Discovery { discovery })
            }
            Request::LoginStart { identity, message } => self.start(identity, message).await,
            Request::LoginFinish { attempt, message } => self.finish(&attempt, message).await,
            Request::LegacyReserve {
                legacy_id,
                username,
            } => {
                self.bridge(bearer)?;
                let ticket = Token::generate()?;
                let identity = self
                    .store
                    .reserve_legacy(&legacy_id, &username, &ticket)
                    .await?;
                Ok(Response::Enrollment { identity, ticket })
            }
            Request::EnrollmentStart { ticket, message } => {
                self.enrollment_challenge(&ticket, message).await
            }
            Request::EnrollmentFinish {
                ticket,
                record,
                wrapped_master,
                document,
            } => {
                self.store
                    .enroll(&ticket, &record, &wrapped_master, &document)
                    .await?;
                Ok(Response::Done)
            }
            Request::ReadDocument => {
                let (revision, document) =
                    self.store.document(bearer.ok_or(Error::Rejected)?).await?;
                Ok(Response::Document { revision, document })
            }
            Request::WriteDocument {
                expected_revision,
                document,
            } => {
                self.store
                    .write_document(bearer.ok_or(Error::Rejected)?, expected_revision, &document)
                    .await?;
                Ok(Response::Done)
            }
            Request::ConfirmRestore { revision } => {
                self.store
                    .confirm_restore(bearer.ok_or(Error::Rejected)?, revision)
                    .await?;
                Ok(Response::Done)
            }
            Request::LegacyStatus { legacy_id } => {
                self.bridge(bearer)?;
                let (identity, state) = self.store.legacy_status(&legacy_id).await?;
                Ok(Response::LegacyStatus { identity, state })
            }
            Request::GatewayTicket { gateway } => {
                if gateway != self.gateway {
                    return Err(Error::Rejected);
                }
                let ticket = Token::generate()?;
                self.store
                    .gateway_ticket(bearer.ok_or(Error::Rejected)?, &gateway, &ticket)
                    .await?;
                Ok(Response::GatewayTicket { ticket })
            }
            Request::RedeemGateway { ticket } => {
                self.bridge(bearer)?;
                let (identity, legacy_id) =
                    self.store.redeem_gateway(&ticket, &self.gateway).await?;
                Ok(Response::GatewayIdentity {
                    identity,
                    legacy_id,
                })
            }
            Request::Logout => {
                self.store.logout(bearer.ok_or(Error::Rejected)?).await?;
                Ok(Response::Done)
            }
        }
    }

    async fn enrollment_challenge(
        &self,
        ticket: &Token,
        message: Vec<u8>,
    ) -> Result<Response, Error> {
        let identity = self.store.enrollment(ticket).await?;
        let context = self.context(identity)?;
        let permit = self
            .workers
            .clone()
            .try_acquire_owned()
            .map_err(|_| Error::Busy)?;
        let keys = self.keys.clone();
        let message = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            keys.enrollment_response(context, &message)
                .map_err(|_| Error::Rejected)
        })
        .await
        .map_err(|_| Error::Unavailable)??;
        Ok(Response::EnrollmentChallenge { message })
    }
}
