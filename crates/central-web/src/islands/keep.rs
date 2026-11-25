//! Ways to keep a recovery code that is shown once, beside copying it
//! (`clip.rs`): a text file, a sheet to print, the browser's password
//! manager, and a mail to oneself (`docs/web-accounts.md`, "Keeping the
//! recovery code"). Each is made in the page when the person asks for it
//! and never before: the file and the mail's address exist from the click
//! on, the sheet is drawn for the print. Nothing of it goes to Central,
//! nothing is logged, kept in the page's storage or put in its address.
//! What the password manager and the mail program are handed, they keep;
//! the step says what that means for the mail.
//!
//! The password manager is reached through `js_sys` alone, as the
//! clipboard is: `PasswordCredential` and `navigator.credentials.store`.
//! Only Chromium's browsers have `PasswordCredential` (Chrome, Edge, Opera,
//! Samsung Internet); Firefox and Safari have neither it nor another way
//! for a page to offer a password, and there the button is not shown.

use std::fmt::Write as _;

use central_api::email::address;
use leptos::prelude::*;

use super::chat::glyph::{self, Shape};
use super::{clip, field, focus, session};
use crate::words::{self, Lang};

/// The file's name.
pub const FILE: &str = "baylee-recovery-code.txt";

/// Lucide's `download` (lucide-static 1.60.0, ISC; NOTICE.md).
const DOWNLOAD: &[Shape] = &[
    Shape::Path("M12 15V3"),
    Shape::Path("M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"),
    Shape::Path("m7 10 5 5 5-5"),
];
/// Lucide's `printer` (lucide-static 1.60.0, ISC; NOTICE.md).
const PRINTER: &[Shape] = &[
    Shape::Path("M6 18H4a2 2 0 0 1-2-2v-5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2h-2"),
    Shape::Path("M6 9V3a1 1 0 0 1 1-1h10a1 1 0 0 1 1 1v6"),
    Shape::Rect("12", "8", "6", "14", "1"),
];

/// The text of the file and of the mail: what the code is, whose, the
/// code on a line of its own, and two lines on what it does and how to
/// keep it.
#[must_use]
pub fn sheet_text(lang: Lang, code: &str, name: &str) -> String {
    format!(
        "{}\n{}\n\n{code}\n\n{}\n{}\n",
        words::code_sheet_title(lang),
        words::code_sheet_name(lang, name),
        words::code_sheet_what(lang),
        words::code_sheet_keep(lang),
    )
}

/// Whether `byte` stands for itself in an address (RFC 3986's unreserved
/// characters); every other byte is percent-encoded.
fn plain(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b"-._~".contains(&byte)
}

