//! The Impressum's details behind a proof of work, as before (Baylee's
//! `docs/feedback.md` §"The Impressum"): the operator's details are in no
//! file of this repository; Baylee's feedback service hands out an
//! obfuscated blob for a solved puzzle. What changed (owner, 10.11.2026):
//! once solved, the details are drawn as ordinary, accessible text, not
//! scrambled and hidden from screen readers.
//!
//! What still keeps a program that reads the finished page from the two
//! things collected most, the e-mail address and the telephone number: no
//! `mailto:` or `tel:` address stands in the page (two buttons build them
//! on a click), and each of the two is written in two halves around a few
//! characters under `hidden`. The page as a person sees, hears, selects and
//! copies it is exact; the page read as markup or as `textContent` holds an
//! address under `.example` and a number that is nobody's. That stops what
//! collects from any page, nothing that was written for this one: whoever
//! solved the puzzle holds the blob, and its key is published above.
//!
//! The puzzle runs in a Worker (`pow-worker.js`): a Worker needs a script of
//! its own, and SHA-256 a million times must not freeze the page.

use leptos::prelude::*;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use super::{net, session};
use crate::words::{self, Lang};

/// The script's half of the obfuscation key, published and no secret: byte
/// for byte Baylee's `crates/baylee-tools/src/site/impressum.rs`, or
/// existing blobs no longer decode.
const SCRIPT_KEY: [u8; 16] = [
    0x6b, 0x1f, 0x0c, 0x9e, 0x52, 0xd8, 0x4a, 0x37, 0xb0, 0xe1, 0xf6, 0xa2, 0xc9, 0xd3, 0x5e, 0x48,
];
const MAGIC: &[u8; 4] = b"BYIM";
const VERSION: u8 = 2;
const NONCE_LEN: usize = 16;
/// The longest field a blob carries, in characters.
const MAX_LEN: usize = 64;
const MAX_POSITION: u64 = 64;

/// The operator's details.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Details {
    name: String,
    street: String,
    postcode: String,
    city: String,
    country_de: String,
    country_en: String,
    email: String,
    phone: String,
}

/// A challenge as the feedback service sends it.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
struct Challenge {
    nonce: String,
    parts: u32,
    difficulty: u32,
    expires: u64,
    sig: String,
}

impl Challenge {
    fn valid(&self) -> bool {
        (1..=128).contains(&self.nonce.len())
            && self
                .nonce
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
            && (1..=64).contains(&self.parts)
            && (1..=32).contains(&self.difficulty)
            && self.sig.len() <= 256
    }
}

fn base64url(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len() * 3 / 4);
    let (mut buffer, mut bits) = (0_u32, 0_u32);
    for byte in text.trim().bytes() {
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'-' | b'+' => 62,
            b'_' | b'/' => 63,
            b'=' => break,
            _ => return None,
        };
        buffer = (buffer << 6) | u32::from(value);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(u8::try_from((buffer >> bits) & 0xff).ok()?);
        }
    }
    Some(out)
}

fn field(fields: &serde_json::Map<String, serde_json::Value>, name: &str) -> Option<String> {
    let pairs = fields.get(name)?.as_array()?;
    if pairs.len() > MAX_LEN {
        return None;
    }
    let mut chars: Vec<(u64, char)> = Vec::with_capacity(pairs.len());
    for pair in pairs {
        let pair = pair.as_array()?;
        let (Some(position), Some(code)) = (pair.first()?.as_u64(), pair.get(1)?.as_u64()) else {
            return None;
        };
        if pair.len() != 2 || position >= MAX_POSITION || code == 0 {
            return None;
        }
        chars.push((position, char::from_u32(u32::try_from(code).ok()?)?));
    }
    chars.sort_unstable_by_key(|(position, _)| *position);
    Some(chars.into_iter().map(|(_, c)| c).collect())
}

