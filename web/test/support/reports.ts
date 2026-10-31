// A stand-in for Central's report triage (`ReportsApi`), in memory: a few
// synthetic reports, the revision rule (a stale one is `conflict`), notes
// removed by their author only, the unread mark, the trash, saved views and
// a selection's changes. Every method is a spy, so a test reads what was
// asked. Synthetic data only.

import { vi } from "vitest";

import { ApiError } from "../../src/account/api";
import { idText } from "../../src/account/owner-api";
import type { BulkAction, ReportChange, ReportDetail, ReportFilter, ReportSummary, ReportsApi, SavedView } from "../../src/admin/feedback/api";

export const id = (n: number): Uint8Array => Uint8Array.from([0x01, 0x92, 0xd0, 0xc5, 0x7b, 0x3e, 0x7a, 0x1f, 0x8c, 0x2d, 0x3e, 0x4f, 0x5a, 0x6b, 0x7c, n]);
/** The administrator the page tests sign in as (test/support/central.ts `ACCOUNT`). */
export const ME = Uint8Array.from([0x01, 0x92, 0xd0, 0xc5, 0x7b, 0x3e, 0x7a, 0x1f, 0x8c, 0x2d, 0x3e, 0x4f, 0x5a, 0x6b, 0x7c, 0x8d]);
export const OTHER_ADMIN = id(0x77);

function summary(n: number, extra: Partial<ReportSummary> = {}): ReportSummary {
  return {
    id: id(n),
    createdAt: 1_800_000_000 + n * 60,
    updatedAt: 1_800_000_000 + n * 60,
    gateway: "Official",
    gatewaySays: "Sanctuary",
    gatewayVersion: "0.1.1-beta.9b",
    channel: "gateway",
    reporter: `5bdc0a${n.toString(16).padStart(2, "0")}`,
    kind: "bug",
    status: "new",
    excerpt: `Synthetic report ${n}: the stack went wrong`,
    build: "0.1.1-beta.9b",
    crash: null,
    hasRecord: false,
    recordComplete: false,
    recordOrigin: null,
    recordBytes: null,
    issue: null,
    assignee: null,
    priority: null,
    labels: [],
    duplicateOf: null,
    signature: null,
    revision: 1,
    doneAt: null,
    deletedAt: null,
    seen: false,
    notes: 0,
    ...extra,
  };
}

export interface FakeReports extends ReportsApi {
  rows: ReportDetail[];
  saved: SavedView[];
  /** Make the next `setReport` find a newer revision. */
  staleOnce(): void;
}

const matches = (f: ReportFilter, r: ReportSummary): boolean =>
  (f.trash === true ? r.deletedAt !== null : r.deletedAt === null) &&
  (f.statuses === undefined || f.statuses.includes(r.status)) &&
  (f.kinds === undefined || f.kinds.includes(r.kind)) &&
  (f.unseen !== true || !r.seen) &&
  (f.q === undefined || r.excerpt.toLowerCase().includes(f.q.toLowerCase())) &&
  (f.label === undefined || r.labels.includes(f.label)) &&
  (f.unassigned !== true || r.assignee === null) &&
  (f.assignee === undefined || (r.assignee !== null && idText(r.assignee) === idText(f.assignee)));

/** A change written: the revision moves on. */
const bump = (d: ReportDetail, s: Partial<ReportSummary>) => {
  d.summary = { ...d.summary, ...s, revision: d.summary.revision + 1, updatedAt: d.summary.updatedAt + 1 };
};
const apply = (d: ReportDetail, c: ReportChange) => {
  const next: Partial<ReportSummary> = {};
  if (c.status !== undefined) next.status = c.status;
  if (Object.hasOwn(c, "priority")) next.priority = c.priority ?? null;
  if (Object.hasOwn(c, "assignee")) next.assignee = c.assignee ?? null;
  if (Object.hasOwn(c, "issue")) next.issue = c.issue ?? null;
  if (c.labels !== undefined) next.labels = [...c.labels];
  if (c.labelsAdd !== undefined) next.labels = [...new Set([...(next.labels ?? d.summary.labels), ...c.labelsAdd])].sort();
  if (c.labelsRemove !== undefined) next.labels = (next.labels ?? d.summary.labels).filter((l) => !c.labelsRemove?.includes(l));
  if (Object.hasOwn(c, "duplicateOf")) {
    next.duplicateOf = c.duplicateOf ?? null;
    next.status = c.duplicateOf ? "duplicate" : "triaged";
  }
  bump(d, next);
};
/** Each value once, with how often it is there. */
const count = <T,>(values: T[]) => [...new Map(values.map((v) => [JSON.stringify(v), { value: v, count: values.filter((x) => JSON.stringify(x) === JSON.stringify(v)).length }])).values()];

