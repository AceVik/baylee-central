//! The legal pages: the Impressum, the privacy notice, Central's terms and
//! the Fan Content notice.

use leptos::prelude::*;

use super::Built;
use crate::format;
use crate::islands::Impressum;
use crate::markdown;
use crate::rich::{plain, rich};
use crate::site::shell::{Meta, policy_url};
use crate::site::{Context, Page};
use crate::words::{self, Lang};

/// Central's terms, the same files `central-service` compiles in.
const TERMS_EN: &str = include_str!("../../../../../legal/terms.en.md");
const TERMS_DE: &str = include_str!("../../../../../legal/terms.de.md");

/// The value of a `<!-- key: value -->` line at the head of a legal text.
fn header<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    text.lines()
        .take(5)
        .filter_map(|line| line.trim().strip_prefix("<!--")?.strip_suffix("-->"))
        .find_map(|inner| {
            inner
                .trim()
                .strip_prefix(key)?
                .strip_prefix(':')
                .map(str::trim)
        })
}

fn head<L: IntoView + 'static>(
    eyebrow: &'static str,
    title: &'static str,
    lead: L,
) -> impl IntoView + use<L> {
    view! {
        <section class="page-head wrap reading">
            <p class="eyebrow">{eyebrow}</p>
            <h1>{title}</h1>
            <p class="lead">{lead}</p>
        </section>
    }
}

/// Above a legal page in a language a machine translated: that it is a
/// help, and that the German and the English text apply.
fn machine_note(page: Page, lang: Lang) -> Option<impl IntoView + use<>> {
    lang.machine_translated().then(|| {
        let note = rich(words::legal_machine(lang), move |tag| match tag {
            "de" => Some(page.path(Lang::De)),
            "en" => Some(page.path(Lang::En)),
            _ => None,
        });
        view! { <p class="notice" role="note">{note}</p> }
    })
}

pub fn impressum(cx: &Context<'_>) -> Built {
    let lang = cx.lang;
    let mut meta = Meta::new(
        words::title_impressum(lang),
        words::desc_impressum(lang),
        "img/og.jpg",
    );
    meta.trail
        .push((words::impressum_title(lang).to_owned(), Page::Impressum));
    meta.islands = true;
    let body = view! {
        {head(words::imp_eyebrow(lang), words::impressum_title(lang), words::impressum_lead(lang))}
        <div class="wrap reading prose">
            {machine_note(Page::Impressum, lang)}
            // what the puzzle is for, in the page itself: the island holds
            // only the button and what it finds
            <div class="legal-box glass">
                <p class="small">{words::impressum_why(lang)}</p>
                <Impressum lang=lang worker=cx.assets.url("pow-worker.js") />
                <noscript><p class="notice">{words::impressum_noscript(lang)}</p></noscript>
                // A link no person sees, hears or reaches by Tab, and that
                // robots.txt forbids: the feedback service answers it with
                // invented details and gives whatever asked for it invented
                // ones here too, behind its hardest puzzle, for a day
                // (Baylee's `docs/feedback.md`, "The Impressum"). Nothing
                // on this site may ever follow or preload it.
                <a class="bait" href="/site/impressum/kontakt" tabindex="-1" aria-hidden="true" rel="nofollow" hidden>"Kontakt"</a>
            </div>
            <p>{words::impressum_responsible(lang)}</p>
            <p>{words::impressum_dispute(lang)}</p>
        </div>
    }
    .into_any();
    (meta, body)
}

/// A line of the privacy notice's contents: the part's anchor, its title.
type Part = (&'static str, fn(Lang) -> &'static str);

/// A privacy section of plain text: its anchor, its title, its text.
type Section = (
    &'static str,
    fn(Lang) -> &'static str,
    fn(Lang) -> &'static str,
);

