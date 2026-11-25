//! Reports (`docs/reports.md`): a gateway or a report token files them on the
//! native listener's raw routes, the metadata first and its gzip parts after;
//! an administrator reads a part on the browser listener's, and triages with
//! `Admin` requests, which `admin.rs` routes here. A report token reads and
//! triages on the native listener's report routes, under its rights
//! ([`ReportCall`]).
//!
//! On the native routes the credential is asked first, before a byte of a
//! body is read: [`IdentityService::report_filing`] and
//! [`IdentityService::report_access`] refuse for the credential alone, and
//! the store asks a report token again in the transaction that changes
//! anything.
use super::{Error, IdentityService, Place, Response, Token};
use central_api::admin::{AdminAnswer, AdminRequest};
use central_api::report_api;
use central_api::report_tokens::{ReportBearer, ReportScope};
use central_api::reports::{
    BulkAction, MAX_BULK, MAX_INTAKE_BYTES, Part, RecordOrigin, ReportIntake,
};
use central_postgres::{Filer, ReportCaller};
use std::time::Instant;

/// What files, its credential known to be good: a request's body may be
/// read.
pub struct Filing(Filer);

impl Filing {
    /// The gateway or report token that files, by its id: what its uploads
    /// at once and its unfinished ones are counted by.
    #[must_use]
    pub fn filer(&self) -> [u8; 16] {
        self.0.id
    }
}

/// Reports of a selection that one request of a token's hour pays for: a
/// change of two hundred writes as many reports and histories, so it costs
/// twenty.
const PER_REQUEST: usize = 10;

/// What a selection of `reports` costs beyond the request it came with.
fn beyond_one(reports: usize) -> u32 {
    let reports = reports.min(MAX_BULK);
    u32::try_from(reports.div_ceil(PER_REQUEST).saturating_sub(1)).unwrap_or(u32::MAX)
}

/// What a report token asks for besides filing and downloads. Its bounds are
/// checked before the store is asked.
pub enum ReportCall {
    /// A page of reports.
    Search(report_api::Search),
    /// Counts per dimension.
    Facets(report_api::Over),
    /// Crashes by signature.
    Crashes(report_api::Over),
    /// Reports per day and kind.
    Stats,
    /// One report.
    Report([u8; 16]),
    /// The administrators a report may be assigned to.
    Admins,
    /// Change one report's triage.
    Change([u8; 16], report_api::Patch),
    /// One report into the trash.
    Trash([u8; 16]),
    /// One report out of the trash.
    Restore([u8; 16]),
    /// A note on a report.
    Note([u8; 16], report_api::NewNote),
    /// The token's own note removed.
    RemoveNote([u8; 16], [u8; 16]),
    /// A selection changed, trashed or restored.
    Bulk(report_api::Bulk),
    /// One report deleted for good, from the trash.
    Purge([u8; 16]),
    /// A selection deleted for good, from the trash.
    PurgeAll(report_api::Selection),
}

/// The answer to a [`ReportCall`].
pub enum ReportReply {
    /// A page.
    Page(report_api::Page),
    /// Counts.
    Facets(report_api::Facets),
    /// Crash groups.
    Crashes(report_api::CrashGroups),
    /// Days.
    Stats(report_api::Stats),
    /// One report.
    Report(Box<report_api::Detail>),
    /// Administrators.
    Admins(report_api::Assignees),
    /// How many changed.
    Changed(report_api::Changed),
    /// Done, and nothing to say.
    Done,
}

impl ReportCall {
    /// The right it needs. Each is the route's as well (`central-http`), so
    /// that it is asked for before the call is read.
    #[must_use]
    pub const fn needs(&self) -> ReportScope {
        match self {
            Self::Search(_)
            | Self::Facets(_)
            | Self::Crashes(_)
            | Self::Stats
            | Self::Report(_) => ReportScope::Read,
            Self::Admins
            | Self::Change(..)
            | Self::Trash(_)
            | Self::Restore(_)
            | Self::Note(..)
            | Self::RemoveNote(..)
            | Self::Bulk(_) => ReportScope::Triage,
            Self::Purge(_) | Self::PurgeAll(_) => ReportScope::Purge,
        }
    }
}

