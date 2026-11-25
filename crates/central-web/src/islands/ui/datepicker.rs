//! Custom styled locale-aware date picker matching Baylee Central's form
//! design, supporting manual typed entry, 3-mode calendar navigation
//! (day/month/decade grids), keyboard navigation, outside-click dismissal,
//! live age display, and minimum-age feedback.

use leptos::html;
use leptos::prelude::*;

use crate::words::{self, Lang};

use super::super::chat::glyph::{self, glyph};
use super::super::{focus, time};

const MIN_YEAR: i32 = 1900;
const MAX_YEAR: i32 = 2100;
const MIN_AGE_YEARS: u64 = 16;

#[derive(Clone, Copy, PartialEq, Eq)]
enum ViewMode {
    Days,
    Months,
    Years,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DateOrder {
    Dmy,
    Mdy,
    Ymd,
}

const fn date_order(lang: Lang) -> DateOrder {
    match lang {
        Lang::En => DateOrder::Mdy,
        Lang::Ja | Lang::Ko | Lang::ZhHans | Lang::ZhHant => DateOrder::Ymd,
        Lang::De | Lang::Fr | Lang::It | Lang::Es | Lang::PtBr | Lang::Ru => DateOrder::Dmy,
    }
}

/// Current UTC Unix timestamp in seconds (`wasm32` reads `Date.now()`, SSR uses
/// a deterministic baseline).
#[must_use]
pub fn unix_today_secs() -> i64 {
    #[cfg(target_arch = "wasm32")]
    {
        // Current wall-clock seconds (~1.8e9) fit well inside `i64`.
        #[allow(clippy::cast_possible_truncation)]
        let secs = (js_sys::Date::now() / 1000.0) as i64;
        secs
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        1_800_000_000
    }
}

fn today_ymd() -> (i32, u8, u8) {
    central_api::profile::utc_date_from_unix_secs(unix_today_secs())
}

fn format_for_input(lang: Lang, year: i32, month: u8, day: u8) -> String {
    match date_order(lang) {
        DateOrder::Mdy => format!("{month:02}/{day:02}/{year:04}"),
        DateOrder::Ymd => format!("{year:04}-{month:02}-{day:02}"),
        DateOrder::Dmy => format!("{day:02}.{month:02}.{year:04}"),
    }
}

fn valid_ymd(year: i32, month: u8, day: u8) -> Option<(i32, u8, u8)> {
    if !(MIN_YEAR..=MAX_YEAR).contains(&year) || !(1..=12).contains(&month) || day == 0 {
        return None;
    }
    if day > time::days_in_month(year, month) {
        return None;
    }
    Some((year, month, day))
}

fn parse_three_parts(a: &str, b: &str, c: &str, first_is_day: bool) -> Option<(i32, u8, u8)> {
    if c.len() != 4 || !(1..=2).contains(&a.len()) || !(1..=2).contains(&b.len()) {
        return None;
    }
    let year: i32 = c.parse().ok()?;
    let p1: u8 = a.parse().ok()?;
    let p2: u8 = b.parse().ok()?;
    let (day, month) = if first_is_day { (p1, p2) } else { (p2, p1) };
    valid_ymd(year, month, day)
}

/// Parse a user-typed date in `DD.MM.YYYY`, `YYYY-MM-DD`, or `MM/DD/YYYY`
/// (accepting localized slash/dot/ISO separators) within `1900..=2100`.
#[must_use]
pub fn parse_manual_date(lang: Lang, raw: &str) -> Option<(i32, u8, u8)> {
    let trimmed = raw.trim();
    if trimmed.is_empty()
        || trimmed.len() > 16
        || !trimmed
            .bytes()
            .all(|b| b.is_ascii_digit() || matches!(b, b'.' | b'-' | b'/'))
    {
        return None;
    }
    if trimmed.contains('-') && !trimmed.contains('.') && !trimmed.contains('/') {
        let mut parts = trimmed.split('-');
        let y_str = parts.next()?;
        let m_str = parts.next()?;
        let d_str = parts.next()?;
        if parts.next().is_some()
            || y_str.len() != 4
            || !(1..=2).contains(&m_str.len())
            || !(1..=2).contains(&d_str.len())
        {
            return None;
        }
        return valid_ymd(
            y_str.parse().ok()?,
            m_str.parse().ok()?,
            d_str.parse().ok()?,
        );
    }
    if trimmed.contains('.') && !trimmed.contains('-') && !trimmed.contains('/') {
        let mut parts = trimmed.split('.');
        let d_str = parts.next()?;
        let m_str = parts.next()?;
        let y_str = parts.next()?;
        if parts.next().is_some() {
            return None;
        }
        return parse_three_parts(d_str, m_str, y_str, true);
    }
    if trimmed.contains('/') && !trimmed.contains('-') && !trimmed.contains('.') {
        let mut parts = trimmed.split('/');
        let p0 = parts.next()?;
        let p1 = parts.next()?;
        let p2 = parts.next()?;
        if parts.next().is_some() {
            return None;
        }
        if p0.len() == 4 && (1..=2).contains(&p1.len()) && (1..=2).contains(&p2.len()) {
            return valid_ymd(p0.parse().ok()?, p1.parse().ok()?, p2.parse().ok()?);
        }
        let first_is_day = !matches!(date_order(lang), DateOrder::Mdy | DateOrder::Ymd);
        return parse_three_parts(p0, p1, p2, first_is_day);
    }
    None
}

fn looks_immediately_invalid(trimmed: &str) -> bool {
    !trimmed.is_empty()
        && (trimmed.len() >= 8
            || !trimmed
                .bytes()
                .all(|b| b.is_ascii_digit() || matches!(b, b'.' | b'-' | b'/')))
}

fn age_in_years(birth: (i32, u8, u8), today: (i32, u8, u8)) -> Option<u64> {
    if birth > today {
        return None;
    }
    let mut years = today.0 - birth.0;
    if birth.1 > today.1 || (birth.1 == today.1 && birth.2 > today.2) {
        years -= 1;
    }
    u64::try_from(years).ok()
}

const fn clamp_ymd(year: i32, month: u8, day: u8) -> (i32, u8, u8) {
    let y = if year < MIN_YEAR {
        MIN_YEAR
    } else if year > MAX_YEAR {
        MAX_YEAR
    } else {
        year
    };
    let m = if month < 1 {
        1
    } else if month > 12 {
        12
    } else {
        month
    };
    let max_d = time::days_in_month(y, m);
    let d = if day == 0 {
        1
    } else if day > max_d {
        max_d
    } else {
        day
    };
    (y, m, d)
}

fn shift_months(year: i32, month: u8, day: u8, delta: i32) -> (i32, u8, u8) {
    let min_total = MIN_YEAR * 12;
    let max_total = MAX_YEAR * 12 + 11;
    let total = (year * 12 + i32::from(month - 1) + delta).clamp(min_total, max_total);
    let new_y = total.div_euclid(12);
    let new_m = u8::try_from(total.rem_euclid(12) + 1).unwrap_or(1);
    clamp_ymd(new_y, new_m, day)
}

fn step_days(year: i32, month: u8, day: u8, delta: i32) -> (i32, u8, u8) {
    let (mut y, mut m, mut d) = clamp_ymd(year, month, day);
    if delta >= 0 {
        for _ in 0..delta {
            let max_d = time::days_in_month(y, m);
            if d < max_d {
                d += 1;
            } else if m < 12 {
                m += 1;
                d = 1;
            } else if y < MAX_YEAR {
                y += 1;
                m = 1;
                d = 1;
            }
        }
    } else {
        for _ in 0..(-delta) {
            if d > 1 {
                d -= 1;
            } else if m > 1 {
                m -= 1;
                d = time::days_in_month(y, m);
            } else if y > MIN_YEAR {
                y -= 1;
                m = 12;
                d = 31;
            }
        }
    }
    (y, m, d)
}

const fn decade_start(year: i32) -> i32 {
    let clamped = if year < MIN_YEAR {
        MIN_YEAR
    } else if year > MAX_YEAR {
        MAX_YEAR
    } else {
        year
    };
    MIN_YEAR + ((clamped - MIN_YEAR) / 12) * 12
}

#[cfg(target_arch = "wasm32")]
fn intl_month_name(lang: Lang, month: u8, short: bool) -> Option<String> {
    use wasm_bindgen::{JsCast, JsValue};

    let intl = js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str("Intl")).ok()?;
    let dtf: js_sys::Function = js_sys::Reflect::get(&intl, &JsValue::from_str("DateTimeFormat"))
        .ok()?
        .dyn_into()
        .ok()?;
    let opts = js_sys::Object::new();
    let style = if short { "short" } else { "long" };
    let _ = js_sys::Reflect::set(
        &opts,
        &JsValue::from_str("month"),
        &JsValue::from_str(style),
    );
    let _ = js_sys::Reflect::set(
        &opts,
        &JsValue::from_str("timeZone"),
        &JsValue::from_str("UTC"),
    );
    let formatter = js_sys::Reflect::construct(
        &dtf,
        &js_sys::Array::of2(&JsValue::from_str(lang.tag()), &opts),
    )
    .ok()?;
    let format_fn: js_sys::Function =
        js_sys::Reflect::get(&formatter, &JsValue::from_str("format"))
            .ok()?
            .dyn_into()
            .ok()?;
    let ms = js_sys::Date::utc(2024.0, f64::from(month.saturating_sub(1))) + 14.0 * 86_400_000.0;
    let date = js_sys::Date::new(&JsValue::from_f64(ms));
    format_fn.call1(&formatter, &date).ok()?.as_string()
}