/// The privacy notice's contents, in the order its parts stand.
const PRIVACY_PARTS: [Part; 23] = [
    ("controller", words::privacy_controller_title),
    ("short", words::privacy_short_title),
    ("hosting", words::privacy_hosting_title),
    ("views", words::privacy_views_title),
    ("device", words::privacy_device_title),
    ("github", words::privacy_github_title),
    ("links", words::privacy_links_title),
    ("directory", words::privacy_directory_title),
    ("account", words::privacy_account_title),
    ("challenges", words::privacy_challenges_title),
    ("tower", words::tower_title),
    ("email", words::privacy_email_title),
    ("picture", words::privacy_picture_title),
    ("cookie", words::privacy_cookie_title),
    ("chat", words::privacy_chat_title),
    ("push", words::privacy_push_title),
    ("counters", words::privacy_counters_title),
    ("contact", words::privacy_contact_title),
    ("reports", words::privacy_reports_title),
    ("impressum", words::privacy_impressum_title),
    ("recipients", words::privacy_recipients_title),
    ("rights", words::privacy_rights_title),
    ("objection", words::privacy_objection_title),
];

pub fn privacy(cx: &Context<'_>) -> Built {
    let lang = cx.lang;
    let mut meta = Meta::new(
        words::title_privacy(lang),
        words::desc_privacy(lang),
        "img/og.jpg",
    );
    meta.trail
        .push((words::privacy_title(lang).to_owned(), Page::Privacy));
    let impressum = Page::Impressum.path(lang);
    let simple: [Section; 7] = [
        (
            "hosting",
            words::privacy_hosting_title,
            words::privacy_hosting,
        ),
        ("github", words::privacy_github_title, words::privacy_github),
        (
            "directory",
            words::privacy_directory_title,
            words::privacy_directory,
        ),
        ("push", words::privacy_push_title, words::privacy_push),
        (
            "counters",
            words::privacy_counters_title,
            words::privacy_counters,
        ),
        (
            "contact",
            words::privacy_contact_title,
            words::privacy_contact,
        ),
        (
            "impressum",
            words::privacy_impressum_title,
            words::privacy_impressum,
        ),
    ];
    let section = |id: &'static str, title: &'static str, body: AnyView| {
        view! {
            <section class="legal-part" aria-labelledby=id>
                <h2 id=id>{title}</h2>
                <p>{body}</p>
            </section>
        }
    };
    let counting = cx.data.views.is_some();
    let contents = PRIVACY_PARTS
        .iter()
        .filter(|(id, _)| counting || *id != "views")
        .map(|(id, title)| view! { <li><a href=format!("#{id}")>{title(lang)}</a></li> })
        .collect_view();
    // said only where the deployment counts
    let views = cx.data.views.is_some().then(|| {
        section(
            "views",
            words::privacy_views_title(lang),
            words::privacy_views(lang).into_any(),
        )
    });
    let [hosting, github, directory, push, counters, contact, puzzle] =
        simple.map(|(id, title, text)| section(id, title(lang), text(lang).into_any()));
    let body = view! {
        {head(words::privacy_eyebrow(lang), words::privacy_title(lang), words::privacy_lead(lang))}
        <div class="wrap reading prose">
            {machine_note(Page::Privacy, lang)}
            <nav class="toc glass" aria-label=words::privacy_contents(lang)><ol>{contents}</ol></nav>
            <p class="meta">{words::privacy_updated(lang)}</p>
            {section("controller", words::privacy_controller_title(lang),
                rich(words::privacy_controller(lang), move |tag| (tag == "impressum").then(|| impressum.clone())))}
            {section("short", words::privacy_short_title(lang), words::privacy_short(lang).into_any())}
            {hosting}
            {views}
            <section class="legal-part" aria-labelledby="device">
                <h2 id="device">{words::privacy_device_title(lang)}</h2>
                <p>{plain(words::privacy_device(lang))}</p>
                // the recovery code, where its step put it at the person's word
                <p>{words::privacy_device_manager(lang)}</p>
            </section>
            {github}
            {section("links", words::privacy_links_title(lang), words::privacy_links(lang).into_any())}
            {directory}
            {section("account", words::privacy_account_title(lang), words::privacy_account(lang).into_any())}
            {section("challenges", words::privacy_challenges_title(lang), words::privacy_challenges(lang).into_any())}
            {section("tower", words::tower_title(lang), words::tower_privacy(lang).into_any())}
            {section("email", words::privacy_email_title(lang), words::privacy_email(lang).into_any())}
            {section("picture", words::privacy_picture_title(lang), words::privacy_picture(lang).into_any())}
            {section("cookie", words::privacy_cookie_title(lang), plain(words::privacy_cookie(lang)))}
            {section("chat", words::privacy_chat_title(lang), plain(words::privacy_chat(lang)))}
            {push}
            {counters}
            {contact}
            {section("reports", words::privacy_reports_title(lang), words::privacy_reports(lang).into_any())}
            {puzzle}
            {section("recipients", words::privacy_recipients_title(lang), words::privacy_recipients(lang).into_any())}
            {section("rights", words::privacy_rights_title(lang), words::privacy_rights(lang).into_any())}
            // Art. 21(4): the right to object stands apart from the rest
            {section("objection", words::privacy_objection_title(lang), words::privacy_objection(lang).into_any())}
        </div>
    }
    .into_any();
    (meta, body)
}

