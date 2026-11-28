//! Central's browser surface (`docs/web-accounts.md`): `POST /api/v1` on this
//! origin, the session in the `__Host-central` cookie only (the page never
//! sees it), `X-Baylee-Central: 1` so a foreign page needs a preflight it
//! never gets. Requests and answers are `central-api`'s own types, so the
//! browser speaks exactly what the native client speaks: every answer is
//! parsed strictly (`deny_unknown_fields`, exact byte lengths) and a
//! refusal is read by its status only, never by its text.

use central_api::admin::AdminRequest;
use central_api::chat::ChatRequest;
use central_api::email;
use central_api::groups::GroupRequest;
use central_api::owner::OwnerRequest;
use central_api::profile::{ProfileDetails, PublicBadges, PublicProfile};
use central_api::social::{SocialRequest, SocialResponse, TaggedRank};
use central_api::wire::{Request, Response};

use super::net::{self, Credentials};
use super::session;

/// Why a request did not give an answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    /// 401: the request or the session was refused (also a wrong password).
    Rejected,
    /// 403: not allowed, or Central's terms are not accepted.
    Forbidden,
    /// 409: a conflict (a taken name, a code already kept).
    Conflict,
    /// 429: too many requests just now.
    Busy,
    /// 5xx: Central is unavailable.
    Unavailable,
    /// An answer of another shape than the request expects.
    Malformed,
    /// No answer at all: no network, a redirect.
    Network,
}

#[cfg(target_arch = "wasm32")]
mod busy {
    use std::cell::Cell;
    use wasm_bindgen::{JsCast, closure::Closure};

    thread_local! {
        static IN_FLIGHT: Cell<u32> = const { Cell::new(0) };
        static TIMER: Cell<i32> = const { Cell::new(0) };
    }

    pub struct Guard;

    impl Guard {
        pub fn enter() -> Self {
            IN_FLIGHT.with(|count| {
                let next = count.get().saturating_add(1);
                count.set(next);
                if next == 1 {
                    schedule_show();
                }
            });
            Self
        }
    }

    impl Drop for Guard {
        fn drop(&mut self) {
            IN_FLIGHT.with(|count| {
                let next = count.get().saturating_sub(1);
                count.set(next);
                if next == 0 {
                    cancel_show();
                    set_busy(false);
                }
            });
        }
    }

    fn set_busy(on: bool) {
        if let Some(root) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.document_element())
        {
            if on {
                let _ = root.set_attribute("data-busy", "");
            } else {
                let _ = root.remove_attribute("data-busy");
            }
        }
    }

    fn cancel_show() {
        TIMER.with(|t| {
            let id = t.replace(0);
            if id != 0
                && let Some(window) = web_sys::window()
            {
                window.clear_timeout_with_handle(id);
            }
        });
    }

    fn schedule_show() {
        cancel_show();
        let Some(window) = web_sys::window() else {
            return;
        };
        let cb = Closure::once(move || {
            TIMER.with(|t| t.set(0));
            IN_FLIGHT.with(|count| {
                if count.get() > 0 {
                    set_busy(true);
                }
            });
        });
        if let Ok(id) = window
            .set_timeout_with_callback_and_timeout_and_arguments_0(cb.as_ref().unchecked_ref(), 120)
        {
            TIMER.with(|t| t.set(id));
        }
        cb.forget();
    }
}

async fn send_once(request: &Request) -> Result<Response, Failure> {
    let body = serde_json::to_string(request).map_err(|_| Failure::Malformed)?;
    let (status, text) = net::request(
        "POST",
        "/api/v1",
        Some(&body),
        &[("X-Baylee-Central", "1")],
        Credentials::SameOrigin,
    )
    .await
    .map_err(|_| Failure::Network)?;
    match status {
        200 => serde_json::from_str(&text).map_err(|_| Failure::Malformed),
        401 => Err(Failure::Rejected),
        403 => Err(Failure::Forbidden),
        409 => Err(Failure::Conflict),
        429 => Err(Failure::Busy),
        500..=599 => Err(Failure::Unavailable),
        _ => Err(Failure::Malformed),
    }
}

