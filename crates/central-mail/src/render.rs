//! The mails as plain text and as HTML (`docs/email.md`). Both are written
//! from the same content in the same order: the title, what happened, its
//! facts as a short list, the code, what to do if this was not you, the
//! button, and the footer with the site's address.
//!
//! The HTML is tables with inline styles and the device's own fonts, in
//! the website's colours: a band of the blue hour with the wordmark in
//! letters, and a card in sunlight below it. It has no image but, where
//! the mailer attaches it, the logo beside the wordmark (`cid:baylee-logo`,
//! a part of the mail itself), no style sheet, no script, nothing loaded
//! from elsewhere and nothing that tracks. Its links go to the configured
//! site and nowhere else: the button to a page of it, the footer to its
//! start page. A code mail's button carries its code in the fragment of
//! its link (`#code=…`), which the page fills into its field and removes
//! from the address bar; never in a path or a query, and in no other link.
//! Everything written into the HTML is escaped, the crate's own words
//! included.
//!
//! Without a style sheet there is no dark variant. The mail says it is
//! light only, and every text stands on a ground the mail sets itself (the
//! band, the card, the code's box, the warning, the button), so a reader
//! that darkens light grounds and lightens dark text keeps each pair
//! apart. The tests measure every pair as drawn and as inverted.
use crate::{Kind, Mail, locale::Lang};
use std::fmt::Write as _;
use zeroize::Zeroizing;

// The website's colours (`crates/central-web/style/tokens.css`): the blue
// hour's for the band, sunlight's for the rest. Two are the site's
// translucent ones laid on the card, written out.
/// The blue hour's `--bg`: the band.
const NIGHT: &str = "#0f1b2d";
/// The blue hour's `--gold`: the band's edge and the mark's tile.
const GOLD: &str = "#e3c07e";
/// The blue hour's `--title`: the name in the band.
const DAWN: &str = "#fff7e4";
/// `--bg`: around the card.
const GROUND: &str = "#f2efe4";
/// `--panel-solid`: the card.
const PANEL: &str = "#fffefa";
/// `--edge-soft` on the card: its edge and the footer's rule.
const RULE: &str = "#dbdfdc";
/// `--violet-soft` on the card: the code's box.
const TINT: &str = "#ebefed";
/// `--ink`.
const INK: &str = "#233e43";
/// `--ink-soft`.
const SOFT: &str = "#415c5e";
/// `--title`.
const TITLE: &str = "#173c43";
/// `--gold`: the button, the code's frame, the line under the title, the
/// list's marks and the footer's link.
const ACCENT: &str = "#235b66";
/// `--gold-ink`: the button's words.
const ON_ACCENT: &str = "#fffaf0";
/// `--warn`: the warning's edge.
const WARN: &str = "#8a5a05";
/// `--warn-tint`: the warning's ground.
const WARN_TINT: &str = "#faecd2";
const SANS: &str = "system-ui,-apple-system,'Segoe UI',Roboto,'Helvetica Neue',Arial,sans-serif";
/// The site's titles are Literata; a mail loads no font and falls back to
/// the device's serif.
const SERIF: &str = "Literata,'Iowan Old Style',Georgia,'Times New Roman',serif";
const MONO: &str = "ui-monospace,'SF Mono',Menlo,Consolas,'Liberation Mono',monospace";
/// The card's width, in CSS pixels. Where the mail is narrower the card
/// is too.
const CARD: u32 = 560;
/// The room left and right of the card's text.
const SIDE: u32 = 28;
const TABLE: &str = "table role=\"presentation\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\"";
/// The mark before each fact of a list: a dot, as text.
const MARK: &str = "\u{2022}";
/// The `Content-ID` under which the mailer attaches the logo, when it is
/// told to (`docs/email.md`, "Configuration").
pub(crate) const LOGO: &str = "baylee-logo";
/// The logo's side in the band, in CSS pixels.
const LOGO_SIDE: u32 = 48;

/// A mail as the mailer takes it. No Debug: the text holds a code. The
/// bodies are wiped when dropped.
pub(crate) struct Rendered {
    pub(crate) subject: String,
    pub(crate) text: Zeroizing<String>,
    pub(crate) html: Zeroizing<String>,
}