/// `text` percent-encoded byte by byte, but for the plain bytes and those
/// in `keep`.
fn percent(text: &str, keep: &[u8]) -> String {
    let mut out = String::with_capacity(text.len() * 3);
    for byte in text.bytes() {
        if plain(byte) || keep.contains(&byte) {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

/// A `data:` address that a browser saves as a file of `text`.
#[must_use]
pub fn data_url(text: &str) -> String {
    format!("data:text/plain;charset=utf-8,{}", percent(text, &[]))
}

/// The `mailto:` address of a mail to `to` with `subject` and `body`
/// (RFC 6068): the address only where it is one Central would mail to,
/// else none and the mail program asks; lines end in CR LF.
#[must_use]
pub fn mailto(to: &str, subject: &str, body: &str) -> String {
    let to = to.trim();
    let to = if address(to).is_ok() {
        percent(to, b"@")
    } else {
        String::new()
    };
    let body = body.replace("\r\n", "\n").replace('\n', "\r\n");
    format!(
        "mailto:{to}?subject={}&body={}",
        percent(subject, &[]),
        percent(&body, &[])
    )
}

/// The browser's side.
#[cfg(target_arch = "wasm32")]
mod browser {
    use leptos::prelude::*;
    use leptos::wasm_bindgen::closure::Closure;
    use leptos::wasm_bindgen::{JsCast, JsValue};
    use leptos::web_sys;

    /// `name` of the object `of`, if it is one.
    fn get(of: &JsValue, name: &str) -> Option<JsValue> {
        js_sys::Reflect::get(of, &JsValue::from_str(name))
            .ok()
            .filter(|value| !value.is_undefined() && !value.is_null())
    }

    /// Saves `text` as the file `name`: a link made for the click, clicked
    /// and taken away again.
    pub fn save(name: &str, url: &str) {
        let Some(body) = document().body() else {
            return;
        };
        let Ok(link) = document().create_element("a") else {
            return;
        };
        let _ = link.set_attribute("href", url);
        let _ = link.set_attribute("download", name);
        let _ = link.set_attribute("hidden", "");
        if body.append_child(&link).is_ok()
            && let Ok(link) = link.dyn_into::<web_sys::HtmlElement>()
        {
            link.click();
            link.remove();
        }
    }

    /// Prints the page once the browser drew the sheet: the print shows
    /// the sheet alone (`pages.css`).
    pub fn print_when_drawn() {
        let print = Closure::once_into_js(|| {
            let window = js_sys::global();
            if let Some(print) =
                get(&window, "print").and_then(|print| print.dyn_into::<js_sys::Function>().ok())
            {
                let _ = print.call0(&window);
            }
        });
        let _ = window().request_animation_frame(print.unchecked_ref());
    }

    /// The constructor of a password credential, and the container that
    /// stores one, where the browser has both.
    fn manager() -> Option<(js_sys::Function, JsValue, js_sys::Function)> {
        let global = js_sys::global();
        let make = get(&global, "PasswordCredential")?
            .dyn_into::<js_sys::Function>()
            .ok()?;
        let container = get(&get(&global, "navigator")?, "credentials")?;
        let store = get(&container, "store")?
            .dyn_into::<js_sys::Function>()
            .ok()?;
        Some((make, container, store))
    }

    /// Whether this browser lets a page offer a password to its manager.
    pub fn manager_offered() -> bool {
        manager().is_some()
    }

    /// Offers `password` under `id` to the password manager; whether the
    /// browser took the offer. It does not say whether the person saved it.
    pub async fn offer(id: &str, password: &str) -> bool {
        let offered = || {
            let (make, container, store) = manager()?;
            let data = js_sys::Object::new();
            let field = |name: &str, value: &str| {
                js_sys::Reflect::set(&data, &JsValue::from_str(name), &JsValue::from_str(value))
            };
            field("id", id).ok()?;
            field("password", password).ok()?;
            let credential = js_sys::Reflect::construct(&make, &js_sys::Array::of1(&data)).ok()?;
            store
                .call1(&container, &credential)
                .ok()?
                .dyn_into::<js_sys::Promise>()
                .ok()
        };
        match offered() {
            Some(promise) => wasm_bindgen_futures::JsFuture::from(promise).await.is_ok(),
            None => false,
        }
    }
}

/// On the server nothing is saved, printed or offered.
#[cfg(not(target_arch = "wasm32"))]
mod browser {
    pub fn save(_name: &str, _url: &str) {}

    pub fn print_when_drawn() {}

    pub fn manager_offered() -> bool {
        false
    }

    pub fn offer(_id: &str, _password: &str) -> std::future::Ready<bool> {
        std::future::ready(false)
    }
}

/// The sheet a print shows, and nothing else of the page with it.
fn sheet(lang: Lang, code: &str, name: &str) -> impl IntoView + use<> {
    view! {
        <div class="code-sheet">
            <p class="code-sheet-brand">{words::site_name(lang)}</p>
            <p class="code-sheet-title">{words::code_sheet_title(lang)}</p>
            <p>{words::code_sheet_name(lang, name)}</p>
            <p class="code-sheet-code" translate="no"><code>{code.to_owned()}</code></p>
            <p>{words::code_sheet_what(lang)}</p>
            <p>{words::code_sheet_keep(lang)}</p>
        </div>
    }
}

/// Onboarding mail-backup fields when the user picks "Mail it to myself".
#[derive(Clone, Copy)]
pub struct MailBackup {
    /// Whether the user selected the mail-backup choice.
    pub chosen: RwSignal<bool>,
    /// E-mail address entered by the user (saved to the account when sending).
    pub email: RwSignal<String>,
    /// Optional ZIP encryption password; empty sends an unencrypted ZIP.
    pub zip_pw: RwSignal<String>,
}

#[derive(Clone, Copy)]
enum MailWay {
    Draft(RwSignal<String>),
    Backup(MailBackup),
}

/// The one-time recovery code inside its dashed frame, paired with an inline
/// copy-to-clipboard icon button whose tooltip and icon reflect copy state.
pub fn secret_frame(lang: Lang, code: String) -> impl IntoView + use<> {
    let copied = RwSignal::new(false);
    let failed = RwSignal::new(false);
    let copy_code = code.clone();
    let on_copy = move |_| {
        let text = copy_code.clone();
        leptos::task::spawn_local(async move {
            if clip::write(&text).await {
                copied.set(true);
                failed.set(false);
            } else {
                copied.set(false);
                failed.set(true);
            }
        });
    };
    view! {
        <div class="secret-frame">
            <p class="eyebrow">{words::code_shown_once(lang)}</p>
            <div class="secret-row">
                <p class="secret"><code translate="no">{code}</code></p>
                <button
                    type="button"
                    class="secret-copy-btn"
                    data-copied=move || copied.get().then_some("true")
                    title=move || if copied.get() { words::code_copied_tooltip(lang) } else { words::code_copy_tooltip(lang) }
                    aria-label=move || if copied.get() { words::code_copied_tooltip(lang) } else { words::code_copy_tooltip(lang) }
                    on:click=on_copy
                >
                    {move || if copied.get() { glyph::glyph(glyph::CHECK) } else { glyph::glyph(glyph::COPY) }}
                </button>
            </div>
            <Show when=move || failed.get()>
                <p class="meta" role="status">{words::code_copy_failed(lang)}</p>
            </Show>
        </div>
    }
}

/// The ways to keep `code`, the recovery code of the account `name` signs
/// in with: file, print and, where the browser has one, its password
/// manager; then the mail to oneself, to the address in `to` where it
/// holds one, with its warning beside it.
pub fn ways(lang: Lang, code: &str, name: &str, to: RwSignal<String>) -> impl IntoView + use<> {
    ways_inner(lang, code, name, MailWay::Draft(to))
}

/// The onboarding ways to keep `code`, where selecting "Mail it to myself"
/// unfolds the account e-mail input and optional ZIP password input so
/// Central mails `baylee-recovery-code.zip` on continue.
pub fn ways_onboarding(
    lang: Lang,
    code: &str,
    name: &str,
    mail: MailBackup,
) -> impl IntoView + use<> {
    ways_inner(lang, code, name, MailWay::Backup(mail))
}

fn choice_button(
    shapes: &'static [Shape],
    label: &'static str,
    on_click: impl Fn(leptos::ev::MouseEvent) + 'static,
) -> impl IntoView {
    view! {
        <button type="button" class="button keep-choice" on:click=on_click>
            {glyph::glyph(shapes)}
            <span>{label}</span>
        </button>
    }
}

fn mail_backup_panel(lang: Lang, backup: MailBackup) -> AnyView {
    let zip_warn =
        (move || {
            backup.zip_pw.get().is_empty().then(
                || view! { <span class="keep-zip-warn">{words::keep_zip_pw_warn(lang)}</span> },
            )
        })
        .into_any();
    view! {
        <div class="form" id="code-mail-panel">
            <div class="field">
                <label for="code-email">{words::email_title(lang)}</label>
                <div class="inset">
                    {field::mark(glyph::MAIL)}
                    <input
                        id="code-email"
                        type="email"
                        inputmode="email"
                        autocomplete="email"
                        autocapitalize="none"
                        spellcheck="false"
                        maxlength="254"
                        aria-describedby="code-email-warning"
                        on:input=move |ev| backup.email.set(event_target_value(&ev))
                        prop:value=move || backup.email.get()
                    />
                </div>
                <p class="hint" id="code-email-warning">{words::email_warning(lang)}</p>
            </div>
            {field::password_box(
                lang,
                "keep-zip-password",
                words::keep_zip_pw_label(lang),
                "new-password",
                backup.zip_pw,
                Some(words::keep_zip_pw_placeholder(lang)),
                Some(zip_warn),
                "128",
            )}
        </div>
    }
    .into_any()
}

fn mail_draft_panel(lang: Lang) -> AnyView {
    view! {
        <div class="keep-warning" id="code-mail-warning" role="note">
            {glyph::glyph(glyph::SHIELD_ALERT)}
            <p>{words::code_mail_warning(lang)}</p>
        </div>
    }
    .into_any()
}

fn ways_inner(lang: Lang, code: &str, name: &str, way: MailWay) -> impl IntoView + use<> {
    let said = RwSignal::new(None::<&'static str>);
    let printing = RwSignal::new(false);
    let manager = RwSignal::new(false);
    Effect::new(move |_| manager.set(browser::manager_offered()));
    let text = StoredValue::new(sheet_text(lang, code, name));
    let id = StoredValue::new(words::code_manager_id(lang, name));
    let secret = StoredValue::new(code.to_owned());
    let save = move |_| browser::save(FILE, &data_url(&text.read_value()));
    let print = move |_| {
        printing.set(true);
        browser::print_when_drawn();
    };
    let offer = move |_| {
        let (id, secret) = (id.get_value(), secret.get_value());
        leptos::task::spawn_local(async move {
            let msg = if browser::offer(&id, &secret).await {
                words::code_manager_offered(lang)
            } else {
                words::code_manager_failed(lang)
            };
            said.set(Some(msg));
        });
    };
    // In standalone password reset (`Draft`), mail opens the browser's mail
    // program after warning first. In registration onboarding (`Backup`), the
    // choice unfolds the account e-mail and optional ZIP-password fields.
    let asked = RwSignal::new(false);
    let expanded = Signal::derive(move || match way {
        MailWay::Draft(to) => asked.get() || !to.get().trim().is_empty(),
        MailWay::Backup(backup) => backup.chosen.get(),
    });
    let mail = move |_| match way {
        MailWay::Draft(to) => {
            if expanded.get_untracked() {
                session::open(&mailto(
                    &to.get_untracked(),
                    words::code_mail_subject(lang),
                    &text.read_value(),
                ));
                said.set(Some(words::code_mail_opened(lang)));
            } else {
                asked.set(true);
            }
        }
        MailWay::Backup(backup) => {
            let next = !backup.chosen.get_untracked();
            backup.chosen.set(next);
            if next {
                focus::after_render("#code-email");
            }
        }
    };
    let (panel_id, mail_unfold) = match way {
        MailWay::Draft(_) => ("code-mail-warning", mail_draft_panel(lang)),
        MailWay::Backup(backup) => ("code-mail-panel", mail_backup_panel(lang, backup)),
    };
    let (code, name) = (code.to_owned(), name.to_owned());
    view! {
        <div class="keep">
            <p class="keep-title">{words::code_keep_title(lang)}</p>
            <div class="keep-ways">
                {choice_button(DOWNLOAD, words::code_download(lang), save)}
                {choice_button(PRINTER, words::code_print(lang), print)}
                {move || manager.get().then(|| choice_button(glyph::KEY, words::code_manager(lang), offer))}
                <button
                    type="button"
                    class="button keep-choice"
                    aria-controls=panel_id
                    aria-expanded=move || if expanded.get() { "true" } else { "false" }
                    aria-pressed=move || matches!(way, MailWay::Backup(_)).then(|| if expanded.get() { "true" } else { "false" })
                    on:click=mail
                >
                    {glyph::glyph(glyph::MAIL)}
                    <span>{words::code_mail(lang)}</span>
                </button>
            </div>
            <div class="unfold" class:open=move || expanded.get()>
                {mail_unfold}
            </div>
            <p class="meta" role="status">{move || said.get()}</p>
        </div>
        {move || printing.get().then(|| sheet(lang, &code, &name))}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CODE: &str = "0123-4567-89AB-CDEF-GHJK-MNPQ-RSTV-WXYZ";

    #[test]
    fn the_file_holds_the_code_alone_on_its_line_and_two_lines_on_it() {
        for lang in Lang::ALL {
            let text = sheet_text(lang, CODE, "erika");
            let lines: Vec<&str> = text.lines().collect();
            assert_eq!(lines.len(), 7, "{lang:?}");
            assert_eq!(lines[0], words::code_sheet_title(lang));
            assert!(lines[1].contains("erika"));
            assert_eq!(lines[3], CODE);
            assert_eq!(lines[5], words::code_sheet_what(lang));
            assert_eq!(lines[6], words::code_sheet_keep(lang));
            assert_eq!(text.matches(CODE).count(), 1);
        }
    }

    /// The file's address holds the text percent-encoded and nothing a
    /// browser would read as more than one address.
    #[test]
    fn the_file_s_address_is_the_text_encoded() {
        let url = data_url("Baylee: ä\n#?&%");
        assert_eq!(
            url,
            "data:text/plain;charset=utf-8,Baylee%3A%20%C3%A4%0A%23%3F%26%25"
        );
    }

    #[test]
    fn the_mail_is_built_from_its_parts_and_only_to_an_address() {
        let body = sheet_text(Lang::De, CODE, "erika");
        let url = mailto(
            " erika@example.org ",
            "Mein Baylee-Wiederherstellungscode",
            &body,
        );
        let (head, query) = url.split_once('?').expect("a query");
        assert_eq!(head, "mailto:erika@example.org");
        let (subject, body_part) = query.split_once('&').expect("two fields");
        assert_eq!(subject, "subject=Mein%20Baylee-Wiederherstellungscode");
        assert!(body_part.starts_with(
            "body=Baylee-Wiederherstellungscode%0D%0AAnmeldename%3A%20erika%0D%0A%0D%0A0123-4567"
        ));
        // every line ends in CR LF, and nothing in it ends a field early
        assert!(
            !body_part.contains("%0A%0A")
                && !body_part[5..].contains(['&', '?', '#', '=', ' ', '\n'])
        );
        assert_eq!(body_part.matches(CODE).count(), 1);
        // no address, or one Central would not mail to: the mail program asks
        for to in [
            "",
            "erika",
            "erika@localhost",
            "a@b.example?cc=x@example.org",
            "x@example.org\r\nBcc: y@example.org",
        ] {
            assert!(
                mailto(to, "s", "b").starts_with("mailto:?subject=s&body=b"),
                "{to}"
            );
        }
        // what a dot-atom may hold is encoded, the at sign is not
        assert_eq!(
            mailto("a+b&c=d?e@example.org", "s", "b"),
            "mailto:a%2Bb%26c%3Dd%3Fe@example.org?subject=s&body=b"
        );
    }

    /// The id under which a password manager lists the code says what it
    /// is and whose, and is never a sign-in name the site takes.
    #[test]
    fn the_password_manager_s_entry_is_no_sign_in_name() {
        for lang in Lang::ALL {
            let id = words::code_manager_id(lang, "erika");
            assert!(id.contains("erika"), "{lang:?}");
            assert!(
                !super::super::central::valid_username(&id.to_ascii_lowercase()),
                "{id}"
            );
        }
    }
}
