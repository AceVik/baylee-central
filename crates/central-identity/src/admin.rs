//! Central administration (`docs/administration.md`): on the browser listener
//! only, with a browser session whose account holds the administrator role,
//! checked again by the store in the statement or transaction that acts.
use super::{Error, IdentityService, Response, Surface, Token, email::ISSUED_HOURS};
use central_api::admin::{AccountQuery, AdminAnswer, AdminRequest};
use central_api::reset_code::ResetCode;
use central_mail::Mail;

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
                    report_tokens: page.report_tokens,
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
            request @ (AdminRequest::Registration | AdminRequest::SetRegistration { .. }) => {
                return self.registration_admin(request, cookie).await;
            }
            // Reports and report tokens (`docs/reports.md`, `reports.rs`): the
            // store checks the administrator, the fresh sign-in of a purge and of a
            // token's making or revocation, and writes the history or the audit.
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
            | AdminRequest::SetReportsSeen { .. }
            | AdminRequest::ReportTokens
            | AdminRequest::CreateReportToken { .. }
            | AdminRequest::RevokeReportToken { .. }) => {
                return self.report_admin(request, cookie).await;
            }
            // Changes: the store checks the sign-in's age and writes the audit.
            AdminRequest::ResetLogin { account } => self.admin_reset_login(cookie, account).await?,
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
            AdminRequest::SetRankedVerification { gateway, verified } => {
                self.store
                    .set_ranked_verification(cookie, gateway, verified)
                    .await?;
                return Ok(Response::Done);
            }
            AdminRequest::RevokeGatewayToken { token } => {
                self.store.admin_revoke_gateway_token(cookie, token).await?;
                return Ok(Response::Done);
            }
            // Profile pictures (`docs/profile-pictures.md`, `pictures.rs`).
            AdminRequest::Picture { request } => return self.picture_admin(request, cookie).await,
            AdminRequest::SetTag { account, tag } => {
                return self.set_tag(cookie, account, tag).await;
            }
        };
        Ok(Response::Admin { answer })
    }

    /// Tell the account's confirmed address only that a reset was issued. The
    /// code itself is returned to the administrator after the store commits.
    async fn admin_reset_login(
        &self,
        cookie: &Token,
        account: [u8; 16],
    ) -> Result<AdminAnswer, Error> {
        let code = ResetCode::generate()?;
        let done = self
            .store
            .issue_reset(cookie, account, &code.digest())
            .await;
        let expires_at = self.mails.committed(done, account, |to, account, lang| {
            Mail::reset_issued(to, account, lang, ISSUED_HOURS)
        })?;
        Ok(AdminAnswer::ResetCode { code, expires_at })
    }

    /// An account's tag (`docs/administration.md`): the store checks the
    /// administrator and writes the audit; whoever counts the account among
    /// their friends is told, and sees the new tag in their presence.
    async fn set_tag(
        &self,
        cookie: &Token,
        account: [u8; 16],
        tag: u32,
    ) -> Result<Response, Error> {
        self.store.set_tag(cookie, account, tag).await?;
        self.live.retagged(&account, tag);
        Ok(Response::Done)
    }

    /// The registration switch (`docs/registration-abuse.md`): the store
    /// checks the administrator, a change's fresh sign-in, and writes the
    /// audit.
    async fn registration_admin(
        &self,
        request: AdminRequest,
        cookie: &Token,
    ) -> Result<Response, Error> {
        match request {
            AdminRequest::Registration => {
                let (open, changed_at) = self.store.admin_registration(cookie).await?;
                Ok(Response::Admin {
                    answer: AdminAnswer::Registration { open, changed_at },
                })
            }
            AdminRequest::SetRegistration { open } => {
                self.store.set_registration(cookie, open).await?;
                Ok(Response::Done)
            }
            _ => Err(Error::Rejected),
        }
    }
}