/// A code as a mail shows it.
struct Code<'a> {
    /// Its printed form.
    value: &'a str,
    /// The size it is set in, in pixels, and the space between its
    /// characters.
    size: u8,
    spacing: &'static str,
    /// The lines under it: how long it lasts, and to keep it to oneself.
    notes: Vec<String>,
}

/// A paragraph's words in pieces: the message's own (`false`) and the
/// values written into it (`true`), which the HTML sets in bold.
struct Paragraph(Vec<(String, bool)>);

impl Paragraph {
    /// Words with nothing set apart.
    fn plain(words: String) -> Self {
        Self(vec![(words, false)])
    }

    /// The words as plain text.
    fn text(&self) -> String {
        self.0.iter().map(|(words, _)| words.as_str()).collect()
    }

    /// The words as HTML: all of them escaped, a value in bold.
    fn html(&self) -> String {
        self.0
            .iter()
            .map(|(words, value)| {
                if *value {
                    format!(
                        "<strong style=\"font-weight:700;color:{TITLE};\">{}</strong>",
                        escape(words)
                    )
                } else {
                    escape(words)
                }
            })
            .collect()
    }
}

/// What a mail says, before it is written as text or as HTML.
struct Content<'a> {
    subject: String,
    title: String,
    /// Paragraphs above the code: what happened.
    before: Vec<Paragraph>,
    /// What came of it, as a short list under those paragraphs.
    facts: Vec<String>,
    code: Option<Code<'a>>,
    /// What to do if this was not you, set apart.
    warning: Option<String>,
    /// The button's words and where it goes: a page of the site, with the
    /// code in the fragment for a code mail (so wiped like the code).
    link: Option<(String, Zeroizing<String>)>,
    /// Paragraphs at the end, in a quieter hand.
    after: Vec<String>,
}

/// Write `mail` for the site at the origin `site`. With `logo` the band
/// shows the logo the mailer attaches as [`LOGO`] instead of the letter's
/// tile.
pub(crate) fn render(mail: &Mail, site: &str, logo: bool) -> Rendered {
    let lang = mail.lang;
    let content = content(mail, site);
    let start = Start::of(site, lang);
    Rendered {
        text: text(lang, &content, &start),
        html: html(lang, &content, &start, logo),
        subject: content.subject,
    }
}

/// The site's start page in the mail's language, which the footer links
/// to under the site's host name.
struct Start {
    url: String,
    host: String,
}

impl Start {
    fn of(site: &str, lang: Lang) -> Self {
        Self {
            url: format!("{site}/{}/", lang.code()),
            host: site
                .split_once("://")
                .map_or(site, |(_, host)| host)
                .to_owned(),
        }
    }
}

fn content<'a>(mail: &'a Mail, site: &str) -> Content<'a> {
    let lang = mail.lang;
    let say = |id: &str| lang.text(id, &[]);
    let page = |path: &str| format!("{site}/{}/{path}", lang.code());
    // A notice: its subject and title, what happened and its facts, what
    // to do if it was not you, and the page that helps then.
    let notice = |which: &str,
                  before: Vec<String>,
                  facts: Vec<String>,
                  not_you: &str,
                  (words, path): (&str, &str)| Content {
        subject: say(&format!("{which}-subject")),
        title: say(&format!("{which}-title")),
        before: before.into_iter().map(Paragraph::plain).collect(),
        facts,
        code: None,
        warning: Some(say(not_you)),
        link: Some((say(words), Zeroizing::new(page(path)))),
        after: Vec::new(),
    };
    let security = ("notice-link", "account/security");
    let reset = ("reset-link", "reset");
    match &mail.kind {
        // where an address that waits takes its code
        Kind::Confirm { code, minutes } => {
            confirm_mail(lang, &page("account/security"), code, *minutes)
        }
        Kind::Reset {
            code,
            name,
            minutes,
        } => reset_mail(lang, &page("reset"), code, name, *minutes),
        Kind::Replaced => notice(
            "replaced",
            vec![say("replaced-body")],
            Vec::new(),
            "notice-check",
            security,
        ),
        Kind::Removed => notice(
            "removed",
            vec![say("removed-body")],
            Vec::new(),
            "notice-check",
            security,
        ),
        // what happened, then what came of it, a fact a line
        Kind::ResetDone { new_vault } => {
            let (vault, recovery) = if *new_vault {
                ("reset-done-new", "reset-done-recovery-new")
            } else {
                ("reset-done-kept", "reset-done-recovery-kept")
            };
            notice(
                "reset-done",
                vec![say("reset-done-body")],
                vec![say(vault), say("reset-done-sessions"), say(recovery)],
                "reset-done-not-you",
                reset,
            )
        }
        Kind::PasswordChanged => notice(
            "password",
            vec![say("password-body"), say("password-sessions")],
            Vec::new(),
            "password-not-you",
            reset,
        ),
        // the security page: where the password is changed, which ends
        // the code
        Kind::ResetIssued { hours } => notice(
            "issued",
            vec![
                say("issued-body"),
                lang.text("issued-valid", &[("hours", &hours.to_string())]),
            ],
            Vec::new(),
            "issued-not-you",
            security,
        ),
    }
}

