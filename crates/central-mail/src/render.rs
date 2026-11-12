//! The mails as plain text and as HTML (`docs/email.md`). Both are written
//! from the same content in the same order: the title, what happened, the
//! code, what to do if this was not you, the one link, the footer.
//!
//! The HTML is tables with inline styles and the device's own fonts, in
//! the website's colours: a band of the blue hour with the wordmark in
//! letters, and a card in sunlight below it. It has no image, no style
//! sheet, no script, nothing loaded from elsewhere and nothing that
//! tracks. Its one link is a button to a page of the configured site, and
//! no link ever holds a code. Everything written into the HTML is escaped,
//! the crate's own words included.
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
/// `--gold`: the button, the code's frame, the line under the title.
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

/// What a mail says, before it is written as text or as HTML.
struct Content<'a> {
    subject: String,
    title: String,
    /// Paragraphs above the code: what happened.
    before: Vec<String>,
    code: Option<Code<'a>>,
    /// What to do if this was not you, set apart.
    warning: Option<String>,
    /// The button's words and where it goes: a page of the site.
    link: Option<(String, String)>,
    /// Paragraphs at the end, in a quieter hand.
    after: Vec<String>,
}

/// Write `mail` for the site at the origin `site`.
pub(crate) fn render(mail: &Mail, site: &str) -> Rendered {
    let lang = mail.lang;
    let content = content(mail, site);
    Rendered {
        text: text(lang, &content),
        html: html(lang, &content),
        subject: content.subject,
    }
}