pub fn terms(cx: &Context<'_>) -> Built {
    let lang = cx.lang;
    let mut meta = Meta::new(
        words::title_terms(lang),
        words::desc_terms(lang),
        "img/og.jpg",
    );
    meta.trail
        .push((words::terms_title(lang).to_owned(), Page::Terms));
    // the terms are written in German and English; every other language
    // reads the English, marked as English for whoever has it read aloud
    let written = lang.of_record();
    let text = match written {
        Lang::De => TERMS_DE,
        _ => TERMS_EN,
    };
    let foreign = (written != lang).then(|| {
        let german = Page::Terms.path(Lang::De);
        let note = rich(words::terms_machine(lang), move |tag| {
            (tag == "de").then(|| german.clone())
        });
        view! { <p class="notice" role="note">{note}</p> }
    });
    let version = header(text, "version").unwrap_or("draft");
    let state = if version == "draft" {
        view! { <p class="notice" role="note">{words::terms_draft(lang)}</p> }.into_any()
    } else {
        let updated =
            header(text, "updated").map_or_else(String::new, |date| format::date(lang, date));
        view! { <p class="meta">{words::terms_version(lang, version, &updated)}</p> }.into_any()
    };
    let body = view! {
        {head(words::terms_eyebrow(lang), words::terms_title(lang), "")}
        <div class="wrap reading prose legal-text">
            {foreign}
            {state}
            <div lang=written.tag()>{markdown::render(text)}</div>
        </div>
    }
    .into_any();
    (meta, body)
}

pub fn fan_content(cx: &Context<'_>) -> Built {
    let lang = cx.lang;
    let mut meta = Meta::new(
        words::title_fan_content(lang),
        words::desc_fan_content(lang),
        "img/og.jpg",
    );
    meta.trail
        .push((words::fc_title(lang).to_owned(), Page::FanContent));
    let policy = policy_url(lang);
    let notice = rich(words::fan_content(lang), move |tag| {
        (tag == "policy").then(|| policy.to_owned())
    });
    let link = rich(words::fc_policy(lang), move |tag| {
        (tag == "policy").then(|| policy.to_owned())
    });
    let body = view! {
        {head(words::fc_eyebrow(lang), words::fc_title(lang), words::fc_lead(lang))}
        <div class="wrap reading prose">
            {machine_note(Page::FanContent, lang)}
            <h2>{words::fc_notice_title(lang)}</h2>
            <blockquote class="quote"><p>{notice}</p></blockquote>
            <h2>{words::fc_means_title(lang)}</h2>
            <p>{words::fc_means(lang)}</p>
            <p>{words::fc_images(lang)}</p>
            <p class="lone-link">{link}</p>
        </div>
    }
    .into_any();
    (meta, body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terms_headers() {
        assert_eq!(
            header(
                "<!-- version: 2026-10-22 -->\n<!-- updated: 2026-10-21 -->\n# T",
                "updated"
            ),
            Some("2026-10-21")
        );
        assert_eq!(header("# T", "version"), None);
        assert!(header(TERMS_EN, "version").is_some());
    }
}
