//! The site's own filled glyphs, as the old site drew them: one path in a
//! 24 × 24 box, filled with the current colour. Drawn as elements (no
//! markup string), so they work in the islands under Trusted Types too, and
//! hidden from assistive technology: the words beside each say what it is.

use leptos::prelude::*;

/// Download (an arrow into a tray).
pub const DOWNLOAD: &str = "M12 3v11.2l3.6-3.6 1.4 1.4-6 6-6-6 1.4-1.4 3.6 3.6V3h2zM4 19h16v2H4z";
/// A chevron pointing down.
pub const DOWN: &str = "M12 15.6 5.4 9l1.4-1.4 5.2 5.2 5.2-5.2L18.6 9z";
/// A check mark.
pub const CHECK: &str = "M9.2 16.6 4.9 12.3l-1.4 1.4 5.7 5.7L20.5 8.1l-1.4-1.4z";
/// GitHub's mark.
pub const GITHUB: &str = "M12 2a10 10 0 0 0-3.2 19.5c.5.1.7-.2.7-.5v-1.7c-2.8.6-3.4-1.3-3.4-1.3-.4-1.2-1.1-1.5-1.1-1.5-.9-.6.1-.6.1-.6 1 .1 1.5 1 1.5 1 .9 1.6 2.4 1.1 2.9.8.1-.7.4-1.1.6-1.3-2.2-.3-4.6-1.1-4.6-5a3.9 3.9 0 0 1 1-2.7 3.6 3.6 0 0 1 .1-2.7s.8-.3 2.8 1a9.6 9.6 0 0 1 5 0c1.9-1.3 2.8-1 2.8-1a3.6 3.6 0 0 1 .1 2.7 3.9 3.9 0 0 1 1 2.7c0 3.9-2.3 4.7-4.6 5 .4.3.7.9.7 1.9V21c0 .3.2.6.7.5A10 10 0 0 0 12 2z";
/// Discord's mark.
pub const DISCORD: &str = "M19.3 5.3A16.5 16.5 0 0 0 15.2 4l-.5 1a15.3 15.3 0 0 0-5.4 0l-.5-1a16.5 16.5 0 0 0-4.1 1.3C2.1 9.2 1.4 13 1.7 16.7a16.7 16.7 0 0 0 5.1 2.6l1.1-1.8a10.7 10.7 0 0 1-1.7-.8l.4-.3a11.9 11.9 0 0 0 10.8 0l.4.3c-.5.3-1.1.6-1.7.8l1.1 1.8a16.7 16.7 0 0 0 5.1-2.6c.4-4.3-.7-8.1-2.9-11.4zM8.7 14.4c-1 0-1.8-.9-1.8-2s.8-2 1.8-2 1.8.9 1.8 2-.8 2-1.8 2zm6.6 0c-1 0-1.8-.9-1.8-2s.8-2 1.8-2 1.8.9 1.8 2-.8 2-1.8 2z";
/// The mark of `WhatsApp`.
pub const WHATSAPP: &str = "M12 2a10 10 0 0 0-8.6 15l-1.3 4.8 4.9-1.3A10 10 0 1 0 12 2zm0 18.2a8.2 8.2 0 0 1-4.2-1.2l-.3-.2-2.9.8.8-2.8-.2-.3A8.2 8.2 0 1 1 12 20.2zm4.5-6.1c-.2-.1-1.4-.7-1.6-.8-.2-.1-.4-.1-.6.1-.2.2-.6.8-.8.9-.1.2-.3.2-.5.1a6.7 6.7 0 0 1-3.3-2.9c-.2-.3 0-.4.1-.6l.4-.5.2-.4v-.4c-.1-.1-.6-1.4-.8-1.9-.2-.5-.4-.4-.6-.4h-.5c-.2 0-.5.1-.7.3-.2.3-.9.9-.9 2.1s.9 2.5 1.1 2.6c.1.2 1.8 2.8 4.4 3.9 1.6.7 2.2.8 3 .6.5-.1 1.4-.6 1.6-1.2.2-.6.2-1.1.1-1.2-.1-.1-.2-.2-.4-.3z";
/// The sun: light colours.
pub const SUN: &str = "M12 7.5a4.5 4.5 0 1 0 0 9 4.5 4.5 0 0 0 0-9zM11 1h2v4h-2zm0 18h2v4h-2zM1 11h4v2H1zm18 0h4v2h-4zM4.2 5.6l1.4-1.4 2.9 2.9-1.4 1.4zm11.3 1.5 2.9-2.9 1.4 1.4-2.9 2.9zM4.2 18.4l2.9-2.9 1.4 1.4-2.9 2.9zm11.3-1.5 1.4-1.4 2.9 2.9-1.4 1.4z";
/// The moon: dark colours.
pub const MOON: &str = "M12 3a9 9 0 1 0 9 9 7 7 0 0 1-9-9z";
/// A half-filled circle: the device's colours.
pub const HALF: &str = "M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20zm0 2v16a8 8 0 0 1 0-16z";

