//! The security page: how a password, the keys and a chat message travel,
//! as four diagrams in steps with the same steps in words beside each.
//!
//! Every sentence and every arrow maps to a row of `docs/security-page.md`;
//! a claim changes there first. The page describes mechanisms and names
//! their limits; it promises nothing.
//!
//! A diagram is a sequence chart: actors on top, one row a step. It is drawn
//! here as inline SVG. The site's policy allows no inline style, so every
//! position is an SVG attribute and every movement a class of
//! `style/security.css`. No script: a radio group above each chart picks
//! "play" or one step, and the stylesheet does the rest.
//!
//! The site is read in many languages, and SVG text neither wraps nor
//! shrinks. So a chart holds no sentence: only its actors' names (a word or
//! two, with room for twice the English width) and the steps' numbers. What
//! a step sends and what it means stand in the list beside the chart, as
//! HTML, under the same number and the same sign. The chart is hidden from
//! assistive technology, since that list says all of it in words.

use leptos::prelude::*;

use super::Built;
use crate::rich::{icon, plain, rich};
use crate::site::icons;
use crate::site::shell::{CENTRAL_REPOSITORY, Meta};
use crate::site::{Context, Page};
use crate::words::{self, Lang, Piece};

/// A plain message.
type Word = fn(Lang) -> &'static str;
/// A message with tags.
type Rich = fn(Lang) -> Vec<Piece>;

/// A play triangle (the site's own drawing).
const PLAY: &str = r#"<path d="M7 4.5v15l12-7.5z" fill="currentColor" />"#;
/// Scattered dots: a value that reads as noise.
const NOISE: &str = r#"<g fill="currentColor" stroke="none"><circle cx="6" cy="7" r="1.7" /><circle cx="13" cy="5" r="1.7" /><circle cx="18.5" cy="10" r="1.7" /><circle cx="8" cy="13.5" r="1.7" /><circle cx="15.5" cy="16" r="1.7" /><circle cx="9.5" cy="19.5" r="1.5" /></g>"#;
/// An asterisk: what was typed into a password field.
const TYPED: &str =
    r#"<path d="M12 5v14" /> <path d="m5.9 8.5 12.2 7" /> <path d="m18.1 8.5-12.2 7" />"#;

/// How a step's sign looks: what kind of thing travels, or that an actor
/// works by itself. The chart's value and the list's sign share the class.
#[derive(Clone, Copy)]
enum Look {
    /// Readable by whoever carries it: a request, a public key.
    Open,
    /// Encrypted for somebody: gold, with a lock.
    Sealed,
    /// A proof that checks out.
    Proof,
    /// A secret that stays on this device.
    Secret,
    /// Not a value: an actor at work.
    Work,
}

impl Look {
    const fn class(self) -> &'static str {
        match self {
            Look::Open => "sec-open",
            Look::Sealed => "sec-sealed",
            Look::Proof => "sec-proof",
            Look::Secret => "sec-secret",
            Look::Work => "sec-work",
        }
    }
}

/// What a step shows.
#[derive(Clone, Copy)]
enum Act {
    /// A value travels from one actor to another.
    Send {
        from: usize,
        to: usize,
        look: Look,
        glyph: &'static str,
    },
    /// An actor works by itself.
    Local { at: usize, glyph: &'static str },
}

impl Act {
    /// The step's sign: its picture and its look.
    const fn sign(self) -> (&'static str, Look) {
        match self {
            Act::Send { look, glyph, .. } => (glyph, look),
            Act::Local { glyph, .. } => (glyph, Look::Work),
        }
    }
}

/// One step: what the chart shows, and in the list what is sent or done
/// (a few words beside the step's sign), its title and its sentence.
struct Step {
    act: Act,
    what: Word,
    title: Word,
    text: fn(Lang) -> AnyView,
}

/// A note under a flow.
#[derive(Clone, Copy)]
enum Note {
    /// More of the same.
    More(Rich),
    /// A limit or an exception.
    Limit(Rich),
}

/// One flow: its section, its chart and its notes.
struct Flow {
    id: &'static str,
    eyebrow: Word,
    title: Word,
    lead: Word,
    /// Each actor's icon and name, left to right.
    actors: &'static [(&'static str, Word)],
    steps: [Step; 5],
    /// What the frame around the whole chart stands for, when it has one.
    frame: Option<Word>,
    /// What a reader has to know before the chart: that the flow cannot be
    /// used yet.
    status: Option<Rich>,
    notes: &'static [Note],
}

