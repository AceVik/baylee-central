//! Reports as CSV, made in the browser from what Central answered (the old
//! administration's `csv.ts`, column for column). A cell a spreadsheet
//! would read as a formula (it starts with `=`, `+`, `-`, `@`, a tab or a
//! carriage return) gets an apostrophe in front, so a report's words never
//! run as one.

use central_api::reports::{Channel, ReportSummary};

use super::text::{hex, iso};

/// The most an export of "all that match" reads: 25 pages of 200.
pub const EXPORT_MAX: u64 = 5_000;

/// The header line's columns.
pub const COLUMNS: [&str; 17] = [
    "id",
    "received",
    "changed",
    "kind",
    "status",
    "priority",
    "assignee",
    "labels",
    "gateway",
    "channel",
    "reporter",
    "build",
    "crash",
    "record_bytes",
    "issue",
    "notes",
    "excerpt",
];

/// `raw` as one CSV field: a formula's first character defused, quoted
/// when it holds a comma, a quote, a line break or the apostrophe.
#[must_use]
pub fn field(raw: &str) -> String {
    let formula = raw.starts_with(['=', '+', '-', '@', '\t', '\r']);
    let text = if formula {
        format!("'{raw}")
    } else {
        raw.to_owned()
    };
    if text.contains([',', '"', '\r', '\n', '\'']) {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text
    }
}

/// `reports` as CSV with a header line, `\r\n` line ends, UTF-8 without a
/// byte order mark; `name` names an assignee.
#[must_use]
pub fn to_csv(reports: &[ReportSummary], name: &dyn Fn([u8; 16]) -> String) -> String {
    let mut lines = vec![COLUMNS.join(",")];
    for r in reports {
        let cells = [
            hex(&r.id),
            iso(r.created_at),
            iso(r.updated_at),
            r.kind.as_str().to_owned(),
            r.status.as_str().to_owned(),
            r.priority
                .map(|p| p.as_str().to_owned())
                .unwrap_or_default(),
            r.assignee.map(name).unwrap_or_default(),
            r.labels.join(" "),
            if r.channel == Channel::Direct {
                String::new()
            } else {
                r.gateway.clone()
            },
            match r.channel {
                Channel::Gateway => "gateway",
                Channel::Direct => "direct",
            }
            .to_owned(),
            r.reporter.clone(),
            r.build.clone().unwrap_or_default(),
            r.crash.clone().unwrap_or_default(),
            r.record_bytes.map(|b| b.to_string()).unwrap_or_default(),
            r.issue.map(|i| i.to_string()).unwrap_or_default(),
            r.notes.to_string(),
            r.excerpt.clone(),
        ];
        lines.push(cells.iter().map(|c| field(c)).collect::<Vec<_>>().join(","));
    }
    let mut out = lines.join("\r\n");
    out.push_str("\r\n");
    out
}

/// A CSV's file name, by the UTC day.
#[must_use]
pub fn file_name(day: &str) -> String {
    format!("baylee-reports-{day}.csv")
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use central_api::reports::{Priority, ReportKind, ReportStatus};

    /// A synthetic report summary.
    pub fn summary(n: u8) -> ReportSummary {
        let mut id = [0x10; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        id[15] = n;
        ReportSummary {
            id,
            created_at: 1_794_000_000,
            updated_at: 1_794_000_060,
            gateway: "Synthetic gateway".into(),
            gateway_says: None,
            gateway_version: None,
            channel: Channel::Gateway,
            reporter: "5bdc".into(),
            kind: ReportKind::Bug,
            status: ReportStatus::New,
            excerpt: "It went wrong".into(),
            build: Some("0.1.1-beta.9b".into()),
            crash: None,
            has_record: false,
            record_complete: false,
            record_origin: None,
            record_bytes: None,
            issue: None,
            assignee: None,
            priority: None,
            labels: Vec::new(),
            duplicate_of: None,
            signature: None,
            revision: 1,
            done_at: None,
            deleted_at: None,
            seen: false,
            notes: 0,
        }
    }

    /// A formula never runs: every character a spreadsheet starts one
    /// with is defused, and the field is quoted where it must be.
    #[test]
    fn a_formula_is_defused() {
        assert_eq!(field("=1+1"), "\"'=1+1\"");
        assert_eq!(field("+49 30"), "\"'+49 30\"");
        assert_eq!(field("-2"), "\"'-2\"");
        assert_eq!(field("@SUM(A1)"), "\"'@SUM(A1)\"");
        assert_eq!(field("\tcmd"), "\"'\tcmd\"");
        assert_eq!(field("\rcmd"), "\"'\rcmd\"");
        assert_eq!(
            field("=HYPERLINK(\"https://x.invalid\",\"y\")"),
            "\"'=HYPERLINK(\"\"https://x.invalid\"\",\"\"y\"\")\""
        );
        assert_eq!(field("plain"), "plain");
        assert_eq!(field("a,b"), "\"a,b\"");
        assert_eq!(field("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(field("two\nlines"), "\"two\nlines\"");
        assert_eq!(field("it's"), "\"it's\"");
        assert_eq!(
            field("a=b"),
            "a=b",
            "only a first character starts a formula"
        );
        assert_eq!(field(""), "");
    }

    #[test]
    fn the_export_has_the_old_columns() {
        let mut hostile = summary(2);
        hostile.excerpt = "=cmd|' /C calc'!A0".into();
        hostile.labels = vec!["ui".into(), "crash-loop".into()];
        hostile.priority = Some(Priority::High);
        hostile.assignee = Some([3; 16]);
        hostile.channel = Channel::Direct;
        hostile.issue = Some(311);
        hostile.record_bytes = Some(4_000);
        let csv = to_csv(&[summary(1), hostile], &|id| format!("admin{}", id[0]));
        let lines: Vec<&str> = csv.split("\r\n").collect();
        assert_eq!(lines[0], COLUMNS.join(","));
        assert!(lines[1].starts_with(&format!(
            "{},2026-11-06T21:20:00.000Z,",
            hex(&summary(1).id)
        )));
        assert!(
            lines[1].ends_with(",Synthetic gateway,gateway,5bdc,0.1.1-beta.9b,,,,0,It went wrong")
        );
        assert!(lines[2].contains(",bug,new,high,admin3,ui crash-loop,,direct,5bdc,"));
        assert!(lines[2].ends_with(",4000,311,0,\"'=cmd|' /C calc'!A0\""));
        assert_eq!(lines.len(), 4, "a line end after the last row");
        assert_eq!(lines[3], "");
        assert_eq!(file_name("2026-11-12"), "baylee-reports-2026-11-12.csv");
    }
}
