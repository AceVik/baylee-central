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
/// `arrow-right`.
pub const FORWARD: &[Shape] = &[Path("M5 12h14"), Path("m12 5 7 7-7 7")];
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
const SHIELD_PATH: &str = "M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z";
/// `shield`.
pub const SHIELD: &[Shape] = &[Path(SHIELD_PATH)];
/// `shield-check`.
pub const SHIELD_CHECK: &[Shape] = &[Path(SHIELD_PATH), Path("m9 12 2 2 4-4")];
/// `shield-alert`.
pub const SHIELD_ALERT: &[Shape] = &[Path(SHIELD_PATH), Path("M12 8v4"), Path("M12 16h.01")];
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
    Path(
        "M10.733 5.076a10.744 10.744 0 0 1 11.205 6.575 1 1 0 0 1 0 .696 10.747 10.747 0 0 1-1.444 2.49",
    ),
    Path("M14.084 14.158a3 3 0 0 1-4.242-4.242"),
    Path(
        "M17.479 17.499a10.75 10.75 0 0 1-15.417-5.151 1 1 0 0 1 0-.696 10.75 10.75 0 0 1 4.446-5.143",
    ),
    Path("m2 2 20 20"),
];
/// `key-round`.
pub const KEY: &[Shape] = &[
    Path(
        "M2.586 17.414A2 2 0 0 0 2 18.828V21a1 1 0 0 0 1 1h3a1 1 0 0 0 1-1v-1a1 1 0 0 1 1-1h1a1 1 0 0 0 1-1v-1a1 1 0 0 1 1-1h.172a2 2 0 0 0 1.414-.586l.814-.814a6.5 6.5 0 1 0-4-4z",
    ),
    Circle("16.5", "7.5", ".5"),
];
/// `mail`.
pub const MAIL: &[Shape] = &[
    Path("m22 7-8.991 5.727a2 2 0 0 1-2.009 0L2 7"),
    Rect("20", "16", "2", "4", "2"),
];
/// `copy`.
pub const COPY: &[Shape] = &[
    Rect("14", "14", "8", "8", "2"),
    Path("M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"),
];
/// `circle-check`.
pub const CIRCLE_CHECK: &[Shape] = &[Circle("12", "12", "10"), Path("m9 12 2 2 4-4")];
/// `check`.
pub const CHECK: &[Shape] = &[Path("M20 6 9 17l-5-5")];
/// `calendar`.
pub const CALENDAR: &[Shape] = &[
    Path("M8 2v4"),
    Path("M16 2v4"),
    Rect("18", "18", "3", "4", "2"),
    Path("M3 10h18"),
];
/// `globe`.
pub const GLOBE: &[Shape] = &[
    Circle("12", "12", "10"),
    Path("M12 2a14.5 14.5 0 0 0 0 20 14.5 14.5 0 0 0 0-20"),
    Path("M2 12h20"),
];
/// `server`.
pub const SERVER: &[Shape] = &[
    Rect("20", "8", "2", "2", "2"),
    Rect("20", "14", "2", "14", "2"),
    Line("6", "6.01", "6", "6"),
    Line("6", "6.01", "18", "18"),
];
/// `download`.
pub const DOWNLOAD: &[Shape] = &[
    Path("M12 15V3"),
    Path("M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"),
    Path("m7 10 5 5 5-5"),
];
/// `upload`.
pub const UPLOAD: &[Shape] = &[
    Path("M12 3v12"),
    Path("m17 8-5-5-5 5"),
    Path("M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"),
];
/// `trash-2`.
pub const TRASH_2: &[Shape] = &[
    Path("M10 11v6"),
    Path("M14 11v6"),
    Path("M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6"),
    Path("M3 6h18"),
    Path("M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"),
];
/// `log-out`.
pub const LOG_OUT: &[Shape] = &[
    Path("m16 17 5-5-5-5"),
    Path("M21 12H9"),
    Path("M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"),
];
/// `external-link`.
pub const EXTERNAL_LINK: &[Shape] = &[
    Path("M15 3h6v6"),
    Path("M10 14 21 3"),
    Path("M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"),
];
/// `bell`.
pub const BELL: &[Shape] = &[
    Path("M10.268 21a2 2 0 0 0 3.464 0"),
    Path(
        "M3.262 15.326A1 1 0 0 0 4 17h16a1 1 0 0 0 .74-1.673C19.41 13.956 18 12.499 18 8A6 6 0 0 0 6 8c0 4.499-1.411 5.956-2.738 7.326",
    ),
];
/// `rotate-ccw`.
pub const ROTATE_CCW: &[Shape] = &[
    Path("M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"),
    Path("M3 3v5h5"),
];
/// `rotate-cw`.
pub const ROTATE_CW: &[Shape] = &[
    Path("M21 12a9 9 0 1 1-9-9c2.52 0 4.93 1 6.74 2.74L21 8"),
    Path("M21 3v5h-5"),
];
/// `zoom-in`.
pub const ZOOM_IN: &[Shape] = &[
    Circle("11", "11", "8"),
    Path("m21 21-4.3-4.3"),
    Line("11", "11", "8", "14"),
    Line("8", "14", "11", "11"),
];
/// `zoom-out`.
pub const ZOOM_OUT: &[Shape] = &[
    Circle("11", "11", "8"),
    Path("m21 21-4.3-4.3"),
    Line("8", "14", "11", "11"),
];
/// `chevron-left`.
pub const CHEVRON_LEFT: &[Shape] = &[Path("m15 18-6-6 6-6")];
/// `chevron-right`.
pub const CHEVRON_RIGHT: &[Shape] = &[Path("m9 18 6-6-6-6")];
/// `arrow-up-down`.
pub const ARROW_UP_DOWN: &[Shape] = &[
    Path("m21 16-4 4-4-4"),
    Path("M17 20V4"),
    Path("m3 8 4-4 4 4"),
    Path("M7 4v16"),
];
/// `pencil`.
pub const PENCIL: &[Shape] = &[
    Path(
        "M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497z",
    ),
    Path("m15 5 4 4"),
];
/// `refresh-cw`.
pub const REFRESH_CW: &[Shape] = &[
    Path("M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8"),
    Path("M21 3v5h-5"),
    Path("M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16"),
    Path("M8 16H3v5"),
];
/// `camera`.
pub const CAMERA: &[Shape] = &[
    Path(
        "M14.5 4h-5L7 7H4a2 2 0 0 0-2 2v9a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V9a2 2 0 0 0-2-2h-3l-2.5-3z",
    ),
    Circle("12", "13", "3"),
];
/// `file-text`.
pub const FILE_TEXT: &[Shape] = &[
    Path("M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z"),
    Path("M14 2v4a2 2 0 0 0 2 2h4"),
    Path("M10 9H8"),
    Path("M16 13H8"),
    Path("M16 17H8"),
];
/// `layers`.
pub const LAYERS: &[Shape] = &[
    Path(
        "M12.83 2.18a2 2 0 0 0-1.66 0L2.6 6.08a1 1 0 0 0 0 1.83l8.58 3.91a2 2 0 0 0 1.66 0l8.58-3.9a1 1 0 0 0 0-1.83z",
    ),
    Path("M2 12a1 1 0 0 0 .58.91l8.6 3.91a2 2 0 0 0 1.65 0l8.58-3.9A1 1 0 0 0 22 12"),
    Path("M2 17a1 1 0 0 0 .58.91l8.6 3.91a2 2 0 0 0 1.65 0l8.58-3.9A1 1 0 0 0 22 17"),
];
/// `at-sign`.
pub const AT_SIGN: &[Shape] = &[
    Circle("12", "12", "4"),
    Path("M16 8v5a3 3 0 0 0 6 0v-1a10 10 0 1 0-4 8"),
];
/// `hash`.
pub const HASH: &[Shape] = &[
    Line("4", "20", "9", "9"),
    Line("4", "20", "15", "15"),
    Line("10", "8", "3", "21"),
    Line("16", "14", "3", "21"),
];
/// `crown`.
pub const CROWN: &[Shape] = &[
    Path(
        "M11.562 3.266a.5.5 0 0 1 .876 0L15.39 8.87a1 1 0 0 0 1.516.294L21.183 5.5a.5.5 0 0 1 .798.519l-2.834 10.246a1 1 0 0 1-.956.734H5.81a1 1 0 0 1-.957-.734L2.02 6.02a.5.5 0 0 1 .798-.519l4.276 3.664a1 1 0 0 0 1.516-.294z",
    ),
    Path("M5 21h14"),
];
/// `ellipsis-vertical`.
pub const ELLIPSIS_VERTICAL: &[Shape] = &[
    Circle("12", "12", "1"),
    Circle("12", "5", "1"),
    Circle("12", "19", "1"),
];
/// `user-check`.
pub const USER_CHECK: &[Shape] = &[
    Path("m16 11 2 2 4-4"),
    Path("M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2"),
    Circle("9", "7", "4"),
];
/// `user-x`.
pub const USER_X: &[Shape] = &[
    Path("M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2"),
    Circle("9", "7", "4"),
    Line("17", "22", "8", "13"),
    Line("22", "17", "8", "13"),
];
/// `ban`.
pub const BAN: &[Shape] = &[Circle("12", "12", "10"), Path("M4.929 4.929 19.07 19.071")];
/// `qr-code`.
pub const QR_CODE: &[Shape] = &[
    Rect("5", "5", "3", "3", "1"),
    Rect("5", "5", "16", "3", "1"),
    Rect("5", "5", "3", "16", "1"),
    Path("M21 16h-3a2 2 0 0 0-2 2v3"),
    Path("M21 21v.01"),
    Path("M12 7v3a2 2 0 0 1-2 2H7"),
    Path("M3 12h.01"),
    Path("M12 3h.01"),
    Path("M12 16v.01"),
    Path("M16 12h1"),
    Path("M21 12v.01"),
    Path("M12 21v-1"),
];
/// `link`.
pub const LINK: &[Shape] = &[
    Path("M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"),
    Path("M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"),
];
/// `clock`.
pub const CLOCK: &[Shape] = &[Circle("12", "12", "10"), Path("M12 6v6l4 2")];
/// `share-2`.
pub const SHARE_2: &[Shape] = &[
    Circle("18", "5", "3"),
    Circle("6", "12", "3"),
    Circle("18", "19", "3"),
    Line("8.59", "15.42", "13.51", "17.49"),
    Line("15.41", "8.59", "6.51", "10.49"),
];
/// `sparkles`.
pub const SPARKLES: &[Shape] = &[
    Path(
        "M11.017 2.814a1 1 0 0 1 1.966 0l1.051 5.558a2 2 0 0 0 1.594 1.594l5.558 1.051a1 1 0 0 1 0 1.966l-5.558 1.051a2 2 0 0 0-1.594 1.594l-1.051 5.558a1 1 0 0 1-1.966 0l-1.051-5.558a2 2 0 0 0-1.594-1.594l-5.558-1.051a1 1 0 0 1 0-1.966l5.558-1.051a2 2 0 0 0 1.594-1.594z",
    ),
    Path("M20 2v4"),
    Path("M22 4h-4"),
    Circle("4", "20", "2"),
];
/// `bot`: a house AI's mark, and the mark of a kind this build does not know.
pub const BOT: &[Shape] = &[
    Path("M12 8V4H8"),
    Rect("16", "12", "4", "8", "2"),
    Path("M2 14h2"),
    Path("M20 14h2"),
    Path("M15 13v2"),
    Path("M9 13v2"),
];
/// `cpu`: the trained AI's mark.
pub const CPU: &[Shape] = &[
    Path("M12 20v2"),
    Path("M12 2v2"),
    Path("M17 20v2"),
    Path("M17 2v2"),
    Path("M2 12h2"),
    Path("M2 17h2"),
    Path("M2 7h2"),
    Path("M20 12h2"),
    Path("M20 17h2"),
    Path("M20 7h2"),
    Path("M7 20v2"),
    Path("M7 2v2"),
    Rect("16", "16", "4", "4", "2"),
    Rect("8", "8", "8", "8", "1"),
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