const fn send(from: usize, to: usize, look: Look, glyph: &'static str) -> Act {
    Act::Send {
        from,
        to,
        look,
        glyph,
    }
}

const fn local(at: usize, glyph: &'static str) -> Act {
    Act::Local { at, glyph }
}

/// A step whose sentence is plain text.
macro_rules! step {
    ($act:expr, $what:ident, $title:ident, $text:ident) => {
        Step {
            act: $act,
            what: words::$what,
            title: words::$title,
            text: |lang| words::$text(lang).into_any(),
        }
    };
}

/// Signing in with OPAQUE (claims A1 to A11).
const SIGN_IN: Flow = Flow {
    id: "signin",
    eyebrow: words::sec_signin_eyebrow,
    title: words::sec_signin_title,
    lead: words::sec_signin_lead,
    actors: &[
        (icons::LAPTOP, words::sec_actor_device),
        (icons::SERVER, words::sec_actor_central),
    ],
    steps: [
        step!(
            send(0, 1, Look::Open, NOISE),
            sec_signin_c1,
            sec_signin_t1,
            sec_signin_s1
        ),
        step!(
            send(1, 0, Look::Open, icons::KEY_ROUND),
            sec_signin_c2,
            sec_signin_t2,
            sec_signin_s2
        ),
        step!(
            local(0, icons::CPU),
            sec_signin_c3,
            sec_signin_t3,
            sec_signin_s3
        ),
        step!(
            send(0, 1, Look::Proof, icons::CHECK),
            sec_signin_c4,
            sec_signin_t4,
            sec_signin_s4
        ),
        step!(
            send(1, 0, Look::Sealed, icons::LOCK),
            sec_signin_c5,
            sec_signin_t5,
            sec_signin_s5
        ),
    ],
    frame: None,
    status: None,
    notes: &[
        Note::More(words::sec_signin_register),
        Note::Limit(words::sec_signin_legacy),
        Note::Limit(words::sec_signin_limit),
    ],
};

/// The password and the keys in the browser's memory (claims B1 to B11).
const MEMORY: Flow = Flow {
    id: "memory",
    eyebrow: words::sec_memory_eyebrow,
    title: words::sec_memory_title,
    lead: words::sec_memory_lead,
    actors: &[
        (icons::MONITOR, words::sec_actor_page),
        (icons::SETTINGS, words::sec_actor_worker),
        (icons::LAYERS, words::sec_actor_store),
    ],
    steps: [
        step!(
            send(0, 1, Look::Secret, TYPED),
            sec_memory_c1,
            sec_memory_t1,
            sec_memory_s1
        ),
        step!(
            local(1, icons::CPU),
            sec_memory_c2,
            sec_memory_t2,
            sec_memory_s2
        ),
        step!(
            local(1, icons::TRASH_2),
            sec_memory_c3,
            sec_memory_t3,
            sec_memory_s3
        ),
        step!(
            send(1, 2, Look::Sealed, icons::LOCK),
            sec_memory_c4,
            sec_memory_t4,
            sec_memory_s4
        ),
        step!(
            local(1, icons::X),
            sec_memory_c5,
            sec_memory_t5,
            sec_memory_s5
        ),
    ],
    frame: Some(words::sec_in_browser),
    status: None,
    notes: &[
        Note::Limit(words::sec_memory_limit),
        Note::More(words::sec_memory_client),
    ],
};

