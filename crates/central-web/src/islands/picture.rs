//! Profile pictures on the site (`docs/profile-pictures.md`): the card on
//! the profile page, a picture in its circle, and the report form.
//!
//! The page opens the chosen file in the browser (`createImageBitmap` with
//! the file's own orientation, or an `<img>` from a `blob:` address for SVG
//! and whatever else the first way refuses), lets the person place a
//! square, and uploads a lossless square PNG of it: the photo's details
//! (place, camera, time) never leave the device. Where this browser will not
//! let the page read its own canvas (a hardened browser), or cannot open the
//! file, the file itself goes to Central, whose worker decodes it and keeps
//! its centred square; the page says so first.
//!
//! The picture is shown everywhere by its random key, as AVIF with a PNG
//! fallback that the browser chooses between.

use central_api::picture::{
    FiledReport, MAX_REPORT_NOTE, Picture, PictureKey, PictureNotice, PictureReason,
    PictureRequest, UploadBlock, key_hex, valid_note,
};
use central_api::social::{SocialRequest, SocialResponse};
use central_api::wire::{Request, Response};
use leptos::prelude::*;

use super::account::when;
use super::auth::{Problem, problem_text};
use super::central::{Failure, call};
use crate::words::{self, Lang};

/// The largest file the page opens. Only the person's own device is at
/// stake: what is sent is the square.
const MAX_FILE: f64 = 25.0 * 1024.0 * 1024.0;
/// The longest side the page lets the browser decode, read from the file's
/// own header first where the page can.
const MAX_SIDE: u32 = 16_384;
/// The preview's side in canvas pixels.
const PREVIEW: u32 = 256;

/// A picture's address on this site.
#[must_use]
pub fn url(key: &PictureKey, avif: bool) -> String {
    format!(
        "/api/v1/pictures/{}.{}",
        key_hex(key),
        if avif { "avif" } else { "png" }
    )
}

/// The picture of `key`, which the browser takes as AVIF or else as PNG.
pub fn face(key: &PictureKey, alt: String) -> impl IntoView + use<> {
    view! {
        <picture class="face">
            <source type="image/avif" srcset=url(key, true) />
            <img src=url(key, false) alt=alt loading="lazy" decoding="async" width="256" height="256" />
        </picture>
    }
}

/// The picture keys of the tags a page shows, by tag: given by the
/// messenger, read by every avatar inside it.
#[derive(Clone, Copy)]
pub struct Keys(RwSignal<std::collections::BTreeMap<u32, PictureKey>>);

impl Keys {
    /// Empty keys for the island's own avatars.
    pub fn provide() -> Keys {
        let keys = Keys(RwSignal::new(std::collections::BTreeMap::new()));
        provide_context(keys);
        keys
    }

    /// Reads the keys of `tags` (at most 300) from Central; keeps the old
    /// ones where the answer does not come.
    pub fn read(self, mut tags: Vec<u32>) {
        tags.sort_unstable();
        tags.dedup();
        tags.truncate(central_api::picture::MAX_PICTURE_TAGS);
        leptos::task::spawn_local(async move {
            let asked = Request::Social {
                request: SocialRequest::Pictures { tags },
            };
            if let Ok(Response::Social {
                response: SocialResponse::Pictures { pictures },
            }) = call(&asked).await
            {
                let found = pictures.into_iter().map(|p| (p.tag, p.key)).collect();
                if self.0.with_untracked(|held| *held != found) {
                    self.0.set(found);
                }
            }
        });
    }
}

/// The picture key of `tag` in the page's keys, read reactively.
#[must_use]
pub fn key_of(tag: u32) -> Option<PictureKey> {
    use_context::<Keys>().and_then(|keys| keys.0.with(|held| held.get(&tag).copied()))
}

/// Where the square is: its left and top edge in the picture's pixels, and
/// how far it is zoomed in (1 is the largest square the picture holds).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Crop {
    width: f64,
    height: f64,
    x: f64,
    y: f64,
    zoom: f64,
}

/// How far the page lets the person zoom in.
const MAX_ZOOM: f64 = 8.0;

impl Crop {
    /// The largest centred square of a `width` × `height` picture.
    #[must_use]
    pub fn new(width: f64, height: f64) -> Self {
        let width = width.max(1.0);
        let height = height.max(1.0);
        let side = width.min(height);
        Crop {
            width,
            height,
            x: (width - side) / 2.0,
            y: (height - side) / 2.0,
            zoom: 1.0,
        }
    }

    /// The square's side in the picture's pixels.
    #[must_use]
    pub fn side(&self) -> f64 {
        self.width.min(self.height) / self.zoom
    }

    fn clamped(mut self) -> Self {
        self.zoom = self.zoom.clamp(1.0, MAX_ZOOM);
        let side = self.side();
        self.x = self.x.clamp(0.0, (self.width - side).max(0.0));
        self.y = self.y.clamp(0.0, (self.height - side).max(0.0));
        self
    }

    /// Moved by a share of its own side (the arrow keys: 5 %, with Shift 25 %).
    #[must_use]
    pub fn moved(mut self, right: f64, down: f64) -> Self {
        let side = self.side();
        self.x += right * side;
        self.y += down * side;
        self.clamped()
    }