/// The address of `page` with `code` in its fragment: `#code=` and the code
/// as printed, whose characters a URL takes as they are. A browser sends
/// no fragment to the site and puts none into a `Referer`; the page fills
/// the code into its field, never sends it by itself, and removes the
/// fragment from the address bar (`docs/email.md`, "The code in the
/// button's link"). Wiped when dropped, as the code is.
fn with_code(page: &str, code: &str) -> Zeroizing<String> {
    let mut url = Zeroizing::new(String::with_capacity(page.len() + 6 + code.len()));
    url.push_str(page);
    url.push_str("#code=");
    url.push_str(code);
    url
}

/// The mail with the `code` that confirms an address, which lasts
/// `minutes`, and a button to the security page at `page`, where an
/// address that waits takes its code, with the code for its field.
fn confirm_mail<'a>(lang: Lang, page: &str, code: &'a str, minutes: u32) -> Content<'a> {
    let say = |id: &str| lang.text(id, &[]);
    Content {
        subject: say("confirm-subject"),
        title: say("confirm-title"),
        before: vec![Paragraph::plain(say("confirm-intro"))],
        facts: Vec::new(),
        code: Some(Code {
            value: code,
            size: 30,
            spacing: "0.04em",
            notes: notes(lang, "confirm-valid", minutes),
        }),
        warning: None,
        link: Some((say("confirm-link"), with_code(page, code))),
        after: vec![say("confirm-ignore")],
    }
}

/// The mail with the reset `code` for the account `name`, which lasts
/// `minutes`, and a button to the reset page at `page` with the code for
/// its field.
fn reset_mail<'a>(lang: Lang, page: &str, code: &'a str, name: &str, minutes: u32) -> Content<'a> {
    let say = |id: &str| lang.text(id, &[]);
    Content {
        subject: say("reset-subject"),
        title: say("reset-title"),
        // the account's name in bold: the reader sees at once which
        // account it is
        before: vec![
            Paragraph(lang.pieces("reset-intro", &[("name", name)])),
            Paragraph::plain(say("reset-how")),
        ],
        facts: Vec::new(),
        // 45 characters at 15 px of a monospace font without extra
        // space are about 406 px, less than the 470 px inside the
        // code's box on the 560 px card: one line on a wide screen,
        // and on a narrow one it breaks at its hyphens.
        code: Some(Code {
            value: code,
            size: 15,
            spacing: "0",
            notes: notes(lang, "reset-valid", minutes),
        }),
        warning: None,
        link: Some((say("reset-link"), with_code(page, code))),
        after: vec![say("reset-ignore")],
    }
}

/// The lines under a code: how long it lasts (the message `valid`, with
/// its `minutes`), and that nobody at Baylee asks for it.
fn notes(lang: Lang, valid: &str, minutes: u32) -> Vec<String> {
    vec![
        lang.text(valid, &[("minutes", &minutes.to_string())]),
        lang.text("code-secret", &[]),
    ]
}