/// A chat message from one device to another (claims C1 to C8).
const CHAT: Flow = Flow {
    id: "chat",
    eyebrow: words::sec_chat_eyebrow,
    title: words::sec_chat_title,
    lead: words::sec_chat_lead,
    actors: &[
        (icons::USER, words::sec_actor_you),
        (icons::SERVER, words::sec_actor_central),
        (icons::USER_CHECK, words::sec_actor_contact),
    ],
    steps: [
        step!(local(0, icons::LOCK), sec_chat_c1, sec_chat_t1, sec_chat_s1),
        step!(
            local(0, icons::FINGERPRINT),
            sec_chat_c2,
            sec_chat_t2,
            sec_chat_s2
        ),
        step!(
            send(0, 1, Look::Sealed, icons::LOCK),
            sec_chat_c3,
            sec_chat_t3,
            sec_chat_s3
        ),
        step!(
            send(1, 2, Look::Sealed, icons::LOCK),
            sec_chat_c4,
            sec_chat_t4,
            sec_chat_s4
        ),
        step!(
            local(2, icons::CHECK),
            sec_chat_c5,
            sec_chat_t5,
            sec_chat_s5
        ),
    ],
    frame: None,
    status: None,
    notes: &[
        Note::More(words::sec_chat_meta),
        Note::Limit(words::sec_chat_limit),
    ],
};

/// Signing the client in from the website by a link (claims D1 to D11).
/// Central stands on the left, so the link from the page to the client
/// visibly passes it by.
const PAIRING: Flow = Flow {
    id: "pairing",
    eyebrow: words::sec_pair_eyebrow,
    title: words::sec_pair_title,
    lead: words::sec_pair_lead,
    actors: &[
        (icons::SERVER, words::sec_actor_central),
        (icons::GLOBE, words::sec_actor_website),
        (icons::GAMEPAD_2, words::sec_actor_client),
    ],
    steps: [
        step!(
            send(1, 0, Look::Open, icons::PLUS),
            sec_pair_c1,
            sec_pair_t1,
            sec_pair_s1
        ),
        Step {
            act: send(1, 2, Look::Secret, icons::LINK),
            what: words::sec_pair_c2,
            title: words::sec_pair_t2,
            text: |lang| plain(words::sec_pair_s2(lang)),
        },
        step!(
            send(2, 0, Look::Open, icons::KEY_ROUND),
            sec_pair_c3,
            sec_pair_t3,
            sec_pair_s3
        ),
        step!(
            send(1, 0, Look::Sealed, icons::LOCK),
            sec_pair_c4,
            sec_pair_t4,
            sec_pair_s4
        ),
        step!(
            send(0, 2, Look::Sealed, icons::LOCK),
            sec_pair_c5,
            sec_pair_t5,
            sec_pair_s5
        ),
    ],
    frame: None,
    // the one sentence to drop when the website has its pairing page and a
    // released client redeems the link (docs/security-page.md, D11)
    status: Some(words::sec_pair_status),
    notes: &[Note::More(words::sec_pair_note)],
};

const FLOWS: [&Flow; 4] = [&SIGN_IN, &MEMORY, &CHAT, &PAIRING];

/// The chart's size in its own units; a phone shows it at about 0.85.
const WIDTH: i32 = 320;
const HEIGHT: i32 = 314;
/// The first row's line and the distance to the next.
const FIRST_ROW: i32 = 116;
const ROW_GAP: i32 = 42;
/// A frame's left edge: the step numbers stay outside it.
const FRAME: i32 = 27;
/// A value's circle, and how far short of an actor's line it stops.
const VALUE: i32 = 12;
const SHORT: i32 = 18;

/// Where the actors stand. The step numbers have the strip to the left.
const fn columns(actors: usize) -> &'static [i32] {
    if actors == 2 {
        &[94, 254]
    } else {
        &[62, 164, 266]
    }
}

/// How far a value travels to either side of its arrow's middle.
const fn reach(from: i32, to: i32) -> i32 {
    (to - from).abs() / 2 - SHORT
}

/// A Lucide icon inside a chart, `size` wide with its middle at (`x`, `y`).
fn glyph_at(class: &'static str, glyph: &'static str, x: i32, y: i32, size: i32) -> impl IntoView {
    view! {
        <svg
            class=class
            x=(x - size / 2).to_string()
            y=(y - size / 2).to_string()
            width=size.to_string()
            height=size.to_string()
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            inner_html=glyph
        ></svg>
    }
}

