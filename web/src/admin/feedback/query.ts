// A list of reports as the address holds it (`/admin/feedback?…`), so that a
// list is a link, a saved view and the way back from a report's page. Every
// part is read as Central would check it (central-api `ReportFilter::
// validate`): a part out of its bounds is dropped, never sent.

import { idOf, idText } from "../../account/owner-api";
import { PATHS } from "../../router";
import {
  BOUNDS,
  DONE,
  KINDS,
  OPEN,
  PRIORITIES,
  SORTS,
  STATUSES,
  validDay,
  validGatewayName,
  validLabel,
  validQuery,
  validReporter,
  validSignature,
  type Channel,
  type ListRequest,
  type Priority,
  type ReportFilter,
  type ReportKind,
  type ReportSort,
  type ReportStatus,
} from "./api";

/** The list as the address says it. Ids are hex, so two lists compare as text. */
export interface ListQuery {
  statuses?: ReportStatus[];
  kinds?: ReportKind[];
  /** `none` is no priority. */
  priorities?: (Priority | "none")[];
  q?: string;
  from?: string;
  to?: string;
  record?: boolean;
  channel?: Channel;
  gateway?: string;
  /** An administrator's id in hex, or `none`. */
  assignee?: string;
  label?: string;
  reporter?: string;
  signature?: string;
  /** A report's id in hex. */
  duplicateOf?: string;
  trash?: true;
  unseen?: true;
  sort?: ReportSort;
  /** Oldest, smallest or first first. */
  asc?: true;
  /** From 1. */
  page?: number;
  limit?: number;
}

/** The page sizes offered; Central takes at most 200. */
export const PAGE_SIZES = [25, 50, 100, 200] as const;
export const PAGE_SIZE = 50;

export type ViewId = "inbox" | "open" | "mine" | "unassigned" | "done" | "all" | "trash";

/** The views above the list: each sets the statuses, the assignee and the trash, and leaves the rest. */
export const VIEWS: readonly { id: ViewId; query: (me: string) => ListQuery }[] = [
  { id: "inbox", query: () => ({ statuses: ["new"] }) },
  { id: "open", query: () => ({ statuses: [...OPEN] }) },
  { id: "mine", query: (me) => ({ statuses: [...OPEN], assignee: me }) },
  { id: "unassigned", query: () => ({ statuses: [...OPEN], assignee: "none" }) },
  { id: "done", query: () => ({ statuses: [...DONE] }) },
  { id: "all", query: () => ({}) },
  { id: "trash", query: () => ({ trash: true }) },
];

/** The parts of a list a view decides. */
export const VIEW_PARTS = ["statuses", "assignee", "trash"] as const satisfies readonly (keyof ListQuery)[];

/** Several of `all`, in `all`'s order, from `a,b,c`; none read is no filter. */
function several<T extends string>(all: readonly T[], text: string | null): T[] | undefined {
  if (text === null) return undefined;
  const asked = new Set(text.split(","));
  const out = all.filter((v) => asked.has(v));
  return out.length > 0 ? out : undefined;
}

/** A search as Central takes it: no control or invisible characters, trimmed, at most 200 characters. */
export function cleanQuery(text: string): string | undefined {
  const plain = text
    // oxlint-disable-next-line no-control-regex -- control characters are what this removes
    .replace(/[\u0000-\u001F\u007F-\u009F\u061C\u200B\u200E\u200F\u202A-\u202E\u2028\u2029\u2060-\u2064\u2066-\u2069\uFEFF]/g, " ")
    .trim();
  const cut = [...plain].slice(0, BOUNDS.query).join("").trim();
  return validQuery(cut) ? cut : undefined;
}

/** The list `search` (a query string) asks for; anything unreadable is left out. */
export function readQuery(search: string): ListQuery {
  const p = new URLSearchParams(search);
  const out: ListQuery = {};
  const statuses = several(STATUSES, p.get("status"));
  if (statuses) out.statuses = statuses;
  const kinds = several(KINDS, p.get("kind"));
  if (kinds) out.kinds = kinds;
  const priorities = several([...PRIORITIES, "none"] as const, p.get("priority"));
  if (priorities) out.priorities = priorities;
  const q = cleanQuery(p.get("q") ?? "");
  if (q !== undefined) out.q = q;
  for (const key of ["from", "to"] as const) {
    const day = p.get(key);
    if (day !== null && validDay(day)) out[key] = day;
  }
  const record = p.get("record");
  if (record === "yes" || record === "no") out.record = record === "yes";
  const channel = p.get("channel");
  if (channel === "gateway" || channel === "direct") out.channel = channel;
  const gateway = p.get("gateway");
  if (gateway !== null && validGatewayName(gateway)) out.gateway = gateway;
  const assignee = p.get("assignee");
  if (assignee === "none" || (assignee !== null && idOf(assignee) !== null)) out.assignee = assignee;
  const label = p.get("label");
  if (label !== null && validLabel(label)) out.label = label;
  const reporter = p.get("reporter");
  if (reporter !== null && validReporter(reporter)) out.reporter = reporter;
  const signature = p.get("signature");
  if (signature !== null && validSignature(signature)) out.signature = signature;
  const original = p.get("duplicate_of");
  if (original !== null && idOf(original) !== null) out.duplicateOf = original;
  if (p.get("trash") === "1") out.trash = true;
  if (p.get("unseen") === "1") out.unseen = true;
  const sort = SORTS.find((s) => s === p.get("sort"));
  if (sort !== undefined && sort !== "received") out.sort = sort;
  if (p.get("dir") === "asc") out.asc = true;
  const limit = Number(p.get("limit"));
  if (PAGE_SIZES.some((s) => s === limit) && limit !== PAGE_SIZE) out.limit = limit;
  const page = Number(p.get("page"));
  if (Number.isSafeInteger(page) && page > 1) out.page = page;
  return out;
}