impl IdentityService {
    /// The filer behind `bearer`: a live report token that may file, the
    /// official gateway for the bridge's credential, else an approved
    /// registered gateway.
    async fn filer(&self, bearer: &ReportBearer) -> Result<Filer, Error> {
        match bearer {
            ReportBearer::Report(token) => self.store.report_token_filer(token).await,
            ReportBearer::Gateway(token) => {
                if self.bridge(Some(token)).is_ok() {
                    self.store.official_filer().await
                } else {
                    self.store.report_filer(token).await
                }
            }
        }
    }

    /// Spend one of the requests everyone shares and of `place`'s share.
    fn public(&self, place: Option<Place>) -> Result<(), Error> {
        self.limits
            .lock()
            .map_err(|_| Error::Unavailable)?
            .global(place, Instant::now())
    }

    /// Begin a filing or a part's upload before its body is read, within the
    /// public budget and its place's share of it (`place` is where the
    /// request came from).
    ///
    /// # Errors
    /// `Rejected` for another credential; `Busy` past the public budget;
    /// storage failure.
    pub async fn report_filing(
        &self,
        bearer: &ReportBearer,
        place: Option<Place>,
    ) -> Result<Filing, Error> {
        self.public(place)?;
        Ok(Filing(self.filer(bearer).await?))
    }

    /// File the metadata of a report (`POST /v1/reports`), within the
    /// filer's reports an hour. Returns the report's id.
    ///
    /// # Errors
    /// `Rejected` for a report out of its bounds or by a channel that is
    /// not its filer's; `Busy` past the filer's reports an hour, or past
    /// what its reports or all reports may take in the store; storage
    /// failure.
    pub async fn file_report(&self, filing: &Filing, body: &[u8]) -> Result<[u8; 16], Error> {
        if body.len() > MAX_INTAKE_BYTES {
            return Err(Error::Rejected);
        }
        let report: ReportIntake = central_api::decode(body)?;
        report.validate()?;
        self.limits
            .lock()
            .map_err(|_| Error::Unavailable)?
            .report(filing.0.id, Instant::now())?;
        self.store.file_report(&filing.0, &report).await
    }

    /// How many bytes to read of a part's upload (`PUT
    /// /v1/reports/{id}/{part}`), and no more: the report and the
    /// announcement are checked before its body is read.
    ///
    /// # Errors
    /// `Rejected` for another filer's report, one too old or a part not
    /// announced; `Conflict` for a part stored already; storage failure.
    pub async fn report_upload(
        &self,
        filing: &Filing,
        report: [u8; 16],
        part: Part,
    ) -> Result<u32, Error> {
        self.store
            .report_part_expected(&filing.0, report, part)
            .await
    }

    /// Store the bytes of an upload [`Self::report_upload`] began.
    ///
    /// # Errors
    /// `Rejected` for bytes other than announced; `Conflict` for a part
    /// stored meanwhile; storage failure.
    pub async fn file_report_part(
        &self,
        filing: &Filing,
        report: [u8; 16],
        part: Part,
        gzip: &[u8],
    ) -> Result<(), Error> {
        self.store
            .file_report_part(&filing.0, report, part, gzip)
            .await
    }

    /// Begin a report token's request before its body is read: within the
    /// public budget, the token must be live and hold `right`, and the
    /// request is one of its hour (a `download` one of its downloads too).
    /// Answers the token's id.
    ///
    /// # Errors
    /// `Rejected` for an unknown token, a revoked or expired one, one whose
    /// maker is no administrator and one without the right, all alike, and
    /// for nothing else; `Busy` past a budget; storage failure.
    pub async fn report_access(
        &self,
        token: &Token,
        right: ReportScope,
        download: bool,
        place: Option<Place>,
    ) -> Result<[u8; 16], Error> {
        self.public(place)?;
        let found = self.store.report_token(token, right).await?;
        self.spend(found.id, 1, u32::from(download))?;
        Ok(found.id)
    }