    /// Dragged by a pointer: the picture follows it, so the square moves
    /// the other way, `dx` and `dy` in display pixels of a preview `shown`
    /// pixels wide.
    #[must_use]
    pub fn dragged(self, from: Crop, dx: f64, dy: f64, shown: f64) -> Self {
        let scale = from.side() / shown.max(1.0);
        Crop {
            x: from.x - dx * scale,
            y: from.y - dy * scale,
            ..self
        }
        .clamped()
    }

    /// Zoomed to `zoom` around its own centre.
    #[must_use]
    pub fn zoomed(mut self, zoom: f64) -> Self {
        let centre = (self.x + self.side() / 2.0, self.y + self.side() / 2.0);
        self.zoom = zoom.clamp(1.0, MAX_ZOOM);
        let side = self.side();
        self.x = centre.0 - side / 2.0;
        self.y = centre.1 - side / 2.0;
        self.clamped()
    }

    /// The side of the PNG the page sends: the square's own, from 256 to
    /// 512 pixels.
    #[cfg(any(target_arch = "wasm32", test))]
    #[must_use]
    pub fn out(&self) -> u32 {
        // the bound keeps the cast exact
        let side = self.side().round().clamp(256.0, 512.0);
        if side >= 512.0 {
            512
        } else if side <= 256.0 {
            256
        } else {
            (256..=512).find(|n| f64::from(*n) >= side).unwrap_or(512)
        }
    }

    /// Where the square is, for the live region: its centre in per cent of
    /// the picture, and the zoom in per cent.
    #[must_use]
    pub fn spoken(&self) -> (u32, u32, u32) {
        let percent = |part: f64, whole: f64| {
            let value = (part / whole * 100.0).round().clamp(0.0, 100.0);
            (0..=100).find(|n| f64::from(*n) >= value).unwrap_or(100)
        };
        let side = self.side();
        (
            percent(self.x + side / 2.0, self.width),
            percent(self.y + side / 2.0, self.height),
            (100..=800)
                .find(|n| f64::from(*n) >= (self.zoom * 100.0).round())
                .unwrap_or(800),
        )
    }
}

/// The sides a file's header claims, where the page can read them: PNG,
/// GIF, JPEG, WebP and BMP. `None` for others and for a header it cannot
/// read; the browser's own limits apply to those.
#[must_use]
pub fn claimed(head: &[u8]) -> Option<(u32, u32)> {
    let be32 = |at: usize| Some(u32::from_be_bytes(head.get(at..at + 4)?.try_into().ok()?));
    let le16 = |at: usize| {
        Some(u32::from(u16::from_le_bytes(
            head.get(at..at + 2)?.try_into().ok()?,
        )))
    };
    let be16 = |at: usize| {
        Some(u32::from(u16::from_be_bytes(
            head.get(at..at + 2)?.try_into().ok()?,
        )))
    };
    if head.starts_with(b"\x89PNG\r\n\x1a\n") && head.get(12..16) == Some(b"IHDR") {
        return Some((be32(16)?, be32(20)?));
    }
    if head.starts_with(b"GIF8") {
        return Some((le16(6)?, le16(8)?));
    }
    if head.starts_with(b"BM") {
        let width = i32::from_le_bytes(head.get(18..22)?.try_into().ok()?);
        let height = i32::from_le_bytes(head.get(22..26)?.try_into().ok()?);
        return Some((width.unsigned_abs(), height.unsigned_abs()));
    }
    if head.len() >= 30 && &head[..4] == b"RIFF" && &head[8..12] == b"WEBP" {
        return match &head[12..16] {
            b"VP8X" => {
                let three =
                    |at: usize| u32::from_le_bytes([head[at], head[at + 1], head[at + 2], 0]) + 1;
                Some((three(24), three(27)))
            }
            b"VP8L" => {
                let bits = u32::from_le_bytes(head.get(21..25)?.try_into().ok()?);
                Some(((bits & 0x3fff) + 1, ((bits >> 14) & 0x3fff) + 1))
            }
            b"VP8 " => Some((le16(26)? & 0x3fff, le16(28)? & 0x3fff)),
            _ => None,
        };
    }
    if head.starts_with(&[0xff, 0xd8]) {
        let mut at = 2;
        while at + 9 < head.len() {
            if head[at] != 0xff {
                return None;
            }
            let marker = head[at + 1];
            if matches!(marker, 0xc0..=0xc3 | 0xc5..=0xc7 | 0xc9..=0xcb | 0xcd..=0xcf) {
                return Some((be16(at + 7)?, be16(at + 5)?));
            }
            at += 2 + usize::try_from(be16(at + 2)?).ok()?;
        }
    }
    None
}

/// Why a picture was not taken, as the page says it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trouble {
    /// The browser could not open the file as a picture.
    Open,
    /// Larger than [`MAX_FILE`].
    Large,
    /// A side longer than [`MAX_SIDE`].
    Huge,
    /// Central refused the picture, with its reason's name.
    Refused(&'static str),
    /// An administrator's block.
    Blocked,
    /// The account's budget is spent.
    Busy,
    /// The session ended.
    SignedOut,
    /// Anything else.
    Failed,
}

/// What the page checks of a file before the browser decodes it: its
/// size, and the sides its header claims where the page can read them.
fn checked(size: f64, head: &[u8]) -> Result<(), Trouble> {
    if size > MAX_FILE {
        return Err(Trouble::Large);
    }
    match claimed(head) {
        Some((width, height)) if width > MAX_SIDE || height > MAX_SIDE => Err(Trouble::Huge),
        _ => Ok(()),
    }
}

