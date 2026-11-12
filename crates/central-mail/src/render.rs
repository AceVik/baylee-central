//! The mails as plain text and as HTML (`docs/email.md`). Both are written
//! from the same content. The HTML is a table with inline styles and system
//! fonts on the website's light ground: no image, no remote content, no
//! script, nothing that tracks. Its only link goes to the configured site,
//! and no link ever holds a code. Everything written into the HTML is
//! escaped, the crate's own words included.
use crate::{Kind, Mail, locale::Lang};
use std::fmt::Write as _;
use zeroize::Zeroizing;

// The website's light scheme (`crates/central-web/style/tokens.css`).
const GROUND: &str = "#f2efe4";
const PANEL: &str = "#fffefa";
const FIELD: &str = "#e8ebe1";
const INK: &str = "#233e43";
const SOFT: &str = "#415c5e";
const TITLE: &str = "#173c43";
const ACCENT: &str = "#235b66";
const LINK: &str = "#76511c";
const SANS: &str = "system-ui,-apple-system,'Segoe UI',Roboto,sans-serif";
const MONO: &str = "ui-monospace,'SF Mono',Menlo,Consolas,'Liberation Mono',monospace";

/// A mail as the mailer takes it. No Debug: the text holds a code. The
/// bodies are wiped when dropped.
pub(crate) struct Rendered {
    pub(crate) subject: String,
    pub(crate) text: Zeroizing<String>,
    pub(crate) html: Zeroizing<String>,
}

/// What a mail says, before it is written as text or as HTML.
struct Content<'a> {
    subject: String,
    title: String,
    /// Paragraphs above the code.
    before: Vec<String>,
    /// The code, the size it is set in and the space between its
    /// characters.
    code: Option<(&'a str, u8, &'static str)>,
    /// Paragraphs below the code.
    after: Vec<String>,
    /// A link's words and where it goes: a page of the site.
    link: Option<(String, String)>,
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
    match &mail.kind {
        Kind::Confirm { code, minutes } => Content {
            subject: say("confirm-subject"),
            title: say("confirm-title"),
            before: vec![say("confirm-intro")],
            code: Some((code.as_str(), 30, "0.04em")),
            after: vec![
                lang.text("confirm-valid", &[("minutes", &minutes.to_string())]),
                say("code-secret"),
                say("confirm-ignore"),
            ],
            link: None,
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
            // space are about 406 px, less than the 438 px inside the
            // 520 px card: one line on a wide screen, and on a narrow one
            // it breaks at its hyphens.
            code: Some((code.as_str(), 15, "0")),
            after: vec![
                lang.text("reset-valid", &[("minutes", &minutes.to_string())]),
                say("code-secret"),
                say("reset-ignore"),
            ],
            link: Some((say("reset-link"), page("reset"))),
        },
        Kind::Replaced | Kind::Removed => {
            let which = if matches!(mail.kind, Kind::Replaced) {
                "replaced"
            } else {
                "removed"
            };
            Content {
                subject: say(&format!("{which}-subject")),
                title: say(&format!("{which}-title")),
                before: vec![say(&format!("{which}-body")), say("notice-check")],
                code: None,
                after: Vec::new(),
                link: Some((say("notice-link"), page("account/security"))),
            }
        }
        Kind::ResetDone { new_vault } => {
            let (vault, recovery) = if *new_vault {
                ("reset-done-new", "reset-done-recovery-new")
            } else {
                ("reset-done-kept", "reset-done-recovery-kept")
            };
            Content {
                subject: say("reset-done-subject"),
                title: say("reset-done-title"),
                before: vec![
                    say("reset-done-body"),
                    say(vault),
                    say("reset-done-sessions"),
                    say("reset-done-not-you"),
                    say(recovery),
                ],
                code: None,
                after: Vec::new(),
                link: Some((say("reset-link"), page("reset"))),
            }
        }
        Kind::PasswordChanged => Content {
            subject: say("password-subject"),
            title: say("password-title"),
            before: vec![
                say("password-body"),
                say("password-sessions"),
                say("password-not-you"),
            ],
            code: None,
            after: Vec::new(),
            link: Some((say("reset-link"), page("reset"))),
        },
        Kind::ResetIssued { hours } => Content {
            subject: say("issued-subject"),
            title: say("issued-title"),
            before: vec![
                say("issued-body"),
                lang.text("issued-valid", &[("hours", &hours.to_string())]),
                say("issued-not-you"),
            ],
            code: None,
            after: Vec::new(),
            // where the password is changed, which ends the code
            link: Some((say("notice-link"), page("account/security"))),
        },
    }
}