/// One row of a chart: its number, and its arrow with the value on it or
/// the badge on its actor's line.
fn row(cols: &[i32], at: usize, step: &Step) -> impl IntoView + use<> {
    let y = FIRST_ROW + ROW_GAP * i32::try_from(at).unwrap_or(0);
    let number = at + 1;
    let art = match step.act {
        Act::Send {
            from,
            to,
            look,
            glyph,
        } => {
            let (x1, x2) = (cols[from], cols[to]);
            let dir = if x2 > x1 { 1 } else { -1 };
            let middle = i32::midpoint(x1, x2);
            let way = if dir > 0 { "sec-east" } else { "sec-west" };
            let head = format!("M{} {y}l{} -5v10z", x2 - 4 * dir, -9 * dir);
            view! {
                <line class="sec-wire" x1=(x1 + 7 * dir).to_string() y1=y.to_string()
                    x2=(x2 - 12 * dir).to_string() y2=y.to_string()></line>
                <path class="sec-head" d=head></path>
                <g transform=format!("translate({middle} {y})")>
                    <g class=format!("sec-token {} {way} sec-reach-{}", look.class(), reach(x1, x2))>
                        <circle r=VALUE.to_string()></circle>
                        {glyph_at("sec-token-ico", glyph, 0, 0, 15)}
                    </g>
                </g>
            }
            .into_any()
        }
        Act::Local { at: who, glyph } => {
            let x = cols[who];
            view! {
                <circle class="sec-halo" cx=x.to_string() cy=y.to_string() r="13"></circle>
                <circle class="sec-badge" cx=x.to_string() cy=y.to_string() r="13"></circle>
                {glyph_at("sec-badge-ico", glyph, x, y, 16)}
            }
            .into_any()
        }
    };
    // a row that stands back dims its drawing; its number keeps its contrast
    view! {
        <g class=format!("sec-row sec-r{number}")>
            <g class="sec-num">
                <circle cx="14" cy=y.to_string() r="9"></circle>
                <text x="14" y=(y + 4).to_string() text-anchor="middle">{number}</text>
            </g>
            <g class="sec-art">{art}</g>
        </g>
    }
}

/// A flow's chart. Its only words are the actors' names.
fn chart(lang: Lang, flow: &Flow) -> impl IntoView + use<> {
    let cols = columns(flow.actors.len());
    let frame = flow.frame.is_some().then(|| {
        view! {
            <rect class="sec-frame" x=FRAME.to_string() y="2" width=(WIDTH - 2 - FRAME).to_string()
                height=(HEIGHT - 4).to_string() rx="14"></rect>
        }
    });
    let actors = flow
        .actors
        .iter()
        .zip(cols)
        .map(|((glyph, name), x)| {
            view! {
                <line class="sec-life" x1=x.to_string() y1="88" x2=x.to_string() y2=(HEIGHT - 10).to_string()></line>
                <circle class="sec-node" cx=x.to_string() cy="36" r="22"></circle>
                {glyph_at("sec-node-ico", glyph, *x, 36, 22)}
                <text class="sec-name" x=x.to_string() y="77" text-anchor="middle">{name(lang)}</text>
            }
        })
        .collect_view();
    let rows = flow
        .steps
        .iter()
        .enumerate()
        .map(|(at, step)| row(cols, at, step))
        .collect_view();
    view! {
        <svg class="sec-chart" viewBox=format!("0 0 {WIDTH} {HEIGHT}") aria-hidden="true" focusable="false">
            {frame}
            {actors}
            {rows}
        </svg>
    }
}