const fn may_refresh_after(request: &Request) -> bool {
    // a public read needs no session, so an expired one must not stop it
    // (the leaderboards' boards and seasons)
    if let Request::Ranked { request } = request
        && request.is_public()
    {
        return false;
    }
    !matches!(
        request,
        Request::RefreshSession
            | Request::Logout
            | Request::LoginStart { .. }
            | Request::LoginFinish { .. }
            | Request::Register { .. }
            | Request::RegisterChallenge
            | Request::RegisterChallengeFor { .. }
            | Request::RegisterInviteChallenge { .. }
            | Request::Discover { .. }
            | Request::PublicProfile { .. }
            | Request::FriendInviteInfo { .. }
            | Request::ResetStart { .. }
            | Request::ResetFinish { .. }
            | Request::ResetMail { .. }
            | Request::DeleteAccountFinish { .. }
            | Request::GatewayDirectory { .. }
            | Request::PublicFigures { .. }
            | Request::PublicFunding {}
            | Request::Admin {
                request: AdminRequest::LegacyAccounts,
            }
    )
}

const fn may_retry_after_401(request: &Request) -> bool {
    // Only idempotent read-only queries may rotate session tokens and replay on
    // 401; mutating or proof-checked operations (e.g. RedeemFriendInvite,
    // Email, ChangePassword*, DeleteAccount*, SetRecovery, SetProfileDetails,
    // SendRecoveryBackupMail, Owner/Admin mutations) can return 401 Rejected on
    // validation failure rather than access expiry.
    matches!(
        request,
        Request::SessionStatus
            | Request::AccountSummary
            | Request::TermsStatus
            | Request::Owner {
                request: OwnerRequest::Sessions | OwnerRequest::Gateways
            }
            | Request::Social {
                request: SocialRequest::Contacts
                    | SocialRequest::Lookup { .. }
                    | SocialRequest::Search { .. }
                    | SocialRequest::Pictures { .. }
                    | SocialRequest::Ranks { .. }
                    | SocialRequest::CardSuggest { .. }
                    | SocialRequest::CardPrintings { .. }
                    | SocialRequest::FriendInvites
                    | SocialRequest::Group {
                        request: GroupRequest::List
                    }
                    | SocialRequest::Chat {
                        request: ChatRequest::Inbox { .. }
                    }
            }
    ) || matches!(
        request,
        Request::Admin { request }
            if !request.changes() && !matches!(request, AdminRequest::LegacyAccounts)
    )
}

#[cfg(target_arch = "wasm32")]
async fn single_flight_refresh() -> bool {
    use std::{
        cell::RefCell,
        future::Future,
        pin::Pin,
        task::{Context, Poll, Waker},
    };

    struct Slot {
        running: bool,
        ok: bool,
        waiters: Vec<Waker>,
    }

    thread_local! {
        static SLOT: RefCell<Slot> = const {
            RefCell::new(Slot {
                running: false,
                ok: false,
                waiters: Vec::new(),
            })
        };
    }

    struct WaitDone;
    impl Future for WaitDone {
        type Output = bool;
        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<bool> {
            SLOT.with(|cell| {
                let mut slot = cell.borrow_mut();
                if slot.running {
                    slot.waiters.push(cx.waker().clone());
                    Poll::Pending
                } else {
                    Poll::Ready(slot.ok)
                }
            })
        }
    }

    let leader = SLOT.with(|cell| {
        let mut slot = cell.borrow_mut();
        if slot.running {
            false
        } else {
            slot.running = true;
            slot.ok = false;
            true
        }
    });
    if !leader {
        return WaitDone.await;
    }

    let ok = match send_once(&Request::RefreshSession).await {
        Ok(Response::Renewed { expires_at }) => {
            session::renew(expires_at);
            true
        }
        Err(Failure::Rejected) => {
            if session::access_expired() {
                session::forget();
            }
            false
        }
        _ => false,
    };
    SLOT.with(|cell| {
        let mut slot = cell.borrow_mut();
        slot.running = false;
        slot.ok = ok;
        for waker in slot.waiters.drain(..) {
            waker.wake();
        }
    });
    ok
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(clippy::unused_async)]
async fn single_flight_refresh() -> bool {
    false
}

