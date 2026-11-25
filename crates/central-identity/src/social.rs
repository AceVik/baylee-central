//! Friend invite orchestration and social dispatch helpers bridging
//! PostgreSQL invite storage, `central-mail` delivery, and live push events.

use super::{IdentityService, Surface};
use central_api::social::{
    MAX_ACTIVE_FRIEND_INVITES, SocialRequest, SocialResponse, invite_expiry_days,
};
use central_api::wire::Response;
use central_api::{Error, Token, profile};
use central_mail::Mail;
use central_postgres::Session;

impl IdentityService {
    /// Serve `Request::FriendInviteInfo { code }` on either surface without
    /// requiring authentication.
    pub(super) async fn friend_invite_info(&self, code: &str) -> Result<Response, Error> {
        let invite = self.store.friend_invite_info(code).await?;
        Ok(Response::FriendInviteInfo { invite })
    }

    /// Execute an authenticated `SocialRequest` with mail delivery and live push
    /// notifications (`self.tell(&concerns)`).
    pub(super) async fn social_operation(
        &self,
        request: SocialRequest,
        bearer: Option<&Token>,
        surface: Surface,
    ) -> Result<Response, Error> {
        let session = surface.session(bearer)?;
        let response = match request {
            SocialRequest::FriendInvites => {
                let invites = self.store.list_friend_invites(session).await?;
                let recruiter_badge = self.store.recruiter_badge(session).await?;
                SocialResponse::FriendInvites {
                    invites,
                    active_limit: u32::try_from(MAX_ACTIVE_FRIEND_INVITES).unwrap_or(10),
                    mail_available: self.mails.mailer.is_some(),
                    recruiter_badge: Some(recruiter_badge),
                }
            }
            SocialRequest::CreateFriendInvite {
                label,
                note,
                email,
                lang,
                expires_days,
            } => {
                self.create_friend_invite(
                    session,
                    label.as_deref(),
                    note.as_deref(),
                    email.as_deref(),
                    lang.as_deref(),
                    expires_days,
                )
                .await?
            }
            SocialRequest::RevokeFriendInvite { invite } => {
                let concerns = self.store.revoke_friend_invite(session, invite).await?;
                self.tell(&concerns);
                SocialResponse::Done
            }
            SocialRequest::RedeemFriendInvite {
                code,
                link_email,
                lang,
            } => {
                self.redeem_friend_invite(session, &code, link_email, lang.as_deref())
                    .await?
            }
            other => {
                let (response, concerns) = self.store.social_concerning(session, other).await?;
                self.tell(&concerns);
                response
            }
        };
        Ok(Response::Social { response })
    }

    async fn create_friend_invite(
        &self,
        session: Session<'_>,
        label: Option<&str>,
        note: Option<&str>,
        email: Option<&str>,
        lang: Option<&str>,
        expires_days: Option<u8>,
    ) -> Result<SocialResponse, Error> {
        let days = invite_expiry_days(expires_days)?;
        let effective_lang = lang
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("en");
        let (invite, code, concerns) = self
            .store
            .create_friend_invite(
                session,
                label,
                note,
                email,
                Some(effective_lang),
                Some(days),
            )
            .await?;
        self.tell(&concerns);

        let mail_sent = match (invite.invited_email.as_deref(), self.mails.mailer.clone()) {
            (Some(recipient), Some(mailer)) => {
                if let Ok((inviter_id, inviter_person)) = self.store.social_caller(session).await {
                    let inviter_handle =
                        profile::handle(&inviter_person.display_name, inviter_person.tag);
                    let mail = Mail::friend_invite(
                        recipient,
                        inviter_id,
                        effective_lang,
                        &inviter_handle,
                        &code,
                        invite.note.as_deref(),
                        days,
                    );
                    self.mails
                        .deliver_checked(mailer, inviter_id, mail)
                        .await
                        .is_ok()
                } else {
                    false
                }
            }
            _ => false,
        };
        Ok(SocialResponse::FriendInviteCreated {
            invite,
            code: code.to_string(),
            mail_sent,
        })
    }

    async fn redeem_friend_invite(
        &self,
        session: Session<'_>,
        code: &str,
        link_email: bool,
        lang: Option<&str>,
    ) -> Result<SocialResponse, Error> {
        let (inviter, pending_email, concerns) = self
            .store
            .redeem_friend_invite(session, code, link_email, lang)
            .await?;
        self.tell(&concerns);

        let mut email_pending = false;
        if let (Some([address, effective_lang]), Some(mailer)) =
            (pending_email, self.mails.mailer.clone())
            && let Ok((account_id, _)) = self.store.social_caller(session).await
            && let Ok(mail_code) = self
                .mails
                .code(mailer, account_id, &address, &effective_lang)
                .await
            && self
                .store
                .save_invite_email_pending(
                    session,
                    &address,
                    &effective_lang,
                    mail_code.digest(&account_id),
                )
                .await
                == Ok(true)
        {
            email_pending = true;
        }

        Ok(SocialResponse::FriendInviteRedeemed {
            inviter,
            email_pending,
        })
    }
}

#[cfg(test)]
mod tests {
    use central_api::social::DEFAULT_FRIEND_INVITE_DAYS;
    use central_mail::{Lang, Mail, What};

    #[test]
    fn friend_invite_mail_construction_preserves_lang_and_target_without_panicking() {
        let mail = Mail::friend_invite(
            "friend@example.org",
            [3; 16],
            "de",
            "Alice#0001",
            "AAAAAAAAAAAAAAAAAAAAAA",
            Some("Let us play!"),
            DEFAULT_FRIEND_INVITE_DAYS,
        );
        assert_eq!(mail.to(), "friend@example.org");
        assert_eq!(mail.what(), What::FriendInvite);
        assert_eq!(mail.lang(), Lang::De);
        assert_eq!(mail.ttl_secs(), None);
    }
}
