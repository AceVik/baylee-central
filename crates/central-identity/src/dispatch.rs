use super::{Discovery, Error, IdentityService, Request, Response, Token};

impl IdentityService {
    pub(super) async fn dispatch(
        &self,
        request: Request,
        bearer: Option<&Token>,
    ) -> Result<Response, Error> {
        match request {
            request @ (Request::CreateObject
            | Request::ReadObject { .. }
            | Request::ListObjects { .. }
            | Request::WriteObject { .. }
            | Request::DeleteObject { .. }) => self.object_request(request, bearer).await,
            Request::SessionStatus => {
                let (identity, expires_at, migration_closed) = self
                    .store
                    .session_status(bearer.ok_or(Error::Rejected)?)
                    .await?;
                Ok(Response::SessionStatus {
                    identity,
                    expires_at,
                    migration_closed,
                })
            }
            request @ (Request::LegacyInvalidate { .. } | Request::LegacyClose { .. }) => {
                self.legacy_mutation(request, bearer).await
            }
            request @ (Request::RegisterGateway { .. }
            | Request::GatewayDirectory { .. }
            | Request::GatewayHeartbeat { .. }
            | Request::SetGatewayAdmins { .. }
            | Request::CheckGatewayAdmin { .. }
            | Request::GatewaySession { .. }
            | Request::GatewayTicket { .. }
            | Request::RedeemGateway { .. }) => self.gateway_request(request, bearer).await,
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
            Request::Logout => {
                self.store.logout(bearer.ok_or(Error::Rejected)?).await?;
                Ok(Response::Done)
            }
        }
    }

    async fn legacy_mutation(
        &self,
        request: Request,
        bearer: Option<&Token>,
    ) -> Result<Response, Error> {
        self.bridge(bearer)?;
        match request {
            Request::LegacyInvalidate { legacy_id } => {
                self.store.invalidate_legacy(&legacy_id).await?;
            }
            Request::LegacyClose {
                legacy_id,
                identity,
            } => self.store.close_legacy(&legacy_id, identity).await?,
            _ => return Err(Error::Rejected),
        }
        Ok(Response::Done)
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