fn trouble_text(lang: Lang, trouble: Trouble) -> &'static str {
    match trouble {
        Trouble::Open => words::picture_error_open(lang),
        Trouble::Large => words::picture_error_large(lang),
        Trouble::Huge | Trouble::Refused("too_many_pixels") => words::picture_error_huge(lang),
        Trouble::Refused("unsupported") => words::picture_error_unsupported(lang),
        Trouble::Refused("wrong_type") => words::picture_error_wrong_type(lang),
        Trouble::Refused("too_large") => words::picture_error_too_large(lang),
        Trouble::Refused("too_small") => words::picture_error_too_small(lang),
        Trouble::Refused("malformed") => words::picture_error_malformed(lang),
        Trouble::Blocked => words::picture_error_blocked(lang),
        Trouble::Busy => words::picture_error_busy(lang),
        Trouble::SignedOut => words::account_signed_out(lang),
        Trouble::Refused(_) | Trouble::Failed => words::picture_error_failed(lang),
    }
}

/// The name Central gives a refusal, from its 422 or 403 answer.
fn refusal(text: &str) -> Option<&'static str> {
    let value: serde_json::Value = serde_json::from_str(text).ok()?;
    let name = value.get("refusal")?.as_str()?;
    [
        "unsupported",
        "wrong_type",
        "too_large",
        "too_many_pixels",
        "too_small",
        "malformed",
        "failed",
        "blocked",
    ]
    .into_iter()
    .find(|known| *known == name)
}

/// Central's answer to an upload.
fn answered(status: u16, text: &str) -> Result<Own, Trouble> {
    match status {
        200 => match serde_json::from_str(text) {
            Ok(Response::MyPicture {
                picture,
                notice,
                block,
            }) => Ok(Own {
                picture,
                notice,
                block,
            }),
            _ => Err(Trouble::Failed),
        },
        403 if refusal(text) == Some("blocked") => Err(Trouble::Blocked),
        422 => Err(refusal(text).map_or(Trouble::Failed, Trouble::Refused)),
        401 => Err(Trouble::SignedOut),
        429 => Err(Trouble::Busy),
        _ => Err(Trouble::Failed),
    }
}

/// The account's own picture, notice and block.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Own {
    picture: Option<Picture>,
    notice: Option<PictureNotice>,
    block: Option<UploadBlock>,
}

async fn mine() -> Result<Own, Problem> {
    match call(&Request::Picture {
        request: PictureRequest::Mine,
    })
    .await
    {
        Ok(Response::MyPicture {
            picture,
            notice,
            block,
        }) => Ok(Own {
            picture,
            notice,
            block,
        }),
        Ok(_) => Err(Problem::Failed),
        Err(failure) => Err(failure.into()),
    }
}

/// What the export adds of pictures: the picture's facts and the reports
/// this account filed, each `None` where it could not be read.
pub async fn for_export() -> (Option<Picture>, Option<Vec<FiledReport>>) {
    let picture = mine().await.ok().and_then(|own| own.picture);
    let reports = match call(&Request::Picture {
        request: PictureRequest::Reports,
    })
    .await
    {
        Ok(Response::MyPictureReports { reports }) => Some(reports),
        _ => None,
    };
    (picture, reports)
}

/// The export's file with the picture's facts under `profile_picture` and
/// the reports filed under `picture_reports`.
#[must_use]
pub fn merged(
    file: &str,
    picture: Option<&Picture>,
    reports: Option<&[FiledReport]>,
) -> Option<String> {
    let mut value: serde_json::Value = serde_json::from_str(file).ok()?;
    let object = value.as_object_mut()?;
    if let Some(picture) = picture {
        object.insert(
            "profile_picture".into(),
            serde_json::to_value(picture).ok()?,
        );
    }
    if let Some(reports) = reports {
        object.insert(
            "picture_reports".into(),
            serde_json::to_value(reports).ok()?,
        );
    }
    serde_json::to_string_pretty(&value).ok()
}

/// A reason's words.
#[must_use]
pub fn reason_text(lang: Lang, reason: PictureReason) -> &'static str {
    match reason {
        PictureReason::Sexual => words::picture_reason_sexual(lang),
        PictureReason::Violence => words::picture_reason_violence(lang),
        PictureReason::Hate => words::picture_reason_hate(lang),
        PictureReason::Harassment => words::picture_reason_harassment(lang),
        PictureReason::Impersonation => words::picture_reason_impersonation(lang),
        PictureReason::Private => words::picture_reason_private(lang),
        PictureReason::Other => words::picture_reason_other(lang),
    }
}

fn notice_view(lang: Lang, own: &Own) -> impl IntoView + use<> {
    let notice = own.notice.clone().map(|notice| {
        let note = notice.note.clone().map(|note| view! { <p translate="no">{note}</p> });
        let contact = format!("/{}/contact", lang.code());
        view! {
            <p>{words::picture_notice(lang, &when(lang, notice.at), reason_text(lang, notice.reason))}</p>
            {note}
            <p>{crate::rich::rich(words::picture_notice_object(lang), move |tag| (tag == "contact").then(|| contact.clone()))}</p>
        }
    });
    let block = own.block.map(|block| match block {
        UploadBlock::Until { until } => {
            view! { <p id="picture-block">{words::picture_blocked_until(lang, &when(lang, until))}</p> }.into_any()
        }
        UploadBlock::UntilLifted => {
            view! { <p id="picture-block">{words::picture_blocked_lifted(lang)}</p> }.into_any()
        }
    });
    view! { <div class="picture-notice" role="note">{notice}{block}</div> }
}

