//! The mails as written: within the mailer's rules, loading nothing,
//! linking to the site only, readable as drawn and inverted, and the plain
//! text complete on its own. Addresses, names and sites are synthetic.
use super::*;
use central_api::{email::MailCode, reset_code::ResetCode};

const SITE: &str = "https://baylee.example";

/// Every mail in `lang`, with the codes the first two carry; the notices
/// follow them.
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

/// The addresses a mail in `lang` links to, in their order: its button's
/// page, if it has one, and the site's start page in the footer.
fn expected_links(mail: &Mail, lang: Lang) -> Vec<String> {
    let button = match mail.what() {
        crate::What::Confirm => None,
        crate::What::Reset | crate::What::ResetDone { .. } | crate::What::PasswordChanged => {
            Some("reset")
        }
        crate::What::Replaced | crate::What::Removed | crate::What::ResetIssued => {
            Some("account/security")
        }
    };
    button
        .into_iter()
        .chain([""])
        .map(|path| format!("{SITE}/{}/{path}", lang.code()))
        .collect()
}

#[test]
fn every_language_renders_every_mail_within_the_mailers_rules() {
    for lang in Lang::ALL {
        let (mails, ..) = mails(lang);
        for mail in &mails {
            let rendered = render(mail, SITE, false);
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
            for empty in [
                "\"></td>",
                "\"></p>",
                "\"></h1>",
                "\"></a>",
                "\"></div>",
                "\"></strong>",
            ] {
                assert!(!rendered.html.contains(empty), "{} {empty}", lang.code());
            }
        }
    }
}

/// `html` has no image, no script, no style sheet and nothing else that
/// loads or runs.
fn loads_nothing(html: &str) {
    let html = html.to_lowercase();
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
        "cid:",
        "onload",
        "onclick",
        "onerror",
        "http-equiv",
    ] {
        assert!(!html.contains(forbidden), "{forbidden}");
    }
}

/// The logo as the band shows it: the mailer's attachment, by its
/// `Content-ID`, 48 px, rounded, without words of its own.
const LOGO_IMG: &str = "<img src=\"cid:baylee-logo\" width=\"48\" height=\"48\" alt=\"\" style=\"display:block;width:48px;height:48px;border:0;border-radius:12px;\">";

#[test]
fn the_html_loads_nothing_and_links_only_to_the_site() {
    for lang in Lang::ALL {
        let (mails, code, reset) = mails(lang);
        for mail in &mails {
            let html = render(mail, SITE, false).html.to_lowercase();
            loads_nothing(&html);
            // The only addresses in the whole mail are the site's: the
            // button's page and the start page.
            let links = attributes(&html, "href");
            let expected = expected_links(mail, lang);
            assert_eq!(links, expected);
            assert_eq!(
                html.matches("http://").count() + html.matches("https://").count(),
                expected.len()
            );
            assert_eq!(html.matches("//").count(), expected.len());
            // No code in any link, in either body.
            let text = render(mail, SITE, false).text.to_lowercase();
            for secret in [code.to_lowercase(), reset.to_lowercase()] {
                assert!(links.iter().all(|link| !link.contains(&secret)));
                assert!(
                    text.lines()
                        .filter(|line| line.contains("://"))
                        .all(|line| !line.contains(&secret))
                );
            }
            assert_eq!(text.matches("://").count(), expected.len());
            for url in &expected {
                assert!(text.lines().any(|line| line == url), "{url}");
            }
        }
    }
}