fn month_name(lang: Lang, month: u8, short: bool) -> String {
    #[cfg(target_arch = "wasm32")]
    if let Some(formatted) = intl_month_name(lang, month, short) {
        return formatted;
    }
    let idx = usize::from(month.saturating_sub(1).min(11));
    let fallback = match lang {
        Lang::De => [
            "Jan", "Feb", "Mär", "Apr", "Mai", "Jun", "Jul", "Aug", "Sep", "Okt", "Nov", "Dez",
        ],
        _ if short => [
            "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
        ],
        _ => [
            "January",
            "February",
            "March",
            "April",
            "May",
            "June",
            "July",
            "August",
            "September",
            "October",
            "November",
            "December",
        ],
    };
    fallback[idx].to_owned()
}

fn target_inside_wrap(
    ev: &leptos::ev::PointerEvent,
    wrap: &leptos::web_sys::HtmlDivElement,
) -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        use js_sys::{Function, Reflect};
        use wasm_bindgen::{JsCast, JsValue};
        let Some(target) = ev.target() else {
            return false;
        };
        let el: &leptos::web_sys::Element = wrap.as_ref();
        let wrap_val: &JsValue = el.as_ref();
        let target_val: JsValue = target.into();
        Reflect::get(wrap_val, &JsValue::from_str("contains"))
            .ok()
            .and_then(|f| f.dyn_into::<Function>().ok())
            .and_then(|f| f.call1(wrap_val, &target_val).ok())
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (ev, wrap);
        false
    }
}