/// What the card shows below the current picture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    Idle,
    Opening,
    Editing,
    Saving,
    Removing,
}

/// The card's signals. All of it is `Copy`, so every handler takes the whole.
#[derive(Clone, Copy)]
struct Card {
    lang: Lang,
    own: RwSignal<Option<Result<Own, Problem>>>,
    step: RwSignal<Step>,
    crop: RwSignal<Crop>,
    trouble: RwSignal<Option<Trouble>>,
    said: RwSignal<Option<&'static str>>,
    original: RwSignal<bool>,
    dragging: StoredValue<Option<(f64, f64, Crop)>>,
}

impl Card {
    fn reload(self) {
        leptos::task::spawn_local(async move { self.own.set(Some(mine().await)) });
    }

    fn choose(self) {
        self.trouble.set(None);
        self.said.set(None);
        self.step.set(Step::Opening);
        leptos::task::spawn_local(async move {
            match browser::open().await {
                Ok((width, height, readable)) => {
                    self.crop.set(Crop::new(width, height));
                    self.original.set(!readable);
                    self.step.set(Step::Editing);
                    super::focus::after_render("#picture-preview");
                }
                Err(problem) => {
                    self.trouble.set(Some(problem));
                    self.step.set(Step::Idle);
                }
            }
        });
    }

    fn save(self) {
        if self.step.get_untracked() == Step::Saving {
            return;
        }
        self.step.set(Step::Saving);
        self.trouble.set(None);
        leptos::task::spawn_local(async move {
            match browser::upload(self.crop.get_untracked()).await {
                Ok(stored) => {
                    browser::close();
                    self.own.set(Some(Ok(stored)));
                    self.said.set(Some(words::picture_saved(self.lang)));
                    self.step.set(Step::Idle);
                }
                Err(problem) => {
                    self.trouble.set(Some(problem));
                    self.step.set(Step::Editing);
                }
            }
        });
    }

    fn remove(self) {
        leptos::task::spawn_local(async move {
            let done = call(&Request::Picture {
                request: PictureRequest::Remove,
            })
            .await;
            self.step.set(Step::Idle);
            match done {
                Ok(_) => {
                    self.said.set(Some(words::picture_removed(self.lang)));
                    self.reload();
                }
                Err(Failure::Rejected) => self.trouble.set(Some(Trouble::SignedOut)),
                Err(_) => self.trouble.set(Some(Trouble::Failed)),
            }
        });
    }

    fn key(self, event: &leptos::ev::KeyboardEvent) {
        let far = if event.shift_key() { 0.25 } else { 0.05 };
        let moved = match event.key().as_str() {
            "ArrowLeft" => Some((-far, 0.0)),
            "ArrowRight" => Some((far, 0.0)),
            "ArrowUp" => Some((0.0, -far)),
            "ArrowDown" => Some((0.0, far)),
            _ => None,
        };
        if let Some((right, down)) = moved {
            event.prevent_default();
            self.crop.update(|c| *c = c.moved(right, down));
        }
    }

    fn press(self, event: &leptos::ev::PointerEvent) {
        let at = (f64::from(event.client_x()), f64::from(event.client_y()));
        self.dragging
            .set_value(Some((at.0, at.1, self.crop.get_untracked())));
        browser::capture(event.pointer_id());
    }

    fn drag(self, event: &leptos::ev::PointerEvent) {
        if let Some((x0, y0, from)) = self.dragging.get_value() {
            let dx = f64::from(event.client_x()) - x0;
            let dy = f64::from(event.client_y()) - y0;
            self.crop
                .update(|c| *c = c.dragged(from, dx, dy, browser::shown()));
        }
    }

    fn current(self) -> AnyView {
        let (lang, step) = (self.lang, self.step);
        match self.own.get() {
            None => view! { <p class="meta" role="status">{words::account_loading(lang)}</p> }
                .into_any(),
            Some(Err(problem)) => {
                view! { <p class="error" role="alert">{problem_text(lang, problem)}</p> }.into_any()
            }
            Some(Ok(own)) => {
                let shown = own.picture.as_ref().map_or_else(
                    || view! { <p id="picture-none">{words::picture_none(lang)}</p> }.into_any(),
                    |picture| view! { <div class="picture-current">{face(&picture.key, words::picture_current(lang).to_owned())}</div> }.into_any(),
                );
                let can_remove = own.picture.is_some();
                let blocked = own.block.is_some();
                view! {
                    {shown}
                    {notice_view(lang, &own)}
                    <div class="actions" hidden=move || step.get() != Step::Idle>
                        <label class="button primary" for="picture-file" hidden=blocked>{words::picture_choose(lang)}</label>
                        <button type="button" class="button" hidden=!can_remove on:click=move |_| step.set(Step::Removing)>{words::picture_remove(lang)}</button>
                    </div>
                }
                .into_any()
            }
        }
    }

