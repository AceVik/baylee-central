//! Changing the signed-in account's password (`docs/native-login.md`
//! §"Password change"). A browser keeps no master key after a sign-in, so
//! the form signs in once more with the current password inside one
//! Worker, which then wraps the same master key under the new one: the
//! encrypted data and the recovery code stay. Central ends every session
//! of the account and answers a new one for this browser.

use central_api::wire::{Request, Response};
use leptos::prelude::*;
use serde_json::json;

use super::auth::{Problem, problem_text};
use super::central::{Failure, call};
use super::chat::glyph;
use super::crypto::{Crypto, field_bytes};
use super::field::{self, Labelled};
use super::session;
use crate::words::{self, Lang};

/// What the form lacks, before anything is sent.
fn form_problem(lang: Lang, current: &str, new: &str, repeat: &str) -> Option<&'static str> {
    if current.is_empty() || new.is_empty() {
        return Some(words::password_missing(lang));
    }
    if new.chars().count() < 8 {
        return Some(words::register_error_short(lang));
    }
    if new.len() > 1024 {
        return Some(words::register_error_long(lang));
    }
    if new != repeat {
        return Some(words::register_error_repeat(lang));
    }
    None
}

/// Why the password is not known to be changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Trouble {
    /// The current password is not right: nothing was changed.
    Current,
    /// This browser is not signed in any more: nothing was changed.
    SignedOut,
    /// The finish's answer is lost: the change may have happened.
    Unsure,
    /// Anything else, in the forms' shared words: nothing was changed.
    Other(Problem),
}

fn other(failure: Failure) -> Trouble {
    Trouble::Other(Problem::outside_sign_in(failure))
}

/// The Worker's refusal of the current password, at the step that checks
/// it; anything else of the Worker's is no one's wrong password.
fn by_worker(failure: Failure, checks_password: bool) -> Trouble {
    match failure {
        Failure::Rejected if checks_password => Trouble::Current,
        _ => Trouble::Other(Problem::Failed),
    }
}

/// What a refused finish means; `None` when its answer is lost and the
/// change may have happened all the same.
fn at_finish(failure: Failure) -> Option<Trouble> {
    match failure {
        // the attempt ran out, or the session or the credential changed
        // meanwhile: Central changes nothing then
        Failure::Rejected | Failure::Forbidden | Failure::Conflict => {
            Some(Trouble::Other(Problem::Failed))
        }
        Failure::Busy => Some(Trouble::Other(Problem::Busy)),
        Failure::Unavailable | Failure::Malformed | Failure::Network => None,
    }
}

/// Signs the session's account in once more in `crypto` with `current`:
/// the Worker then holds the key that opens the master key, and the answer
/// is the wrapper it opens.
async fn sign_in_again(crypto: &Crypto, current: &str) -> Result<Vec<u8>, Trouble> {
    let identity = match call(&Request::AccountSummary).await {
        Ok(Response::AccountSummary { identity, .. }) => identity,
        Ok(_) => return Err(Trouble::Other(Problem::Failed)),
        Err(Failure::Rejected) => return Err(Trouble::SignedOut),
        Err(failure) => return Err(other(failure)),
    };
    let start = crypto
        .ask(
            "login_start",
            json!({ "password": current, "account": identity.account, "epoch": identity.epoch }),
        )
        .await
        .map_err(|failure| by_worker(failure, false))?;
    let message = field_bytes(&start, "message", Some(96)).map_err(other)?;
    let Response::LoginChallenge {
        attempt,
        message: challenge,
    } = call(&Request::LoginStart { identity, message })
        .await
        .map_err(other)?
    else {
        return Err(Trouble::Other(Problem::Failed));
    };
    // A wrong current password fails here, in the Worker, before anything
    // more is sent.
    let finish = crypto
        .ask("login_finish", json!({ "response": challenge }))
        .await
        .map_err(|failure| by_worker(failure, true))?;
    let proof = field_bytes(&finish, "message", Some(64)).map_err(other)?;
    let Response::BrowserSession {
        identity: signed,
        wrapped_master,
        expires_at,
        ..
    } = call(&Request::LoginFinish {
        attempt,
        message: proof,
    })
    .await
    .map_err(other)?
    else {
        return Err(Trouble::Other(Problem::Failed));
    };
    if signed.account != identity.account || signed.epoch != identity.epoch {
        return Err(Trouble::Other(Problem::Failed));
    }
    session::remember(expires_at);
    Ok(wrapped_master)
}