fn content<'a>(mail: &'a Mail, site: &str) -> Content<'a> {
    let lang = mail.lang;
    let say = |id: &str| lang.text(id, &[]);
    let page = |path: &str| format!("{site}/{}/{path}", lang.code());
    // A notice: its subject and title, what happened, what to do if it
    // was not you, and the page that helps then.
    let notice =
        |which: &str, before: Vec<String>, not_you: &str, (words, path): (&str, &str)| Content {
            subject: say(&format!("{which}-subject")),
            title: say(&format!("{which}-title")),
            before,
            code: None,
            warning: Some(say(not_you)),
            link: Some((say(words), page(path))),
            after: Vec::new(),
        };
    let security = ("notice-link", "account/security");
    let reset = ("reset-link", "reset");
    match &mail.kind {
        Kind::Confirm { code, minutes } => Content {
            subject: say("confirm-subject"),
            title: say("confirm-title"),
            before: vec![say("confirm-intro")],
            code: Some(Code {
                value: code.as_str(),
                size: 30,
                spacing: "0.04em",
                notes: notes(lang, "confirm-valid", *minutes),
            }),
            warning: None,
            link: None,
            after: vec![say("confirm-ignore")],
        },
        Kind::Reset {
            code,
            name,
            minutes,
        } => Content {
            subject: say("reset-subject"),
            title: say("reset-title"),
            before: vec![
                lang.text("reset-intro", &[("name", name)]),
                say("reset-how"),
            ],
            // 45 characters at 15 px of a monospace font without extra
            // space are about 406 px, less than the 470 px inside the
            // code's box on the 560 px card: one line on a wide screen,
            // and on a narrow one it breaks at its hyphens.
            code: Some(Code {
                value: code.as_str(),
                size: 15,
                spacing: "0",
                notes: notes(lang, "reset-valid", *minutes),
            }),
            warning: None,
            link: Some((say("reset-link"), page("reset"))),
            after: vec![say("reset-ignore")],
        },
        Kind::Replaced => notice(
            "replaced",
            vec![say("replaced-body")],
            "notice-check",
            security,
        ),
        Kind::Removed => notice(
            "removed",
            vec![say("removed-body")],
            "notice-check",
            security,
        ),
        Kind::ResetDone { new_vault } => {
            let (vault, recovery) = if *new_vault {
                ("reset-done-new", "reset-done-recovery-new")
            } else {
                ("reset-done-kept", "reset-done-recovery-kept")
            };
            notice(
                "reset-done",
                vec![
                    say("reset-done-body"),
                    say(vault),
                    say("reset-done-sessions"),
                    say(recovery),
                ],
                "reset-done-not-you",
                reset,
            )
        }
        Kind::PasswordChanged => notice(
            "password",
            vec![say("password-body"), say("password-sessions")],
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
            "issued-not-you",
            security,
        ),
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

/// Whether `c` takes two columns of a fixed-width font: the wide East
/// Asian scripts and the full-width forms.
fn wide(c: char) -> bool {
    matches!(
        u32::from(c),
        0x1100..=0x115F
            | 0x2E80..=0x303E
            | 0x3041..=0x33FF
            | 0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0xA000..=0xA4CF
            | 0xAC00..=0xD7A3
            | 0xF900..=0xFAFF
            | 0xFE30..=0xFE4F
            | 0xFF00..=0xFF60
            | 0xFFE0..=0xFFE6
    )
}

/// The line under a title in the plain text: as many `=` as the title
/// takes columns.
fn underline(title: &str) -> String {
    "=".repeat(title.chars().map(|c| if wide(c) { 2 } else { 1 }).sum())
}

/// The plain text, complete without the HTML: the title with a line under
/// it, paragraphs with an empty line between them, the code alone on a
/// line under its label, the link's address alone on a line, and the
/// footer under the signature's mark (`-- `), which readers set apart.
fn text(lang: Lang, content: &Content<'_>) -> Zeroizing<String> {
    let mut blocks = vec![format!("{}\n{}", content.title, underline(&content.title))];
    blocks.extend(content.before.iter().cloned());
    if let Some(code) = &content.code {
        blocks.push(format!("{}:\n{}", lang.text("code-label", &[]), code.value));
        blocks.push(code.notes.join("\n"));
    }
    blocks.extend(content.warning.iter().cloned());
    if let Some((words, url)) = &content.link {
        blocks.push(format!("{words}:\n{url}"));
    }
    blocks.extend(content.after.iter().cloned());
    blocks.push(format!("-- \n{}", lang.text("footer", &[])));
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

/// A paragraph of `words`, `size` pixels high, in `colour`.
fn paragraph(words: &str, size: u8, colour: &str) -> String {
    format!(
        "<p style=\"margin:0;font-family:{SANS};font-size:{size}px;line-height:1.6;color:{colour};\">{}</p>",
        escape(words)
    )
}

/// The band at the card's top: the blue hour with a gold edge under it,
/// and the wordmark in letters, the name's first letter on a gold tile
/// beside the name as the site's header has its picture there.
fn band(out: &mut String, lang: Lang) {
    let brand = lang.text("brand", &[]);
    let letter: String = brand.chars().take(1).collect();
    let _ = writeln!(
        out,
        "<tr><td bgcolor=\"{NIGHT}\" style=\"padding:20px {SIDE}px;background:{NIGHT};border-bottom:3px solid {GOLD};border-radius:13px 13px 0 0;\">\
         <{TABLE}><tr>\
         <td width=\"34\" height=\"34\" align=\"center\" valign=\"middle\" bgcolor=\"{GOLD}\" aria-hidden=\"true\" style=\"width:34px;height:34px;background:{GOLD};border-radius:9px;font-family:{SERIF};font-size:20px;line-height:34px;font-weight:700;color:{NIGHT};mso-line-height-rule:exactly;\">{}</td>\
         <td bgcolor=\"{NIGHT}\" style=\"padding-left:12px;background:{NIGHT};font-family:{SERIF};font-size:24px;line-height:34px;font-weight:700;letter-spacing:0.01em;color:{DAWN};mso-line-height-rule:exactly;\">{}</td>\
         </tr></table></td></tr>",
        escape(&letter),
        escape(&brand),
    );
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

/// The one link, as a button that holds in every reader: the cell carries
/// the colour and Outlook's room (`mso-padding-alt`), the link its own
/// room for the others, and the button is 48 px high.
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

/// The HTML beside the plain text, for a reader that shows it.
fn html(lang: Lang, content: &Content<'_>) -> Zeroizing<String> {
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
            escape(first)
        );
    }
    let _ = write!(
        out,
        "<{TABLE} width=\"100%\" bgcolor=\"{GROUND}\" style=\"background:{GROUND};\">\n\
         <tr><td align=\"center\" style=\"padding:32px 10px;\">\n\
         <!--[if mso]><{TABLE} width=\"{CARD}\" align=\"center\"><tr><td><![endif]-->\n\
         <{TABLE} width=\"100%\" bgcolor=\"{PANEL}\" style=\"max-width:{CARD}px;background:{PANEL};border:1px solid {RULE};border-radius:14px;border-collapse:separate;\">\n",
    );
    band(&mut out, lang);
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
            &paragraph(words, 16, INK),
        );
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
        row(&mut out, 22, &paragraph(words, 15, SOFT));
    }
    // the footer under a thin rule
    row(
        &mut out,
        32,
        &format!(
            "<{TABLE} width=\"100%\"><tr><td style=\"padding:16px 0 26px 0;border-top:1px solid {RULE};font-family:{SANS};font-size:13px;line-height:1.55;color:{SOFT};\">{}</td></tr></table>",
            escape(&lang.text("footer", &[]))
        ),
    );
    out.push_str(
        "</table>\n<!--[if mso]></td></tr></table><![endif]-->\n</td></tr>\n</table>\n</body>\n</html>\n",
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use central_api::{email::MailCode, reset_code::ResetCode};

    const SITE: &str = "https://baylee.example";

    /// Every mail in `lang`, with the codes the first two carry; the
    /// notices follow them.
    fn mails(lang: Lang) -> (Vec<Mail>, String, String) {
        let code = MailCode::generate().unwrap();
        let reset = ResetCode::generate().unwrap();
        let account = [7; 16];
        let to = "player@example.org";
        (
            vec![
                Mail::confirm(to, account, lang.code(), &code, 15),
                Mail::reset(to, account, lang.code(), "player.one", &reset, 60),
                Mail::replaced(to, account, lang.code()),
                Mail::removed(to, account, lang.code()),
                Mail::reset_done(to, account, lang.code(), false),
                Mail::reset_done(to, account, lang.code(), true),
                Mail::password_changed(to, account, lang.code()),
                Mail::reset_issued(to, account, lang.code(), 24),
            ],
            code.display().to_string(),
            reset.display().to_string(),
        )
    }

    /// Every value of the attribute `name` in `html`.
    fn attributes<'a>(html: &'a str, name: &str) -> Vec<&'a str> {
        let open = format!("{name}=\"");
        html.match_indices(&open)
            .map(|(at, _)| {
                let value = &html[at + open.len()..];
                &value[..value.find('"').unwrap()]
            })
            .collect()
    }

    #[test]
    fn every_language_renders_every_mail_within_the_mailers_rules() {
        for lang in Lang::ALL {
            let (mails, ..) = mails(lang);
            for mail in &mails {
                let rendered = render(mail, SITE);
                let subject = &rendered.subject;
                assert!((1..=200).contains(&subject.chars().count()), "{subject}");
                assert!(!subject.chars().any(char::is_control), "{subject}");
                assert!(subject.contains("Baylee"), "{subject}");
                assert!(!rendered.text.trim().is_empty() && rendered.text.len() < 65_536);
                assert!(
                    !rendered.text.chars().any(|c| c.is_control() && c != '\n'),
                    "line breaks are the text's only control characters"
                );
                assert!(!rendered.html.trim().is_empty() && rendered.html.len() < 262_144);
                assert!(rendered.html.contains(&format!(
                    "<html lang=\"{}\" style=\"color-scheme:light only;\">",
                    lang.code()
                )));
                // every message was found: no paragraph is empty
                assert!(!rendered.text.contains("\n\n\n"), "{}", lang.code());
                for empty in ["\"></td>", "\"></p>", "\"></h1>", "\"></a>", "\"></div>"] {
                    assert!(!rendered.html.contains(empty), "{} {empty}", lang.code());
                }
            }
        }
    }

    #[test]
    fn the_html_loads_nothing_and_links_only_to_the_site() {
        for lang in Lang::ALL {
            let (mails, code, reset) = mails(lang);
            for mail in &mails {
                let html = render(mail, SITE).html.to_lowercase();
                for forbidden in [
                    "<img",
                    "<script",
                    "<style",
                    "<link",
                    "<iframe",
                    "<object",
                    "<embed",
                    "<form",
                    "<video",
                    "<audio",
                    "<svg",
                    "<base",
                    "<v:",
                    "src=",
                    "srcset=",
                    "background=",
                    "url(",
                    "@import",
                    "@font-face",
                    "javascript:",
                    "data:",
                    "onload",
                    "onclick",
                    "onerror",
                    "http-equiv",
                ] {
                    assert!(!html.contains(forbidden), "{forbidden}");
                }
                // The only addresses in the whole mail are the site's.
                let links = attributes(&html, "href");
                let expected: &[&str] = match mail.what() {
                    crate::What::Confirm => &[],
                    crate::What::Reset
                    | crate::What::ResetDone { .. }
                    | crate::What::PasswordChanged => &["reset"],
                    crate::What::Replaced | crate::What::Removed | crate::What::ResetIssued => {
                        &["account/security"]
                    }
                };
                let expected: Vec<String> = expected
                    .iter()
                    .map(|path| format!("{SITE}/{}/{path}", lang.code()))
                    .collect();
                assert_eq!(links, expected);
                assert_eq!(
                    html.matches("http://").count() + html.matches("https://").count(),
                    expected.len()
                );
                assert_eq!(html.matches("//").count(), expected.len());
                // No code in any link, in either body.
                let text = render(mail, SITE).text.to_lowercase();
                for secret in [code.to_lowercase(), reset.to_lowercase()] {
                    assert!(links.iter().all(|link| !link.contains(&secret)));
                    assert!(
                        text.lines()
                            .filter(|line| line.contains("://"))
                            .all(|line| !line.contains(&secret))
                    );
                }
                assert_eq!(text.matches("://").count(), expected.len());
            }
        }
    }

    #[test]
    fn a_mail_carries_its_code_once_in_each_body_and_a_notice_none() {
        let (mails, code, reset) = mails(Lang::De);
        let confirm = render(&mails[0], SITE);
        assert_eq!(confirm.text.matches(&code).count(), 1);
        assert_eq!(confirm.html.matches(&code).count(), 1);
        assert!(confirm.html.contains(&format!("font-family:{MONO};font-size:30px;line-height:1.4;font-weight:700;letter-spacing:0.04em;color:{TITLE};\">{code}</td>")));
        assert!(confirm.text.contains("Der Code gilt 15 Minuten."));
        assert!(!confirm.subject.contains(&code));
        let mailed = render(&mails[1], SITE);
        assert_eq!(mailed.text.matches(&reset).count(), 1);
        assert_eq!(mailed.html.matches(&reset).count(), 1);
        // 45 characters on one line of the 560 px card: 15 px and no
        // space between them.
        assert_eq!(reset.len(), 45);
        assert!(mailed.html.contains(&format!("font-family:{MONO};font-size:15px;line-height:1.4;font-weight:700;letter-spacing:0;color:{TITLE};\">{reset}</td>")));
        assert!(mailed.text.contains("player.one"));
        assert!(mailed.text.contains("60 Minuten"));
        assert!(mailed.text.contains("https://baylee.example/de/reset"));
        assert!(!mailed.subject.contains(&reset) && !mailed.subject.contains("player"));
        for (notice, page) in mails[2..].iter().zip([
            "account/security",
            "account/security",
            "reset",
            "reset",
            "reset",
            "account/security",
        ]) {
            let notice = render(notice, SITE);
            assert!(!notice.subject.contains("player"));
            for body in [&notice.text, &notice.html] {
                assert!(!body.contains(&code) && !body.contains(&reset));
                assert!(!body.contains("RESET-"), "a notice carries no code");
                assert!(!body.contains("player"), "a notice names no account");
                assert!(body.contains(&format!("https://baylee.example/de/{page}")));
            }
        }
        // No mail says the address it goes to.
        for mail in &mails {
            let rendered = render(mail, SITE);
            assert!(!rendered.text.contains("example.org"));
            assert!(!rendered.html.contains("example.org"));
        }
    }

    /// The part of `html` between `from` and the next `to` after it.
    fn between<'a>(html: &'a str, from: &str, to: &str) -> &'a str {
        let start = html.find(from).unwrap_or_else(|| panic!("{from}")) + from.len();
        let rest = &html[start..];
        &rest[..rest.find(to).unwrap_or_else(|| panic!("{to}"))]
    }

    /// Every mail wears the band with the wordmark in letters, says it is
    /// light only, has its title as its one heading, a box for its code, a
    /// button for its one link, the warning set apart, and the footer
    /// under a rule, in that order.
    #[test]
    fn every_mail_has_the_band_the_heading_one_button_and_the_footer() {
        for lang in Lang::ALL {
            let (mails, code, reset) = mails(lang);
            for mail in &mails {
                let content = content(mail, SITE);
                let html = render(mail, SITE).html;
                let html = html.as_str();
                for light in [
                    "<meta name=\"color-scheme\" content=\"light only\">",
                    "<meta name=\"supported-color-schemes\" content=\"light only\">",
                ] {
                    assert_eq!(html.matches(light).count(), 1, "{light}");
                }
                // the tables close as they open
                for (open, close) in [("<table ", "</table>"), ("<tr>", "</tr>"), ("<td", "</td>")]
                {
                    assert_eq!(
                        html.matches(open).count(),
                        html.matches(close).count(),
                        "{open}"
                    );
                }
                // the band: the blue hour, a gold edge, the name's letter
                // on a gold tile (not read aloud) and the name
                let band = between(
                    html,
                    &format!(
                        "<td bgcolor=\"{NIGHT}\" style=\"padding:20px {SIDE}px;background:{NIGHT};border-bottom:3px solid {GOLD};"
                    ),
                    "</td></tr>\n",
                );
                assert!(band.contains(&format!("bgcolor=\"{GOLD}\" aria-hidden=\"true\"")));
                assert!(band.contains(&format!(
                    "color:{NIGHT};mso-line-height-rule:exactly;\">B</td>"
                )));
                assert!(band.contains(&format!(
                    "color:{DAWN};mso-line-height-rule:exactly;\">Baylee</td>"
                )));
                // one heading, the title
                assert_eq!(html.matches("<h1 ").count(), 1);
                let title = between(html, "<h1 ", "</h1>");
                assert!(title.ends_with(&format!("color:{TITLE};\">{}", escape(&content.title))));
                // the first paragraph again where a list of mails shows it, never a code
                let preview = between(html, "<div style=\"display:none;", "</div>");
                assert!(preview.ends_with(&escape(&content.before[0])));
                assert!(!preview.contains(&code) && !preview.contains(&reset));
                // the button: a cell in the accent with Outlook's room, and
                // in it the one link, a block with room of its own
                let links = html.matches("<a ").count();
                assert_eq!(links, usize::from(content.link.is_some()));
                if let Some((words, url)) = &content.link {
                    let cell = between(
                        html,
                        &format!(
                            "<td align=\"center\" bgcolor=\"{ACCENT}\" style=\"background:{ACCENT};border-radius:8px;mso-padding-alt:14px 26px;\">"
                        ),
                        "</td>",
                    );
                    assert_eq!(
                        cell,
                        format!(
                            "<a href=\"{}\" style=\"display:inline-block;padding:14px 26px;font-family:{SANS};font-size:16px;line-height:20px;font-weight:700;color:{ON_ACCENT};text-decoration:none;border-radius:8px;mso-padding-alt:0;\">{}</a>",
                            escape(url),
                            escape(words)
                        )
                    );
                }
                // the code in its own box, framed in the accent, under its label
                let boxes = html.matches(&format!("border:2px dashed {ACCENT}")).count();
                assert_eq!(boxes, usize::from(mail.code().is_some()));
                if let Some(code) = mail.code() {
                    let label = escape(&lang.text("code-label", &[]));
                    assert!(html.contains(&format!("color:{SOFT};\">{label}</td></tr>")));
                    let framed = between(html, &format!("bgcolor=\"{TINT}\""), "</td>");
                    assert!(framed.ends_with(&format!("\">{code}")));
                    assert!(framed.contains("user-select:all"));
                }
                // what to do if this was not you, on the warning's ground
                let warned = html
                    .matches(&format!("border-left:4px solid {WARN}"))
                    .count();
                assert_eq!(warned, usize::from(content.warning.is_some()));
                if let Some(words) = &content.warning {
                    assert!(html.contains(&format!("color:{INK};\">{}</td>", escape(words))));
                }
                // the footer, last, under a thin rule
                let footer = escape(&lang.text("footer", &[]));
                assert!(html.contains(&format!("border-top:1px solid {RULE};font-family:{SANS};font-size:13px;line-height:1.55;color:{SOFT};\">{footer}</td>")));
                // in the order the text has them too
                let mut at = 0;
                let mut order = vec![format!("bgcolor=\"{NIGHT}\""), "<h1 ".to_owned()];
                order.extend(content.before.iter().map(|words| escape(words)));
                if mail.code().is_some() {
                    order.push(format!("bgcolor=\"{TINT}\""));
                }
                order.extend(content.warning.iter().map(|words| escape(words)));
                if content.link.is_some() {
                    order.push("<a ".to_owned());
                }
                order.extend(content.after.iter().map(|words| escape(words)));
                order.push(footer);
                for piece in &order {
                    let found = html[at..]
                        .find(piece.as_str())
                        .unwrap_or_else(|| panic!("{} {piece}", lang.code()));
                    at += found + piece.len();
                }
            }
        }
    }

    /// The plain text is the whole mail on its own: the title with a line
    /// under it as wide, every paragraph, the code alone on its line, the
    /// link's address alone on its line, and the footer last, under the
    /// signature's mark.
    #[test]
    fn the_plain_text_stands_on_its_own() {
        for lang in Lang::ALL {
            let (mails, ..) = mails(lang);
            for mail in &mails {
                let content = content(mail, SITE);
                let text = render(mail, SITE).text;
                let lines: Vec<&str> = text.lines().collect();
                assert_eq!(lines[0], content.title);
                assert_eq!(lines[1], underline(&content.title));
                assert!(!lines[1].is_empty() && lines[1].chars().all(|c| c == '='));
                assert_eq!(lines[2], "");
                if let Some(code) = mail.code() {
                    let at = lines
                        .iter()
                        .position(|line| *line == code)
                        .expect("the code alone on its line");
                    assert_eq!(lines[at - 1], format!("{}:", lang.text("code-label", &[])));
                    assert_eq!(lines[at + 1], "");
                }
                if let Some((words, url)) = &content.link {
                    let at = lines
                        .iter()
                        .position(|line| line == url)
                        .expect("the address alone on its line");
                    assert_eq!(lines[at - 1], format!("{words}:"));
                }
                for words in content
                    .before
                    .iter()
                    .chain(&content.warning)
                    .chain(&content.after)
                {
                    assert!(lines.contains(&words.as_str()), "{words}");
                }
                for note in content.code.iter().flat_map(|code| &code.notes) {
                    assert!(lines.contains(&note.as_str()), "{note}");
                }
                let mark = lines
                    .iter()
                    .position(|line| *line == "-- ")
                    .expect("the signature's mark");
                assert_eq!(&lines[mark + 1..], [lang.text("footer", &[]).as_str()]);
            }
        }
        // a character of the wide scripts takes two columns
        assert_eq!(underline("Set"), "===");
        assert_eq!(underline("コード"), "======");
        assert_eq!(underline("設定 a"), "======");
    }

    /// WCAG's relative luminance of a colour written `#rrggbb`.
    fn luminance(hex: &str) -> f64 {
        let channel = |at: usize| {
            let value = f64::from(u8::from_str_radix(&hex[at..at + 2], 16).unwrap()) / 255.0;
            if value <= 0.040_45 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(1) + 0.7152 * channel(3) + 0.0722 * channel(5)
    }

    fn contrast(a: &str, b: &str) -> f64 {
        let (a, b) = (luminance(a), luminance(b));
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }

    /// A colour as a reader that inverts the mail draws it.
    fn inverted(hex: &str) -> String {
        let channel = |at: usize| 255 - u8::from_str_radix(&hex[at..at + 2], 16).unwrap();
        format!("#{:02x}{:02x}{:02x}", channel(1), channel(3), channel(5))
    }

    /// Every text the mail draws keeps WCAG's 4.5:1 against its ground,
    /// and so does it where a reader inverts the colours; a reader that
    /// darkens only light grounds and lightens only dark text turns the
    /// light pairs into dark ones the same way, and leaves the band and
    /// the button, which are dark under light words, as they are. The
    /// frames and edges keep 3:1 against what they stand on.
    #[test]
    fn every_pair_the_mail_draws_is_readable_as_drawn_and_inverted() {
        let words = [
            (DAWN, NIGHT),
            (NIGHT, GOLD),
            (TITLE, PANEL),
            (INK, PANEL),
            (SOFT, PANEL),
            (TITLE, TINT),
            (SOFT, TINT),
            (INK, WARN_TINT),
            (ON_ACCENT, ACCENT),
        ];
        for (ink, ground) in words {
            assert!(contrast(ink, ground) >= 4.5, "{ink} on {ground}");
            assert!(
                contrast(&inverted(ink), &inverted(ground)) >= 4.5,
                "{ink} on {ground}, inverted"
            );
        }
        for (edge, ground) in [
            (ACCENT, TINT),
            (WARN, WARN_TINT),
            (ACCENT, PANEL),
            (GOLD, NIGHT),
        ] {
            assert!(contrast(edge, ground) >= 3.0, "{edge} on {ground}");
        }
        assert_eq!(inverted("#0f1b2d"), "#f0e4d2");
        assert!((contrast("#000000", "#ffffff") - 21.0).abs() < 1e-9);
    }

    #[test]
    fn the_notices_of_a_changed_login_say_what_changed() {
        let (mails, ..) = mails(Lang::De);
        let [kept, new, password, issued] = [4, 5, 6, 7].map(|at| render(&mails[at], SITE));
        // A reset says whether the data stayed and what the recovery code
        // still does; the two say it differently.
        assert!(kept.text.contains("Die Daten des Accounts sind geblieben."));
        assert!(
            kept.text
                .contains("Wiederherstellungscode, öffnet er die Daten weiterhin")
        );
        assert!(new.text.contains("leeren Tresor"));
        assert!(new.text.contains("öffnet nichts mehr"));
        assert!(!new.text.contains("sind geblieben") && !kept.text.contains("Tresor"));
        for reset in [&kept, &new] {
            assert!(reset.text.contains("überall abgemeldet"));
            assert!(reset.text.contains("Administrator"));
            assert_eq!(reset.subject, kept.subject);
        }
        // A changed password: every other session was signed out.
        assert!(password.text.contains("Alle anderen Sitzungen"));
        // An administrator's code: how long it lasts, that the password
        // still works, and what ends it.
        assert!(issued.text.contains("24 Stunden"));
        assert!(issued.text.contains("gilt dein Passwort wie bisher"));
        assert!(issued.text.contains("ändere dein Passwort"));
        let english = render(
            &Mail::reset_issued("player@example.org", [7; 16], "en", 24),
            SITE,
        );
        assert!(english.text.contains("for 24 hours"));
        // Each says so in its own subject.
        let subjects = [&kept, &password, &issued].map(|mail| mail.subject.clone());
        assert!(subjects[0] != subjects[1] && subjects[1] != subjects[2]);
    }

    #[test]
    fn what_is_written_into_the_html_is_escaped() {
        assert_eq!(
            escape("<a href=\"x\">O'Brien & co</a>"),
            "&lt;a href=&quot;x&quot;&gt;O&#39;Brien &amp; co&lt;/a&gt;"
        );
        // A name is the only text a person chose. `Mail::reset` leaves it
        // nothing to escape; the writer escapes it all the same, in the
        // paragraph and in the preview beside the subject.
        let reset = ResetCode::generate().unwrap();
        let mut mail = Mail::reset("player@example.org", [7; 16], "en", "player", &reset, 60);
        let Kind::Reset { name, .. } = &mut mail.kind else {
            panic!("a reset mail");
        };
        *name = "<img src=x onerror=alert(1)>&\"'".into();
        let html = render(&mail, SITE).html;
        assert!(!html.contains("<img"));
        assert_eq!(
            html.matches("&lt;img src=x onerror=alert(1)&gt;&amp;&quot;&#39;")
                .count(),
            2
        );
        // The German words hold an apostrophe-free text, the English ones
        // none either; an origin with a quote would be escaped too.
        let odd = render(&mail, "https://baylee.example\"><b>");
        assert!(!odd.html.contains("\"><b>"));
    }

    /// Writes every mail in English and German, as HTML and as text, into
    /// the folder `BAYLEE_MAIL_SHOTS` names, to be looked at in a browser
    /// (`docs/email.md`, "The mails"). Its codes are fresh random ones and
    /// its address and site are reserved names.
    #[test]
    #[ignore = "writes files: BAYLEE_MAIL_SHOTS=<folder> cargo test -p central-mail -- --ignored"]
    fn every_mail_written_out_for_a_look() {
        let folder = std::path::PathBuf::from(
            std::env::var_os("BAYLEE_MAIL_SHOTS").expect("BAYLEE_MAIL_SHOTS names a folder"),
        );
        std::fs::create_dir_all(&folder).unwrap();
        let names = [
            "confirm",
            "reset",
            "replaced",
            "removed",
            "reset-done-kept",
            "reset-done-new",
            "password-changed",
            "reset-issued",
        ];
        for lang in [Lang::En, Lang::De] {
            let (mails, ..) = mails(lang);
            for (mail, name) in mails.iter().zip(names) {
                let rendered = render(mail, SITE);
                let stem = folder.join(format!("{}-{name}", lang.code()));
                std::fs::write(stem.with_extension("html"), rendered.html.as_bytes()).unwrap();
                std::fs::write(stem.with_extension("txt"), rendered.text.as_bytes()).unwrap();
            }
        }
    }
}