    fn editor(self) -> impl IntoView {
        let (lang, crop, step) = (self.lang, self.crop, self.step);
        let zoom = move |event: leptos::ev::Event| {
            if let Ok(percent) = event_target_value(&event).parse::<f64>() {
                crop.update(|c| *c = c.zoomed(percent / 100.0));
            }
        };
        let position = move || {
            let (x, y, zoom) = crop.get().spoken();
            words::picture_position(lang, &x.to_string(), &y.to_string(), &zoom.to_string())
        };
        view! {
            <div class="picture-editor">
                <p>{words::picture_editor_lead(lang)}</p>
                <canvas id="picture-preview" class="picture-preview" width=PREVIEW height=PREVIEW tabindex="0"
                    role="img" aria-label=words::picture_preview(lang) aria-describedby="picture-position"
                    on:keydown=move |e| self.key(&e) on:pointerdown=move |e| self.press(&e)
                    on:pointermove=move |e| self.drag(&e)
                    on:pointerup=move |_| self.dragging.set_value(None)
                    on:pointercancel=move |_| self.dragging.set_value(None)></canvas>
                <p id="picture-position" class="meta" aria-live="polite">{position}</p>
                <div class="field">
                    <label for="picture-zoom">{words::picture_zoom(lang)}</label>
                    <input id="picture-zoom" type="range" min="100" max="800" step="10"
                        prop:value=move || crop.get().spoken().2.to_string() on:input=zoom />
                </div>
                <Show when=move || self.original.get()>
                    <p class="meta" id="picture-original">{words::picture_original(lang)}</p>
                </Show>
                <div class="actions">
                    <button type="button" class="button primary" on:click=move |_| self.save()
                        aria-disabled=move || (step.get() == Step::Saving).then_some("true")>
                        {move || if step.get() == Step::Saving { words::picture_saving(lang) } else { words::picture_save(lang) }}
                    </button>
                    <button type="button" class="button" on:click=move |_| { browser::close(); step.set(Step::Idle); }>
                        {words::picture_cancel(lang)}
                    </button>
                </div>
            </div>
        }
    }
}

/// The profile page's card: the picture, choosing one, placing its square,
/// saving it, removing it.
pub fn card(lang: Lang, name: String) -> impl IntoView {
    let card = Card {
        lang,
        own: RwSignal::new(None),
        step: RwSignal::new(Step::Idle),
        crop: RwSignal::new(Crop::new(1.0, 1.0)),
        trouble: RwSignal::new(None),
        said: RwSignal::new(None),
        original: RwSignal::new(false),
        dragging: StoredValue::new(None),
    };
    let step = card.step;
    Effect::new(move |_| card.reload());
    // The preview follows the square.
    Effect::new(move |_| {
        let at = card.crop.get();
        if step.get() == Step::Editing {
            browser::draw(at);
        }
    });
    view! {
        <section class="card picture-card" aria-labelledby="picture-title">
            <h2 id="picture-title">{words::picture_title(lang)}</h2>
            <p class="meta">{words::picture_who_sees(lang)}</p>
            {move || card.current()}
            <input id="picture-file" class="visually-hidden" type="file" accept="image/*" aria-label=words::picture_choose(lang) on:change=move |_| card.choose() />
            <p class="hint">{words::picture_choose_hint(lang)}</p>
            <Show when=move || step.get() == Step::Opening>
                <p class="meta" role="status">{words::picture_opening(lang)}</p>
            </Show>
            <Show when=move || step.get() == Step::Removing>
                <div class="confirm" role="group" aria-labelledby="picture-remove-ask">
                    <p id="picture-remove-ask">{words::picture_remove_confirm(lang)}</p>
                    <div class="actions">
                        <button type="button" class="button danger" on:click=move |_| card.remove()>{words::picture_remove_yes(lang)}</button>
                        <button type="button" class="button" on:click=move |_| step.set(Step::Idle)>{words::picture_cancel(lang)}</button>
                    </div>
                </div>
            </Show>
            <Show when=move || matches!(step.get(), Step::Editing | Step::Saving)>
                {card.editor()}
            </Show>
            <p class="error" role="alert">{move || card.trouble.get().map(|t| trouble_text(lang, t))}</p>
            <p class="meta" role="status">{move || card.said.get()}</p>
            <p class="visually-hidden" translate="no">{name}</p>
        </section>
    }
}