/// The plain text, complete without the HTML: the title on the first line
/// and an empty one under it, so that a list of mails that shows the text's
/// beginning shows the title and the first sentence; paragraphs with an
/// empty line between them, the facts a line each after `- `, the code
/// alone on a line under its label, the link's address alone on a line,
/// and the footer and the start page's address under the signature's mark
/// (`-- `), which readers set apart.
fn text(lang: Lang, content: &Content<'_>, start: &Start) -> Zeroizing<String> {
    let mut blocks = vec![content.title.clone()];
    blocks.extend(content.before.iter().map(Paragraph::text));
    if !content.facts.is_empty() {
        let facts: Vec<String> = content
            .facts
            .iter()
            .map(|fact| format!("- {fact}"))
            .collect();
        blocks.push(facts.join("\n"));
    }
    if let Some(code) = &content.code {
        blocks.push(format!("{}:\n{}", lang.text("code-label", &[]), code.value));
        blocks.push(code.notes.join("\n"));
    }
    blocks.extend(content.warning.iter().cloned());
    if let Some((words, url)) = &content.link {
        blocks.push(format!("{words}:\n{}", url.as_str()));
    }
    blocks.extend(content.after.iter().cloned());
    blocks.push(format!("-- \n{}\n{}", lang.text("footer", &[]), start.url));
    let mut out = Zeroizing::new(blocks.join("\n\n"));
    out.push('\n');
    blocks.iter_mut().for_each(zeroize::Zeroize::zeroize);
    out
}

/// `text` for an HTML text node or a quoted attribute.
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

/// A row of the card: `inner` (HTML already) in a cell with the card's
/// room at its sides and `top` pixels above it.
fn row(out: &mut String, top: u32, inner: &str) {
    let _ = writeln!(
        out,
        "<tr><td style=\"padding:{top}px {SIDE}px 0 {SIDE}px;\">{inner}</td></tr>"
    );
}

/// A paragraph of `inner` (HTML already), `size` pixels high, in `colour`.
fn paragraph(inner: &str, size: u8, colour: &str) -> String {
    format!(
        "<p style=\"margin:0;font-family:{SANS};font-size:{size}px;line-height:1.6;color:{colour};\">{inner}</p>"
    )
}

/// The band at the card's top: the blue hour with a gold edge under it,
/// and the wordmark in letters beside a picture, as the site's header has
/// it. With `logo`, the picture is the logo the mailer attaches, rounded,
/// with no text of its own since the name stands beside it; without, the
/// name's first letter on a gold tile, not read aloud.
fn band(out: &mut String, lang: Lang, logo: bool) {
    let brand = lang.text("brand", &[]);
    let (mark, high) = if logo {
        (
            format!(
                "<td width=\"{LOGO_SIDE}\" height=\"{LOGO_SIDE}\" valign=\"middle\" style=\"width:{LOGO_SIDE}px;height:{LOGO_SIDE}px;\">\
                 <img src=\"cid:{LOGO}\" width=\"{LOGO_SIDE}\" height=\"{LOGO_SIDE}\" alt=\"\" style=\"display:block;width:{LOGO_SIDE}px;height:{LOGO_SIDE}px;border:0;border-radius:12px;\"></td>"
            ),
            LOGO_SIDE,
        )
    } else {
        let letter: String = brand.chars().take(1).collect();
        (
            format!(
                "<td width=\"34\" height=\"34\" align=\"center\" valign=\"middle\" bgcolor=\"{GOLD}\" aria-hidden=\"true\" style=\"width:34px;height:34px;background:{GOLD};border-radius:9px;font-family:{SERIF};font-size:20px;line-height:34px;font-weight:700;color:{NIGHT};mso-line-height-rule:exactly;\">{}</td>",
                escape(&letter)
            ),
            34,
        )
    };
    let _ = writeln!(
        out,
        "<tr><td bgcolor=\"{NIGHT}\" style=\"padding:20px {SIDE}px;background:{NIGHT};border-bottom:3px solid {GOLD};border-radius:13px 13px 0 0;\">\
         <{TABLE}><tr>{mark}\
         <td bgcolor=\"{NIGHT}\" style=\"padding-left:12px;background:{NIGHT};font-family:{SERIF};font-size:24px;line-height:{high}px;font-weight:700;letter-spacing:0.01em;color:{DAWN};mso-line-height-rule:exactly;\">{}</td>\
         </tr></table></td></tr>",
        escape(&brand),
    );
}

