use super::{Error, IdentityService, Request, Response, Token};
use central_api::object::ObjectState;

impl IdentityService {
    pub(super) async fn object_request(
        &self,
        request: Request,
        bearer: Option<&Token>,
    ) -> Result<Response, Error> {
        let session = bearer.ok_or(Error::Rejected)?;
        match request {
            Request::CreateObject => Ok(Response::ObjectState {
                state: self.store.create_object(session).await?,
            }),
            Request::ReadObject { id } => {
                let (state, document) = self.store.read_object(session, id).await?;
                Ok(Response::Object { state, document })
            }
            Request::ListObjects { after } => Ok(Response::Objects {
                objects: self.store.list_objects(session, after).await?,
            }),
            Request::WriteObject {
                id,
                expected_revision,
                document,
            } => {
                let revision = self
                    .store
                    .write_object(session, id, expected_revision, &document)
                    .await?;
                Ok(Response::ObjectState {
                    state: ObjectState {
                        id,
                        revision,
                        deleted: false,
                    },
                })
            }
            Request::DeleteObject {
                id,
                expected_revision,
            } => {
                let revision = self
                    .store
                    .delete_object(session, id, expected_revision)
                    .await?;
                Ok(Response::ObjectState {
                    state: ObjectState {
                        id,
                        revision,
                        deleted: true,
                    },
                })
            }
            _ => Err(Error::Rejected),
        }
    }
}