/// The report form for the picture `key` of the account `tag`, opened by
/// its button. The person reported is never told who reported.
pub fn report(lang: Lang, tag: u32, key: PictureKey) -> impl IntoView {
    let open = RwSignal::new(false);
    let reason = RwSignal::new(PictureReason::Other);
    let note = RwSignal::new(String::new());
    let said = RwSignal::new(None::<&'static str>);
    let failed = RwSignal::new(None::<&'static str>);
    let send = move |event: leptos::ev::SubmitEvent| {
        event.prevent_default();
        let typed = note.get_untracked();
        let note = (!typed.trim().is_empty()).then_some(typed);
        if note
            .as_deref()
            .is_some_and(|n| !valid_note(n, MAX_REPORT_NOTE))
        {
            failed.set(Some(words::picture_report_note_hint(lang)));
            return;
        }
        let request = Request::Social {
            request: SocialRequest::ReportPicture {
                tag,
                key,
                reason: reason.get_untracked(),
                note,
            },
        };
        leptos::task::spawn_local(async move {
            match call(&request).await {
                Ok(Response::Social {
                    response: SocialResponse::Done,
                }) => {
                    open.set(false);
                    failed.set(None);
                    said.set(Some(words::picture_report_sent(lang)));
                }
                Err(Failure::Conflict) => failed.set(Some(words::picture_report_twice(lang))),
                Err(Failure::Busy) => failed.set(Some(words::picture_report_busy(lang))),
                Err(Failure::Rejected) => failed.set(Some(words::picture_report_gone(lang))),
                _ => failed.set(Some(words::picture_error_failed(lang))),
            }
        });
    };
    let reasons = PictureReason::ALL
        .into_iter()
        .map(|r| view! { <option value=r.as_str()>{reason_text(lang, r)}</option> })
        .collect_view();
    let id = format!("picture-report-{tag}");
    let note_id = format!("{id}-note");
    let reason_id = format!("{id}-reason");
    view! {
        <div class="picture-report">
            <button type="button" class="button" aria-expanded=move || open.get().to_string()
                on:click=move |_| { open.update(|o| *o = !*o); said.set(None); }>{words::picture_report(lang)}</button>
            <Show when=move || open.get()>
                <form class="form card" novalidate=true on:submit=send aria-label=words::picture_report_title(lang)>
                    <p>{words::picture_report_lead(lang)}</p>
                    <div class="field">
                        <label for=reason_id.clone()>{words::picture_report_reason(lang)}</label>
                        <select id=reason_id.clone() on:change=move |ev| {
                            if let Some(r) = PictureReason::parse(&event_target_value(&ev)) { reason.set(r) }
                        }>
                            {reasons.clone()}
                        </select>
                    </div>
                    <div class="field">
                        <label for=note_id.clone()>{words::picture_report_note(lang)}</label>
                        <textarea id=note_id.clone() maxlength="1000" rows="3"
                            on:input=move |ev| note.set(event_target_value(&ev))></textarea>
                        <p class="hint">{words::picture_report_note_hint(lang)}</p>
                    </div>
                    <p class="error" role="alert">{move || failed.get()}</p>
                    <button type="submit" class="button primary">{words::picture_report_send(lang)}</button>
                </form>
            </Show>
            <p class="meta" role="status">{move || said.get()}</p>
        </div>
    }
}

/// The browser's half: opening the file, drawing the square, sending it.
#[cfg(target_arch = "wasm32")]
mod browser {
    use super::{Crop, MAX_SIDE, Own, PREVIEW, Trouble, answered, checked};
    use leptos::prelude::*;
    use wasm_bindgen::{JsCast, JsValue, closure::Closure};
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{
        Blob, CanvasRenderingContext2d, File, HtmlCanvasElement, HtmlImageElement,
        HtmlInputElement, ImageBitmap, ImageBitmapOptions, ImageOrientation, Url,
    };

    /// What the browser opened.
    enum Source {
        /// `createImageBitmap`, the file's orientation applied.
        Bitmap(ImageBitmap),
        /// An `<img>` of a `blob:` address, revoked once it is closed.
        Image(HtmlImageElement, String),
        /// Nothing this browser could open; the file goes to Central as it is.
        Unopened,
    }

    struct Opened {
        source: Source,
        file: File,
        readable: bool,
    }

    thread_local! {
        static OPENED: std::cell::RefCell<Option<Opened>> = const { std::cell::RefCell::new(None) };
    }

    fn element<T: JsCast>(id: &str) -> Option<T> {
        document().get_element_by_id(id)?.dyn_into().ok()
    }

    /// The media types Central takes as they are, for a file the page
    /// sends itself.
    fn accepted(kind: &str) -> Option<&'static str> {
        [
            "image/png",
            "image/jpeg",
            "image/gif",
            "image/webp",
            "image/avif",
            "image/svg+xml",
            "image/bmp",
            "image/x-icon",
            "image/vnd.microsoft.icon",
        ]
        .into_iter()
        .find(|known| *known == kind)
    }

    async fn head(file: &File) -> Vec<u8> {
        let Ok(part) = file.slice_with_i32_and_i32(0, 65_536) else {
            return Vec::new();
        };
        match JsFuture::from(part.array_buffer()).await {
            Ok(buffer) => js_sys::Uint8Array::new(&buffer).to_vec(),
            Err(_) => Vec::new(),
        }
    }

    async fn image(file: &File) -> Option<(HtmlImageElement, String)> {
        let address = Url::create_object_url_with_blob(file).ok()?;
        let image = HtmlImageElement::new().ok()?;
        let loaded = js_sys::Promise::new(&mut |resolve, reject| {
            image.set_onload(Some(&resolve));
            image.set_onerror(Some(&reject));
        });
        image.set_src(&address);
        if JsFuture::from(loaded).await.is_err() {
            let _ = Url::revoke_object_url(&address);
            return None;
        }
        Some((image, address))
    }

    /// Whether this browser lets the page read its own canvas: a drawn
    /// pattern must not read back as an empty canvas does (hardened
    /// browsers give a blank picture).
    fn readable() -> bool {
        let canvas = |paint: bool| -> Option<String> {
            let canvas: HtmlCanvasElement =
                document().create_element("canvas").ok()?.dyn_into().ok()?;
            canvas.set_width(4);
            canvas.set_height(4);
            let context: CanvasRenderingContext2d =
                canvas.get_context("2d").ok()??.dyn_into().ok()?;
            if paint {
                context.set_fill_style_str("#ff0000");
                context.fill_rect(0.0, 0.0, 2.0, 4.0);
                context.set_fill_style_str("#0000ff");
                context.fill_rect(2.0, 0.0, 2.0, 4.0);
            }
            canvas.to_data_url().ok()
        };
        matches!((canvas(true), canvas(false)), (Some(a), Some(b)) if a != b)
    }

    /// Opens the chosen file: its sides, and whether the page can read
    /// what it draws.
    pub async fn open() -> Result<(f64, f64, bool), Trouble> {
        close();
        let input: HtmlInputElement = element("picture-file").ok_or(Trouble::Failed)?;
        let file = input
            .files()
            .and_then(|files| files.get(0))
            .ok_or(Trouble::Open)?;
        input.set_value("");
        checked(file.size(), &head(&file).await)?;
        let options = ImageBitmapOptions::new();
        options.set_image_orientation(ImageOrientation::FromImage);
        let bitmap = match window()
            .create_image_bitmap_with_blob_and_image_bitmap_options(&file, &options)
        {
            Ok(promise) => JsFuture::from(promise)
                .await
                .ok()
                .and_then(|b| b.dyn_into::<ImageBitmap>().ok()),
            Err(_) => None,
        };
        let readable = readable();
        let (source, width, height) = if let Some(bitmap) = bitmap {
            let (w, h) = (f64::from(bitmap.width()), f64::from(bitmap.height()));
            (Source::Bitmap(bitmap), w, h)
        } else if let Some((image, address)) = image(&file).await {
            // An SVG without its own size is drawn at 512.
            let w = if image.natural_width() > 0 {
                f64::from(image.natural_width())
            } else {
                512.0
            };
            let h = if image.natural_height() > 0 {
                f64::from(image.natural_height())
            } else {
                512.0
            };
            (Source::Image(image, address), w, h)
        } else if accepted(&file.type_()).is_some() {
            // Central may still read what this browser cannot.
            OPENED.with(|o| {
                *o.borrow_mut() = Some(Opened {
                    source: Source::Unopened,
                    file,
                    readable: false,
                });
            });
            return Ok((1.0, 1.0, false));
        } else {
            return Err(Trouble::Open);
        };
        if width > f64::from(MAX_SIDE) || height > f64::from(MAX_SIDE) {
            close_source(source);
            return Err(Trouble::Huge);
        }
        OPENED.with(|o| {
            *o.borrow_mut() = Some(Opened {
                source,
                file,
                readable,
            })
        });
        Ok((width, height, readable))
    }

    fn close_source(source: Source) {
        match source {
            Source::Bitmap(bitmap) => bitmap.close(),
            Source::Image(_, address) => {
                let _ = Url::revoke_object_url(&address);
            }
            Source::Unopened => {}
        }
    }

    /// Lets go of the opened file.
    pub fn close() {
        if let Some(opened) = OPENED.with(|o| o.borrow_mut().take()) {
            close_source(opened.source);
        }
    }

    fn paint(canvas: &HtmlCanvasElement, crop: Crop, side: f64) -> Option<()> {
        let context: CanvasRenderingContext2d = canvas.get_context("2d").ok()??.dyn_into().ok()?;
        context.clear_rect(0.0, 0.0, side, side);
        let square = crop.side();
        OPENED.with(|o| {
            let opened = o.borrow();
            let opened = opened.as_ref()?;
            match &opened.source {
                Source::Bitmap(bitmap) => context
                    .draw_image_with_image_bitmap_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                        bitmap, crop.x, crop.y, square, square, 0.0, 0.0, side, side,
                    )
                    .ok(),
                Source::Image(image, _) => context
                    .draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                        image, crop.x, crop.y, square, square, 0.0, 0.0, side, side,
                    )
                    .ok(),
                Source::Unopened => None,
            }
        })
    }

    /// Draws the square into the preview.
    pub fn draw(crop: Crop) {
        if let Some(canvas) = element::<HtmlCanvasElement>("picture-preview") {
            let _ = paint(&canvas, crop, f64::from(PREVIEW));
        }
    }

    /// The preview's width on screen, for dragging.
    pub fn shown() -> f64 {
        element::<HtmlCanvasElement>("picture-preview")
            .map_or(f64::from(PREVIEW), |c| f64::from(c.client_width()))
    }

    /// Keeps the pointer's moves on the preview while it is pressed.
    pub fn capture(pointer: i32) {
        if let Some(canvas) = element::<HtmlCanvasElement>("picture-preview") {
            let _ = canvas.set_pointer_capture(pointer);
        }
    }

    async fn png(crop: Crop) -> Option<Blob> {
        let canvas: HtmlCanvasElement =
            document().create_element("canvas").ok()?.dyn_into().ok()?;
        let side = crop.out();
        canvas.set_width(side);
        canvas.set_height(side);
        paint(&canvas, crop, f64::from(side))?;
        let made = js_sys::Promise::new(&mut |resolve, _| {
            let done = Closure::once_into_js(move |blob: JsValue| {
                let _ = resolve.call1(&JsValue::NULL, &blob);
            });
            let _ = canvas.to_blob_with_type(done.unchecked_ref(), "image/png");
        });
        JsFuture::from(made).await.ok()?.dyn_into().ok()
    }

    /// Sends the square as a PNG, or, where the page cannot read its
    /// canvas or open the file, the file itself.
    pub async fn upload(crop: Crop) -> Result<Own, Trouble> {
        let (readable, file) = OPENED
            .with(|o| {
                o.borrow()
                    .as_ref()
                    .map(|opened| (opened.readable, opened.file.clone()))
            })
            .ok_or(Trouble::Failed)?;
        let (blob, kind): (Blob, String) = if readable {
            (
                png(crop).await.ok_or(Trouble::Failed)?,
                "image/png".to_owned(),
            )
        } else {
            let kind = accepted(&file.type_()).ok_or(Trouble::Refused("unsupported"))?;
            (file.into(), kind.to_owned())
        };
        let (status, text) = super::super::net::put_blob("/api/v1/picture", &blob, &kind)
            .await
            .map_err(|_| Trouble::Failed)?;
        answered(status, &text)
    }
}

