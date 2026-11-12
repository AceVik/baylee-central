//! A report and Baylee's GitHub issues (the old administration's
//! `github.ts`). Central holds no GitHub token: the page opens GitHub's own
//! "new issue" form with the fields filled in, the administrator reads it
//! over and files it there, then links the number here.
//!
//! What goes into the public issue (owner, 08.10.2026): only a neutral
//! technical summary the administrator writes, a neutral category and the
//! build. Never the player's words, their pseudonym, anything the client
//! sent, nor a link back to the report. Every address goes to the one
//! repository and is built from a number or from those three things alone.

use central_api::reports::ReportKind;

use super::query::encode;
use super::text::first_line;

/// Baylee's issues.
pub const ISSUES: &str = "https://github.com/AceVik/baylee/issues";

/// The longest summary copied into an issue; GitHub's address has a limit.
pub const SUMMARY_CHARS: usize = 4_000;

/// The neutral category an issue is filed under, one per kind.
#[must_use]
pub const fn category(kind: ReportKind) -> &'static str {
    match kind {
        ReportKind::Bug => "Bug",
        ReportKind::Improvement => "Improvement",
        ReportKind::Feedback => "Feedback",
        ReportKind::Crash => "Crash",
        ReportKind::Other => "Report",
    }
}

/// A build as a version reads (`0.1.1-beta.9b`); anything else a gateway
/// said stays out of a public issue.
#[must_use]
pub fn plain_build(build: Option<&str>) -> Option<&str> {
    build.filter(|b| {
        (1..=64).contains(&b.len())
            && b.bytes().next().is_some_and(|c| c.is_ascii_alphanumeric())
            && b.bytes()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'+' | b'-'))
    })
}

/// GitHub's "new issue" form for the administrator's `summary`, or `None`
/// while it is blank. It takes no report, so nothing of one reaches it but
/// the category and a build that reads as a version.
#[must_use]
pub fn new_issue_url(summary: &str, kind: ReportKind, build: Option<&str>) -> Option<String> {
    let summary = summary.trim();
    if summary.is_empty() {
        return None;
    }
    let title = format!("{}: {}", category(kind), first_line(summary, 80));
    let mut body = if summary.chars().count() > SUMMARY_CHARS {
        let cut: String = summary.chars().take(SUMMARY_CHARS).collect();
        format!("{cut}\n[…]")
    } else {
        summary.to_owned()
    };
    body.push_str("\n\n- Category: ");
    body.push_str(kind.as_str());
    if let Some(build) = plain_build(build) {
        body.push_str("\n- Build: ");
        body.push_str(build);
    }
    Some(format!(
        "{ISSUES}/new?title={}&body={}",
        encode(&title),
        encode(&body)
    ))
}

/// Issue `n`'s page, for a number Central keeps (1 to 10,000,000).
#[must_use]
pub fn issue_url(n: u32) -> Option<String> {
    (1..=10_000_000)
        .contains(&n)
        .then(|| format!("{ISSUES}/{n}"))
}

/// An issue as an administrator types it: `311`, `#311`, or its address in
/// Baylee's repository (an issue or a pull request); `None` for anything
/// else.
#[must_use]
pub fn parse_issue(input: &str) -> Option<u32> {
    let text = input.trim();
    let digits = |s: &str| (1..=8).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_digit());
    let number = if let Some(rest) = text.strip_prefix('#').or(Some(text)).filter(|s| digits(s)) {
        rest
    } else {
        let lower = text.to_ascii_lowercase();
        let repo = "https://github.com/acevik/baylee/";
        let rest = lower.strip_prefix(repo)?;
        let rest = rest
            .strip_prefix("issues/")
            .or_else(|| rest.strip_prefix("pull/"))?;
        let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
        let (number, tail) = rest.split_at(end);
        let tail_ok = tail.is_empty()
            || tail == "/"
            || tail.starts_with('?')
            || tail.starts_with('#')
            || tail.starts_with("/?")
            || tail.starts_with("/#");
        if !digits(number) || !tail_ok {
            return None;
        }
        // the digits are at the same place in the original text
        let at = text.len() - rest.len();
        &text[at..at + number.len()]
    };
    let n: u32 = number.parse().ok()?;
    (1..=10_000_000).contains(&n).then_some(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_issue_as_typed() {
        assert_eq!(parse_issue("311"), Some(311));
        assert_eq!(parse_issue(" #311 "), Some(311));
        assert_eq!(
            parse_issue("https://github.com/AceVik/baylee/issues/311"),
            Some(311)
        );
        assert_eq!(
            parse_issue("https://github.com/AceVik/baylee/pull/12/"),
            Some(12)
        );
        assert_eq!(
            parse_issue("https://github.com/AceVik/baylee/issues/311#issuecomment-1"),
            Some(311)
        );
        for bad in [
            "",
            "#",
            "0",
            "10000001",
            "123456789",
            "abc",
            "https://github.com/other/repo/issues/1",
            "https://github.com/AceVik/baylee/issues/1/files",
            "javascript:alert(1)",
            "https://evil.invalid/AceVik/baylee/issues/1",
        ] {
            assert_eq!(parse_issue(bad), None, "{bad}");
        }
        assert_eq!(
            issue_url(311).as_deref(),
            Some("https://github.com/AceVik/baylee/issues/311")
        );
        assert_eq!(issue_url(0), None);
    }

    /// A new issue carries the administrator's words, a category and a
    /// build that reads as a version: nothing of the report.
    #[test]
    fn a_new_issue_carries_only_neutral_words() {
        assert_eq!(new_issue_url("   ", ReportKind::Bug, None), None);
        let url = new_issue_url(
            "Stack overflows\nwhen attacking",
            ReportKind::Crash,
            Some("0.1.1-beta.9b"),
        )
        .expect("an address");
        assert!(url.starts_with(
            "https://github.com/AceVik/baylee/issues/new?title=Crash%3A+Stack+overflows&body="
        ));
        assert!(url.ends_with("%0A%0A-+Category%3A+crash%0A-+Build%3A+0.1.1-beta.9b"));
        let odd = new_issue_url("x", ReportKind::Other, Some("<script>")).expect("an address");
        assert!(!odd.contains("Build") && !odd.contains("script"));
        assert_eq!(plain_build(Some("1.2.3+git.abc")), Some("1.2.3+git.abc"));
        assert_eq!(plain_build(Some("-1")), None);
        assert_eq!(plain_build(Some("1 2")), None);
    }
}