/** `q` as a query string, its parts in a fixed order; empty for the whole list. */
export function writeQuery(q: ListQuery): string {
  const p = new URLSearchParams();
  const put = (key: string, value: string | undefined) => {
    if (value !== undefined && value !== "") p.set(key, value);
  };
  if (q.trash === true) put("trash", "1");
  put("status", q.statuses?.join(","));
  put("kind", q.kinds?.join(","));
  put("priority", q.priorities?.join(","));
  put("assignee", q.assignee);
  put("label", q.label);
  put("gateway", q.gateway);
  put("channel", q.channel);
  put("reporter", q.reporter);
  put("signature", q.signature);
  put("duplicate_of", q.duplicateOf);
  put("q", q.q);
  put("from", q.from);
  put("to", q.to);
  if (q.record !== undefined) put("record", q.record ? "yes" : "no");
  if (q.unseen === true) put("unseen", "1");
  if (q.sort !== undefined && q.sort !== "received") put("sort", q.sort);
  if (q.asc === true) put("dir", "asc");
  if (q.limit !== undefined && q.limit !== PAGE_SIZE) put("limit", String(q.limit));
  if (q.page !== undefined && q.page > 1) put("page", String(q.page));
  return p.toString();
}

/** `q` without its page: what a saved view and a report's way back keep. */
export function withoutPage(q: ListQuery): ListQuery {
  const { page: _page, ...rest } = q;
  return rest;
}

/** `q` with `change` applied, back on the first page; `undefined` takes a part out. */
export function changed(q: ListQuery, change: { [K in keyof ListQuery]?: ListQuery[K] | undefined }): ListQuery {
  const next: Record<string, unknown> = { ...q };
  delete next["page"];
  for (const [key, value] of Object.entries(change)) {
    if (value === undefined || (Array.isArray(value) && value.length === 0)) delete next[key];
    else next[key] = value;
  }
  // through the address and back: whatever the change made, the list is one the address can say
  return readQuery(writeQuery(next as ListQuery));
}

/** The filter Central reads for `q`. */
export function filterOf(q: ListQuery): ReportFilter {
  const f: ReportFilter = {};
  if (q.statuses) f.statuses = q.statuses;
  if (q.kinds) f.kinds = q.kinds;
  if (q.priorities) f.priorities = q.priorities.map((p) => (p === "none" ? null : p));
  if (q.q !== undefined) f.q = q.q;
  if (q.from !== undefined) f.from = q.from;
  if (q.to !== undefined) f.to = q.to;
  if (q.record !== undefined) f.hasRecord = q.record;
  if (q.channel !== undefined) f.channel = q.channel;
  if (q.gateway !== undefined) f.gateway = q.gateway;
  if (q.assignee === "none") f.unassigned = true;
  else if (q.assignee !== undefined) {
    const who = idOf(q.assignee);
    if (who !== null) f.assignee = who;
  }
  if (q.label !== undefined) f.label = q.label;
  if (q.reporter !== undefined) f.reporter = q.reporter;
  if (q.signature !== undefined) f.signature = q.signature;
  if (q.duplicateOf !== undefined) {
    const original = idOf(q.duplicateOf);
    if (original !== null) f.duplicateOf = original;
  }
  if (q.trash === true) f.trash = true;
  if (q.unseen === true) f.unseen = true;
  return f;
}

/** The sorts read best from the top down; the others newest or most first. */
const ASCENDING_FIRST: ReadonlySet<ReportSort> = new Set(["status", "kind", "reporter", "gateway"]);

/** Whether `sort` starts ascending when it is picked. */
export const startsAscending = (sort: ReportSort): boolean => ASCENDING_FIRST.has(sort);

export const limitOf = (q: ListQuery): number => q.limit ?? PAGE_SIZE;

/** The request for `q`'s page. */
export function requestOf(q: ListQuery): ListRequest {
  const limit = limitOf(q);
  return { filter: filterOf(q), sort: q.sort ?? "received", descending: q.asc !== true, offset: ((q.page ?? 1) - 1) * limit, limit };
}

const sameSet = (a: readonly string[] | undefined, b: readonly string[] | undefined): boolean => {
  const x = new Set(a ?? []);
  const y = new Set(b ?? []);
  return x.size === y.size && [...x].every((v) => y.has(v));
};

/** The view `q` is in, if any. */
export function viewOf(q: ListQuery, me: string): ViewId | null {
  for (const view of VIEWS) {
    const v = view.query(me);
    if (sameSet(q.statuses, v.statuses) && q.assignee === v.assignee && (q.trash === true) === (v.trash === true)) return view.id;
  }
  return null;
}

/** The address of the list `q`. */
export const listPath = (q: ListQuery): string => {
  const s = writeQuery(q);
  return s === "" ? PATHS.adminFeedback : `${PATHS.adminFeedback}?${s}`;
};

/** The address of report `id` opened from the list `q`, which its page steps through. */
export const reportPath = (id: Uint8Array | string, q: ListQuery = {}): string => {
  const s = writeQuery(withoutPage(q));
  const hex = typeof id === "string" ? id : idText(id);
  return s === "" ? `${PATHS.adminReport}/${hex}` : `${PATHS.adminReport}/${hex}?${s}`;
};