/// After a finish whose answer is lost. The change ends the session that
/// asked for it, so a session Central no longer knows is forgotten here;
/// whether the password changed, the page cannot say either way (the new
/// session's cookie may have arrived without its answer).
async fn lost() -> Trouble {
    if matches!(call(&Request::AccountSummary).await, Err(Failure::Rejected)) {
        session::forget();
    }
    Trouble::Unsure
}

/// Changes the password from `current` to `new` in a fresh Worker.
///
/// # Errors
/// Why the password is not known to be changed.
async fn change(worker: &str, current: String, new: String) -> Result<(), Trouble> {
    let crypto = Crypto::start(worker).ok_or(Trouble::Other(Problem::Failed))?;
    let wrapped_master = sign_in_again(&crypto, &current).await?;
    let begun = crypto
        .ask(
            "password_start",
            json!({ "wrapped_master": wrapped_master, "current": current, "new": new }),
        )
        .await
        .map_err(|failure| by_worker(failure, false))?;
    let login = field_bytes(&begun, "login", Some(96)).map_err(other)?;
    let registration = field_bytes(&begun, "registration", Some(32)).map_err(other)?;
    let Response::PasswordChallenge {
        attempt,
        login: challenge,
        registration: response,
    } = call(&Request::ChangePasswordStart {
        login,
        registration,
    })
    .await
    .map_err(other)?
    else {
        return Err(Trouble::Other(Problem::Failed));
    };
    let done = crypto
        .ask(
            "password_finish",
            json!({ "challenge": challenge, "registration": response }),
        )
        .await
        .map_err(|failure| by_worker(failure, false))?;
    let proof = field_bytes(&done, "proof", Some(64)).map_err(other)?;
    let record = field_bytes(&done, "record", None).map_err(other)?;
    let sent = field_bytes(&done, "wrapped_master", Some(73)).map_err(other)?;
    crypto.close();
    let finish = Request::ChangePasswordFinish {
        attempt,
        proof,
        record,
        wrapped_master: sent.clone(),
    };
    match call(&finish).await {
        Ok(Response::BrowserSession {
            wrapped_master: stored,
            expires_at,
            ..
        }) => {
            session::remember(expires_at);
            // Central stores what the Worker made, or something is wrong
            if stored == sent {
                Ok(())
            } else {
                Err(Trouble::Unsure)
            }
        }
        Ok(_) => Err(lost().await),
        Err(failure) => match at_finish(failure) {
            Some(trouble) => Err(trouble),
            None => Err(lost().await),
        },
    }
}

/// Where the form stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    Form,
    Working,
    Changed,
}

/// What the form holds; the passwords go when it is sent.
#[derive(Clone, Copy)]
struct Held {
    current: RwSignal<String>,
    new: RwSignal<String>,
    repeat: RwSignal<String>,
    error: RwSignal<Option<&'static str>>,
}

fn form_view(
    lang: Lang,
    held: Held,
    username: String,
    submit: impl Fn(leptos::ev::SubmitEvent) + 'static,
) -> impl IntoView {
    let text = move |what: Labelled, held: RwSignal<String>| {
        field::labelled(lang, what, move || held.get(), move |value| held.set(value))
    };
    let spec = |id, label, hint, mark, auto| Labelled {
        id,
        label,
        hint,
        mark,
        auto,
        max: "1024",
    };
    view! {
        <form class="form" method="post" novalidate=true on:submit=submit>
            // for a password manager: whose password this is
            <input type="text" autocomplete="username" readonly=true hidden=true value=username />
            {text(spec("password-current", words::password_current(lang), None, glyph::LOCK, "current-password"), held.current)}
            {text(spec("password-new", words::password_new(lang), Some(words::register_password_hint(lang)), glyph::KEY, "new-password"), held.new)}
            {text(spec("password-repeat", words::password_new_repeat(lang), Some(words::register_repeat_hint(lang)), glyph::LOCK_KEYHOLE, "new-password"), held.repeat)}
            <p class="error" role="alert">{move || held.error.get()}</p>
            <button type="submit" class="button primary">{words::password_change_submit(lang)}</button>
        </form>
    }
}