/// Starts a background refresh timer (every 12 minutes) on signed-in browser
/// pages so active visits silently slide the 3-day refresh window without an
/// interruption.
#[cfg(target_arch = "wasm32")]
pub fn start_refresh_keepalive() {
    use wasm_bindgen::{JsCast, closure::Closure};

    let Some(window) = web_sys::window() else {
        return;
    };
    let cb = Closure::<dyn FnMut()>::new(|| {
        if session::remembered().is_some() {
            leptos::task::spawn_local(async {
                let _ = single_flight_refresh().await;
            });
        }
    });
    if let Ok(id) = window.set_interval_with_callback_and_timeout_and_arguments_0(
        cb.as_ref().unchecked_ref(),
        720_000,
    ) {
        cb.forget();
        leptos::prelude::on_cleanup(move || {
            if let Some(win) = web_sys::window() {
                win.clear_interval_with_handle(id);
            }
        });
    } else {
        cb.forget();
    }
}

/// Server stub for background refresh timer.
#[cfg(not(target_arch = "wasm32"))]
pub fn start_refresh_keepalive() {}

/// Sends `request` and reads Central's answer, automatically rotating the
/// short-lived session cookie once via [`Request::RefreshSession`] if it
/// expires mid-visit.
///
/// # Errors
/// The refusal, by its status.
pub async fn call(request: &Request) -> Result<Response, Failure> {
    #[cfg(target_arch = "wasm32")]
    let _guard = busy::Guard::enter();
    if may_refresh_after(request) && session::access_expired() && !single_flight_refresh().await {
        return Err(Failure::Rejected);
    }
    match send_once(request).await {
        Err(Failure::Rejected)
            if may_retry_after_401(request) && session::remembered().is_some() =>
        {
            if single_flight_refresh().await {
                send_once(request).await
            } else {
                Err(Failure::Rejected)
            }
        }
        other => other,
    }
}

/// Fetch extended private profile fields (`first_name`, `last_name`, `birth_date`, `public_profile`).
///
/// # Errors
/// Returns a [`Failure`] when the request is rejected or Central is unreachable.
pub async fn profile_details() -> Result<ProfileDetails, Failure> {
    match call(&Request::ProfileDetails).await? {
        Response::ProfileDetails { details } => Ok(details),
        _ => Err(Failure::Malformed),
    }
}

/// Save extended private profile fields (`first_name`, `last_name`, `birth_date`, `public_profile`).
///
/// # Errors
/// Returns a [`Failure`] when validation fails or the session is rejected.
pub async fn set_profile_details(details: ProfileDetails) -> Result<(), Failure> {
    match call(&Request::SetProfileDetails { details }).await? {
        Response::Done => Ok(()),
        _ => Err(Failure::Malformed),
    }
}

/// Export extended profile fields for inclusion in GDPR export data.
///
/// # Errors
/// Returns a [`Failure`] when the request fails or the session is rejected.
pub async fn export_profile_details() -> Result<ProfileDetails, Failure> {
    match call(&Request::ExportProfileDetails).await? {
        Response::ProfileDetails { details } => Ok(details),
        _ => Err(Failure::Malformed),
    }
}