    /// Spend `requests` of the report token's hour, and `downloads`.
    fn spend(&self, token: [u8; 16], requests: u32, downloads: u32) -> Result<(), Error> {
        self.limits
            .lock()
            .map_err(|_| Error::Unavailable)?
            .report_token(token, requests, downloads, Instant::now())
    }

    /// Answer a report token's call, begun by [`Self::report_access`] as
    /// `id`. The store asks the token again, with the right the call needs,
    /// in the transaction that reads or changes. A selection spends a
    /// request of the token's hour for every [`PER_REQUEST`] reports in it.
    ///
    /// # Errors
    /// `Rejected` for a call out of its bounds or against the rules, an
    /// unknown report or a token that no longer holds the right; `Conflict`
    /// on another revision; `Busy` past the token's hour; storage failure.
    pub async fn report_call(
        &self,
        id: [u8; 16],
        token: &Token,
        call: ReportCall,
    ) -> Result<ReportReply, Error> {
        let by = ReportCaller::Token(token);
        Ok(match call {
            ReportCall::Search(search) => {
                let filter = search.filter.typed()?;
                let (reports, total) = self
                    .store
                    .reports(
                        by,
                        &filter,
                        search.sort,
                        search.descending,
                        search.offset,
                        search.limit,
                    )
                    .await?;
                ReportReply::Page(report_api::Page {
                    reports: reports.into_iter().map(report_api::Report::from).collect(),
                    total,
                })
            }
            ReportCall::Facets(over) => {
                let filter = over.filter.typed()?;
                ReportReply::Facets(self.store.report_facets(by, &filter).await?.into())
            }
            ReportCall::Crashes(over) => {
                let filter = over.filter.typed()?;
                let groups = self.store.crash_groups(by, &filter).await?;
                ReportReply::Crashes(report_api::CrashGroups {
                    groups: groups.into_iter().map(report_api::Crashes::from).collect(),
                })
            }
            ReportCall::Stats => ReportReply::Stats(report_api::Stats {
                days: self.store.report_stats(by).await?,
            }),
            ReportCall::Report(report) => {
                let detail = self.store.report_detail(by, report).await?;
                ReportReply::Report(Box::new(detail.into()))
            }
            ReportCall::Admins => {
                let admins = self.store.report_admins(by).await?;
                ReportReply::Admins(report_api::Assignees {
                    admins: admins.into_iter().map(report_api::Assignee::from).collect(),
                })
            }
            ReportCall::Change(report, patch) => {
                let change = patch.change.typed(false)?;
                let detail = self
                    .store
                    .set_report(by, report, &change, patch.expected_revision)
                    .await?;
                ReportReply::Report(Box::new(detail.into()))
            }
            ReportCall::Trash(report) => self.binned(by, report, &BulkAction::Trash).await?,
            ReportCall::Restore(report) => self.binned(by, report, &BulkAction::Restore).await?,
            ReportCall::Purge(report) => self.binned(by, report, &BulkAction::Purge).await?,
            ReportCall::Note(report, note) => {
                let text = note.typed()?;
                let detail = self.store.add_report_note(by, report, &text).await?;
                ReportReply::Report(Box::new(detail.into()))
            }
            ReportCall::RemoveNote(report, note) => {
                let detail = self.store.remove_report_note(by, report, note).await?;
                ReportReply::Report(Box::new(detail.into()))
            }
            ReportCall::Bulk(bulk) => {
                let (reports, action) = bulk.typed()?;
                self.spend(id, beyond_one(reports.len()), 0)?;
                let changed = self.store.reports_bulk(by, &reports, &action).await?;
                ReportReply::Changed(report_api::Changed { changed })
            }
            ReportCall::PurgeAll(selection) => {
                let reports: Vec<_> = selection.reports.into_iter().map(|id| id.0).collect();
                self.spend(id, beyond_one(reports.len()), 0)?;
                let changed = self
                    .store
                    .reports_bulk(by, &reports, &BulkAction::Purge)
                    .await?;
                ReportReply::Changed(report_api::Changed { changed })
            }
        })
    }