/// The form in the account area's security section, then what became of
/// the change. `changed` runs after a change (the sessions listed beside
/// it are others now), `signed_out` when this browser turns out not to be
/// signed in.
pub fn change_form(
    lang: Lang,
    worker: String,
    username: String,
    changed: Callback<()>,
    signed_out: Callback<()>,
) -> impl IntoView {
    let step = RwSignal::new(Step::Form);
    let held = Held {
        current: RwSignal::new(String::new()),
        new: RwSignal::new(String::new()),
        repeat: RwSignal::new(String::new()),
        error: RwSignal::new(None),
    };
    let submit = move |event: leptos::ev::SubmitEvent| {
        event.prevent_default();
        let (old, fresh) = (held.current.get_untracked(), held.new.get_untracked());
        if let Some(problem) = form_problem(lang, &old, &fresh, &held.repeat.get_untracked()) {
            held.error.set(Some(problem));
            return;
        }
        for password in [held.current, held.new, held.repeat] {
            password.set(String::new());
        }
        held.error.set(None);
        step.set(Step::Working);
        let worker = worker.clone();
        leptos::task::spawn_local(async move {
            let said = match change(&worker, old, fresh).await {
                Ok(()) => {
                    step.set(Step::Changed);
                    changed.run(());
                    return;
                }
                Err(Trouble::SignedOut) => {
                    session::forget();
                    signed_out.run(());
                    return;
                }
                Err(Trouble::Current) => words::password_error_current(lang),
                Err(Trouble::Unsure) => words::password_unsure(lang),
                Err(Trouble::Other(problem)) => problem_text(lang, problem),
            };
            held.error.set(Some(said));
            step.set(Step::Form);
            // the form is drawn anew: the keyboard goes on at its start
            super::focus::after_render("#password-current");
        });
    };
    move || match step.get() {
        Step::Form => form_view(lang, held, username.clone(), submit.clone()).into_any(),
        Step::Working => {
            super::focus::when_shown("#password-working");
            view! {
                <p class="working" role="status" tabindex="-1" id="password-working">{words::signin_working(lang)}</p>
            }
            .into_any()
        }
        Step::Changed => {
            super::focus::when_shown("#password-changed");
            view! {
                <p role="status" tabindex="-1" id="password-changed">{words::password_changed(lang)}</p>
            }
            .into_any()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_form_says_what_it_lacks_before_anything_is_sent() {
        let good = ("the old words", "long enough", "long enough");
        assert_eq!(form_problem(Lang::En, good.0, good.1, good.2), None);
        let cases = [
            (("", good.1, good.2), words::password_missing(Lang::En)),
            ((good.0, "", ""), words::password_missing(Lang::En)),
            (
                (good.0, "short", "short"),
                words::register_error_short(Lang::En),
            ),
            (
                (good.0, good.1, "other words"),
                words::register_error_repeat(Lang::En),
            ),
        ];
        for ((current, new, repeat), want) in cases {
            assert_eq!(form_problem(Lang::En, current, new, repeat), Some(want));
        }
        let long = "x".repeat(1025);
        assert_eq!(
            form_problem(Lang::En, good.0, &long, &long),
            Some(words::register_error_long(Lang::En))
        );
    }

    /// Only the step that checks the current password says that it is
    /// wrong; no other refusal reads as a wrong password, of either kind.
    #[test]
    fn only_the_checked_password_is_called_wrong() {
        assert_eq!(by_worker(Failure::Rejected, true), Trouble::Current);
        assert_eq!(
            by_worker(Failure::Rejected, false),
            Trouble::Other(Problem::Failed)
        );
        for failure in [
            Failure::Rejected,
            Failure::Forbidden,
            Failure::Conflict,
            Failure::Busy,
            Failure::Unavailable,
            Failure::Malformed,
            Failure::Network,
        ] {
            assert_ne!(other(failure), Trouble::Other(Problem::WrongLogin));
            assert_ne!(other(failure), Trouble::Current);
            assert_ne!(at_finish(failure), Some(Trouble::Current));
            assert_ne!(
                at_finish(failure),
                Some(Trouble::Other(Problem::WrongLogin))
            );
        }
    }

    /// A finish that Central refused changed nothing; one whose answer did
    /// not arrive may have, and is never reported as "nothing changed".
    #[test]
    fn a_lost_finish_is_no_refusal() {
        for lost in [Failure::Network, Failure::Unavailable, Failure::Malformed] {
            assert_eq!(at_finish(lost), None);
        }
        assert_eq!(
            at_finish(Failure::Rejected),
            Some(Trouble::Other(Problem::Failed))
        );
        assert_eq!(
            at_finish(Failure::Busy),
            Some(Trouble::Other(Problem::Busy))
        );
    }
}