/// A flow's chart with its switch, and the same steps in words.
fn figure(lang: Lang, flow: &Flow, flip: bool) -> impl IntoView + use<> {
    let name = format!("sec-{}", flow.id);
    let picks = (0..=5)
        .map(|n| {
            view! {
                <input class=format!("visually-hidden sec-pick sec-p{n}") type="radio" name=name.clone()
                    id=format!("{name}-{n}") checked=n == 0 />
            }
        })
        .collect_view();
    let play = view! {
        <label class="sec-l sec-l0" for=format!("{name}-0")>
            {icon(PLAY)}
            <span class="sec-when-moving">{words::sec_play(lang)}</span>
            <span class="sec-when-still">{words::sec_all(lang)}</span>
        </label>
    };
    let steps = (1..=5)
        .map(|n| {
            let number = n.to_string();
            view! {
                <label class=format!("sec-l sec-l{n}") for=format!("{name}-{n}")>
                    <span class="visually-hidden">{words::sec_step(lang, &number)}</span>
                    <span aria-hidden="true">{number}</span>
                </label>
            }
        })
        .collect_view();
    let list = flow
        .steps
        .iter()
        .enumerate()
        .map(|(at, step)| {
            let (glyph, look) = step.act.sign();
            view! {
                <li class=format!("sec-s{}", at + 1)>
                    <span class="sec-what">
                        <span class=format!("sec-sign {}", look.class())>{icon(glyph)}</span>
                        <span>{(step.what)(lang)}</span>
                    </span>
                    <strong>{(step.title)(lang)}</strong>
                    <span class="sec-text">{(step.text)(lang)}</span>
                </li>
            }
        })
        .collect_view();
    let frame = flow
        .frame
        .map(|means| view! { <p class="sec-frame-note">{means(lang)}</p> });
    let class = if flip { "sec-fig sec-flip" } else { "sec-fig" };
    view! {
        <div class=class role="group" aria-label=words::sec_figure(lang, (flow.title)(lang))>
            {picks}
            <div class="sec-stage glass">
                <div class="sec-controls">{play}<span class="sec-steps">{steps}</span></div>
                {chart(lang, flow)}
                {frame}
                <p class="sec-hint">
                    <span class="sec-when-moving">{words::sec_hint(lang)}</span>
                    <span class="sec-when-still">{words::sec_hint_still(lang)}</span>
                </p>
            </div>
            <ol class="sec-list" role="list">{list}</ol>
        </div>
    }
}

/// The start page's way in: the flows that can be used today and the
/// limits, each by the title its section has here, so the start page says
/// nothing this page does not.
pub fn teaser(lang: Lang) -> impl IntoView + use<> {
    const SIGNS: [&str; 4] = [icons::KEY_ROUND, icons::CPU, icons::LOCK, icons::LINK];
    let page = Page::Security.path(lang);
    let way = |sign: &'static str, id: &str, kind: &'static str, title: &'static str| {
        view! {
            <li>
                <a class="sec-way glass reveal" href=format!("{page}#{id}")>
                    <span class="sec-way-sign">{icon(sign)}</span>
                    <span class="sec-way-text">
                        <span class="sec-way-kind">{kind}</span>
                        <span class="sec-way-title">{title}</span>
                    </span>
                </a>
            </li>
        }
    };
    let flows = FLOWS
        .iter()
        .zip(SIGNS)
        .filter(|(flow, _)| flow.status.is_none())
        .map(|(flow, sign)| way(sign, flow.id, (flow.eyebrow)(lang), (flow.title)(lang)))
        .collect_view();
    let limits = way(
        icons::TRIANGLE_ALERT,
        "limits",
        words::sec_limits_eyebrow(lang),
        words::sec_limits_title(lang),
    );
    view! {
        <section class="band alt" id="security" aria-labelledby="security-way-title">
            <div class="wrap">
                <p class="eyebrow">{words::sec_eyebrow(lang)}</p>
                <h2 id="security-way-title">{words::sec_title(lang)}</h2>
                <p class="lead">{words::sec_lead(lang)}</p>
                <ul class="sec-ways stagger">{flows}{limits}</ul>
                <p class="sec-ways-more">
                    <a class="button button-ghost" href=page.clone()>{words::sec_teaser_more(lang)}</a>
                </p>
            </div>
        </section>
    }
}