/// On the server nothing is opened, drawn or sent.
#[cfg(not(target_arch = "wasm32"))]
mod browser {
    use super::{Crop, Own, Trouble};

    /// The checks run as in the browser, on nothing; nothing is opened.
    pub fn open() -> std::future::Ready<Result<(f64, f64, bool), Trouble>> {
        std::future::ready(super::checked(0.0, &[]).and(Err(Trouble::Open)))
    }
    pub fn close() {}
    pub fn draw(_: Crop) {}
    pub fn shown() -> f64 {
        f64::from(super::PREVIEW)
    }
    pub fn capture(_: i32) {}
    /// Nothing is sent; there is no answer to read.
    pub fn upload(_: Crop) -> std::future::Ready<Result<Own, Trouble>> {
        std::future::ready(super::answered(0, ""))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_square_starts_centred_and_stays_inside() {
        let crop = Crop::new(400.0, 200.0);
        assert_eq!((crop.x, crop.y, crop.side()), (100.0, 0.0, 200.0));
        let far = crop.moved(10.0, 10.0);
        assert_eq!((far.x, far.y), (200.0, 0.0));
        let back = far.moved(-10.0, 0.0);
        assert_eq!(back.x, 0.0);
        let zoomed = Crop::new(400.0, 400.0).zoomed(2.0);
        assert_eq!((zoomed.x, zoomed.y, zoomed.side()), (100.0, 100.0, 200.0));
        assert_eq!(Crop::new(400.0, 400.0).zoomed(100.0).zoom, MAX_ZOOM);
        assert_eq!(Crop::new(400.0, 400.0).zoomed(0.1).zoom, 1.0);
        // dragging right moves the picture right: the square left
        let start = Crop::new(800.0, 400.0);
        let dragged = start.dragged(start, 50.0, 0.0, 200.0);
        assert_eq!(dragged.x, 200.0 - 100.0);
        assert_eq!(start.spoken(), (50, 50, 100));
    }

    #[test]
    fn the_sent_side_is_the_square_s_own_within_bounds() {
        assert_eq!(Crop::new(100.0, 100.0).out(), 256);
        assert_eq!(Crop::new(300.0, 300.0).out(), 300);
        assert_eq!(Crop::new(4000.0, 3000.0).out(), 512);
    }

    #[test]
    fn headers_name_their_sides() {
        let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
        png.extend_from_slice(&30_000_u32.to_be_bytes());
        png.extend_from_slice(&20_u32.to_be_bytes());
        assert_eq!(claimed(&png), Some((30_000, 20)));
        assert_eq!(claimed(b"GIF89a\x10\x00\x20\x00"), Some((16, 32)));
        let jpeg = [
            0xff, 0xd8, 0xff, 0xe0, 0, 4, 0, 0, 0xff, 0xc0, 0, 11, 8, 0x75, 0x30, 0x00, 0x20, 3, 0,
            0,
        ];
        assert_eq!(claimed(&jpeg), Some((32, 30_000)));
        let mut bmp = b"BM".to_vec();
        bmp.extend_from_slice(&[0; 16]);
        bmp.extend_from_slice(&20_000_i32.to_le_bytes());
        bmp.extend_from_slice(&(-10_i32).to_le_bytes());
        assert_eq!(claimed(&bmp), Some((20_000, 10)));
        let mut webp = b"RIFF\0\0\0\0WEBPVP8X\x0a\0\0\0\0\0\0\0".to_vec();
        webp.extend_from_slice(&[0xff, 0xff, 0x00, 0x0f, 0x00, 0x00]);
        webp.extend_from_slice(&[0; 4]);
        assert_eq!(claimed(&webp), Some((65_536, 16)));
        assert_eq!(claimed(b"<svg/>"), None);
        assert_eq!(claimed(b"\x89PNG"), None);
    }

    #[test]
    fn central_s_answers_become_the_page_s_words() {
        assert_eq!(
            answered(422, r#"{"refusal":"too_small"}"#),
            Err(Trouble::Refused("too_small"))
        );
        assert_eq!(
            answered(422, r#"{"refusal":"<script>"}"#),
            Err(Trouble::Failed)
        );
        assert_eq!(
            answered(403, r#"{"refusal":"blocked"}"#),
            Err(Trouble::Blocked)
        );
        assert_eq!(
            answered(403, "account request rejected\n"),
            Err(Trouble::Failed)
        );
        assert_eq!(answered(401, ""), Err(Trouble::SignedOut));
        assert_eq!(answered(429, ""), Err(Trouble::Busy));
        let stored = answered(
            200,
            r#"{"result":"my_picture","picture":null,"notice":null,"block":null}"#,
        );
        assert_eq!(
            stored,
            Ok(Own {
                picture: None,
                notice: None,
                block: None
            })
        );
        assert_eq!(answered(200, r#"{"result":"done"}"#), Err(Trouble::Failed));
    }

    #[test]
    fn addresses_name_the_key_and_encoding() {
        assert_eq!(
            url(&[0xab; 16], true),
            format!("/api/v1/pictures/{}.avif", "ab".repeat(16))
        );
        assert!(url(&[1; 16], false).ends_with("01.png"));
    }
}
