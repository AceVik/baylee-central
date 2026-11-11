//! The gateway directory, the questions and the contact page.

use leptos::prelude::*;
use serde_json::json;

use super::Built;
use super::home::{contact_columns, gateway_card};
use crate::data::Gateway;
use crate::rich::{plain, rich};
use crate::site::shell::{DISCORD, Meta, REPOSITORY};
use crate::site::{Context, Page};
use crate::words::{self, Lang, Piece};

fn head<L: IntoView + 'static>(
    eyebrow: Option<&'static str>,
    title: &'static str,
    lead: L,
) -> impl IntoView + use<L> {
    view! {
        <section class="page-head wrap">
            {eyebrow.map(|eyebrow| view! { <p class="eyebrow">{eyebrow}</p> })}
            <h1>{title}</h1>
            <p class="lead">{lead}</p>
        </section>
    }
}

pub fn gateways(cx: &Context<'_>) -> Built {
    let lang = cx.lang;
    let mut meta = Meta::new(
        words::title_gateways(lang),
        words::desc_gateways(lang),
        "img/og.jpg",
    );
    meta.trail
        .push((words::gws_title(lang).to_owned(), Page::Gateways));
    let body = view! {
        <section class="page-head wrap" aria-labelledby="gateways-title">
            <p class="eyebrow">{words::gw_eyebrow(lang)}</p>
            <h1 id="gateways-title">{words::gws_title(lang)}</h1>
            <p class="lead">{words::gw_lead(lang)}</p>
            <p class="small page-note">{words::gw_how_to(lang)}</p>
            {directory(lang, cx.data.gateways.as_deref())}
        </section>
        {self_host(lang)}
    }
    .into_any();
    (meta, body)
}

/// Every gateway Central lists, the official one first; finding one by
/// name or address, or only those online, is `directory.js`'s (without
/// it every card shows).
fn directory(lang: Lang, all: Option<&[Gateway]>) -> impl IntoView + use<> {
    let Some(all) = all else {
        return view! { <p class="notice dir-failure glass">{words::gws_failed(lang)}</p> }
            .into_any();
    };
    if all.is_empty() {
        return view! { <p class="small dir-count">{words::gw_none(lang)}</p> }.into_any();
    }
    let online = all.iter().filter(|gateway| gateway.online).count();
    let count = words::gw_count(
        lang,
        u64::try_from(all.len()).unwrap_or(u64::MAX),
        &words::number(lang, u64::try_from(online).unwrap_or(u64::MAX)),
    );
    let find = (all.len() > 1).then(|| {
        view! {
            <div class="dir-find" hidden=true data-find-bar="">
                <label class="visually-hidden" for="dir-find">{words::gw_find(lang)}</label>
                <input id="dir-find" type="search" placeholder=words::gw_find_hint(lang) maxlength="80"
                    autocomplete="off" spellcheck="false" aria-keyshortcuts="Control+K Meta+K" />
                <button type="button" class="chip-btn" aria-pressed="false" data-online-only="">
                    <span class="dot ok" aria-hidden="true"></span>{words::gw_online_only(lang)}
                </button>
            </div>
        }
    });
    let cards = all
        .iter()
        .map(|gateway| gateway_card(lang, gateway, true))
        .collect_view();
    view! {
        <div class="directory">
            {find}
            <output class="small dir-count" data-dir-count="" data-shown=words::gw_shown(lang, "{n}", "{all}")>{count}</output>
            <p class="dir-none" hidden=true data-no-match="">
                {words::gw_no_match(lang)}" "
                <button type="button" class="button button-ghost button-sm">{words::gw_show_all(lang)}</button>
            </p>
            <ul class="gateways grid-cards stagger" data-directory="">{cards}</ul>
        </div>
    }
    .into_any()
}

/// Running a gateway of one's own, as the old site told it.
fn self_host(lang: Lang) -> impl IntoView + use<> {
    let code = lang.code();
    let central = rich(words::selfhost_central(lang), move |tag| {
        (tag == "mine").then(|| format!("/{code}/account/gateways"))
    });
    let guide = rich(words::selfhost_guide(lang), move |tag| match tag {
        "guide" => Some(format!("{REPOSITORY}{}", words::link_readme_gateway(lang))),
        "protocol" => Some(format!("{REPOSITORY}/blob/main/docs/protocol.md")),
        _ => None,
    });
    view! {
        <section class="band alt" id="host" aria-labelledby="host-title">
            <div class="wrap">
                <p class="eyebrow">{words::selfhost_eyebrow(lang)}</p>
                <h2 id="host-title">{words::selfhost_title(lang)}</h2>
                <div class="two-col">
                    <div class="reveal">
                        <p>{plain(words::selfhost_processes(lang))}</p>
                        <p>{plain(words::selfhost_proxy(lang))}</p>
                        <p>{central}</p>
                        <p>{guide}</p>
                    </div>
                    <div class="reveal">
                        <pre class="code" tabindex="0"><code translate="no">{SHELL}</code></pre>
                        <p class="small">{plain(words::selfhost_source(lang))}</p>
                    </div>
                </div>
            </div>
        </section>
    }
}

