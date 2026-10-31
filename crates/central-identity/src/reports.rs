//! Reports (`docs/reports.md`): a gateway files them on the native
//! listener's raw routes, the metadata first and its gzip parts after; an
//! administrator reads a part on the browser listener's. The triage itself
//! is `Admin` requests (`admin.rs`).
use super::{Error, IdentityService, Token};
use central_api::reports::{MAX_INTAKE_BYTES, Part, RecordOrigin, ReportIntake};
use central_postgres::Filer;
use std::time::Instant;

/// An upload of a report's part its gateway may make: who, which, and how
/// many bytes were announced.
pub struct PartUpload {
    filer: Filer,
    report: [u8; 16],
    part: Part,
    /// The announced length: read exactly this many bytes.
    pub bytes: u32,
}

impl IdentityService {
    /// The gateway behind `bearer`: the official one for the bridge's
    /// credential, else an approved registered one.
    async fn filer(&self, bearer: Option<&Token>) -> Result<Filer, Error> {
        if self.bridge(bearer).is_ok() {
            self.store.official_filer().await
        } else {
            self.store
                .report_filer(bearer.ok_or(Error::Rejected)?)
                .await
        }
    }

    /// File a report's metadata (`POST /v1/reports`), within the public
    /// budget and the gateway's reports an hour. Returns its id.
    ///
    /// # Errors
    /// `Rejected` for another credential or a report out of its bounds;
    /// `Busy` past a budget; storage failure.
    pub async fn file_report(
        &self,
        bearer: Option<&Token>,
        body: &[u8],
    ) -> Result<[u8; 16], Error> {
        if body.len() > MAX_INTAKE_BYTES {
            return Err(Error::Rejected);
        }
        self.limits
            .lock()
            .map_err(|_| Error::Unavailable)?
            .global(Instant::now())?;
        let report: ReportIntake = central_api::decode(body)?;
        report.validate()?;
        let filer = self.filer(bearer).await?;
        self.limits
            .lock()
            .map_err(|_| Error::Unavailable)?
            .report(filer.gateway, Instant::now())?;
        self.store.file_report(&filer, &report).await
    }

    /// Begin a part's upload (`PUT /v1/reports/{id}/{part}`) before its body
    /// is read: the credential, the report and the announcement are checked
    /// first, and the answer says how many bytes to read, no more.
    ///
    /// # Errors
    /// `Rejected` for another credential, another gateway's report, one too
    /// old or a part not announced; `Conflict` for a part stored already;
    /// `Busy` past the public budget; storage failure.
    pub async fn report_upload(
        &self,
        bearer: Option<&Token>,
        report: [u8; 16],
        part: Part,
    ) -> Result<PartUpload, Error> {
        self.limits
            .lock()
            .map_err(|_| Error::Unavailable)?
            .global(Instant::now())?;
        let filer = self.filer(bearer).await?;
        let bytes = self
            .store
            .report_part_expected(&filer, report, part)
            .await?;
        Ok(PartUpload {
            filer,
            report,
            part,
            bytes,
        })
    }

    /// Store the bytes of an upload [`Self::report_upload`] began.
    ///
    /// # Errors
    /// `Rejected` for bytes other than announced; `Conflict` for a part
    /// stored meanwhile; storage failure.
    pub async fn file_report_part(&self, upload: &PartUpload, gzip: &[u8]) -> Result<(), Error> {
        self.store
            .file_report_part(&upload.filer, upload.report, upload.part, gzip)
            .await
    }

    /// One part of a report as stored, for an administrator's browser session.
    ///
    /// # Errors
    /// `Rejected` without an administrator's live browser session, for an
    /// unknown report or a part not arrived; storage failure.
    pub async fn report_part(
        &self,
        cookie: Option<&Token>,
        report: [u8; 16],
        part: Part,
    ) -> Result<(Vec<u8>, Option<RecordOrigin>), Error> {
        self.store
            .report_part(cookie.ok_or(Error::Rejected)?, report, part)
            .await
    }
}