/// The plain text: paragraphs with an empty line between them.
fn text(lang: Lang, content: &Content<'_>) -> Zeroizing<String> {
    let mut blocks = vec![content.title.clone()];
    blocks.extend(content.before.iter().cloned());
    if let Some((code, ..)) = content.code {
        blocks.push(format!("{}: {code}", lang.text("code-label", &[])));
    }
    blocks.extend(content.after.iter().cloned());
    if let Some((words, url)) = &content.link {
        blocks.push(format!("{words}: {url}"));
    }
    blocks.push(lang.text("footer", &[]));
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

/// One row of the mail's table: `inner` is HTML already.
fn row(out: &mut String, padding: &str, style: &str, inner: &str) {
    let _ = writeln!(
        out,
        "<tr><td style=\"padding:{padding};font-family:{SANS};{style}\">{inner}</td></tr>"
    );
}

/// The HTML beside the plain text, for a reader that shows it.
fn html(lang: Lang, content: &Content<'_>) -> Zeroizing<String> {
    let mut out = Zeroizing::new(String::with_capacity(4096));
    let table = "table role=\"presentation\" width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\"";
    let _ = write!(
        out,
        "<!doctype html>\n<html lang=\"{}\">\n<head>\n<meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <meta name=\"color-scheme\" content=\"light only\">\n<title>{}</title>\n</head>\n\
         <body style=\"margin:0;padding:0;background:{GROUND};\">\n\
         <{table} style=\"background:{GROUND};\">\n\
         <tr><td align=\"center\" style=\"padding:24px 12px;\">\n\
         <{table} style=\"max-width:520px;background:{PANEL};border:1px solid {FIELD};border-radius:8px;\">\n",
        lang.code(),
        escape(&content.subject),
    );
    row(
        &mut out,
        "24px 28px 6px 28px",
        &format!(
            "font-size:13px;font-weight:700;letter-spacing:0.12em;text-transform:uppercase;color:{ACCENT};"
        ),
        &escape(&lang.text("brand", &[])),
    );
    row(
        &mut out,
        "0 28px",
        &format!("font-size:22px;line-height:1.3;font-weight:700;color:{TITLE};"),
        &escape(&content.title),
    );
    let paragraph = format!("font-size:16px;line-height:1.5;color:{INK};");
    for words in &content.before {
        row(&mut out, "12px 28px 0 28px", &paragraph, &escape(words));
    }
    if let Some((code, size, spacing)) = content.code {
        row(
            &mut out,
            "20px 28px 6px 28px",
            &format!("font-size:13px;line-height:1.4;color:{SOFT};"),
            &escape(&lang.text("code-label", &[])),
        );
        let _ = writeln!(
            out,
            "<tr><td style=\"padding:0 28px;\"><{table}><tr><td align=\"center\" \
             style=\"padding:16px 12px;background:{FIELD};border:1px solid {ACCENT};\
             border-radius:6px;font-family:{MONO};font-size:{size}px;line-height:1.4;\
             font-weight:700;letter-spacing:{spacing};color:{TITLE};\">{}</td></tr></table></td></tr>",
            escape(code)
        );
    }
    for words in &content.after {
        row(&mut out, "12px 28px 0 28px", &paragraph, &escape(words));
    }
    if let Some((words, url)) = &content.link {
        let link = format!(
            "<a href=\"{}\" style=\"color:{LINK};text-decoration:underline;\">{}</a>",
            escape(url),
            escape(words)
        );
        row(&mut out, "16px 28px 0 28px", &paragraph, &link);
    }
    row(
        &mut out,
        "24px 28px 24px 28px",
        &format!("font-size:13px;line-height:1.5;color:{SOFT};"),
        &escape(&lang.text("footer", &[])),
    );
    out.push_str("</table>\n</td></tr>\n</table>\n</body>\n</html>\n");
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
                assert!(
                    rendered
                        .html
                        .contains(&format!("<html lang=\"{}\">", lang.code()))
                );
                // every message was found: no paragraph is empty
                assert!(!rendered.text.contains("\n\n\n"), "{}", lang.code());
                assert!(!rendered.html.contains("\"></td>"), "{}", lang.code());
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
        assert!(confirm.html.contains(&format!("font-size:30px;line-height:1.4;font-weight:700;letter-spacing:0.04em;color:{TITLE};\">{code}</td>")));
        assert!(confirm.html.contains(MONO));
        assert!(confirm.text.contains("Der Code gilt 15 Minuten."));
        assert!(!confirm.subject.contains(&code));
        let mailed = render(&mails[1], SITE);
        assert_eq!(mailed.text.matches(&reset).count(), 1);
        assert_eq!(mailed.html.matches(&reset).count(), 1);
        // 45 characters on one line of the 520 px card: 15 px and no
        // space between them.
        assert_eq!(reset.len(), 45);
        assert!(mailed.html.contains(&format!("font-size:15px;line-height:1.4;font-weight:700;letter-spacing:0;color:{TITLE};\">{reset}</td>")));
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
        // nothing to escape; the writer escapes it all the same.
        let reset = ResetCode::generate().unwrap();
        let mut mail = Mail::reset("player@example.org", [7; 16], "en", "player", &reset, 60);
        let Kind::Reset { name, .. } = &mut mail.kind else {
            panic!("a reset mail");
        };
        *name = "<img src=x onerror=alert(1)>&\"'".into();
        let html = render(&mail, SITE).html;
        assert!(!html.contains("<img"));
        assert!(html.contains("&lt;img src=x onerror=alert(1)&gt;&amp;&quot;&#39;"));
        // The German words hold an apostrophe-free text, the English ones
        // none either; an origin with a quote would be escaped too.
        let odd = render(&mail, "https://baylee.example\"><b>");
        assert!(!odd.html.contains("\"><b>"));
    }
}