/// An eye, struck through: what stays hidden.
pub const EYE_OFF: &str = "M12 5c-5 0-9 4.5-10 7 .5 1.3 1.7 3 3.4 4.4L3.3 18.5l1.4 1.4 16-16-1.4-1.4-2.6 2.6A10.6 10.6 0 0 0 12 5zm-3.9 9.3A4 4 0 0 1 13.3 8.1L8.1 13.3zM12 19c5 0 9-4.5 10-7-.4-1-1.2-2.3-2.4-3.5l-3.7 3.7A4 4 0 0 1 12.2 16l-2.7 2.7c.8.2 1.6.3 2.5.3z";
/// An open eye: watching.
pub const EYE: &str = "M12 5C7 5 3 9.5 2 12c1 2.5 5 7 10 7s9-4.5 10-7c-1-2.5-5-7-10-7zm0 11a4 4 0 1 1 4-4 4 4 0 0 1-4 4zm0-6a2 2 0 1 0 2 2 2 2 0 0 0-2-2z";
/// Two people.
pub const PEOPLE: &str = "M8 11a3 3 0 1 0-3-3 3 3 0 0 0 3 3zm8 0a3 3 0 1 0-3-3 3 3 0 0 0 3 3zm-8 2c-2.7 0-6 1.3-6 3.5V19h12v-2.5C14 14.3 10.7 13 8 13zm8 0c-.4 0-.9 0-1.4.1A4.3 4.3 0 0 1 16 16.5V19h6v-2.5c0-2.2-3.3-3.5-6-3.5z";
/// A crown.
pub const CROWN: &str = "M3 8l4.5 3.5L12 4l4.5 7.5L21 8l-2 11H5zm3.5 13h11v-1.2h-11z";
/// A chip.
pub const CHIP: &str = "M9 2v2H7a3 3 0 0 0-3 3v2H2v2h2v2H2v2h2v2a3 3 0 0 0 3 3h2v2h2v-2h2v2h2v-2h2a3 3 0 0 0 3-3v-2h2v-2h-2v-2h2V9h-2V7a3 3 0 0 0-3-3h-2V2h-2v2h-2V2zm0 6h6v8H9z";
/// Four squares.
pub const GRID: &str = "M4 4h7v7H4zm9 0h7v7h-7zM4 13h7v7H4zm9 0h7v7h-7z";
/// A globe.
pub const GLOBE: &str = "M12 2a10 10 0 1 0 10 10A10 10 0 0 0 12 2zm6.9 6h-3a15.7 15.7 0 0 0-1.4-3.6A8 8 0 0 1 18.9 8zM12 4a14 14 0 0 1 1.9 4h-3.8A14 14 0 0 1 12 4zM4.3 14a8.2 8.2 0 0 1 0-4h3.4a16.5 16.5 0 0 0 0 4zm.8 2h3a15.7 15.7 0 0 0 1.4 3.6A8 8 0 0 1 5.1 16zm3-8h-3a8 8 0 0 1 4.4-3.6A15.7 15.7 0 0 0 8.1 8zM12 20a14 14 0 0 1-1.9-4h3.8A14 14 0 0 1 12 20zm2.3-6H9.7a14.7 14.7 0 0 1 0-4h4.6a14.7 14.7 0 0 1 0 4zm.2 5.6a15.7 15.7 0 0 0 1.4-3.6h3a8 8 0 0 1-4.4 3.6zm1.8-5.6a16.5 16.5 0 0 0 0-4h3.4a8.2 8.2 0 0 1 0 4z";
/// A signal: the connection.
pub const WIFI: &str = "M12 6a11 11 0 0 0-8.5 4L2 8.6A13 13 0 0 1 12 4a13 13 0 0 1 10 4.6L20.5 10A11 11 0 0 0 12 6zm0 4a7 7 0 0 0-5.6 2.7L5 11.3A9 9 0 0 1 12 8a9 9 0 0 1 7 3.3l-1.4 1.4A7 7 0 0 0 12 10zm0 4a3 3 0 0 0-2.4 1.2L12 18l2.4-2.8A3 3 0 0 0 12 14z";
/// Sliders: settings.
pub const SLIDERS: &str = "M3 17v2h6v-2zM3 5v2h10V5zm10 16v-2h8v-2h-8v-2h-2v6zM7 9v2H3v2h4v2h2V9zm14 4v-2H11v2zm-6-4h2V7h4V5h-4V3h-2z";
/// A speech bubble.
pub const CHAT: &str = "M4 4h16a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H8l-4 4V6a2 2 0 0 1 2-2zm3 6v2h2v-2zm4 0v2h2v-2zm4 0v2h2v-2z";
/// A cross: close.
pub const CLOSE: &str = "M18.3 5.7 12 12l6.3 6.3-1.4 1.4L10.6 13.4 4.3 19.7l-1.4-1.4L9.2 12 2.9 5.7l1.4-1.4 6.3 6.3 6.3-6.3z";
/// A chevron to the left.
pub const PREV: &str = "M15.4 7.4 14 6l-6 6 6 6 1.4-1.4L10.8 12z";
/// A chevron to the right.
pub const NEXT: &str = "M8.6 16.6 10 18l6-6-6-6-1.4 1.4 4.6 4.6z";
/// Two servers in a rack.
pub const SERVER: &str = "M4 3h16a1 1 0 0 1 1 1v6a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1zm0 10h16a1 1 0 0 1 1 1v6a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1v-6a1 1 0 0 1 1-1zm3-7v2h2V6zm0 10v2h2v-2z";
/// A box with an arrow out of it: another site.
pub const EXTERNAL: &str = "M14 3h7v7h-2V6.4l-9.3 9.3-1.4-1.4L17.6 5H14zM5 5h6v2H5v12h12v-6h2v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7a2 2 0 0 1 2-2z";
/// A shield with a check.
pub const SHIELD: &str =
    "M12 2 4 5v6c0 5 3.4 9.7 8 11 4.6-1.3 8-6 8-11V5zm-1 14-4-4 1.4-1.4 2.6 2.6 5.6-5.6L18 9z";

/// Two sheets: copy.
pub const COPY: &str = "M16 1H4a2 2 0 0 0-2 2v14h2V3h12zm3 4H8a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h11a2 2 0 0 0 2-2V7a2 2 0 0 0-2-2zm0 16H8V7h11z";

/// A glyph, `class="icon"`, hidden from assistive technology.
pub fn icon(path: &'static str) -> impl IntoView + use<> {
    view! {
        <svg class="icon" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
            <path d=path fill="currentColor"></path>
        </svg>
    }
}

/// A glyph with a class of its own besides `icon`.
pub fn icon_as(path: &'static str, class: &'static str) -> impl IntoView + use<> {
    view! {
        <svg class=format!("icon {class}") viewBox="0 0 24 24" aria-hidden="true" focusable="false">
            <path d=path fill="currentColor"></path>
        </svg>
    }
}