#[test]
fn a_mail_carries_its_code_once_in_each_body_and_a_notice_none() {
    let (mails, code, reset) = mails(Lang::De);
    let confirm = render(&mails[0], SITE, false);
    assert_eq!(confirm.text.matches(&code).count(), 1);
    assert_eq!(confirm.html.matches(&code).count(), 1);
    assert!(confirm.html.contains(&format!("font-family:{MONO};font-size:30px;line-height:1.4;font-weight:700;letter-spacing:0.04em;color:{TITLE};\">{code}</td>")));
    assert!(confirm.text.contains("Der Code gilt 15 Minuten."));
    assert!(!confirm.subject.contains(&code));
    let mailed = render(&mails[1], SITE, false);
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
        let notice = render(notice, SITE, false);
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
        let rendered = render(mail, SITE, false);
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
/// light only, has its title as its one heading, its facts as a list, a
/// box for its code, a button for its page, the warning set apart, and
/// the footer under a rule with the start page's link, in that order.
#[test]
fn every_mail_has_the_band_the_heading_one_button_and_the_footer() {
    for lang in Lang::ALL {
        let (mails, code, reset) = mails(lang);
        for mail in &mails {
            let content = content(mail, SITE);
            let start = Start::of(SITE, lang);
            let html = render(mail, SITE, false).html;
            let html = html.as_str();
            for light in [
                "<meta name=\"color-scheme\" content=\"light only\">",
                "<meta name=\"supported-color-schemes\" content=\"light only\">",
            ] {
                assert_eq!(html.matches(light).count(), 1, "{light}");
            }
            // the tables close as they open
            for (open, close) in [("<table ", "</table>"), ("<tr>", "</tr>"), ("<td", "</td>")] {
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
            assert!(preview.ends_with(&escape(&content.before[0].text())));
            assert!(!preview.contains(&code) && !preview.contains(&reset));
            assert!(!preview.contains("<strong"));
            // the button: a cell in the accent with Outlook's room, and
            // in it the link to the page, a block with room of its own;
            // the footer's link is the only other one
            let links = html.matches("<a ").count();
            assert_eq!(links, usize::from(content.link.is_some()) + 1);
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
            // the facts, each behind a mark that is not read aloud
            let marks = html
                .matches("aria-hidden=\"true\" style=\"width:22px;padding:0 0 ")
                .count();
            assert_eq!(marks, content.facts.len());
            assert_eq!(html.matches(MARK).count(), content.facts.len());
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
            // the footer, last, under a thin rule, and the start page's
            // link under it, named by the site's host
            let footer = escape(&lang.text("footer", &[]));
            assert!(html.contains(&format!("border-top:1px solid {RULE};font-family:{SANS};font-size:13px;line-height:1.55;color:{SOFT};\">{footer}</td>")));
            let home = format!(
                "<a href=\"{}/{}/\" style=\"color:{ACCENT};text-decoration:underline;\">baylee.example</a>",
                SITE,
                lang.code()
            );
            assert_eq!(html.matches(&home).count(), 1, "{home}");
            assert_eq!(start.host, "baylee.example");
            in_order(html, &content, footer, home);
        }
    }
}

/// `html` has the parts of `content` in the order the text has them too,
/// and `footer` and `home` last.
fn in_order(html: &str, content: &Content<'_>, footer: String, home: String) {
    let mut order = vec![format!("bgcolor=\"{NIGHT}\""), "<h1 ".to_owned()];
    order.extend(content.before.iter().map(Paragraph::html));
    order.extend(content.facts.iter().map(|fact| escape(fact)));
    if content.code.is_some() {
        order.push(format!("bgcolor=\"{TINT}\""));
    }
    order.extend(content.warning.iter().map(|words| escape(words)));
    if content.link.is_some() {
        order.push("<a ".to_owned());
    }
    order.extend(content.after.iter().map(|words| escape(words)));
    order.push(footer);
    order.push(home);
    let mut at = 0;
    for piece in &order {
        let found = html[at..]
            .find(piece.as_str())
            .unwrap_or_else(|| panic!("{piece}"));
        at += found + piece.len();
    }
}

/// With the logo, the band shows the mailer's attachment beside the
/// wordmark instead of the letter's tile: the one image of the mail,
/// named by its `Content-ID` and loaded from nowhere. Everything else is
/// the mail without it, and the plain text is the same.
#[test]
fn with_the_logo_the_band_shows_it_and_nothing_else_changes() {
    let start = format!(
        "<td bgcolor=\"{NIGHT}\" style=\"padding:20px {SIDE}px;background:{NIGHT};border-bottom:3px solid {GOLD};"
    );
    for lang in Lang::ALL {
        let (mails, ..) = mails(lang);
        for mail in &mails {
            let (off, on) = (render(mail, SITE, false), render(mail, SITE, true));
            assert_eq!(on.html.matches("<img").count(), 1);
            assert_eq!(on.html.matches("src=").count(), 1);
            assert_eq!(on.html.matches(LOGO_IMG).count(), 1);
            loads_nothing(&on.html.replace(LOGO_IMG, ""));
            let band = between(&on.html, &start, "</td></tr>\n");
            assert!(band.contains(&format!(
                "<td width=\"48\" height=\"48\" valign=\"middle\" style=\"width:48px;height:48px;\">{LOGO_IMG}</td>"
            )));
            // the name beside it, as high as the logo; no letter's tile
            assert!(band.ends_with(&format!(
                "line-height:48px;font-weight:700;letter-spacing:0.01em;color:{DAWN};mso-line-height-rule:exactly;\">Baylee</td></tr></table>"
            )));
            assert!(band.find(LOGO_IMG) < band.find(">Baylee<"));
            assert!(!band.contains(&format!("bgcolor=\"{GOLD}\"")));
            assert!(!band.contains("aria-hidden"));
            // the rest is the mail without the logo
            let plain = between(&off.html, &start, "</td></tr>\n");
            assert_eq!(on.html.replace(band, ""), off.html.replace(plain, ""));
            assert_eq!(on.text, off.text);
            assert_eq!(on.subject, off.subject);
        }
    }
}

/// The plain text is the whole mail on its own: the title, an empty line
/// and the first paragraph, so that a list of mails that shows the text's
/// beginning shows the title and the first sentence; every paragraph, the
/// facts a line each, the code alone on its line, the link's address alone
/// on its line, and the footer and the start page last, under the
/// signature's mark.
#[test]
fn the_plain_text_stands_on_its_own() {
    for lang in Lang::ALL {
        let (mails, ..) = mails(lang);
        for mail in &mails {
            let content = content(mail, SITE);
            let text = render(mail, SITE, false).text;
            let lines: Vec<&str> = text.lines().collect();
            assert_eq!(lines[0], content.title);
            assert_eq!(lines[1], "");
            assert_eq!(lines[2], content.before[0].text());
            // no line under the title, nor anywhere
            assert!(
                !lines
                    .iter()
                    .any(|line| !line.is_empty() && line.chars().all(|c| c == '=')),
                "{}",
                lines[1]
            );
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
                .map(Paragraph::text)
                .chain(content.warning.iter().cloned())
                .chain(content.after.iter().cloned())
            {
                assert!(lines.contains(&words.as_str()), "{words}");
            }
            // the facts together, a line each, with an empty line around them
            if let Some(first) = content.facts.first() {
                let at = lines
                    .iter()
                    .position(|line| *line == format!("- {first}"))
                    .expect("the first fact");
                assert_eq!(lines[at - 1], "");
                for (offset, fact) in content.facts.iter().enumerate() {
                    assert_eq!(lines[at + offset], format!("- {fact}"));
                }
                assert_eq!(lines[at + content.facts.len()], "");
            }
            for note in content.code.iter().flat_map(|code| &code.notes) {
                assert!(lines.contains(&note.as_str()), "{note}");
            }
            let mark = lines
                .iter()
                .position(|line| *line == "-- ")
                .expect("the signature's mark");
            assert_eq!(
                &lines[mark + 1..],
                [
                    lang.text("footer", &[]).as_str(),
                    format!("{SITE}/{}/", lang.code()).as_str()
                ]
            );
        }
    }
}

/// The reset mail names the account it is for, and the HTML sets that
/// name in bold so that the reader sees at once which account it is; the
/// plain text and the preview say it plainly. No other mail names one.
#[test]
fn the_reset_mail_sets_the_account_s_name_in_bold() {
    for lang in Lang::ALL {
        let (mails, ..) = mails(lang);
        let rendered = render(&mails[1], SITE, false);
        let strong =
            format!("<strong style=\"font-weight:700;color:{TITLE};\">player.one</strong>");
        assert_eq!(rendered.html.matches(&strong).count(), 1, "{}", lang.code());
        assert_eq!(rendered.html.matches("<strong").count(), 1);
        assert_eq!(rendered.html.matches("player.one").count(), 2);
        let said = lang.text("reset-intro", &[("name", "player.one")]);
        assert!(rendered.text.lines().any(|line| line == said), "{said}");
        assert!(!rendered.text.contains('*') && !rendered.text.contains('<'));
        for (at, mail) in mails.iter().enumerate() {
            if at != 1 {
                assert!(!render(mail, SITE, false).html.contains("<strong"));
            }
        }
    }
}

/// The notices of a finished reset have the most to say: what happened,
/// then what came of it as three short facts, a line each behind a mark,
/// before what to do if this was not you and the button.
#[test]
fn a_finished_reset_lists_what_came_of_it() {
    for lang in Lang::ALL {
        let (mails, ..) = mails(lang);
        for (at, vault, recovery) in [
            (4, "reset-done-kept", "reset-done-recovery-kept"),
            (5, "reset-done-new", "reset-done-recovery-new"),
        ] {
            let content = content(&mails[at], SITE);
            assert_eq!(content.before.len(), 1);
            assert_eq!(content.before[0].text(), lang.text("reset-done-body", &[]));
            assert_eq!(
                content.facts,
                [vault, "reset-done-sessions", recovery].map(|id| lang.text(id, &[]))
            );
            let html = render(&mails[at], SITE, false).html;
            for fact in &content.facts {
                assert!(html.contains(&format!(
                    "font-size:16px;line-height:26px;color:{INK};mso-line-height-rule:exactly;\">{}</td></tr>",
                    escape(fact)
                )));
            }
            assert_eq!(
                html.matches(&format!(
                    "color:{ACCENT};mso-line-height-rule:exactly;\">{MARK}</td>"
                ))
                .count(),
                3
            );
            // the warning and the button stay, after the facts
            assert!(content.warning.is_some() && content.link.is_some());
        }
        // the other mails say theirs in paragraphs
        for (at, mail) in mails.iter().enumerate() {
            if at != 4 && at != 5 {
                assert_eq!(content(mail, SITE).facts, Vec::<String>::new());
            }
        }
    }
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
        // the list's marks and the footer's link
        (ACCENT, PANEL),
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
    let [kept, new, password, issued] = [4, 5, 6, 7].map(|at| render(&mails[at], SITE, false));
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
        false,
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
    // paragraph (in bold) and in the preview beside the subject.
    let reset = ResetCode::generate().unwrap();
    let mut mail = Mail::reset("player@example.org", [7; 16], "en", "player", &reset, 60);
    let Kind::Reset { name, .. } = &mut mail.kind else {
        panic!("a reset mail");
    };
    *name = "<img src=x onerror=alert(1)>&\"'".into();
    let html = render(&mail, SITE, false).html;
    assert!(!html.contains("<img"));
    assert_eq!(
        html.matches("&lt;img src=x onerror=alert(1)&gt;&amp;&quot;&#39;")
            .count(),
        2
    );
    // The German words hold an apostrophe-free text, the English ones
    // none either; an origin with a quote would be escaped too, in the
    // links and in the host the footer shows.
    let odd = render(&mail, "https://baylee.example\"><b>", false);
    assert!(!odd.html.contains("\"><b>"));
    assert!(odd.html.contains(">baylee.example&quot;&gt;&lt;b&gt;</a>"));
}

/// Writes every mail in English and German, as HTML and as text, into
/// the folder `BAYLEE_MAIL_SHOTS` names, to be looked at in a browser
/// (`docs/email.md`, "The mails"); with the logo too, as `…-logo.html`,
/// whose `cid:` a browser cannot resolve (point it at a local picture to
/// look). Its codes are fresh random ones and its address and site are
/// reserved names.
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
            let rendered = render(mail, SITE, false);
            let stem = folder.join(format!("{}-{name}", lang.code()));
            std::fs::write(stem.with_extension("html"), rendered.html.as_bytes()).unwrap();
            std::fs::write(stem.with_extension("txt"), rendered.text.as_bytes()).unwrap();
            let logo = render(mail, SITE, true);
            let stem = folder.join(format!("{}-{name}-logo.html", lang.code()));
            std::fs::write(stem, logo.html.as_bytes()).unwrap();
        }
    }
}