/// The facts of what happened, a line each behind a small mark in the
/// accent: a table, as every reader draws one, the marks not read aloud.
fn list(facts: &[String]) -> String {
    let mut out = format!("<{TABLE} width=\"100%\">");
    for (at, fact) in facts.iter().enumerate() {
        let below = if at + 1 == facts.len() { 0 } else { 10 };
        let _ = write!(
            out,
            "<tr><td width=\"22\" valign=\"top\" aria-hidden=\"true\" style=\"width:22px;padding:0 0 {below}px 0;font-family:{SANS};font-size:16px;line-height:26px;font-weight:700;color:{ACCENT};mso-line-height-rule:exactly;\">{MARK}</td>\
             <td valign=\"top\" style=\"padding:0 0 {below}px 0;font-family:{SANS};font-size:16px;line-height:26px;color:{INK};mso-line-height-rule:exactly;\">{}</td></tr>",
            escape(fact)
        );
    }
    out.push_str("</table>");
    out
}

/// The code in a box of its own, its label above it and its notes below.
/// Where a reader allows it, a click selects the whole code.
fn code_box(lang: Lang, code: &Code<'_>) -> String {
    let notes = code
        .notes
        .iter()
        .map(|note| escape(note))
        .collect::<Vec<_>>()
        .join("<br>");
    format!(
        "<{TABLE} width=\"100%\">\
         <tr><td style=\"padding:0 0 8px 0;font-family:{SANS};font-size:12px;line-height:1.4;font-weight:700;letter-spacing:0.1em;text-transform:uppercase;color:{SOFT};\">{}</td></tr>\
         <tr><td align=\"center\" bgcolor=\"{TINT}\" style=\"padding:18px 14px;background:{TINT};border:2px dashed {ACCENT};border-radius:10px;-webkit-user-select:all;user-select:all;font-family:{MONO};font-size:{}px;line-height:1.4;font-weight:700;letter-spacing:{};color:{TITLE};\">{}</td></tr>\
         <tr><td style=\"padding:10px 0 0 0;font-family:{SANS};font-size:14px;line-height:1.55;color:{SOFT};\">{notes}</td></tr>\
         </table>",
        escape(&lang.text("code-label", &[])),
        code.size,
        code.spacing,
        escape(code.value),
    )
}

/// What to do if this was not you, on the warning's ground with its edge.
fn warning_box(words: &str) -> String {
    format!(
        "<{TABLE} width=\"100%\"><tr>\
         <td bgcolor=\"{WARN_TINT}\" style=\"padding:14px 16px;background:{WARN_TINT};border-left:4px solid {WARN};border-radius:6px;font-family:{SANS};font-size:15px;line-height:1.55;color:{INK};\">{}</td>\
         </tr></table>",
        escape(words)
    )
}

/// The button, as one that holds in every reader: the cell carries the
/// colour and Outlook's room (`mso-padding-alt`), the link its own room
/// for the others, and the button is 48 px high.
fn button(words: &str, url: &str) -> String {
    format!(
        "<{TABLE}><tr>\
         <td align=\"center\" bgcolor=\"{ACCENT}\" style=\"background:{ACCENT};border-radius:8px;mso-padding-alt:14px 26px;\">\
         <a href=\"{}\" style=\"display:inline-block;padding:14px 26px;font-family:{SANS};font-size:16px;line-height:20px;font-weight:700;color:{ON_ACCENT};text-decoration:none;border-radius:8px;mso-padding-alt:0;\">{}</a>\
         </td></tr></table>",
        escape(url),
        escape(words)
    )
}

