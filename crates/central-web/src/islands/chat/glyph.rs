//! The messenger's icons: Lucide (lucide-static 1.60.0, ISC licence, see
//! NOTICE.md), drawn element by element. The pages' icons are inserted as
//! markup, which an island cannot do in the browser: Trusted Types allow no
//! `innerHTML` there, and none is needed.

use leptos::prelude::*;

/// One element of a 24 × 24 icon.
pub enum Shape {
    /// `<path d>`.
    Path(&'static str),
    /// `<circle cx cy r>`.
    Circle(&'static str, &'static str, &'static str),
    /// `<line x1 x2 y1 y2>`.
    Line(&'static str, &'static str, &'static str, &'static str),
    /// `<rect width height x y rx>`.
    Rect(
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        &'static str,
    ),
}

use Shape::{Circle, Line, Path, Rect};

/// `send`.
pub const SEND: &[Shape] = &[
    Path(
        "M14.536 21.686a.5.5 0 0 0 .937-.024l6.5-19a.496.496 0 0 0-.635-.635l-19 6.5a.5.5 0 0 0-.024.937l7.93 3.18a2 2 0 0 1 1.112 1.11z",
    ),
    Path("m21.854 2.147-10.94 10.939"),
];
/// `arrow-left`.
pub const BACK: &[Shape] = &[Path("m12 19-7-7 7-7"), Path("M19 12H5")];
/// `users`.
pub const USERS: &[Shape] = &[
    Path("M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2"),
    Path("M16 3.128a4 4 0 0 1 0 7.744"),
    Path("M22 21v-2a4 4 0 0 0-3-3.87"),
    Circle("9", "7", "4"),
];
/// `user-plus`.
pub const USER_PLUS: &[Shape] = &[
    Path("M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2"),
    Circle("9", "7", "4"),
    Line("19", "19", "8", "14"),
    Line("22", "16", "11", "11"),
];
/// `search`.
pub const SEARCH: &[Shape] = &[Path("m21 21-4.34-4.34"), Circle("11", "11", "8")];
/// `x`.
pub const CLOSE: &[Shape] = &[Path("M18 6 6 18"), Path("m6 6 12 12")];
/// `info`.
pub const INFO: &[Shape] = &[
    Circle("12", "12", "10"),
    Path("M12 16v-4"),
    Path("M12 8h.01"),
];
const SHIELD: &str = "M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z";
/// `shield-check`.
pub const SHIELD_CHECK: &[Shape] = &[Path(SHIELD), Path("m9 12 2 2 4-4")];
/// `shield-alert`.
pub const SHIELD_ALERT: &[Shape] = &[Path(SHIELD), Path("M12 8v4"), Path("M12 16h.01")];
/// `lock`.
pub const LOCK: &[Shape] = &[
    Rect("18", "11", "3", "11", "2"),
    Path("M7 11V7a5 5 0 0 1 10 0v4"),
];
/// `message-circle`.
pub const MESSAGE: &[Shape] = &[Path(
    "M2.992 16.342a2 2 0 0 1 .094 1.167l-1.065 3.29a1 1 0 0 0 1.236 1.168l3.413-.998a2 2 0 0 1 1.099.092 10 10 0 1 0-4.777-4.719",
)];
/// `plus`.
pub const PLUS: &[Shape] = &[Path("M5 12h14"), Path("M12 5v14")];
/// `user`.
pub const USER: &[Shape] = &[
    Path("M19 21v-2a4 4 0 0 0-4-4H9a4 4 0 0 0-4 4v2"),
    Circle("12", "7", "4"),
];
/// `id-card`.
pub const ID_CARD: &[Shape] = &[
    Path("M16 10h2"),
    Path("M16 14h2"),
    Path("M6.17 15a3 3 0 0 1 5.66 0"),
    Circle("9", "11", "2"),
    Rect("20", "14", "2", "5", "2"),
];
/// `lock-keyhole`.
pub const LOCK_KEYHOLE: &[Shape] = &[
    Circle("12", "16", "1"),
    Rect("18", "12", "3", "10", "2"),
    Path("M7 10V7a5 5 0 0 1 10 0v3"),
];
/// `eye`.
pub const EYE: &[Shape] = &[
    Path(
        "M2.062 12.348a1 1 0 0 1 0-.696 10.75 10.75 0 0 1 19.876 0 1 1 0 0 1 0 .696 10.75 10.75 0 0 1-19.876 0",
    ),
    Circle("12", "12", "3"),
];
/// `eye-off`.
pub const EYE_OFF: &[Shape] = &[
    Path("M10.733 5.076a10.744 10.744 0 0 1 11.205 6.575 1 1 0 0 1 0 .696 10.747 10.747 0 0 1-1.444 2.49"),
    Path("M14.084 14.158a3 3 0 0 1-4.242-4.242"),
    Path("M17.479 17.499a10.75 10.75 0 0 1-15.417-5.151 1 1 0 0 1 0-.696 10.75 10.75 0 0 1 4.446-5.143"),
    Path("m2 2 20 20"),
];

/// The icon, hidden from assistive technology: its control names itself.
pub fn glyph(shapes: &'static [Shape]) -> AnyView {
    let parts = shapes
        .iter()
        .map(|shape| match *shape {
            Path(d) => view! { <path d=d /> }.into_any(),
            Circle(cx, cy, r) => view! { <circle cx=cx cy=cy r=r /> }.into_any(),
            Line(x1, x2, y1, y2) => view! { <line x1=x1 x2=x2 y1=y1 y2=y2 /> }.into_any(),
            Rect(width, height, x, y, rx) => {
                view! { <rect width=width height=height x=x y=y rx=rx ry=rx /> }.into_any()
            }
        })
        .collect_view();
    view! {
        <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
            stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false">
            {parts}
        </svg>
    }
    .into_any()
}
