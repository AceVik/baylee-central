//! Exporting the account (`docs/account-deletion.md`, GDPR Art. 15 and 20):
//! what Central stores about it, and the plaintext of its private objects,
//! which only this device can decrypt.
use super::{Error, ObjectDocument, Request, Response, Session, Transport};
use central_api::export::{AccountExport, EXPORT_LIST};

impl Session {
    /// What Central stores about the account, apart from its secrets and
    /// ciphertext. Needs a password sign-in less than 30 minutes old (sign
    /// in again otherwise); Central's terms need not be accepted. The
    /// decrypted account document is [`Session::document`].
    ///
    /// # Errors
    /// `Rejected` for an ended session, a sign-in 30 minutes old or older and
    /// an answer about another account or out of its bounds.
    pub fn export(&self, transport: &impl Transport) -> Result<AccountExport, Error> {
        self.same_endpoint(transport)?;
        let Response::AccountExport { export } =
            transport.exchange(&Request::AccountExport, Some(&self.token))?
        else {
            return Err(Error::Rejected);
        };
        if export.account != self.identity.account
            || export.vault_epoch != self.identity.epoch
            || export.sessions.len() > 2
            || [
                export.terms.len(),
                export.gateways.len(),
                export.administers.len(),
                export.audit.len(),
            ]
            .into_iter()
            .any(|n| n > EXPORT_LIST)
        {
            return Err(Error::Rejected);
        }
        Ok(*export)
    }

    /// Read, authenticate and decrypt every private object of the account,
    /// handing each to `each` in id order, one at a time, so an export of
    /// the whole quota never sits in memory at once. Deleted objects and
    /// empty reservations are skipped. Returns how many were handed over.
    ///
    /// # Errors
    /// What [`Session::list_objects`] and [`Session::read_object`] report,
    /// and the first error `each` returns, which stops the export.
    pub fn export_objects(
        &self,
        transport: &impl Transport,
        mut each: impl FnMut(ObjectDocument) -> Result<(), Error>,
    ) -> Result<u64, Error> {
        let mut after = None;
        let mut count = 0;
        loop {
            let page = self.list_objects(transport, after)?;
            let Some(last) = page.last() else {
                return Ok(count);
            };
            after = Some(last.id);
            for object in page {
                if object.deleted || object.revision == 0 {
                    continue;
                }
                each(self.read_object(transport, object.id, object.revision)?)?;
                count += 1;
            }
        }
    }
}