/// Fetch the signed-in account's unlocked badges (ranked + challenges + tower + echoes).
///
/// # Errors
/// Returns a [`Failure`] when the request fails or the session is rejected.
pub async fn own_badges() -> Result<PublicBadges, Failure> {
    match call(&Request::OwnBadges).await? {
        Response::OwnBadges { badges } => Ok(*badges),
        _ => Err(Failure::Malformed),
    }
}

/// Fetch a public player profile by handle (`Name#tag`).
///
/// # Errors
/// Returns a [`Failure`] when the player is hidden/unknown or Central is unavailable.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub async fn public_profile(handle: &str) -> Result<PublicProfile, Failure> {
    match call(&Request::PublicProfile {
        handle: handle.to_owned(),
    })
    .await?
    {
        Response::PublicProfile { profile } => Ok(*profile),
        _ => Err(Failure::Malformed),
    }
}

/// Save onboarding pending email (if unconfirmed) and deliver the recovery-code ZIP backup email.
///
/// # Errors
/// Returns a [`Failure`] when mail delivery fails or the session is rejected.
pub async fn send_recovery_backup_mail(
    address: &str,
    attachment_base64: &str,
    encrypted: bool,
    lang: &str,
) -> Result<(), Failure> {
    email::address(address).map_err(|_| Failure::Malformed)?;
    match call(&Request::SendRecoveryBackupMail {
        email: address.to_owned(),
        attachment_base64: attachment_base64.to_owned(),
        encrypted,
        lang: lang.to_owned(),
    })
    .await?
    {
        Response::Done => Ok(()),
        _ => Err(Failure::Malformed),
    }
}

/// Rename an owner-registered gateway.
///
/// # Errors
/// Returns a [`Failure`] when the name is rejected or step-up proof has expired.
pub async fn update_gateway(gateway: [u8; 16], name: String) -> Result<(), Failure> {
    match call(&Request::Owner {
        request: OwnerRequest::UpdateGateway { gateway, name },
    })
    .await?
    {
        Response::Done => Ok(()),
        _ => Err(Failure::Malformed),
    }
}

/// Soft-delete an owner-registered gateway.
///
/// # Errors
/// Returns a [`Failure`] when deletion fails or step-up proof has expired.
pub async fn delete_gateway(gateway: [u8; 16]) -> Result<(), Failure> {
    match call(&Request::Owner {
        request: OwnerRequest::DeleteGateway { gateway },
    })
    .await?
    {
        Response::Done => Ok(()),
        _ => Err(Failure::Malformed),
    }
}

/// Fetch highest current-season player ranks for a list of peer tags.
///
/// # Errors
/// Returns a [`Failure`] when the social request fails.
pub async fn social_ranks(tags: Vec<u32>) -> Result<Vec<TaggedRank>, Failure> {
    if tags.is_empty() {
        return Ok(Vec::new());
    }
    match call(&Request::Social {
        request: SocialRequest::Ranks { tags },
    })
    .await?
    {
        Response::Social {
            response: SocialResponse::Ranks { ranks },
        } => Ok(ranks),
        _ => Err(Failure::Malformed),
    }
}

/// Whether `id` is a `UUIDv7`, as Central makes every account's.
#[must_use]
pub fn uuid_v7(id: &[u8; 16]) -> bool {
    id[6] >> 4 == 7 && id[8] >> 6 == 2
}

