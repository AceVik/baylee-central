// Reports as CSV, made in the browser from what Central answered (Baylee
// `web/feedback/src/reports/csv.ts`). A cell that a spreadsheet would read as
// a formula (it starts with `=`, `+`, `-`, `@`, a tab or a carriage return)
// gets an apostrophe in front, so a report's words never run as one.

import { idText } from "../../account/owner-api";
import type { ReportSummary } from "./api";

/** `value` as one CSV field: a formula's first character defused, quoted when it holds a comma, a quote or a line break. */
export function field(value: string | number | boolean | null): string {
  const raw = value === null ? "" : String(value);
  const text = /^[=+\-@\t\r]/.test(raw) ? `'${raw}` : raw;
  return /[",\r\n']/.test(text) ? `"${text.replaceAll('"', '""')}"` : text;
}

export const COLUMNS = [
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
] as const;

const iso = (seconds: number | null): string => (seconds === null ? "" : new Date(seconds * 1000).toISOString());

/** `reports` as CSV with a header line, `\r\n` line ends, UTF-8 without a byte order mark; `name` names an assignee. */
export function toCsv(reports: readonly ReportSummary[], name: (id: Uint8Array | null) => string): string {
  const lines = [COLUMNS.join(",")];
  for (const r of reports) {
    lines.push(
      [
        idText(r.id),
        iso(r.createdAt),
        iso(r.updatedAt),
        r.kind,
        r.status,
        r.priority,
        r.assignee === null ? "" : name(r.assignee),
        r.labels.join(" "),
        r.channel === "direct" ? "" : r.gateway,
        r.channel,
        r.reporter,
        r.build,
        r.crash,
        r.recordBytes,
        r.issue,
        r.notes,
        r.excerpt,
      ]
        .map(field)
        .join(","),
    );
  }
  return `${lines.join("\r\n")}\r\n`;
}

/** Hands `data` to the browser as a file called `name`; the address is given back half a minute later. */
export function download(name: string, data: BlobPart, type = "text/csv;charset=utf-8"): void {
  const url = URL.createObjectURL(new Blob([data], { type }));
  const a = document.createElement("a");
  a.href = url;
  a.download = name;
  a.rel = "noopener";
  document.body.append(a);
  a.click();
  a.remove();
  // long enough for the browser to take a large record, then the memory is given back
  window.setTimeout(() => URL.revokeObjectURL(url), 30_000);
}

/** The most an export of "all that match" reads: 25 pages of 200. */
export const EXPORT_MAX = 5_000;
