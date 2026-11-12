//! Profile pictures (`docs/profile-pictures.md`): an account's own picture
//! and reports, the upload's admission and conversion, and the public read.
//!
//! An upload is admitted on its headers' session alone, before its body is
//! read: Central's terms accepted, no block in force, room in the budget,
//! and one of [`SLOTS`] upload slots (one per account). Once its body was
//! read it spends a unit of the budget, stored or not; the picture worker
//! converts it in a fresh process ([`central_picture::Worker`]), the
//! service checks the answer and computes the digest, and one transaction
//! stores it under a new random key.
use super::{Error, IdentityService, Response, Surface, Token};
use central_api::admin::AdminAnswer;
use central_api::picture::{PictureAdmin, PictureDigest, PictureKey, PictureRequest};
use central_picture::{Format, Refusal, Worker};
use central_postgres::pictures::{Encoding, OwnPicture};
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex, PoisonError};

/// Uploads converted at once in all; the production socket's
/// `MaxConnections` is the same.
pub const SLOTS: usize = 2;

/// Why an upload was not stored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UploadRefusal {
    /// The account vocabulary: no session (`Rejected`), the terms
    /// (`TermsRequired`), the budget or every slot taken (`Busy`), the
    /// worker not configured or the store away (`Unavailable`), or a block
    /// that began meanwhile (`Conflict`).
    Account(Error),
    /// An administrator's block is in force.
    Blocked,
    /// The picture itself was refused.
    Picture(Refusal),
}

impl From<Error> for UploadRefusal {
    fn from(error: Error) -> Self {
        Self::Account(error)
    }
}

/// The worker and the upload slots.
#[derive(Default)]
pub(super) struct Pictures {
    worker: Option<Worker>,
    slots: Arc<Mutex<BTreeSet<[u8; 16]>>>,
}

/// An admitted upload: its account and its slot, given back when dropped.
pub struct Admitted {
    account: [u8; 16],
    slots: Arc<Mutex<BTreeSet<[u8; 16]>>>,
}

impl Drop for Admitted {
    fn drop(&mut self) {
        // The set is whole at every step, so a poisoned lock's is too.
        self.slots
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&self.account);
    }
}

fn answer(own: OwnPicture) -> Response {
    Response::MyPicture {
        picture: own.picture,
        notice: own.notice,
        block: own.block,
    }
}

impl IdentityService {
    /// Convert pictures with `worker` (`docs/profile-pictures.md`). Without
    /// one every upload is `Unavailable`; reading, removing and serving
    /// pictures work without it.
    #[must_use]
    pub fn with_pictures(mut self, worker: Worker) -> Self {
        self.pictures.worker = Some(worker);
        self
    }

    /// The account's own picture requests, on the browser listener.
    pub(super) async fn picture_request(
        &self,
        request: PictureRequest,
        cookie: &Token,
    ) -> Result<Response, Error> {
        match request {
            PictureRequest::Mine => Ok(answer(self.store.my_picture(cookie).await?)),
            PictureRequest::Remove => {
                self.store.remove_picture(cookie).await?;
                Ok(Response::Done)
            }
            PictureRequest::Reports => Ok(Response::MyPictureReports {
                reports: self.store.my_picture_reports(cookie).await?,
            }),
        }
    }

    /// The administrators' picture requests: the store checks the
    /// administrator, a decision's fresh sign-in, and writes the audit.
    pub(super) async fn picture_admin(
        &self,
        request: PictureAdmin,
        cookie: &Token,
    ) -> Result<Response, Error> {
        let answer = match request {
            PictureAdmin::Reports { state, before } => {
                let (reports, more) = self
                    .store
                    .admin_picture_reports(cookie, state, before)
                    .await?;
                AdminAnswer::PictureReports { reports, more }
            }
            PictureAdmin::Case { account } => {
                let case = self.store.admin_picture_case(cookie, account).await?;
                AdminAnswer::PictureCase {
                    account: case.account,
                    picture: case.own.picture,
                    reports: case.reports,
                    notice: case.own.notice,
                    block: case.own.block,
                }
            }
            PictureAdmin::Decide {
                account,
                sha256,
                decision,
            } => {
                self.store
                    .decide_picture(cookie, account, sha256, &decision)
                    .await?;
                return Ok(Response::Done);
            }
            PictureAdmin::LiftBlock { account } => {
                self.store.lift_picture_block(cookie, account).await?;
                return Ok(Response::Done);
            }
        };
        Ok(Response::Admin { answer })
    }

    /// Admit an upload by the browser session `cookie`, before its body is
    /// read.
    ///
    /// # Errors
    /// `Account(Rejected)` without a live browser session,
    /// `Account(TermsRequired)` before Central's terms are accepted,
    /// `Blocked` while an administrator's block is in force,
    /// `Account(Busy)` when the budget is spent or no slot is free, and
    /// `Account(Unavailable)` without a worker.
    pub async fn admit_picture(&self, cookie: &Token) -> Result<Admitted, UploadRefusal> {
        if self.pictures.worker.is_none() {
            return Err(Error::Unavailable.into());
        }
        self.require_terms(Some(cookie), Surface::Browser).await?;
        let admission = self.store.picture_admission(cookie).await?;
        if admission.block.is_some() {
            return Err(UploadRefusal::Blocked);
        }
        if !admission.room {
            return Err(Error::Busy.into());
        }
        let mut slots = self
            .pictures
            .slots
            .lock()
            .map_err(|_| UploadRefusal::Account(Error::Unavailable))?;
        if slots.len() >= SLOTS || !slots.insert(admission.account) {
            return Err(Error::Busy.into());
        }
        Ok(Admitted {
            account: admission.account,
            slots: Arc::clone(&self.pictures.slots),
        })
    }

    /// Convert and store an admitted upload of `bytes`, declared `format`.
    /// The upload counts against the budget from here, stored or not.
    ///
    /// # Errors
    /// `Account(Busy)` when the budget ran out meanwhile, `Picture` with the
    /// worker's or the service's refusal, `Account(Conflict)` when a block
    /// began meanwhile, `Account(Rejected)` when the session ended.
    pub async fn store_picture(
        &self,
        admitted: Admitted,
        cookie: &Token,
        format: Format,
        bytes: Vec<u8>,
    ) -> Result<Response, UploadRefusal> {
        let worker = self
            .pictures
            .worker
            .clone()
            .ok_or(UploadRefusal::Account(Error::Unavailable))?;
        self.store.charge_picture(admitted.account).await?;
        let converted = tokio::task::spawn_blocking(move || worker.convert(format, &bytes))
            .await
            .map_err(|_| UploadRefusal::Picture(Refusal::Failed))?
            .map_err(UploadRefusal::Picture)?;
        let mut key: PictureKey = [0; 16];
        getrandom::fill(&mut key).map_err(|_| UploadRefusal::Account(Error::Unavailable))?;
        let own = self
            .store
            .set_picture(
                cookie,
                admitted.account,
                key,
                &converted.avif,
                &converted.png,
                converted.sha256,
            )
            .await?;
        drop(admitted);
        Ok(answer(own))
    }

    /// A picture's bytes by its key, for anyone, with the PNG master's
    /// digest; `None` for a key that names no current picture.
    ///
    /// # Errors
    /// Storage failure.
    pub async fn picture_bytes(
        &self,
        key: PictureKey,
        encoding: Encoding,
    ) -> Result<Option<(Vec<u8>, PictureDigest)>, Error> {
        self.store.picture_bytes(key, encoding).await
    }
}