/// One flow's section.
fn section(lang: Lang, at: usize, flow: &Flow) -> impl IntoView + use<> {
    let title = format!("{}-title", flow.id);
    let notes = flow
        .notes
        .iter()
        .map(|note| match *note {
            Note::More(text) => view! {
                <p class="sec-note">{icon(icons::INFO)}<span>{plain(text(lang))}</span></p>
            }
            .into_any(),
            Note::Limit(text) => view! {
                <p class="sec-note sec-warn">{icon(icons::TRIANGLE_ALERT)}<span>{plain(text(lang))}</span></p>
            }
            .into_any(),
        })
        .collect_view();
    let status = flow.status.map(|text| {
        view! {
            <p class="sec-note sec-warn sec-status">{icon(icons::TRIANGLE_ALERT)}<span>{plain(text(lang))}</span></p>
        }
    });
    let class = if at.is_multiple_of(2) {
        "band alt"
    } else {
        "band"
    };
    view! {
        <section class=class id=flow.id aria-labelledby=title>
            <div class="wrap">
                <p class="eyebrow">{(flow.eyebrow)(lang)}</p>
                <h2 id=title.clone()>{(flow.title)(lang)}</h2>
                <p class="lead">{(flow.lead)(lang)}</p>
                {status}
                {figure(lang, flow, !at.is_multiple_of(2))}
                <div class="sec-notes">{notes}</div>
            </div>
        </section>
    }
}

