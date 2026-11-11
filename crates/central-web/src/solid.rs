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