#[derive(Clone, Copy)]
struct PickerState {
    id: &'static str,
    lang: Lang,
    value: RwSignal<Option<String>>,
    draft: RwSignal<String>,
    last_synced_iso: RwSignal<Option<String>>,
    format_invalid: RwSignal<bool>,
    open: RwSignal<bool>,
    mode: RwSignal<ViewMode>,
    view_year: RwSignal<i32>,
    view_month: RwSignal<u8>,
    focused_day: RwSignal<u8>,
}

impl PickerState {
    fn select_ymd(self, year: i32, month: u8, day: u8) {
        let (y, m, d) = clamp_ymd(year, month, day);
        let iso = time::format_iso_date(y, m, d);
        self.view_year.set(y);
        self.view_month.set(m);
        self.focused_day.set(d);
        self.format_invalid.set(false);
        self.draft.set(format_for_input(self.lang, y, m, d));
        self.last_synced_iso.set(Some(iso.clone()));
        self.value.set(Some(iso));
        self.open.set(false);
        focus::after_render(format!("#{}", self.id));
    }

    fn clear_value(self) {
        self.format_invalid.set(false);
        self.draft.set(String::new());
        self.last_synced_iso.set(None);
        self.value.set(None);
        self.open.set(false);
        focus::after_render(format!("#{}", self.id));
    }

    fn close_and_focus_trigger(self) {
        self.open.set(false);
        focus::after_render(format!("#{}-trigger", self.id));
    }

    fn focus_grid_cell(self) {
        focus::after_render(format!("#{}-pop [data-dp-focus=\"true\"]", self.id));
    }

    fn set_cursor_ymd(self, year: i32, month: u8, day: u8) {
        let (y, m, d) = clamp_ymd(year, month, day);
        self.view_year.set(y);
        self.view_month.set(m);
        self.focused_day.set(d);
    }

