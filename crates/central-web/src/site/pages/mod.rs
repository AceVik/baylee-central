//! Every page: its head (`Meta`) and its body.

use leptos::prelude::*;

use super::shell::{self, Meta};
use super::{Context, Page};

mod account;
mod download;
mod errors;
mod home;
mod info;
mod legal;

/// The document for `page`.
pub fn document(page: Page, cx: &Context<'_>) -> impl IntoView + use<> {
    let (meta, body) = match page {
        Page::Home => home::page(cx),
        Page::Download => download::page(cx),
        Page::Gateways => info::gateways(cx),
        Page::Faq => info::faq(cx),
        Page::Contact => info::contact(cx),
        Page::Impressum => legal::impressum(cx),
        Page::Privacy => legal::privacy(cx),
        Page::Terms => legal::terms(cx),
        Page::FanContent => legal::fan_content(cx),
        Page::SignIn => account::sign_in(cx),
        Page::Register => account::register(cx),
        Page::Account(section) => account::area(cx, section),
        Page::NotFound => errors::not_found(cx),
        Page::Gone => errors::gone(cx),
    };
    shell::document(page, cx, &meta, body)
}

/// A page's head and body.
pub type Built = (Meta, AnyView);

/// A section's heading block: a small eyebrow above the title.
pub fn section_head(eyebrow: &str, title: &str, id: &str) -> impl IntoView + use<> {
    view! {
        <p class="eyebrow">{eyebrow.to_owned()}</p>
        <h2 id=id.to_owned()>{title.to_owned()}</h2>
    }
}
