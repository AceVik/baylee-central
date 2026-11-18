//! The donation page: the monthly goal and how much of it is covered, what
//! the money pays for, and the three places to give, each as a link and as
//! a QR code a phone reads off the screen.
//!
//! What binds every word here (Baylee `docs/legal.md`, "Donations"): a
//! donation buys nothing, no donor is named, Wizards is not put beside a
//! request for money, and the page says that donations are not
//! tax-deductible. The codes are files made by Baylee's
//! `cargo tools site qr`, one per address in [`WAYS`]; a test reads every
//! file back and compares it with its address.

use leptos::prelude::*;

use super::Built;
use crate::data::{Funding, GIVING};
use crate::rich::icon;
use crate::site::shell::Meta;
use crate::site::{Context, Page, icons};
use crate::solid;
use crate::words::{self, Lang};

/// A place to give: its name and address (`data::GIVING`), its code's
/// file, the line on what kind of giving it is, and its sign.
struct Way {
    name: &'static str,
    address: &'static str,
    /// The code's logical asset name.
    code: &'static str,
    lead: Words,
    sign: Sign,
}

#[derive(Clone, Copy)]
enum Sign {
    Solid(&'static str),
    Line(&'static str),
}

/// The three places, in the order they are shown.
const WAYS: [Way; 3] = [
    Way {
        name: GIVING[0].0,
        address: GIVING[0].1,
        code: "img/qr-github.svg",
        lead: words::donate_github_lead,
        sign: Sign::Solid(solid::GITHUB),
    },
    Way {
        name: GIVING[1].0,
        address: GIVING[1].1,
        code: "img/qr-liberapay.svg",
        lead: words::donate_liberapay_lead,
        sign: Sign::Line(icons::REPEAT),
    },
    Way {
        name: GIVING[2].0,
        address: GIVING[2].1,
        code: "img/qr-ko-fi.svg",
        lead: words::donate_kofi_lead,
        sign: Sign::Line(icons::COFFEE),
    },
];

/// A sentence of the page in a language.
type Words = fn(Lang) -> &'static str;

/// What a month costs, rounded, in euros: its name, a line on it and the
/// amount, from the owner's breakdown of the goal (18.11.2026). The bars
/// are drawn against the goal.
const COSTS: [(Words, Words, u64); 5] = [
    (words::donate_cost_ai, words::donate_cost_ai_note, 430),
    (
        words::donate_cost_servers,
        words::donate_cost_servers_note,
        100,
    ),
    (words::donate_cost_fees, words::donate_cost_fees_note, 80),
    (words::donate_cost_art, words::donate_cost_art_note, 50),
    (words::donate_cost_tools, words::donate_cost_tools_note, 50),
];

/// An amount of whole euros as `lang` writes it.
fn euros(lang: Lang, euros: u64) -> String {
    let n = words::number(lang, euros);
    match lang {
        Lang::En | Lang::Ja | Lang::Ko | Lang::ZhHans | Lang::ZhHant => format!("€{n}"),
        Lang::PtBr => format!("€\u{a0}{n}"),
        _ => format!("{n}\u{a0}€"),
    }
}

/// The goal and how much of it is covered: the bar, the two amounts and
/// which platforms the figure counts. Only what a platform said is drawn;
/// without a figure the bar is left out and a line says so.
fn progress(lang: Lang, funding: Option<&Funding>) -> impl IntoView + use<> {
    let goal = euros(lang, Funding::GOAL / 100);
    let Some((month, patrons)) = funding.and_then(Funding::covered) else {
        return view! { <p class="give-counted small">{words::donate_progress_none(lang)}</p> }
            .into_any();
    };
    let percent = Funding::percent(month);
    let have = euros(lang, (month + 50) / 100);
    let label = words::donate_progress_label(lang);
    // nobody yet: an invitation says more than a count of none
    let counted = if month == 0 && patrons == 0 {
        words::donate_progress_first(lang).to_owned()
    } else {
        words::donate_progress_counted(lang, &words::number(lang, patrons))
    };
    view! {
        <div class="give-progress">
            <p class="give-figures">
                <strong class="num">{words::donate_progress(lang, &have, &goal)}</strong>
                <span class="give-percent num">{words::donate_progress_percent(lang, &percent.to_string())}</span>
            </p>
            <progress class="give-meter" max="100" value=percent.to_string() aria-label=label></progress>
            <p class="give-counted small">{counted}</p>
        </div>
    }
    .into_any()
}

fn costs(lang: Lang) -> impl IntoView + use<> {
    let goal = Funding::GOAL / 100;
    let rows = COSTS
        .iter()
        .map(|(name, note, amount)| {
            view! {
                <li class="cost">
                    <p class="cost-head">
                        <span class="cost-name">{name(lang)}</span>
                        <span class="cost-amount num">{words::donate_cost_about(lang, &euros(lang, *amount))}</span>
                    </p>
                    <progress class="cost-bar" max=goal.to_string() value=amount.to_string() aria-hidden="true"></progress>
                    <p class="cost-note small">{note(lang)}</p>
                </li>
            }
        })
        .collect_view();
    view! { <ul class="costs">{rows}</ul> }
}

fn way(cx: &Context<'_>, way: &Way) -> impl IntoView + use<> {
    let lang = cx.lang;
    let shown = way.address.trim_start_matches("https://");
    let sign = match way.sign {
        Sign::Solid(path) => solid::icon(path).into_any(),
        Sign::Line(paths) => icon(paths).into_any(),
    };
    view! {
        <li class="way card glass">
            <h3 class="way-name"><span class="way-sign" aria-hidden="true">{sign}</span>{way.name}</h3>
            <p class="way-lead">{(way.lead)(lang)}</p>
            <a class="way-code" href=way.address target="_blank" rel="noopener noreferrer" tabindex="-1" aria-hidden="true">
                <img src=cx.assets.url(way.code) width="220" height="220" alt="" loading="lazy" decoding="async" />
            </a>
            <p class="way-address num" translate="no">{shown}</p>
            <a class="button primary give" href=way.address target="_blank" rel="noopener noreferrer">
                {icon(icons::HEART)}
                <span>{words::donate_open(lang, way.name)}</span>
                {icon(icons::EXTERNAL_LINK)}
            </a>
        </li>
    }
}

pub fn page(cx: &Context<'_>) -> Built {
    let lang = cx.lang;
    let mut meta = Meta::new(
        words::donate_title(lang),
        words::donate_description(lang),
        "img/og.jpg",
    );
    meta.trail
        .push((words::nav_donate(lang).to_owned(), Page::Donate));
    let goal = euros(lang, Funding::GOAL / 100);
    let ways = WAYS.iter().map(|w| way(cx, w)).collect_view();
    let body = view! {
        <section class="page-head wrap" aria-labelledby="donate-title">
            <p class="eyebrow">{words::donate_eyebrow(lang)}</p>
            <h1 id="donate-title">{words::donate_title(lang)}</h1>
            <p class="lead">{words::donate_lead(lang)}</p>
        </section>
        <section class="wrap give-goal" aria-labelledby="goal-title">
            <div class="card glass goal-card">
                <p class="eyebrow" id="goal-title">{words::donate_goal_title(lang)}</p>
                <p class="goal-amount">{words::donate_goal_amount(lang, &goal)}</p>
                <p class="goal-lead">{words::donate_goal_lead(lang)}</p>
                {progress(lang, cx.data.funding.as_ref())}
                <a class="button primary give goal-go" href="#ways">
                    {icon(icons::HEART)}
                    <span>{words::donate_band_button(lang)}</span>
                </a>
            </div>
            <div class="card glass costs-card">
                <h2>{words::donate_costs_title(lang)}</h2>
                <p class="small">{words::donate_costs_lead(lang)}</p>
                {costs(lang)}
            </div>
        </section>
        <section class="wrap give-ways" id="ways" aria-labelledby="ways-title">
            <h2 id="ways-title">{words::donate_ways_title(lang)}</h2>
            <p class="lead">{words::donate_ways_lead(lang)}</p>
            <ul class="ways">{ways}</ul>
            <p class="small give-privacy">
                {words::donate_privacy(lang)}{" "}
                <a href=format!("{}#donate", Page::Privacy.path(lang))>{words::privacy_title(lang)}</a>
            </p>
        </section>
        <section class="wrap give-notes">
            <div class="card glass">
                <h2>{words::donate_nothing_title(lang)}</h2>
                <p>{words::donate_nothing(lang)}</p>
            </div>
            <div class="card glass">
                <h2>{words::donate_other_title(lang)}</h2>
                <p>{words::donate_other(lang)}</p>
            </div>
        </section>
    }
    .into_any();
    (meta, body)
}