    fn apply_manual_input(self, raw: &str) {
        self.draft.set(raw.to_owned());
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            self.format_invalid.set(false);
            self.last_synced_iso.set(None);
            self.value.set(None);
        } else if let Some((y, m, d)) = parse_manual_date(self.lang, trimmed) {
            let iso = time::format_iso_date(y, m, d);
            self.format_invalid.set(false);
            self.set_cursor_ymd(y, m, d);
            self.last_synced_iso.set(Some(iso.clone()));
            self.value.set(Some(iso));
        } else {
            self.last_synced_iso.set(None);
            self.value.set(None);
            self.format_invalid.set(looks_immediately_invalid(trimmed));
        }
    }

    fn validate_on_commit(self) {
        let raw = self.draft.get_untracked();
        let trimmed = raw.trim();
        if !trimmed.is_empty() && parse_manual_date(self.lang, trimmed).is_none() {
            self.format_invalid.set(true);
        }
    }

    fn toggle_from_trigger(self) {
        if self.open.get_untracked() {
            self.open.set(false);
            return;
        }
        if let Some(iso) = self.value.get_untracked()
            && let Some((y, m, d)) = time::parse_iso_date(&iso)
        {
            self.set_cursor_ymd(y, m, d);
        }
        self.mode.set(ViewMode::Days);
        self.open.set(true);
        self.focus_grid_cell();
    }

    fn handle_input_keydown(self, ev: &leptos::ev::KeyboardEvent) {
        if ev.key() == "Enter" {
            self.validate_on_commit();
            if self.format_invalid.get_untracked() || self.open.get_untracked() {
                ev.prevent_default();
                self.open.set(false);
            }
        } else if ev.key() == "ArrowDown" && ev.alt_key() {
            ev.prevent_default();
            self.mode.set(ViewMode::Days);
            self.open.set(true);
            self.focus_grid_cell();
        }
    }

    fn sync_from_value(self) {
        let current = self.value.get();
        if current == self.last_synced_iso.get_untracked() {
            return;
        }
        self.last_synced_iso.set(current.clone());
        self.format_invalid.set(false);
        if let Some(iso) = current
            && let Some((y, m, d)) = time::parse_iso_date(&iso)
        {
            self.draft.set(format_for_input(self.lang, y, m, d));
            self.set_cursor_ymd(y, m, d);
        } else {
            self.draft.set(String::new());
        }
    }
}

fn age_pill(lang: Lang, value: RwSignal<Option<String>>) -> impl IntoView {
    move || {
        let raw = value.get()?;
        let birth = time::parse_iso_date(&raw)?;
        let yrs = age_in_years(birth, today_ymd())?;
        let cls = if yrs < MIN_AGE_YEARS {
            "pill warn dp-age-pill"
        } else {
            "pill dp-age-pill"
        };
        Some(view! { <span class=cls>{words::dp_age_years(lang, yrs)}</span> })
    }
}

