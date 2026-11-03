// A CSV of reports (`admin/feedback/csv.ts`) never lets a report's words run
// as a spreadsheet formula, and the chart's days are the last 30 UTC days.

import { describe, expect, it } from "vitest";

import { lastDays } from "../../src/admin/feedback/Chart";
import { COLUMNS, field, toCsv } from "../../src/admin/feedback/csv";
import { fakeReports } from "../support/reports";

describe("a CSV of reports", () => {
  it("defuses a formula and quotes what needs it", () => {
    expect(field("=HYPERLINK(1)")).toBe(`"'=HYPERLINK(1)"`);
    for (const lead of ["+", "-", "@", "\t", "\r"]) expect(field(`${lead}1`).startsWith(`"'`) || field(`${lead}1`).startsWith("'")).toBe(true);
    expect(field('a "b", c')).toBe('"a ""b"", c"');
    expect(field(null)).toBe("");
    expect(field(42)).toBe("42");
  });

  it("has a header and a line per report, CRLF ended", async () => {
    const api = fakeReports();
    const { reports } = await api.reports({ filter: {}, sort: "received", descending: true, offset: 0, limit: 50 });
    const csv = toCsv(reports, () => "Synthetic_Admin#1000");
    const lines = csv.split("\r\n");
    expect(lines[0]).toBe(COLUMNS.join(","));
    expect(lines).toHaveLength(reports.length + 2);
    expect(csv.endsWith("\r\n")).toBe(true);
    expect(csv).toContain("Synthetic_Admin#1000");
  });
});

describe("the chart's days", () => {
  it("are the last 30 UTC days, crashes apart from the rest", () => {
    const days = lastDays(
      [
        { day: "2026-10-31", kind: "crash", count: 2 },
        { day: "2026-10-31", kind: "bug", count: 3 },
        { day: "2026-09-01", kind: "bug", count: 9 },
      ],
      30,
      new Date("2026-10-31T23:30:00Z"),
    );
    expect(days).toHaveLength(30);
    expect(days[0]?.day).toBe("2026-10-02");
    expect(days.at(-1)).toEqual({ day: "2026-10-31", crashes: 2, others: 3 });
    expect(days.reduce((n, d) => n + d.others, 0)).toBe(3);
  });
});