/// The shell lines that start a gateway, as the old site showed them.
const SHELL: &str = "docker compose up -d
export DATABASE_URL=postgres://baylee:baylee@127.0.0.1:5432/baylee
cargo build --release -p baylee-gateway \\
  -p baylee-agent -p baylee-engine-server

export BAYLEE_AGENT_TOKEN=$(openssl rand -hex 32)
RUST_LOG=info BAYLEE_GATEWAY_NAME=\"My table\" ./target/release/baylee-gateway
RUST_LOG=info ./target/release/baylee-agent";

fn text(message: &'static str) -> Vec<Piece> {
    vec![Piece {
        tag: None,
        text: message.to_owned(),
    }]
}

/// The questions, each with its answer and the answer's links.
fn questions(lang: Lang) -> Vec<(&'static str, Vec<Piece>)> {
    vec![
        (words::faq_what_q(lang), text(words::faq_what_a(lang))),
        (words::faq_cost_q(lang), text(words::faq_cost_a(lang))),
        (words::faq_account_q(lang), text(words::faq_account_a(lang))),
        (words::faq_data_q(lang), words::faq_data_a(lang)),
        (
            words::faq_password_q(lang),
            text(words::faq_password_a(lang)),
        ),
        (words::faq_gateway_q(lang), words::faq_gateway_a(lang)),
        (words::faq_systems_q(lang), text(words::faq_systems_a(lang))),
        (words::faq_open_q(lang), text(words::faq_open_a(lang))),
        (words::faq_bug_q(lang), words::faq_bug_a(lang)),
    ]
}

pub fn faq(cx: &Context<'_>) -> Built {
    let lang = cx.lang;
    let mut meta = Meta::new(words::title_faq(lang), words::desc_faq(lang), "img/og.jpg");
    meta.trail
        .push((words::faq_title(lang).to_owned(), Page::Faq));
    let link = move |tag: &str| match tag {
        "privacy" => Some(Page::Privacy.path(lang)),
        "gateways" => Some(Page::Gateways.path(lang)),
        "contact" => Some(Page::Contact.path(lang)),
        _ => None,
    };
    let all = questions(lang);
    let entities: Vec<serde_json::Value> = all
        .iter()
        .map(|(question, answer)| {
            let text: String = answer.iter().map(|piece| piece.text.as_str()).collect();
            json!({"@type": "Question", "name": question, "acceptedAnswer": {"@type": "Answer", "text": text}})
        })
        .collect();
    meta.data.push(
        json!({"@context": "https://schema.org", "@type": "FAQPage", "mainEntity": entities}),
    );
    let items = all
        .into_iter()
        .enumerate()
        .map(|(at, (question, answer))| {
            let id = format!("q{}", at + 1);
            let label = id.clone();
            view! {
                <section class="qa" aria-labelledby=label>
                    <h2 id=id>{question}</h2>
                    <p>{rich(answer, link)}</p>
                </section>
            }
        })
        .collect_view();
    let body = view! {
        {head(Some(words::faq_eyebrow(lang)), words::faq_title(lang), "")}
        <div class="wrap narrow faq">{items}</div>
    }
    .into_any();
    (meta, body)
}

pub fn contact(cx: &Context<'_>) -> Built {
    let lang = cx.lang;
    let mut meta = Meta::new(
        words::title_contact(lang),
        words::desc_contact(lang),
        "img/og.jpg",
    );
    meta.trail
        .push((words::contact_title(lang).to_owned(), Page::Contact));
    meta.islands = true;
    let links = move |tag: &str| match tag {
        "discord" => Some(DISCORD.to_owned()),
        "issues" => Some(format!("{REPOSITORY}/issues")),
        "docs" => Some(format!("{REPOSITORY}/blob/main/docs/privacy.md")),
        "notice" => Some(Page::Privacy.path(lang)),
        _ => None,
    };
    let body = view! {
        <section class="page-head wrap" aria-labelledby="contact-title">
            <p class="eyebrow">{words::contact_eyebrow(lang)}</p>
            <h1 id="contact-title">{words::contact_title(lang)}</h1>
            {contact_columns(lang, "#reports".to_owned())}
        </section>
        <section class="band alt" id="reports" aria-labelledby="reports-title">
            <div class="wrap">
                <p class="eyebrow">{words::reports_eyebrow(lang)}</p>
                <h2 id="reports-title">{words::reports_title(lang)}</h2>
                <div class="two-col">
                    <div class="reveal">
                        <p>{plain(words::reports_f8(lang))}</p>
                        <p>{plain(words::reports_usage(lang))}</p>
                    </div>
                    <div class="reveal">
                        <p>{rich(words::reports_discord(lang), links)}</p>
                        <p>{rich(words::reports_issues(lang), links)}</p>
                        <p>{rich(words::reports_privacy(lang), links)}</p>
                    </div>
                </div>
            </div>
        </section>
    }
    .into_any();
    (meta, body)
}