/// Renders an interactive date input + 3-mode popover calendar (day/month/decade)
/// bound bidirectionally to ISO `YYYY-MM-DD` `value`, with keyboard navigation,
/// outside-click dismissal, Today/Clear controls, and a live age pill.
pub fn date_picker(
    lang: Lang,
    id: &'static str,
    label: &'static str,
    value: RwSignal<Option<String>>,
    error_text: Signal<Option<&'static str>>,
) -> AnyView {
    let init_raw = value.get_untracked();
    let init_ymd = init_raw
        .as_deref()
        .and_then(time::parse_iso_date)
        .unwrap_or((2000, 1, 1));
    let init_draft = init_raw
        .as_deref()
        .and_then(time::parse_iso_date)
        .map(|(y, m, d)| format_for_input(lang, y, m, d))
        .unwrap_or_default();

    let state = PickerState {
        id,
        lang,
        value,
        draft: RwSignal::new(init_draft),
        last_synced_iso: RwSignal::new(init_raw),
        format_invalid: RwSignal::new(false),
        open: RwSignal::new(false),
        mode: RwSignal::new(ViewMode::Days),
        view_year: RwSignal::new(init_ymd.0),
        view_month: RwSignal::new(init_ymd.1),
        focused_day: RwSignal::new(init_ymd.2),
    };

    let wrap_ref = NodeRef::<html::Div>::new();
    let outside = window_event_listener(leptos::ev::pointerdown, move |ev| {
        if state.open.get_untracked()
            && let Some(wrap) = wrap_ref.get_untracked()
            && !target_inside_wrap(&ev, &wrap)
        {
            state.open.set(false);
        }
    });
    on_cleanup(move || outside.remove());

    // Keep text input and popover view synced when `value` changes externally.
    Effect::new(move |_| state.sync_from_value());

    let active_error = move || {
        state
            .format_invalid
            .get()
            .then(|| words::dp_invalid_format(lang))
            .or_else(|| error_text.get())
    };
    let hint_id = format!("{id}-hint");
    let trigger_id = format!("{id}-trigger");

    view! {
        <div
            class="field datepicker-field"
            node_ref=wrap_ref
            on:keydown=move |ev: leptos::ev::KeyboardEvent| {
                if ev.key() == "Escape" && state.open.get_untracked() {
                    ev.prevent_default();
                    ev.stop_propagation();
                    state.close_and_focus_trigger();
                }
            }
        >
            <div class="dp-label-row">
                <label for=id>{label}</label>
                {age_pill(lang, value)}
            </div>
            <div class="inset">
                <span class="inset-mark">{glyph(glyph::CALENDAR)}</span>
                <input
                    id=id
                    type="text"
                    inputmode="numeric"
                    autocomplete="bday"
                    placeholder=words::date_picker_placeholder(lang)
                    aria-describedby=hint_id.clone()
                    aria-invalid=move || active_error().is_some().then_some("true")
                    prop:value=move || state.draft.get()
                    on:input=move |ev| state.apply_manual_input(&event_target_value(&ev))
                    on:blur=move |_| state.validate_on_commit()
                    on:keydown=move |ev| state.handle_input_keydown(&ev)
                />
                <button
                    id=trigger_id
                    type="button"
                    class="inset-end datepicker-trigger"
                    title=words::date_picker_open(lang)
                    aria-label=words::date_picker_open(lang)
                    aria-expanded=move || if state.open.get() { "true" } else { "false" }
                    on:click=move |_| state.toggle_from_trigger()
                >
                    {glyph(glyph::CALENDAR)}
                </button>
            </div>
            <p class="hint dp-hint" id=hint_id>{words::dp_type_hint(lang)}</p>
            {move || state.open.get().then(|| calendar_popover(state, label))}
            {move || active_error().map(|err| view! { <p class="hint dp-error" role="alert">{err}</p> })}
        </div>
    }
    .into_any()
}

fn days_header(state: PickerState, y: i32, m: u8) -> AnyView {
    let lang = state.lang;
    let month_title = month_name(lang, m, false);
    let prev_disabled = y <= MIN_YEAR && m == 1;
    let next_disabled = y >= MAX_YEAR && m == 12;
    view! {
        <div class="dp-head">
            <button
                type="button"
                class="dp-nav"
                title=words::date_picker_prev_month(lang)
                aria-label=words::date_picker_prev_month(lang)
                disabled=prev_disabled
                on:click=move |_| {
                    let (ny, nm, nd) = shift_months(y, m, state.focused_day.get_untracked(), -1);
                    state.set_cursor_ymd(ny, nm, nd);
                }
            >
                {glyph(glyph::CHEVRON_LEFT)}
            </button>
            <div class="dp-title-group">
                <button
                    type="button"
                    class="dp-title-btn"
                    title=words::dp_pick_month(lang)
                    aria-label=words::dp_pick_month(lang)
                    on:click=move |_| {
                        state.mode.set(ViewMode::Months);
                        state.focus_grid_cell();
                    }
                >
                    {month_title}
                </button>
                <button
                    type="button"
                    class="dp-title-btn"
                    title=words::dp_pick_year(lang)
                    aria-label=words::dp_pick_year(lang)
                    on:click=move |_| {
                        state.mode.set(ViewMode::Years);
                        state.focus_grid_cell();
                    }
                >
                    {y.to_string()}
                </button>
            </div>
            <button
                type="button"
                class="dp-nav"
                title=words::date_picker_next_month(lang)
                aria-label=words::date_picker_next_month(lang)
                disabled=next_disabled
                on:click=move |_| {
                    let (ny, nm, nd) = shift_months(y, m, state.focused_day.get_untracked(), 1);
                    state.set_cursor_ymd(ny, nm, nd);
                }
            >
                {glyph(glyph::CHEVRON_RIGHT)}
            </button>
        </div>
    }
    .into_any()
}

