//! The contact form: posts to `/site/contact` on this origin, which Caddy
//! passes to Baylee's feedback service (Baylee `docs/feedback.md`): the
//! message, the name and address only if given, the page's language and
//! the honeypot. Without script the form cannot be sent at all (the CSP's
//! `form-action 'none'`), so nothing ever ends up in an address bar.

use leptos::prelude::*;

use super::net;
use crate::words::{self, Lang};

/// The longest message the feedback service takes, in characters.
const MAX_MESSAGE: usize = 5000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Ready,
    Sending,
    Sent,
    Empty,
    Email,
    Busy,
    Refused,
    Network,
}

/// Whether `email` looks like an address (one `@`, a dot after it, no
/// spaces), as the feedback service checks it.
fn plausible(email: &str) -> bool {
    let Some((local, domain)) = email.split_once('@') else {
        return false;
    };
    !local.is_empty()
        && !domain.contains('@')
        && domain
            .split_once('.')
            .is_some_and(|(a, b)| !a.is_empty() && !b.is_empty())
        && !email.chars().any(char::is_whitespace)
}

fn body(name: &str, email: &str, message: &str, website: &str, lang: Lang) -> String {
    let optional = |value: &str| {
        let value = value.trim();
        if value.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::Value::from(value)
        }
    };
    serde_json::json!({
        "name": optional(name),
        "email": optional(email),
        "message": message.trim(),
        "language": lang.code(),
        "website": website,
    })
    .to_string()
}

/// The form's values and state, all `Copy`.
#[derive(Clone, Copy)]
struct Form {
    state: RwSignal<State>,
    name: RwSignal<String>,
    email: RwSignal<String>,
    message: RwSignal<String>,
    trap: RwSignal<String>,
}

/// Checks the form and sends it.
fn submit(lang: Lang, form: Form, event: &leptos::ev::SubmitEvent) {
    let Form {
        state,
        name,
        email,
        message,
        trap,
    } = form;
    event.prevent_default();
    if state.get_untracked() == State::Sending {
        return;
    }
    let text = message.get_untracked();
    if text.trim().is_empty() {
        state.set(State::Empty);
        return;
    }
    let address = email.get_untracked();
    if !address.trim().is_empty() && !plausible(address.trim()) {
        state.set(State::Email);
        return;
    }
    state.set(State::Sending);
    let json = body(
        &name.get_untracked(),
        &address,
        &text,
        &trap.get_untracked(),
        lang,
    );
    leptos::task::spawn_local(async move {
        let outcome = match net::request(
            "POST",
            "/site/contact",
            Some(&json),
            &[],
            net::Credentials::Omit,
        )
        .await
        {
            Ok((201, _)) => State::Sent,
            Ok((429, _)) => State::Busy,
            Ok((400 | 413, _)) => State::Refused,
            _ => State::Network,
        };
        state.set(outcome);
    });
}

#[island]
pub fn ContactForm(lang: Lang) -> impl IntoView {
    let state = RwSignal::new(State::Ready);
    let name = RwSignal::new(String::new());
    let email = RwSignal::new(String::new());
    let message = RwSignal::new(String::new());
    let trap = RwSignal::new(String::new());
    let count = move || message.with(|m| m.chars().count());
    let near = move || count() * 10 >= MAX_MESSAGE * 9;
    let form = Form {
        state,
        name,
        email,
        message,
        trap,
    };
    let send = move |event: leptos::ev::SubmitEvent| submit(lang, form, &event);
    let problem = move || match state.get() {
        State::Empty => Some(words::contact_error_empty(lang)),
        State::Email => Some(words::contact_error_email(lang)),
        State::Busy => Some(words::contact_error_busy(lang)),
        State::Refused => Some(words::contact_error_refused(lang)),
        State::Network => Some(words::contact_error_network(lang)),
        State::Ready | State::Sending | State::Sent => None,
    };
    let field_error = move |which: State| move || state.get() == which;
    view! {
        <Show
            when=move || state.get() == State::Sent
            fallback=move || {
                view! {
                    <form class="form" method="post" novalidate=true on:submit=send>
                        <div class="field">
                            <label for="contact-name">{words::contact_name(lang)}</label>
                            <input id="contact-name" name="name" type="text" autocomplete="name" maxlength="100"
                                aria-describedby="contact-name-hint"
                                on:input=move |ev| name.set(event_target_value(&ev))
                                prop:value=move || name.get() />
                            <p class="hint" id="contact-name-hint">{words::contact_name_hint(lang)}</p>
                        </div>
                        <div class="field">
                            <label for="contact-email">{words::contact_email(lang)}</label>
                            <input id="contact-email" name="email" type="email" autocomplete="email" maxlength="254"
                                aria-describedby="contact-email-hint contact-error"
                                aria-invalid=move || field_error(State::Email)().then_some("true")
                                on:input=move |ev| email.set(event_target_value(&ev))
                                prop:value=move || email.get() />
                            <p class="hint" id="contact-email-hint">{words::contact_email_hint(lang)}</p>
                        </div>
                        <div class="field">
                            <label for="contact-message">{words::contact_message(lang)}</label>
                            <textarea id="contact-message" name="message" rows="7" maxlength=MAX_MESSAGE.to_string() required=true
                                aria-describedby="contact-count contact-error"
                                aria-invalid=move || field_error(State::Empty)().then_some("true")
                                on:input=move |ev| message.set(event_target_value(&ev))
                                prop:value=move || message.get()></textarea>
                            <p class="hint num" id="contact-count" class:near=near>
                                {move || words::contact_count(lang, &words::number(lang, u64::try_from(count()).unwrap_or(u64::MAX)))}
                            </p>
                        </div>
                        <div class="trap" aria-hidden="true">
                            <label for="contact-website">Website</label>
                            <input id="contact-website" name="website" type="text" tabindex="-1" autocomplete="off"
                                on:input=move |ev| trap.set(event_target_value(&ev)) />
                        </div>
                        <p class="error" id="contact-error" role="alert">{problem}</p>
                        <button type="submit" class="button primary" aria-disabled=move || (state.get() == State::Sending).then_some("true")>
                            {move || if state.get() == State::Sending { words::contact_sending(lang) } else { words::contact_send(lang) }}
                        </button>
                    </form>
                }
            }
        >
            <div class="card done" role="status">
                <h2>{words::contact_sent_title(lang)}</h2>
                <p>{words::contact_sent(lang)}</p>
            </div>
        </Show>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses() {
        assert!(plausible("a@b.c"));
        assert!(!plausible("a@b"));
        assert!(!plausible("a b@c.d"));
        assert!(!plausible("@b.c"));
        assert!(!plausible("a@@b.c"));
    }

    #[test]
    fn body_leaves_out_what_was_not_given() {
        let json: serde_json::Value =
            serde_json::from_str(&body(" ", "", " Hi ", "", Lang::De)).expect("json");
        assert_eq!(
            json,
            serde_json::json!({"name": null, "email": null, "message": "Hi", "language": "de", "website": ""})
        );
    }
}