    async fn binned(
        &self,
        by: ReportCaller<'_>,
        report: [u8; 16],
        how: &BulkAction,
    ) -> Result<ReportReply, Error> {
        self.store.bin_report(by, report, how).await?;
        Ok(ReportReply::Done)
    }

    /// One part of a report as stored, for a report token that may read.
    ///
    /// # Errors
    /// `Rejected` for a token that no longer may, an unknown report or a
    /// part not arrived; storage failure.
    pub async fn report_download(
        &self,
        token: &Token,
        report: [u8; 16],
        part: Part,
    ) -> Result<(Vec<u8>, Option<RecordOrigin>), Error> {
        self.store
            .report_part(ReportCaller::Token(token), report, part)
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
            .report_part(
                ReportCaller::Admin(cookie.ok_or(Error::Rejected)?),
                report,
                part,
            )
            .await
    }

    /// An administrator's request about reports, as `admin_request` routes
    /// them: every other request is refused here.
    pub(super) async fn report_admin(
        &self,
        request: AdminRequest,
        cookie: &Token,
    ) -> Result<Response, Error> {
        let by = ReportCaller::Admin(cookie);
        let answer = match request {
            AdminRequest::Reports {
                filter,
                sort,
                descending,
                offset,
                limit,
            } => {
                let (reports, total) = self
                    .store
                    .reports(by, &filter, sort, descending, offset, limit)
                    .await?;
                AdminAnswer::Reports { reports, total }
            }
            AdminRequest::ReportFacets { filter } => AdminAnswer::ReportFacets {
                facets: self.store.report_facets(by, &filter).await?,
            },
            AdminRequest::Report { report } => AdminAnswer::Report {
                report: Box::new(self.store.report_detail(by, report).await?),
            },
            AdminRequest::ReportNeighbours {
                report,
                filter,
                sort,
                descending,
            } => AdminAnswer::Neighbours {
                neighbours: self
                    .store
                    .report_neighbours(cookie, report, &filter, sort, descending)
                    .await?,
            },
            AdminRequest::CrashGroups { filter } => AdminAnswer::CrashGroups {
                groups: self.store.crash_groups(by, &filter).await?,
            },
            AdminRequest::ReportStats => AdminAnswer::ReportStats {
                days: self.store.report_stats(by).await?,
            },
            AdminRequest::ReportAdmins => AdminAnswer::ReportAdmins {
                admins: self.store.report_admins(by).await?,
            },
            AdminRequest::SetReport {
                report,
                change,
                expected_revision,
            } => AdminAnswer::Report {
                report: Box::new(
                    self.store
                        .set_report(by, report, &change, expected_revision)
                        .await?,
                ),
            },
            AdminRequest::ReportsBulk { reports, bulk } => AdminAnswer::Changed {
                changed: self.store.reports_bulk(by, &reports, &bulk).await?,
            },
            AdminRequest::TrashReport { report } => {
                self.store
                    .bin_report(by, report, &BulkAction::Trash)
                    .await?;
                return Ok(Response::Done);
            }
            AdminRequest::RestoreReport { report } => {
                self.store
                    .bin_report(by, report, &BulkAction::Restore)
                    .await?;
                return Ok(Response::Done);
            }
            AdminRequest::PurgeReport { report } => {
                self.store
                    .bin_report(by, report, &BulkAction::Purge)
                    .await?;
                return Ok(Response::Done);
            }
            AdminRequest::AddReportNote { report, text } => AdminAnswer::Report {
                report: Box::new(self.store.add_report_note(by, report, &text).await?),
            },
            AdminRequest::RemoveReportNote { report, note } => AdminAnswer::Report {
                report: Box::new(self.store.remove_report_note(by, report, note).await?),
            },
            AdminRequest::ReportViews => AdminAnswer::ReportViews {
                views: self.store.report_views(cookie).await?,
            },
            AdminRequest::SaveReportView { name, query } => AdminAnswer::ReportViews {
                views: self.store.save_report_view(cookie, &name, &query).await?,
            },
            AdminRequest::DeleteReportView { view } => AdminAnswer::ReportViews {
                views: self.store.delete_report_view(cookie, view).await?,
            },
            AdminRequest::SetReportsSeen { reports, seen } => {
                self.store.set_reports_seen(cookie, &reports, seen).await?;
                return Ok(Response::Done);
            }
            request => return self.report_token_admin(request, cookie).await,
        };
        Ok(Response::Admin { answer })
    }