fn months_header(state: PickerState, y: i32, m: u8) -> AnyView {
    let lang = state.lang;
    view! {
        <div class="dp-head">
            <button
                type="button"
                class="dp-nav"
                title=words::date_picker_prev_year(lang)
                aria-label=words::date_picker_prev_year(lang)
                disabled=y <= MIN_YEAR
                on:click=move |_| state.set_cursor_ymd(y - 1, m, state.focused_day.get_untracked())
            >
                {glyph(glyph::CHEVRON_LEFT)}
            </button>
            <div class="dp-title-group">
                <button
                    type="button"
                    class="dp-title-btn"
                    title=words::dp_pick_year(lang)
                    aria-label=words::dp_pick_year(lang)
                    on:click=move |_| {
                        state.mode.set(ViewMode::Years);
                        state.focus_grid_cell();
                    }
                >
                    {y.to_string()}
                </button>
            </div>
            <button
                type="button"
                class="dp-nav"
                title=words::date_picker_next_year(lang)
                aria-label=words::date_picker_next_year(lang)
                disabled=y >= MAX_YEAR
                on:click=move |_| state.set_cursor_ymd(y + 1, m, state.focused_day.get_untracked())
            >
                {glyph(glyph::CHEVRON_RIGHT)}
            </button>
        </div>
    }
    .into_any()
}

fn years_header(state: PickerState, y: i32, m: u8) -> AnyView {
    let lang = state.lang;
    let start = decade_start(y);
    let end = (start + 11).min(MAX_YEAR);
    view! {
        <div class="dp-head">
            <button
                type="button"
                class="dp-nav"
                title=words::dp_prev_decade(lang)
                aria-label=words::dp_prev_decade(lang)
                disabled=start <= MIN_YEAR
                on:click=move |_| state.set_cursor_ymd(y - 12, m, state.focused_day.get_untracked())
            >
                {glyph(glyph::CHEVRON_LEFT)}
            </button>
            <span class="dp-title" translate="no">{format!("{start} – {end}")}</span>
            <button
                type="button"
                class="dp-nav"
                title=words::dp_next_decade(lang)
                aria-label=words::dp_next_decade(lang)
                disabled=start + 12 > MAX_YEAR
                on:click=move |_| state.set_cursor_ymd(y + 12, m, state.focused_day.get_untracked())
            >
                {glyph(glyph::CHEVRON_RIGHT)}
            </button>
        </div>
    }
    .into_any()
}

fn calendar_header(state: PickerState) -> AnyView {
    let y = state.view_year.get();
    let m = state.view_month.get();
    match state.mode.get() {
        ViewMode::Days => days_header(state, y, m),
        ViewMode::Months => months_header(state, y, m),
        ViewMode::Years => years_header(state, y, m),
    }
}

fn handle_day_keydown(state: PickerState, ev: &leptos::ev::KeyboardEvent) {
    let y = state.view_year.get_untracked();
    let m = state.view_month.get_untracked();
    let d = state.focused_day.get_untracked();
    let next = match ev.key().as_str() {
        "ArrowLeft" => Some(step_days(y, m, d, -1)),
        "ArrowRight" => Some(step_days(y, m, d, 1)),
        "ArrowUp" => Some(step_days(y, m, d, -7)),
        "ArrowDown" => Some(step_days(y, m, d, 7)),
        "Home" => Some((y, m, 1)),
        "End" => Some((y, m, time::days_in_month(y, m))),
        "PageUp" => {
            let delta = if ev.shift_key() { -12 } else { -1 };
            Some(shift_months(y, m, d, delta))
        }
        "PageDown" => {
            let delta = if ev.shift_key() { 12 } else { 1 };
            Some(shift_months(y, m, d, delta))
        }
        _ => None,
    };
    if let Some((ny, nm, nd)) = next {
        ev.prevent_default();
        state.set_cursor_ymd(ny, nm, nd);
        state.focus_grid_cell();
    }
}

fn handle_month_keydown(state: PickerState, ev: &leptos::ev::KeyboardEvent) {
    let y = state.view_year.get_untracked();
    let m = state.view_month.get_untracked();
    let d = state.focused_day.get_untracked();
    let next = match ev.key().as_str() {
        "ArrowLeft" => Some(shift_months(y, m, d, -1)),
        "ArrowRight" => Some(shift_months(y, m, d, 1)),
        "ArrowUp" => Some(shift_months(y, m, d, -3)),
        "ArrowDown" => Some(shift_months(y, m, d, 3)),
        "Home" => Some(clamp_ymd(y, 1, d)),
        "End" => Some(clamp_ymd(y, 12, d)),
        "PageUp" => Some(clamp_ymd(y - 1, m, d)),
        "PageDown" => Some(clamp_ymd(y + 1, m, d)),
        _ => None,
    };
    if let Some((ny, nm, nd)) = next {
        ev.prevent_default();
        state.set_cursor_ymd(ny, nm, nd);
        state.focus_grid_cell();
    }
}

