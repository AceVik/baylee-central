//! The gateway directory, the questions and the contact page.

use leptos::prelude::*;
use serde_json::json;

use super::Built;
use super::home::gateway_card;
use crate::islands::ContactForm;
use crate::rich::{icon, plain, rich};
use crate::site::icons;
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
    let list = match cx.data.gateways.as_deref() {
        None => view! { <p class="notice">{words::gws_failed(lang)}</p> }.into_any(),
        Some([]) => view! { <p class="notice">{words::gws_none(lang)}</p> }.into_any(),
        Some(all) => {
            let count = u64::try_from(all.len()).unwrap_or(u64::MAX);
            let cards = all
                .iter()
                .map(|gateway| gateway_card(lang, gateway))
                .collect_view();
            view! {
                <h2 class="count">{words::gws_count(lang, count)}</h2>
                <ul class="gateways grid-cards">{cards}</ul>
                <p class="hint">{words::gws_copy_hint(lang)}</p>
            }
            .into_any()
        }
    };
    let code = lang.code();
    let steps = rich(words::host_steps(lang), move |tag| match tag {
        "guide" => Some(format!("{REPOSITORY}/blob/main/docs/gateway.md")),
        "account" => Some(format!("/{code}/account/gateways")),
        _ => None,
    });
    let body = view! {
        {head(None, words::gws_title(lang), words::gws_lead(lang))}
        <section class="wrap" aria-label=words::gws_title(lang)>{list}</section>
        <section class="wrap prose-block" aria-labelledby="host">
            <h2 id="host">{words::host_title(lang)}</h2>
            <p>{words::host_lead(lang)}</p>
            <p>{steps}</p>
        </section>
    }
    .into_any();
    (meta, body)
}

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
    let privacy = Page::Privacy.path(lang);
    let note = rich(words::contact_privacy(lang), move |tag| {
        (tag == "privacy").then(|| privacy.clone())
    });
    let body = view! {
        {head(Some(words::contact_eyebrow(lang)), words::contact_title(lang), words::contact_lead(lang))}
        <div class="wrap narrow">
            <ContactForm lang=lang />
            <noscript><p class="notice">{words::contact_noscript(lang)}</p></noscript>
            <p class="hint">{note}</p>
            <section class="card" aria-labelledby="bugs">
                <h2 id="bugs">{icon(icons::FLAG)}{words::contact_bugs_title(lang)}</h2>
                <p>{plain(words::contact_bugs(lang))}</p>
                <p class="actions">
                    <a class="button quiet" href=format!("{REPOSITORY}/issues") rel="noopener noreferrer">{icon(icons::EXTERNAL_LINK)}{words::foot_issues(lang)}</a>
                    <a class="button quiet" href=DISCORD rel="noopener noreferrer">{icon(icons::MESSAGE_CIRCLE)}{words::foot_discord(lang)}</a>
                </p>
            </section>
        </div>
    }
    .into_any();
    (meta, body)
}