/// The footer under a thin rule, and under it a quiet line with the
/// site's host name, which links to its start page.
fn footer(lang: Lang, start: &Start) -> String {
    format!(
        "<{TABLE} width=\"100%\">\
         <tr><td style=\"padding:16px 0 0 0;border-top:1px solid {RULE};font-family:{SANS};font-size:13px;line-height:1.55;color:{SOFT};\">{}</td></tr>\
         <tr><td style=\"padding:8px 0 26px 0;font-family:{SANS};font-size:13px;line-height:1.55;color:{SOFT};\">\
         <a href=\"{}\" style=\"color:{ACCENT};text-decoration:underline;\">{}</a></td></tr>\
         </table>",
        escape(&lang.text("footer", &[])),
        escape(&start.url),
        escape(&start.host),
    )
}

/// The HTML beside the plain text, for a reader that shows it.
fn html(lang: Lang, content: &Content<'_>, start: &Start, logo: bool) -> Zeroizing<String> {
    let mut out = Zeroizing::new(String::with_capacity(8192));
    let _ = write!(
        out,
        "<!doctype html>\n<html lang=\"{}\" style=\"color-scheme:light only;\">\n<head>\n<meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <meta name=\"color-scheme\" content=\"light only\">\n\
         <meta name=\"supported-color-schemes\" content=\"light only\">\n\
         <meta name=\"x-apple-disable-message-reformatting\">\n<title>{}</title>\n</head>\n\
         <body style=\"margin:0;padding:0;background:{GROUND};-webkit-text-size-adjust:100%;text-size-adjust:100%;\">\n",
        lang.code(),
        escape(&content.subject),
    );
    // What a list of mails shows beside the subject: the first paragraph,
    // never a code.
    if let Some(first) = content.before.first() {
        let _ = writeln!(
            out,
            "<div style=\"display:none;max-height:0;max-width:0;overflow:hidden;opacity:0;font-size:1px;line-height:1px;color:{GROUND};mso-hide:all;\">{}</div>",
            escape(&first.text())
        );
    }
    let _ = write!(
        out,
        "<{TABLE} width=\"100%\" bgcolor=\"{GROUND}\" style=\"background:{GROUND};\">\n\
         <tr><td align=\"center\" style=\"padding:32px 10px;\">\n\
         <!--[if mso]><{TABLE} width=\"{CARD}\" align=\"center\"><tr><td><![endif]-->\n\
         <{TABLE} width=\"100%\" bgcolor=\"{PANEL}\" style=\"max-width:{CARD}px;background:{PANEL};border:1px solid {RULE};border-radius:14px;border-collapse:separate;\">\n",
    );
    band(&mut out, lang, logo);
    row(
        &mut out,
        34,
        &format!(
            "<h1 style=\"margin:0;font-family:{SERIF};font-size:26px;line-height:1.3;font-weight:700;color:{TITLE};\">{}</h1>",
            escape(&content.title)
        ),
    );
    // a short line under the title
    row(
        &mut out,
        14,
        &format!(
            "<{TABLE}><tr><td width=\"44\" height=\"3\" bgcolor=\"{ACCENT}\" style=\"width:44px;height:3px;background:{ACCENT};border-radius:2px;font-size:3px;line-height:3px;mso-line-height-rule:exactly;\">&nbsp;</td></tr></table>"
        ),
    );
    for (at, words) in content.before.iter().enumerate() {
        row(
            &mut out,
            if at == 0 { 20 } else { 14 },
            &paragraph(&words.html(), 16, INK),
        );
    }
    if !content.facts.is_empty() {
        row(&mut out, 16, &list(&content.facts));
    }
    if let Some(code) = &content.code {
        row(&mut out, 26, &code_box(lang, code));
    }
    if let Some(words) = &content.warning {
        row(&mut out, 24, &warning_box(words));
    }
    if let Some((words, url)) = &content.link {
        row(&mut out, 26, &button(words, url));
    }
    for words in &content.after {
        row(&mut out, 22, &paragraph(&escape(words), 15, SOFT));
    }
    row(&mut out, 32, &footer(lang, start));
    out.push_str(
        "</table>\n<!--[if mso]></td></tr></table><![endif]-->\n</td></tr>\n</table>\n</body>\n</html>\n",
    );
    out
}

#[cfg(test)]
mod tests;