fn handle_year_keydown(state: PickerState, ev: &leptos::ev::KeyboardEvent) {
    let y = state.view_year.get_untracked();
    let m = state.view_month.get_untracked();
    let d = state.focused_day.get_untracked();
    let start = decade_start(y);
    let end = (start + 11).min(MAX_YEAR);
    let next_y = match ev.key().as_str() {
        "ArrowLeft" => Some(y - 1),
        "ArrowRight" => Some(y + 1),
        "ArrowUp" => Some(y - 3),
        "ArrowDown" => Some(y + 3),
        "Home" => Some(start),
        "End" => Some(end),
        "PageUp" => Some(y - 12),
        "PageDown" => Some(y + 12),
        _ => None,
    };
    if let Some(ny) = next_y {
        ev.prevent_default();
        state.set_cursor_ymd(ny, m, d);
        state.focus_grid_cell();
    }
}

fn day_grid(state: PickerState) -> AnyView {
    let lang = state.lang;
    let y = state.view_year.get();
    let m = state.view_month.get();
    let focus_d = state.focused_day.get().min(time::days_in_month(y, m));
    let lead_blanks = usize::from(time::weekday_monday0(y, m, 1));
    let days = time::days_in_month(y, m);
    let selected = state.value.get();
    let (ty, tm, td) = today_ymd();

    let wd_views = time::weekday_shorts(lang)
        .into_iter()
        .map(|wd| view! { <span class="dp-wd" aria-hidden="true">{wd}</span> })
        .collect_view();

    let cells = (1..=days)
        .map(|d| {
            let iso = time::format_iso_date(y, m, d);
            let is_sel = selected.as_deref() == Some(iso.as_str());
            let is_today = y == ty && m == tm && d == td;
            let is_focus = d == focus_d;
            let col_start = (d == 1 && lead_blanks > 0).then(|| (lead_blanks + 1).to_string());
            view! {
                <button
                    type="button"
                    class="dp-day"
                    class:is-selected=is_sel
                    class:is-today=is_today
                    class:is-focused=is_focus
                    style:grid-column-start=col_start
                    tabindex=if is_focus { "0" } else { "-1" }
                    data-dp-focus=is_focus.then_some("true")
                    aria-pressed=if is_sel { "true" } else { "false" }
                    aria-current=is_today.then_some("date")
                    on:click=move |_| state.select_ymd(y, m, d)
                >
                    {d.to_string()}
                </button>
            }
        })
        .collect_view();

    view! {
        <div class="dp-weekdays">{wd_views}</div>
        <div
            class="dp-grid"
            role="grid"
            on:keydown=move |ev: leptos::ev::KeyboardEvent| handle_day_keydown(state, &ev)
        >
            {cells}
        </div>
    }
    .into_any()
}

fn month_grid(state: PickerState) -> AnyView {
    let lang = state.lang;
    let y = state.view_year.get();
    let active_m = state.view_month.get();
    let sel_ym = state
        .value
        .get()
        .as_deref()
        .and_then(time::parse_iso_date)
        .map(|(sy, sm, _)| (sy, sm));

    let cells = (1_u8..=12)
        .map(|m| {
            let is_sel = sel_ym == Some((y, m));
            let is_focus = m == active_m;
            view! {
                <button
                    type="button"
                    class="dp-cell"
                    class:is-selected=is_sel
                    class:is-focused=is_focus
                    tabindex=if is_focus { "0" } else { "-1" }
                    data-dp-focus=is_focus.then_some("true")
                    aria-pressed=if is_sel { "true" } else { "false" }
                    on:click=move |_| {
                        state.set_cursor_ymd(y, m, state.focused_day.get_untracked());
                        state.mode.set(ViewMode::Days);
                        state.focus_grid_cell();
                    }
                >
                    {month_name(lang, m, true)}
                </button>
            }
        })
        .collect_view();

    view! {
        <div
            class="dp-months"
            role="grid"
            on:keydown=move |ev: leptos::ev::KeyboardEvent| handle_month_keydown(state, &ev)
        >
            {cells}
        </div>
    }
    .into_any()
}

