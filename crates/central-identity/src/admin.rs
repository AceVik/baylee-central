//! Central administration (`docs/administration.md`): on the browser listener
//! only, with a browser session whose account holds the administrator role,
//! checked again by the store in the statement or transaction that acts.
use super::{Error, IdentityService, Response, Surface, Token};
use central_api::admin::{AccountQuery, AdminAnswer, AdminRequest};
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
                let page = self.store.admin_audit(cookie, account, before).await?;
                AdminAnswer::Audit {
                    events: page.events,
                    more: page.more,
                    accounts: page.accounts,
                    gateways: page.gateways,
                }
            }
            AdminRequest::Overview => AdminAnswer::Overview {
                overview: self.store.admin_overview(cookie).await?,
            },
            AdminRequest::Gateways { state, before } => {
                let (gateways, more) = self.store.admin_gateways(cookie, state, before).await?;
                AdminAnswer::Gateways { gateways, more }
            }
            AdminRequest::Gateway { gateway } => {
                let (gateway, tokens) = self.store.admin_gateway(cookie, gateway).await?;
                AdminAnswer::Gateway { gateway, tokens }
            }
            AdminRequest::LegacyAccounts => self.admin_legacy(cookie).await?,
            // Reports (`docs/reports.md`, `reports.rs`): the store checks the
            // administrator, a purge's fresh sign-in, and writes the report's history.
            request @ (AdminRequest::Reports { .. }
            | AdminRequest::ReportFacets { .. }
            | AdminRequest::Report { .. }
            | AdminRequest::ReportNeighbours { .. }
            | AdminRequest::CrashGroups { .. }
            | AdminRequest::ReportStats
            | AdminRequest::ReportAdmins
            | AdminRequest::SetReport { .. }
            | AdminRequest::ReportsBulk { .. }
            | AdminRequest::TrashReport { .. }
            | AdminRequest::RestoreReport { .. }
            | AdminRequest::PurgeReport { .. }
            | AdminRequest::AddReportNote { .. }
            | AdminRequest::RemoveReportNote { .. }
            | AdminRequest::ReportViews
            | AdminRequest::SaveReportView { .. }
            | AdminRequest::DeleteReportView { .. }
            | AdminRequest::SetReportsSeen { .. }) => {
                return self.report_admin(request, cookie).await;
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