/// The details in a blob; `None` for anything that is not one.
fn decode(text: &str) -> Option<Details> {
    let raw = base64url(text)?;
    let head = MAGIC.len() + 1 + NONCE_LEN;
    if raw.len() <= head || &raw[..MAGIC.len()] != MAGIC || raw[MAGIC.len()] != VERSION {
        return None;
    }
    let nonce = &raw[MAGIC.len() + 1..head];
    let mut plain = raw[head..].to_vec();
    for (block, chunk) in plain.chunks_mut(32).enumerate() {
        let mut hash = Sha256::new();
        hash.update(SCRIPT_KEY);
        hash.update(nonce);
        hash.update(u32::try_from(block).ok()?.to_be_bytes());
        for (byte, key) in chunk.iter_mut().zip(hash.finalize().iter()) {
            *byte ^= key;
        }
    }
    let document: serde_json::Value = serde_json::from_slice(&plain).ok()?;
    if document.get("v")?.as_u64()? != u64::from(VERSION) {
        return None;
    }
    let fields = document.get("fields")?.as_object()?;
    Some(Details {
        name: field(fields, "name")?,
        street: field(fields, "street")?,
        postcode: field(fields, "postcode")?,
        city: field(fields, "city")?,
        country_de: field(fields, "country_de")?,
        country_en: field(fields, "country_en")?,
        email: field(fields, "email")?,
        phone: field(fields, "phone")?,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum State {
    Hidden,
    Solving(u32),
    Shown(Details),
    Failed,
}

/// Whether a WebGL renderer's name is a program's, not a graphics card's:
/// the names Baylee's page looked for (`web/site/impressum.js`).
#[cfg(any(target_arch = "wasm32", test))]
fn software(renderer: &str) -> bool {
    let renderer = renderer.to_ascii_lowercase();
    ["swiftshader", "llvmpipe", "software"]
        .iter()
        .any(|name| renderer.contains(name))
}

/// The name of what draws WebGL here, if the browser says. Firefox names it
/// as `RENDERER`; Chromium and Safari say only `WebKit WebGL` there and name
/// it through an extension (which Firefox warns about, so it is asked last).
#[cfg(target_arch = "wasm32")]
fn renderer() -> Option<String> {
    use wasm_bindgen::{JsCast, JsValue};
    const RENDERER: u32 = 0x1F01;
    const UNMASKED_RENDERER_WEBGL: u32 = 0x9246;
    let call = |target: &JsValue, name: &str, argument: JsValue| -> Option<JsValue> {
        let function: js_sys::Function = js_sys::Reflect::get(target, &JsValue::from_str(name))
            .ok()?
            .dyn_into()
            .ok()?;
        function
            .call1(target, &argument)
            .ok()
            .filter(|value| !value.is_null() && !value.is_undefined())
    };
    let canvas: JsValue = document().create_element("canvas").ok()?.into();
    let gl = call(&canvas, "getContext", JsValue::from_str("webgl"))?;
    let named = call(&gl, "getParameter", JsValue::from(RENDERER)).and_then(|v| v.as_string());
    if let Some(name) = named.filter(|name| !name.starts_with("WebKit")) {
        return Some(name);
    }
    call(
        &gl,
        "getExtension",
        JsValue::from_str("WEBGL_debug_renderer_info"),
    )?;
    call(&gl, "getParameter", JsValue::from(UNMASKED_RENDERER_WEBGL)).and_then(|v| v.as_string())
}

/// What the page knows of its own browser, for the service's difficulty
/// (Baylee's `impressum.rs`, `CLIENT_…`): 1 a browser driven by a program,
/// 2 one without languages, 4 one that draws with a program instead of a
/// graphics card. Three yes-or-no answers, asked only after the visitor's
/// click and never when the page loads; a yes makes the puzzle harder and
/// refuses nobody.
#[cfg(target_arch = "wasm32")]
fn browser_bits() -> u32 {
    // the first two are the bits registration's page reports too
    let mut bits = u32::from(super::pow::browser_signals());
    if renderer().is_some_and(|name| software(&name)) {
        bits |= 4;
    }
    bits
}

#[cfg(not(target_arch = "wasm32"))]
fn browser_bits() -> u32 {
    0
}

/// One round with the feedback service: a challenge, solved, answered;
/// a challenge that expired or was answered already is asked again once.
async fn fetch(worker: &str, state: RwSignal<State>) -> Option<Details> {
    let url = format!("/site/impressum/challenge?s={}", browser_bits());
    for _ in 0..2 {
        let (status, text) = net::request("GET", &url, None, &[], net::Credentials::Omit)
            .await
            .ok()?;
        if status != 200 {
            return None;
        }
        let challenge: Challenge = serde_json::from_str(&text).ok().filter(Challenge::valid)?;
        let counters = super::pow::solve(
            worker,
            &challenge.nonce,
            challenge.parts,
            challenge.difficulty,
            move |percent| state.set(State::Solving(percent)),
        )
        .await?;
        let answer = serde_json::json!({
            "nonce": challenge.nonce, "parts": challenge.parts, "difficulty": challenge.difficulty,
            "expires": challenge.expires, "sig": challenge.sig, "counters": counters,
        })
        .to_string();
        let (status, blob) = net::request(
            "POST",
            "/site/impressum",
            Some(&answer),
            &[],
            net::Credentials::Omit,
        )
        .await
        .ok()?;
        match status {
            200 => return decode(&blob),
            409 | 410 => {}
            _ => return None,
        }
    }
    None
}

/// What stands between the halves of the e-mail address, under `hidden`:
/// read with it, the address ends in a name nobody can register (RFC 2606).
const BETWEEN_MAIL: &str = "invalid.example ";
/// And between the telephone number's: read with it, the number has two
/// digits too many.
const BETWEEN_PHONE: &str = "00 ";

/// The e-mail address in two halves, the first up to and with its `@`.
fn mail_halves(mail: &str) -> (&str, &str) {
    mail.find('@')
        .map_or((mail, ""), |at| mail.split_at(at + 1))
}

/// The telephone number in two halves: before its last block of digits,
/// or in the middle when it is written without spaces.
fn phone_halves(phone: &str) -> (&str, &str) {
    let phone = phone.trim();
    let at = phone.rfind(' ').map_or_else(
        || {
            phone
                .char_indices()
                .nth(phone.chars().count() / 2)
                .map_or(phone.len(), |(index, _)| index)
        },
        |space| space + 1,
    );
    phone.split_at(at)
}

/// The telephone number as a `tel:` address takes it.
fn dialled(phone: &str) -> String {
    phone
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '+')
        .collect()
}

fn details_view(lang: Lang, details: &Details) -> impl IntoView + use<> {
    // the country's name is kept in German and English only
    let country = match lang.of_record() {
        Lang::De => details.country_de.clone(),
        _ => details.country_en.clone(),
    };
    let (mail_a, mail_b) = mail_halves(&details.email);
    let (mail_a, mail_b) = (mail_a.to_owned(), mail_b.to_owned());
    let (phone_a, phone_b) = phone_halves(&details.phone);
    let (phone_a, phone_b) = (phone_a.to_owned(), phone_b.to_owned());
    // the addresses exist from the click on, nowhere in the page before
    let mail = format!("mailto:{}", details.email);
    let phone = format!("tel:{}", dialled(&details.phone));
    view! {
        <dl class="facts impressum-details" tabindex="-1">
            <dt>{words::impressum_name(lang)}</dt>
            <dd>{details.name.clone()}</dd>
            <dt>{words::impressum_address(lang)}</dt>
            <dd>
                <address>
                    {details.street.clone()}<br />
                    {format!("{} {}", details.postcode, details.city)}<br />
                    {country}
                </address>
            </dd>
            <dt>{words::impressum_email(lang)}</dt>
            <dd>
                <span class="impressum-value">{mail_a}<span hidden>{BETWEEN_MAIL}</span>{mail_b}</span>
                <button type="button" class="button quiet button-sm" on:click=move |_| session::open(&mail)>
                    {words::impressum_write(lang)}
                </button>
            </dd>
            <dt>{words::impressum_phone(lang)}</dt>
            <dd>
                <span class="impressum-value">{phone_a}<span hidden>{BETWEEN_PHONE}</span>{phone_b}</span>
                <button type="button" class="button quiet button-sm" on:click=move |_| session::open(&phone)>
                    {words::impressum_call(lang)}
                </button>
            </dd>
        </dl>
    }
}

#[island]
pub fn Impressum(lang: Lang, worker: String) -> impl IntoView {
    let state = RwSignal::new(State::Hidden);
    let show = move |_| {
        if matches!(state.get_untracked(), State::Solving(_) | State::Shown(_)) {
            return;
        }
        state.set(State::Solving(0));
        let worker = worker.clone();
        leptos::task::spawn_local(async move {
            let outcome = fetch(&worker, state).await;
            let shown = outcome.is_some();
            state.set(outcome.map_or(State::Failed, State::Shown));
            // the button that held the focus went while solving
            super::focus::after_render(if shown {
                ".impressum-details"
            } else {
                ".impressum button"
            });
        });
    };
    let contact = format!("/{}/contact", lang.code());
    view! {
        <div class="impressum" aria-live="polite">
            {move || match state.get() {
                State::Shown(details) => details_view(lang, &details).into_any(),
                State::Solving(percent) => view! {
                    <p id="impressum-solving">{words::impressum_solving(lang, &percent.to_string())}</p>
                    <progress
                        max="100"
                        value=percent.to_string()
                        aria-labelledby="impressum-solving"
                    ></progress>
                }
                .into_any(),
                State::Hidden | State::Failed => {
                    let failed = (state.get() == State::Failed).then(|| {
                        let contact = contact.clone();
                        view! {
                            <p class="error" role="alert">
                                {crate::rich::rich(words::impressum_failed(lang), move |tag| (tag == "contact").then(|| contact.clone()))}
                            </p>
                        }
                    });
                    view! {
                        <button type="button" class="button primary" on:click=show.clone()>{words::impressum_show(lang)}</button>
                        {failed}
                    }
                    .into_any()
                }
            }}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

    /// A blob as Baylee's tool writes one, from made-up details.
    fn blob(document: &str, nonce: [u8; 16]) -> String {
        let mut body = document.as_bytes().to_vec();
        for (block, chunk) in body.chunks_mut(32).enumerate() {
            let mut hash = Sha256::new();
            hash.update(SCRIPT_KEY);
            hash.update(nonce);
            hash.update(u32::try_from(block).expect("small").to_be_bytes());
            for (byte, key) in chunk.iter_mut().zip(hash.finalize().iter()) {
                *byte ^= key;
            }
        }
        let mut raw = MAGIC.to_vec();
        raw.push(VERSION);
        raw.extend(nonce);
        raw.extend(body);
        let mut out = String::new();
        for chunk in raw.chunks(3) {
            let n = chunk
                .iter()
                .enumerate()
                .fold(0_u32, |n, (i, b)| n | u32::from(*b) << (16 - 8 * i));
            for i in 0..=chunk.len() {
                out.push(char::from(
                    ALPHABET[usize::try_from((n >> (18 - 6 * i)) & 63).expect("six bits")],
                ));
            }
        }
        out
    }

    fn pairs(text: &str) -> serde_json::Value {
        let mut pairs: Vec<(usize, u32)> = text
            .chars()
            .enumerate()
            .map(|(i, c)| (i, u32::from(c)))
            .collect();
        pairs.reverse();
        serde_json::json!(pairs)
    }

    #[test]
    fn decodes_a_blob_and_refuses_anything_else() {
        let fields = serde_json::json!({
            "name": pairs("Erika Mustermann"), "street": pairs("Musterweg 1"), "postcode": pairs("12345"),
            "city": pairs("Musterstadt"), "country_de": pairs("Deutschland"), "country_en": pairs("Germany"),
            "email": pairs("erika@example.invalid"), "phone": pairs("+49 30 1234567"),
        });
        let document = serde_json::json!({"v": 2, "fields": fields}).to_string();
        let details = decode(&blob(&document, [7; 16])).expect("a blob");
        assert_eq!(details.name, "Erika Mustermann");
        assert_eq!(details.email, "erika@example.invalid");
        assert_eq!(details.country_de, "Deutschland");
        assert!(decode("not a blob").is_none());
        let wrong = serde_json::json!({"v": 1, "fields": fields}).to_string();
        assert!(decode(&blob(&wrong, [7; 16])).is_none());
    }

    #[test]
    fn a_renderer_that_is_a_program_is_told_from_a_graphics_card() {
        for name in [
            "ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero)), SwiftShader driver)",
            "llvmpipe (LLVM 15.0.7, 256 bits)",
            "Google SwiftShader",
            "Software Rasterizer",
        ] {
            assert!(software(name), "{name}");
        }
        for name in [
            "Apple GPU",
            "ANGLE (Intel, Intel(R) UHD Graphics 630, OpenGL 4.1)",
            "Mali-G78",
            "",
        ] {
            assert!(!software(name), "{name}");
        }
    }

    #[test]
    fn the_halves_are_the_whole_and_what_stands_between_them_spoils_it() {
        for mail in ["erika@example.invalid", "a@b.example", "no-at-sign"] {
            let (a, b) = mail_halves(mail);
            assert_eq!(format!("{a}{b}"), mail);
        }
        assert_eq!(
            mail_halves("erika@example.invalid"),
            ("erika@", "example.invalid")
        );
        // read with what is hidden, the address is one under `.example`
        let (a, b) = mail_halves("erika@example.invalid");
        let read = format!("{a}{BETWEEN_MAIL}{b}");
        assert!(read.starts_with("erika@invalid.example "));
        assert!(!read.contains("erika@example.invalid"));

        for phone in ["+49 30 1234567", "+49301234567", "030/1234567", "1", ""] {
            let (a, b) = phone_halves(phone);
            assert_eq!(format!("{a}{b}"), phone);
        }
        assert_eq!(phone_halves("+49 30 1234567"), ("+49 30 ", "1234567"));
        assert_eq!(phone_halves("+49301234567"), ("+49301", "234567"));
        let (a, b) = phone_halves("+49 30 1234567");
        assert_ne!(
            dialled(&format!("{a}{BETWEEN_PHONE}{b}")),
            dialled("+49 30 1234567")
        );
        assert_eq!(dialled("+49 (0) 30 / 123-45 67"), "+490301234567");
    }

    #[test]
    fn challenges_are_bounded() {
        let ok = Challenge {
            nonce: "abc_-1".into(),
            parts: 8,
            difficulty: 17,
            expires: 1,
            sig: "s".into(),
        };
        assert!(ok.valid());
        assert!(
            !Challenge {
                nonce: "a b".into(),
                ..ok.clone()
            }
            .valid()
        );
        assert!(
            !Challenge {
                parts: 65,
                ..ok.clone()
            }
            .valid()
        );
        assert!(
            !Challenge {
                difficulty: 33,
                ..ok
            }
            .valid()
        );
    }
}