fn year_grid(state: PickerState) -> AnyView {
    let active_y = state.view_year.get().clamp(MIN_YEAR, MAX_YEAR);
    let start = decade_start(active_y);
    let sel_y = state
        .value
        .get()
        .as_deref()
        .and_then(time::parse_iso_date)
        .map(|(sy, _, _)| sy);

    let cells = (0..12_i32)
        .map(|offset| {
            let yr = start + offset;
            let in_bounds = yr <= MAX_YEAR;
            let is_sel = in_bounds && sel_y == Some(yr);
            let is_focus = yr == active_y;
            view! {
                <button
                    type="button"
                    class="dp-cell"
                    class:is-selected=is_sel
                    class:is-focused=is_focus
                    disabled=!in_bounds
                    tabindex=if is_focus { "0" } else { "-1" }
                    data-dp-focus=is_focus.then_some("true")
                    aria-pressed=if is_sel { "true" } else { "false" }
                    on:click=move |_| {
                        if in_bounds {
                            state.set_cursor_ymd(yr, state.view_month.get_untracked(), state.focused_day.get_untracked());
                            state.mode.set(ViewMode::Months);
                            state.focus_grid_cell();
                        }
                    }
                >
                    {yr.to_string()}
                </button>
            }
        })
        .collect_view();

    view! {
        <div
            class="dp-years"
            role="grid"
            on:keydown=move |ev: leptos::ev::KeyboardEvent| handle_year_keydown(state, &ev)
        >
            {cells}
        </div>
    }
    .into_any()
}

fn calendar_popover(state: PickerState, label: &'static str) -> AnyView {
    let lang = state.lang;
    let pop_id = format!("{}-pop", state.id);
    view! {
        <div class="dp-popover" id=pop_id role="dialog" aria-label=label>
            {move || calendar_header(state)}
            {move || match state.mode.get() {
                ViewMode::Days => day_grid(state),
                ViewMode::Months => month_grid(state),
                ViewMode::Years => year_grid(state),
            }}
            <div class="dp-foot">
                <button
                    type="button"
                    class="button subtle"
                    on:click=move |_| {
                        let (ty, tm, td) = today_ymd();
                        state.select_ymd(ty, tm, td);
                    }
                >
                    {words::dp_today(lang)}
                </button>
                <button
                    type="button"
                    class="button subtle"
                    on:click=move |_| state.clear_value()
                >
                    {words::dp_clear(lang)}
                </button>
            </div>
        </div>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_locale_manual_dates() {
        assert_eq!(
            parse_manual_date(Lang::De, "24.05.1998"),
            Some((1998, 5, 24))
        );
        assert_eq!(parse_manual_date(Lang::De, "5.4.2000"), Some((2000, 4, 5)));
        assert_eq!(
            parse_manual_date(Lang::De, "1998-05-24"),
            Some((1998, 5, 24))
        );
        assert_eq!(
            parse_manual_date(Lang::En, "05/24/1998"),
            Some((1998, 5, 24))
        );
        assert_eq!(parse_manual_date(Lang::En, "24/05/1998"), None);
        assert_eq!(
            parse_manual_date(Lang::En, "24.05.1998"),
            Some((1998, 5, 24))
        );
        assert_eq!(
            parse_manual_date(Lang::Fr, "24/05/1998"),
            Some((1998, 5, 24))
        );
        assert_eq!(
            parse_manual_date(Lang::Ja, "1998-05-24"),
            Some((1998, 5, 24))
        );
        assert_eq!(
            parse_manual_date(Lang::De, "29.02.2000"),
            Some((2000, 2, 29))
        );
        assert_eq!(parse_manual_date(Lang::De, "29.02.2001"), None);
        assert_eq!(parse_manual_date(Lang::En, "31.12.1899"), None);
        assert_eq!(parse_manual_date(Lang::En, "not-a-date"), None);
    }

    #[test]
    fn steps_days_and_months_cleanly() {
        assert_eq!(step_days(2024, 2, 29, 1), (2024, 3, 1));
        assert_eq!(step_days(2024, 3, 1, -1), (2024, 2, 29));
        assert_eq!(step_days(2024, 12, 28, 7), (2025, 1, 4));
        assert_eq!(shift_months(2024, 1, 31, 1), (2024, 2, 29));
        assert_eq!(shift_months(2024, 1, 15, -1), (2023, 12, 15));
        assert_eq!(shift_months(MIN_YEAR, 1, 15, -1), (MIN_YEAR, 1, 15));
        assert_eq!(shift_months(MAX_YEAR, 12, 15, 1), (MAX_YEAR, 12, 15));
        assert_eq!(decade_start(2000), 1996);
        assert_eq!(age_in_years((2000, 5, 24), (2026, 5, 23)), Some(25));
        assert_eq!(age_in_years((2000, 5, 24), (2026, 5, 24)), Some(26));
    }
}