export function fakeReports(): FakeReports {
  const rows: ReportDetail[] = [
    summary(1),
    summary(2, { kind: "crash", crash: "index 7 out of range for 3", signature: "a".repeat(32), labels: ["engine"], seen: true }),
    summary(3, { status: "triaged", priority: "high", assignee: ME, seen: true, notes: 1 }),
    summary(4, { status: "resolved", seen: true }),
  ].map((s) => ({
    summary: s,
    text: `${s.excerpt}\nwhen I cast a counterspell.`,
    gameId: null,
    gatewayUrl: "https://gateway.invalid",
    commit: null,
    crashLocation: s.crash === null ? null : "crates/engine/src/stack.rs:120:9",
    client: null,
    record: null,
    notes:
      s.notes > 0
        ? [
            { id: id(0x50), at: 1_800_000_500, author: OTHER_ADMIN, text: "Seen on Windows too." },
          ]
        : [],
    audit: [{ at: s.createdAt, actor: null, action: "report.filed", detail: "by Official" }],
  }));
  let stale = false;
  const find = (r: Uint8Array) => {
    const found = rows.find((x) => idText(x.summary.id) === idText(r));
    if (found === undefined) throw new ApiError("rejected");
    return found;
  };
  const saved: SavedView[] = [];
  const api: FakeReports = {
    rows,
    saved,
    staleOnce: () => {
      stale = true;
    },
    reports: vi.fn<ReportsApi["reports"]>(async (l) => {
      const all = rows.map((r) => r.summary).filter((r) => matches(l.filter, r));
      const sorted = l.descending ? all.toReversed() : all;
      return { reports: sorted.slice(l.offset, l.offset + l.limit), total: all.length };
    }),
    facets: vi.fn<ReportsApi["facets"]>(async (f) => {
      // each dimension counted under the filter but its own: the statuses without the statuses
      const { statuses: _own, ...rest } = f;
      const all = rows.map((r) => r.summary).filter((r) => matches(rest, r));
      return {
        statuses: count(all.map((r) => r.status)),
        kinds: count(rows.map((r) => r.summary).filter((r) => matches(f, r)).map((r) => r.kind)),
        priorities: count(all.map((r) => r.priority)),
        assignees: count(rows.map((r) => r.summary).filter((r) => matches(f, r)).map((r) => r.assignee)),
        labels: count(all.flatMap((r) => r.labels)),
        gateways: count(all.map((r) => r.gateway)),
        channels: count(all.map((r) => r.channel)),
        reporters: count(all.map((r) => r.reporter)),
        total: rows.map((r) => r.summary).filter((r) => matches(f, r)).length,
      };
    }),
    report: vi.fn<ReportsApi["report"]>(async (r) => structuredClone(find(r))),
    neighbours: vi.fn<ReportsApi["neighbours"]>(async (r, f) => {
      const list = rows.map((x) => x.summary).filter((x) => matches(f, x)).toReversed();
      const at = list.findIndex((x) => idText(x.id) === idText(r));
      return { previous: list[at - 1]?.id ?? null, next: list[at + 1]?.id ?? null, position: at < 0 ? null : at + 1, total: list.length };
    }),
    crashGroups: vi.fn<ReportsApi["crashGroups"]>(async () => [
      { signature: "a".repeat(32), count: 1, open: 1, first: 1_800_000_120, last: 1_800_000_120, latest: id(2), message: "index 7 out of range for 3", location: "crates/engine/src/stack.rs" },
    ]),
    stats: vi.fn<ReportsApi["stats"]>(async () => [
      { day: "2026-10-30", kind: "bug", count: 2 },
      { day: "2026-10-31", kind: "crash", count: 1 },
    ]),
    admins: vi.fn<ReportsApi["admins"]>(async () => [
      { id: ME, displayName: "Synthetic_Admin", tag: 0x1000 },
      { id: OTHER_ADMIN, displayName: "Other_Admin", tag: 0x2000 },
    ]),
    setReport: vi.fn<ReportsApi["setReport"]>(async (r, c, expected) => {
      const d = find(r);
      if (stale) {
        stale = false;
        bump(d, {});
      }
      if (expected !== null && expected !== d.summary.revision) throw new ApiError("conflict");
      apply(d, c);
      return structuredClone(d);
    }),
    bulk: vi.fn<ReportsApi["bulk"]>(async (ids, action: BulkAction) => {
      for (const r of ids) {
        const d = find(r);
        if (action.do === "change") apply(d, action.change);
        else if (action.do === "trash") bump(d, { deletedAt: 1_800_001_000 });
        else if (action.do === "restore") bump(d, { deletedAt: null });
        else rows.splice(rows.indexOf(d), 1);
      }
      return ids.length;
    }),
    trash: vi.fn<ReportsApi["trash"]>(async (r) => bump(find(r), { deletedAt: 1_800_001_000 })),
    restore: vi.fn<ReportsApi["restore"]>(async (r) => bump(find(r), { deletedAt: null })),
    purge: vi.fn<ReportsApi["purge"]>(async (r) => {
      rows.splice(rows.indexOf(find(r)), 1);
    }),
    addNote: vi.fn<ReportsApi["addNote"]>(async (r, text) => {
      const d = find(r);
      d.notes = [...d.notes, { id: id(0x60 + d.notes.length), at: 1_800_002_000, author: ME, text }];
      bump(d, { notes: d.notes.length });
      return structuredClone(d);
    }),
    removeNote: vi.fn<ReportsApi["removeNote"]>(async (r, note) => {
      const d = find(r);
      const n = d.notes.find((x) => idText(x.id) === idText(note));
      if (n === undefined || idText(n.author) !== idText(ME)) throw new ApiError("rejected");
      d.notes = d.notes.filter((x) => x !== n);
      bump(d, { notes: d.notes.length });
      return structuredClone(d);
    }),
    views: vi.fn<ReportsApi["views"]>(async () => [...saved]),
    saveView: vi.fn<ReportsApi["saveView"]>(async (name, query) => {
      const same = saved.find((v) => v.name === name);
      if (same) same.query = query;
      else saved.push({ id: id(0x90 + saved.length), name, query });
      return [...saved];
    }),
    deleteView: vi.fn<ReportsApi["deleteView"]>(async (v) => {
      saved.splice(saved.findIndex((x) => idText(x.id) === idText(v)), 1);
      return [...saved];
    }),
    setSeen: vi.fn<ReportsApi["setSeen"]>(async (ids, seen) => {
      for (const r of ids) find(r).summary.seen = seen;
    }),
    part: vi.fn<ReportsApi["part"]>(async () => {
      throw new ApiError("rejected");
    }),
  };
  return api;
}