pub fn page(cx: &Context<'_>) -> Built {
    let lang = cx.lang;
    let mut meta = Meta::new(
        words::title_security(lang),
        words::desc_security(lang),
        "img/og.jpg",
    );
    meta.trail
        .push((words::nav_security(lang).to_owned(), Page::Security));
    let contents = FLOWS
        .iter()
        .map(|flow| (flow.id, (flow.eyebrow)(lang)))
        .chain([("limits", words::sec_limits_eyebrow(lang))])
        .enumerate()
        .map(|(at, (id, name))| {
            view! {
                <li><a href=format!("#{id}")><span class="num" aria-hidden="true">{at + 1}</span>{name}</a></li>
            }
        })
        .collect_view();
    let sections = FLOWS
        .iter()
        .enumerate()
        .map(|(at, flow)| section(lang, at, flow))
        .collect_view();
    let limits: [Rich; 5] = [
        words::sec_limit_device,
        words::sec_limit_site,
        words::sec_limit_central,
        words::sec_limit_recovery,
        words::sec_limit_audit,
    ];
    let contact = Page::Contact.path(lang);
    let limits = limits
        .into_iter()
        .map(|text| {
            let contact = contact.clone();
            view! { <li>{rich(text(lang), move |tag| (tag == "contact").then(|| contact.clone()))}</li> }
        })
        .collect_view();
    let privacy = Page::Privacy.path(lang);
    let documents = rich(words::sec_docs(lang), move |tag| {
        let document = |name: &str| format!("{CENTRAL_REPOSITORY}/blob/main/docs/{name}");
        match tag {
            "protocol" => Some(document("crypto-protocol.md")),
            "chat" => Some(document("central-chat.md")),
            "accounts" => Some(document("web-accounts.md")),
            "threats" => Some(document("threat-model.md")),
            "privacy" => Some(privacy.clone()),
            _ => None,
        }
    });
    let body = view! {
        <section class="page-head wrap" aria-labelledby="security-title">
            <p class="eyebrow">{words::sec_eyebrow(lang)}</p>
            <h1 id="security-title">{words::sec_title(lang)}</h1>
            <p class="lead">{words::sec_lead(lang)}</p>
            <nav class="sec-toc" aria-label=words::sec_toc_label(lang)><ol>{contents}</ol></nav>
        </section>
        {sections}
        <section class="band alt" id="limits" aria-labelledby="limits-title">
            <div class="wrap reading">
                <p class="eyebrow">{words::sec_limits_eyebrow(lang)}</p>
                <h2 id="limits-title">{words::sec_limits_title(lang)}</h2>
                <ul class="plain sec-limits">{limits}</ul>
                <h3 class="sec-docs-title">{words::sec_docs_title(lang)}</h3>
                <p class="sec-docs">{documents}</p>
            </div>
        </section>
    }
    .into_any();
    (meta, body)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::*;

    /// Every arrow joins two different actors the chart has, every badge
    /// sits on one of them, a value's travel is one the stylesheet knows,
    /// and every row lies inside the chart.
    #[test]
    fn charts_are_drawable() {
        let css = include_str!("../../../style/security.css");
        for flow in FLOWS {
            let cols = columns(flow.actors.len());
            assert_eq!(cols.len(), flow.actors.len(), "{}", flow.id);
            for step in &flow.steps {
                match step.act {
                    Act::Send { from, to, .. } => {
                        assert!(
                            from != to && from < cols.len() && to < cols.len(),
                            "{}",
                            flow.id
                        );
                        let reach = reach(cols[from], cols[to]);
                        assert!(
                            css.contains(&format!(".sec-reach-{reach} ")),
                            "{}: {reach}",
                            flow.id
                        );
                    }
                    Act::Local { at, .. } => assert!(at < cols.len(), "{}", flow.id),
                }
            }
        }
        let last = FIRST_ROW + ROW_GAP * 4;
        assert!(last + 13 < HEIGHT - 10, "the last row is on the lifelines");
    }

    /// How wide `text` is drawn at an actor's name's size, in the chart's
    /// units and on the generous side: a Latin or Cyrillic letter about
    /// 0.56 em, a Chinese, Japanese or Korean sign a full em.
    fn drawn(text: &str) -> i32 {
        text.chars()
            .map(|c| if u32::from(c) >= 0x2E80 { 120 } else { 67 })
            .sum::<i32>()
            / 10
    }

    /// The room an actor's name has: up to the chart's edge (or its frame)
    /// on either side and half the way to its neighbours.
    fn slot(flow: &Flow, cols: &[i32], at: usize) -> i32 {
        let x = cols[at];
        let left = if flow.frame.is_some() { FRAME + 4 } else { 2 };
        let edge = 2 * (x - left).min(WIDTH - 6 - x);
        let neighbours = cols
            .iter()
            .filter(|other| **other != x)
            .map(|other| (other - x).abs() - 6)
            .min()
            .unwrap_or(edge);
        edge.min(neighbours)
    }

    /// A chart's only words are its actors' names. SVG text does not wrap,
    /// so each has to fit its place in every language, and the English one
    /// twice over: that is the room a longer language gets.
    #[test]
    fn names_fit_with_room_to_grow() {
        for flow in FLOWS {
            let cols = columns(flow.actors.len());
            for (at, (_, name)) in flow.actors.iter().enumerate() {
                let room = slot(flow, cols, at);
                let english = name(Lang::En);
                assert!(
                    2 * drawn(english) <= room,
                    "{}: `{english}` twice over is {} of {room}",
                    flow.id,
                    2 * drawn(english)
                );
                for lang in Lang::ALL {
                    let name = name(lang);
                    assert!(
                        drawn(name) <= room,
                        "{}: `{name}` is {} of {room}",
                        flow.id,
                        drawn(name)
                    );
                }
            }
        }
    }

    /// The page makes no promise: the words it must not use are absent from
    /// its block in every language's file. (The list holds the German and
    /// English words; a new language adds its own.)
    #[test]
    fn no_promises() {
        let locales = Path::new(env!("CARGO_MANIFEST_DIR")).join("locales");
        let mut read = 0;
        for entry in fs::read_dir(locales).expect("the locales") {
            let file = entry.expect("an entry").path().join("site.ftl");
            let Ok(text) = fs::read_to_string(&file) else {
                continue;
            };
            let name = file.display();
            let block = text
                .split("# ====")
                .find(|part| part.contains("title-security"))
                .unwrap_or_else(|| panic!("{name}: the security page's words"));
            read += 1;
            for line in block.lines().filter(|line| !line.starts_with('#')) {
                let lower = line.to_lowercase();
                for word in [
                    "unknackbar",
                    "unbreakable",
                    "dsgvo",
                    "gdpr",
                    "100 %",
                    "100%",
                    "garantiert",
                    "guarantee",
                    "absolut",
                    "military",
                    "militär",
                ] {
                    assert!(!lower.contains(word), "{name}: `{word}` in: {line}");
                }
                // "sicher", "secure" and "safe" only in the page's own name
                // and in the safety number, which is the feature's name
                let said = lower
                    .replace("sicherheit", "")
                    .replace("security", "")
                    .replace("safety number", "");
                for word in ["sicher", "secure", "safe"] {
                    assert!(!said.contains(word), "{name}: `{word}` in: {line}");
                }
            }
        }
        assert!(read >= 2, "the files were found");
    }
}