    /// An administrator's request about report tokens: the store checks the
    /// administrator, a change's fresh sign-in, and writes the audit. A token
    /// is generated here and leaves in its creation answer alone.
    pub(super) async fn report_token_admin(
        &self,
        request: AdminRequest,
        cookie: &Token,
    ) -> Result<Response, Error> {
        let answer = match request {
            AdminRequest::ReportTokens => AdminAnswer::ReportTokens {
                tokens: self.store.admin_report_tokens(cookie).await?,
            },
            AdminRequest::CreateReportToken { name, scopes, days } => {
                let token = Token::generate()?;
                let (id, expires_at) = self
                    .store
                    .create_report_token(cookie, &name, scopes, days, &token)
                    .await?;
                AdminAnswer::ReportTokenCreated {
                    id,
                    token,
                    expires_at,
                }
            }
            AdminRequest::RevokeReportToken { token } => {
                self.store.revoke_report_token(cookie, token).await?;
                return Ok(Response::Done);
            }
            _ => return Err(Error::Rejected),
        };
        Ok(Response::Admin { answer })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_selection_costs_a_request_for_every_ten_reports() {
        assert_eq!(
            [0, 1, 10, 11, 100, 200, usize::MAX].map(beyond_one),
            [0, 0, 0, 1, 9, 19, 19]
        );
    }

    /// The right each call needs, by what it does: what reads, reads; what
    /// changes, triages; what cannot be undone, purges.
    #[test]
    fn each_call_needs_the_right_for_what_it_does() {
        let id = [7; 16];
        let search = report_api::Search::default;
        let over = report_api::Over::default;
        let patch = report_api::Patch {
            change: report_api::Change::default(),
            expected_revision: None,
        };
        let bulk = report_api::Bulk {
            reports: Vec::new(),
            bulk: report_api::BulkDo::Trash,
        };
        let selection = report_api::Selection {
            reports: Vec::new(),
        };
        let note = report_api::NewNote { text: "n".into() };
        for (calls, right) in [
            (
                vec![
                    ReportCall::Search(search()),
                    ReportCall::Facets(over()),
                    ReportCall::Crashes(over()),
                    ReportCall::Stats,
                    ReportCall::Report(id),
                ],
                ReportScope::Read,
            ),
            (
                vec![
                    ReportCall::Admins,
                    ReportCall::Change(id, patch),
                    ReportCall::Trash(id),
                    ReportCall::Restore(id),
                    ReportCall::Note(id, note),
                    ReportCall::RemoveNote(id, id),
                    ReportCall::Bulk(bulk),
                ],
                ReportScope::Triage,
            ),
            (
                vec![ReportCall::Purge(id), ReportCall::PurgeAll(selection)],
                ReportScope::Purge,
            ),
        ] {
            for call in calls {
                assert_eq!(call.needs(), right);
            }
        }
    }
}
