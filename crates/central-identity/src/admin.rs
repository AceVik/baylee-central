//! Central administration (`docs/administration.md`): on the browser listener
//! only, with a browser session whose account holds the administrator role,
//! checked again by the store in the statement or transaction that acts.
use super::{Error, IdentityService, Response, Surface, Token};
use central_api::admin::{AccountQuery, AdminAnswer, AdminRequest};
use central_api::reports::BulkAction;
use central_api::reset_code::ResetCode;

impl IdentityService {
    pub(super) async fn admin_request(
        &self,
        request: AdminRequest,
        bearer: Option<&Token>,
        surface: Surface,
    ) -> Result<Response, Error> {
        let (Surface::Browser, Some(cookie)) = (surface, bearer) else {
            return Err(Error::Rejected);
        };
        let answer = match request {
            AdminRequest::Accounts { query, before } => {
                let query = query.as_deref().map(AccountQuery::parse).transpose()?;
                let (accounts, more) = self
                    .store
                    .admin_accounts(cookie, query.as_ref(), before)
                    .await?;
                AdminAnswer::Accounts { accounts, more }
            }
            AdminRequest::Account { account } => {
                let (account, recovery, gateways, audit) =
                    self.store.admin_account(cookie, account).await?;
                AdminAnswer::Account {
                    account,
                    recovery,
                    gateways,
                    audit,
                }
            }
            AdminRequest::Audit { account, before } => {
                let (events, more) = self.store.admin_audit(cookie, account, before).await?;
                AdminAnswer::Audit { events, more }
            }
            AdminRequest::Gateways { state, before } => {
                let (gateways, more) = self.store.admin_gateways(cookie, state, before).await?;
                AdminAnswer::Gateways { gateways, more }
            }
            AdminRequest::Gateway { gateway } => {
                let (gateway, tokens) = self.store.admin_gateway(cookie, gateway).await?;
                AdminAnswer::Gateway { gateway, tokens }
            }
            AdminRequest::LegacyAccounts => self.admin_legacy(cookie).await?,
            // Reports (`docs/reports.md`): the store checks the administrator,
            // a purge's fresh sign-in, and writes the report's history.
            AdminRequest::Reports {
                filter,
                sort,
                descending,
                offset,
                limit,
            } => {
                let (reports, total) = self
                    .store
                    .reports(cookie, &filter, sort, descending, offset, limit)
                    .await?;
                AdminAnswer::Reports { reports, total }
            }
            AdminRequest::ReportFacets { filter } => AdminAnswer::ReportFacets {
                facets: self.store.report_facets(cookie, &filter).await?,
            },
            AdminRequest::Report { report } => AdminAnswer::Report {
                report: self.store.report_detail(cookie, report).await?,
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
                groups: self.store.crash_groups(cookie, &filter).await?,
            },
            AdminRequest::ReportStats => AdminAnswer::ReportStats {
                days: self.store.report_stats(cookie).await?,
            },
            AdminRequest::ReportAdmins => AdminAnswer::ReportAdmins {
                admins: self.store.report_admins(cookie).await?,
            },
            AdminRequest::SetReport {
                report,
                change,
                expected_revision,
            } => AdminAnswer::Report {
                report: self
                    .store
                    .set_report(cookie, report, &change, expected_revision)
                    .await?,
            },
            AdminRequest::ReportsBulk { reports, bulk } => AdminAnswer::Changed {
                changed: self.store.reports_bulk(cookie, &reports, &bulk).await?,
            },
            AdminRequest::TrashReport { report } => {
                self.store
                    .bin_report(cookie, report, &BulkAction::Trash)
                    .await?;
                return Ok(Response::Done);
            }
            AdminRequest::RestoreReport { report } => {
                self.store
                    .bin_report(cookie, report, &BulkAction::Restore)
                    .await?;
                return Ok(Response::Done);
            }
            AdminRequest::PurgeReport { report } => {
                self.store
                    .bin_report(cookie, report, &BulkAction::Purge)
                    .await?;
                return Ok(Response::Done);
            }
            AdminRequest::AddReportNote { report, text } => AdminAnswer::Report {
                report: self.store.add_report_note(cookie, report, &text).await?,
            },
            AdminRequest::RemoveReportNote { report, note } => AdminAnswer::Report {
                report: self.store.remove_report_note(cookie, report, note).await?,
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
            // Changes: the store checks the sign-in's age and writes the audit.
            AdminRequest::ResetLogin { account } => {
                let code = ResetCode::generate()?;
                let expires_at = self
                    .store
                    .issue_reset(cookie, account, &code.digest())
                    .await?;
                AdminAnswer::ResetCode { code, expires_at }
            }
            AdminRequest::SetAdmin { account, admin } => {
                self.store.set_admin(cookie, account, admin).await?;
                return Ok(Response::Done);
            }
            AdminRequest::SetGatewayApproval { gateway, enabled } => {
                self.store
                    .set_gateway_approval(cookie, gateway, enabled)
                    .await?;
                return Ok(Response::Done);
            }
            AdminRequest::RevokeGatewayToken { token } => {
                self.store.admin_revoke_gateway_token(cookie, token).await?;
                return Ok(Response::Done);
            }
            AdminRequest::SetTag { account, tag } => {
                self.store.set_tag(cookie, account, tag).await?;
                return Ok(Response::Done);
            }
        };
        Ok(Response::Admin { answer })
    }
}
