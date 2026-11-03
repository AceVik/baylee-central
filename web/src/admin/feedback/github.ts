// A report and Baylee's GitHub issues (Baylee `web/feedback/src/github.ts`).
// Central holds no GitHub token: the page opens GitHub's own "new issue"
// form with the fields filled in, the administrator reads it over and files
// it there, and then links the number here.
//
// What goes into the public issue (owner, 08.10.2026): only a neutral
// technical summary the administrator writes, a neutral category and the
// build. Never the player's words, their pseudonym, anything the client
// sent, nor a link back to the report. Every link goes to the one
// repository and is built from a number or from those three things alone.

import { ISSUES } from "../../config";
import type { ReportKind } from "./api";

/** The longest summary copied into an issue; GitHub's address has a limit. */
export const SUMMARY_CHARS = 4_000;

/** The neutral category an issue is filed under, one per kind. */
export const CATEGORY: Record<ReportKind, string> = {
  bug: "Bug",
  improvement: "Improvement",
  feedback: "Feedback",
  crash: "Crash",
  other: "Report",
};

/** A build as a version reads (`0.1.1-beta.9b`); anything else a gateway said stays out of a public issue. */
export const plainBuild = (build: string | null): string | null => (build !== null && /^[0-9A-Za-z][0-9A-Za-z.+-]{0,63}$/.test(build) ? build : null);

/** The first non-blank line of `text`, cut to `max` characters with an ellipsis. */
export function firstLine(text: string, max: number): string {
  const line = (text.split(/\r?\n/).find((l) => l.trim() !== "") ?? "").trim();
  const chars = [...line];
  return chars.length > max ? `${chars.slice(0, max - 1).join("").trimEnd()}…` : line;
}

/** What an administrator files: their own words, a category, the build. */
export interface IssueDraft {
  summary: string;
  category: ReportKind;
  build: string | null;
}

/**
 * GitHub's "new issue" form for `draft`, or `null` while the summary is
 * blank. It takes a draft, not a report, so nothing of the report reaches
 * it but the category and a build that reads as a version.
 */
export function newIssueUrl(draft: IssueDraft): string | null {
  const summary = draft.summary.trim();
  if (summary === "") return null;
  const title = `${CATEGORY[draft.category]}: ${firstLine(summary, 80)}`;
  const body = [
    summary.length > SUMMARY_CHARS ? `${summary.slice(0, SUMMARY_CHARS)}\n[…]` : summary,
    "",
    `- Category: ${draft.category}`,
    ...(plainBuild(draft.build) === null ? [] : [`- Build: ${plainBuild(draft.build) ?? ""}`]),
  ].join("\n");
  return `${ISSUES}/new?${new URLSearchParams({ title, body }).toString()}`;
}

/** Issue `n`'s page, for a number Central keeps (1 to 10,000,000). */
export function issueUrl(n: number): string | null {
  return Number.isSafeInteger(n) && n >= 1 && n <= 10_000_000 ? `${ISSUES}/${n}` : null;
}

/** An issue as an administrator types it: `311`, `#311`, or its address in Baylee's repository; `null` for anything else. */
export function parseIssue(input: string): number | null {
  const text = input.trim();
  const escaped = ISSUES.replace(/[.*+?^${}()|[\]\\/]/g, "\\$&").replace(/issues$/, "(?:issues|pull)");
  const match = /^#?(\d{1,8})$/.exec(text) ?? new RegExp(`^${escaped}/(\\d{1,8})/?(?:[?#].*)?$`, "i").exec(text);
  const digits = match?.[1];
  if (digits === undefined) return null;
  const n = Number(digits);
  return n >= 1 && n <= 10_000_000 ? n : null;
}
