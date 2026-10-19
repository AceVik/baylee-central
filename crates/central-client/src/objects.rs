//! Object encryption uses the session's private master key, never a returned raw key.
use super::{Document, Error, ObjectContext, Request, Response, Session, Transport, context};
use central_api::object::ObjectState;

/// Authenticated object content. Plaintext is wiped on drop and cannot be printed.
pub struct ObjectDocument {
    /// Immutable object identity.
    pub id: [u8; 16],
    /// Authenticated ciphertext revision.
    pub revision: u64,
    /// Decrypted chunk; applications validate its format before restoring it.
    pub plaintext: Document,
}

impl Session {
    fn object_context(&self, id: [u8; 16], revision: u64) -> Result<ObjectContext, Error> {
        // Reserve the account UUID exclusively for the account manifest. Even a
        // malicious server cannot substitute that ciphertext for a private object.
        if id == self.identity.account {
            return Err(Error::Rejected);
        }
        ObjectContext::new(context(&self.endpoint, self.identity)?, id, revision)
            .map_err(|_| Error::Rejected)
    }

    /// Reserve an opaque ID; a lost reply may leave an empty reservation for later reconciliation.
    ///
    /// # Errors
    /// Rejects foreign endpoints, malformed IDs/states, quota or unavailable service.
    pub fn create_object(&self, transport: &impl Transport) -> Result<ObjectState, Error> {
        self.same_endpoint(transport)?;
        let Response::ObjectState { state } =
            transport.exchange(&Request::CreateObject, Some(&self.token))?
        else {
            return Err(Error::Rejected);
        };
        self.object_context(state.id, 1)?;
        if state.revision != 0 || state.deleted {
            return Err(Error::Rejected);
        }
        Ok(state)
    }

    /// Authenticate and decrypt one chunk against its expected identity and revision floor.
    ///
    /// # Errors
    /// Rejects substitution, rollback, deletion, malformed ciphertext or transport failure.
    pub fn read_object(
        &self,
        transport: &impl Transport,
        id: [u8; 16],
        minimum_revision: u64,
    ) -> Result<ObjectDocument, Error> {
        self.same_endpoint(transport)?;
        let Response::Object {
            state,
            document: Some(document),
        } = transport.exchange(&Request::ReadObject { id }, Some(&self.token))?
        else {
            return Err(Error::Rejected);
        };
        if state.id != id || state.deleted || state.revision < minimum_revision {
            return Err(Error::Rejected);
        }
        let plaintext = self
            .master
            .open_object(self.object_context(id, state.revision)?, &document)
            .map_err(|_| Error::Rejected)?;
        Ok(ObjectDocument {
            id,
            revision: state.revision,
            plaintext,
        })
    }

    /// Encrypt and CAS one chunk. A lost reply requires an authenticated read before retrying.
    ///
    /// # Errors
    /// Rejects oversized data, revision exhaustion, concurrent writes or transport failure.
    pub fn write_object(
        &self,
        transport: &impl Transport,
        id: [u8; 16],
        expected_revision: u64,
        plaintext: &[u8],
    ) -> Result<u64, Error> {
        self.same_endpoint(transport)?;
        let revision = expected_revision.checked_add(1).ok_or(Error::Rejected)?;
        let document = self
            .master
            .seal_object(self.object_context(id, revision)?, plaintext)
            .map_err(|_| Error::Rejected)?;
        let response = transport.exchange(
            &Request::WriteObject {
                id,
                expected_revision,
                document,
            },
            Some(&self.token),
        )?;
        Self::changed_object(&response, id, revision, false)
    }

    /// Tombstone an unreferenced chunk. Update the encrypted manifest before removing referenced data.
    ///
    /// # Errors
    /// Rejects stale revisions, revoked sessions and unavailable service.
    pub fn delete_object(
        &self,
        transport: &impl Transport,
        id: [u8; 16],
        expected_revision: u64,
    ) -> Result<u64, Error> {
        self.same_endpoint(transport)?;
        let revision = expected_revision.checked_add(1).ok_or(Error::Rejected)?;
        self.object_context(id, revision)?;
        let response = transport.exchange(
            &Request::DeleteObject {
                id,
                expected_revision,
            },
            Some(&self.token),
        )?;
        Self::changed_object(&response, id, revision, true)
    }

    fn changed_object(
        response: &Response,
        id: [u8; 16],
        revision: u64,
        deleted: bool,
    ) -> Result<u64, Error> {
        match response {
            Response::ObjectState { state }
                if state
                    == &(ObjectState {
                        id,
                        revision,
                        deleted,
                    }) =>
            {
                Ok(revision)
            }
            _ => Err(Error::Rejected),
        }
    }

    /// Inspect a bounded page for reconciliation. IDs are untrusted metadata until content authenticates.
    ///
    /// # Errors
    /// Rejects malformed/unsorted/oversized pages, revoked sessions and unavailable service.
    pub fn list_objects(
        &self,
        transport: &impl Transport,
        after: Option<[u8; 16]>,
    ) -> Result<Vec<ObjectState>, Error> {
        self.same_endpoint(transport)?;
        let Response::Objects { objects } =
            transport.exchange(&Request::ListObjects { after }, Some(&self.token))?
        else {
            return Err(Error::Rejected);
        };
        if objects.len() > 100 {
            return Err(Error::Rejected);
        }
        let mut previous = after;
        for object in &objects {
            self.object_context(object.id, 1)?;
            if previous.is_some_and(|id| id >= object.id)
                || (object.deleted && object.revision == 0)
            {
                return Err(Error::Rejected);
            }
            previous = Some(object.id);
        }
        Ok(objects)
    }
}