/// A sign-in name as Central keeps it: 3 to 24 of `a-z`, `0-9`, `_`, `.`,
/// `-`, starting with a letter or digit.
#[must_use]
pub fn valid_username(name: &str) -> bool {
    (3..=24).contains(&name.len())
        && name
            .bytes()
            .next()
            .is_some_and(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        && name.bytes().all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'.' | b'-')
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usernames() {
        assert!(valid_username("erika"));
        assert!(valid_username("e.r-k_9"));
        assert!(!valid_username("Erika"));
        assert!(!valid_username("er"));
        assert!(!valid_username("_erika"));
        assert!(!valid_username(&"e".repeat(25)));
    }

    #[test]
    fn requests_serialize_as_central_reads_them() {
        let json = serde_json::to_string(&Request::Discover {
            username: "erika".into(),
        })
        .expect("json");
        assert_eq!(json, r#"{"operation":"discover","username":"erika"}"#);
        let answer: Response = serde_json::from_str(r#"{"result":"done"}"#).expect("done");
        assert!(matches!(answer, Response::Done));
        let renewed = r#"{"result":"renewed","expires_at":1}"#;
        assert!(matches!(
            serde_json::from_str(renewed),
            Ok(Response::Renewed { expires_at: 1 })
        ));
        let extra = r#"{"result":"renewed","expires_at":1,"extra":1}"#;
        assert!(
            serde_json::from_str::<Response>(extra).is_err(),
            "unknown fields are refused"
        );
    }

    #[test]
    fn legacy_accounts_does_not_trigger_session_refresh() {
        assert!(!may_refresh_after(&Request::Admin {
            request: AdminRequest::LegacyAccounts,
        }));
        assert!(!may_refresh_after(&Request::RefreshSession));
        assert!(may_refresh_after(&Request::ProfileDetails));
    }

    /// A visitor whose remembered session ran out still reads the public
    /// leaderboards; the account's own ranked data still refreshes first.
    #[test]
    fn public_ranked_reads_do_not_wait_for_a_session() {
        use central_api::ranked::{self, Format, League, Track};
        let board = Request::Ranked {
            request: ranked::Request::Board {
                season: None,
                format: Format::Standard,
                track: Track::Player,
                league: League::Champion,
                division: None,
                after: None,
                players: ranked::ai::BoardPlayers::Everyone,
            },
        };
        assert!(!may_refresh_after(&board));
        assert!(!may_refresh_after(&Request::Ranked {
            request: ranked::Request::Seasons { before: None },
        }));
        assert!(may_refresh_after(&Request::Ranked {
            request: ranked::Request::Export {
                table: ranked::export::Table::Seasons,
                after: None,
            },
        }));
    }

    #[test]
    fn mutating_and_proof_requests_never_retry_after_401() {
        use central_api::admin::AdminRequest;
        assert!(may_retry_after_401(&Request::AccountSummary));
        assert!(may_retry_after_401(&Request::Social {
            request: SocialRequest::Contacts,
        }));
        assert!(may_retry_after_401(&Request::Social {
            request: SocialRequest::FriendInvites,
        }));
        assert!(may_retry_after_401(&Request::Owner {
            request: OwnerRequest::Gateways,
        }));
        assert!(may_retry_after_401(&Request::Admin {
            request: AdminRequest::Overview,
        }));

        assert!(!may_retry_after_401(&Request::Social {
            request: SocialRequest::RedeemFriendInvite {
                code: "AAAAAAAAAAAAAAAAAAAAAA".into(),
                link_email: true,
                lang: None,
            },
        }));
        assert!(!may_retry_after_401(&Request::Social {
            request: SocialRequest::CreateFriendInvite {
                label: None,
                note: None,
                email: None,
                lang: None,
                expires_days: None,
            },
        }));
        assert!(!may_retry_after_401(&Request::SendRecoveryBackupMail {
            email: "a@example.org".into(),
            attachment_base64: "AAAA".into(),
            encrypted: false,
            lang: "en".into(),
        }));
        assert!(!may_retry_after_401(&Request::SetDisplayName {
            display_name: "Alice".into(),
        }));
        assert!(!may_retry_after_401(&Request::Owner {
            request: OwnerRequest::DeleteGateway { gateway: [0; 16] },
        }));
        assert!(!may_retry_after_401(&Request::Email {
            request: email::EmailRequest::Status,
        }));
        assert!(!may_retry_after_401(&Request::Admin {
            request: AdminRequest::SetRegistration { open: true },
        }));
    }
}
