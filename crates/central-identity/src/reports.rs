//! Reports (`docs/reports.md`): a gateway files them on the native
//! listener's raw routes, the metadata first and its gzip parts after; an
//! administrator reads a part on the browser listener's. The triage itself
//! is `Admin` requests (`admin.rs`).
use super::{Error, IdentityService, Token};
use central_api::reports::{MAX_INTAKE_BYTES, Part, RecordOrigin, ReportIntake};
use central_postgres::Filer;
use std::time::Instant;

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

    /// Store a part of a report the same gateway filed (`PUT /v1/reports/{id}/{part}`).
    ///
    /// # Errors
    /// `Rejected` for another gateway's report, a part not announced or
    /// bytes other than announced; `Conflict` for a part stored already;
    /// `Busy` past the public budget; storage failure.
    pub async fn file_report_part(
        &self,
        bearer: Option<&Token>,
        report: [u8; 16],
        part: Part,
        gzip: &[u8],
    ) -> Result<(), Error> {
        self.limits
            .lock()
            .map_err(|_| Error::Unavailable)?
            .global(Instant::now())?;
        let filer = self.filer(bearer).await?;
        self.store
            .file_report_part(&filer, report, part, gzip)
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
