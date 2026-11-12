//! The moderation of bug and crash reports (`docs/reports.md` §"Triage"),
//! for an administrator, inside the account area's island: the list at
//! `/{lang}/account/admin/feedback` (views, search, filters with their
//! counts, saved lists, the chart, crash groups, a selection's actions, the
//! CSV) and one report at `/{lang}/account/admin/feedback/<id in hex>`
//! (what was written and sent, the triage, notes, history, the trash).
//! It rebuilds the old build's `web/src/admin/feedback/` on the wire it
//! used, `AdminRequest`'s report actions, unchanged.
//!
//! A report is a game client's words and data: untrusted. Every field is
//! drawn as text, never as markup; nothing of it goes into an address but
//! a report's id (hex of its 16 bytes) and the list's filter as the old
//! administration wrote it; nothing of it is logged or written to the
//! console. A change that cannot be undone (deleting for good) asks first,
//! asks for the password where Central wants a fresh sign-in, and is sent
//! once. An answer that did not arrive is never a refusal.

mod api;
mod chart;
mod client;
mod csv;
mod github;
mod list;
mod query;
mod report;
mod shared;
mod text;
mod tree;

use leptos::prelude::*;

use crate::data::Section;
use crate::words::Lang;

/// The list or one report, for the administrator `me` (the area checked
/// the role); `username` is their sign-in name, for a password manager;
/// `signed_out` runs when this browser turns out not to be signed in.
pub fn page(
    lang: Lang,
    section: Section,
    worker: String,
    username: String,
    me: [u8; 16],
    signed_out: Callback<()>,
) -> AnyView {
    let c = shared::Ctx::new(lang, me, worker, username, signed_out);
    if section == Section::Report {
        report::page(c)
    } else {
        list::page(c)
    }
}
