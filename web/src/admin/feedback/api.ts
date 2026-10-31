// Bug and crash reports in the administration (central-api `reports` and
// `admin`, docs/reports.md §"Triage"): what an administrator reads and
// decides, as `Admin` requests on the browser surface, and the two parts
// Central keeps as gzip (the client's details, a game's record), read from
// their own route. Every answer is checked against its exact shape and
// central-api's bounds, as admin-api.ts does; anything else is `malformed`
// and nothing of it is used. Triage asks for no fresh password; only a
// purge does (`AdminRequest::fresh`).

import { ApiError, answer, bytes, exact, isObject, isUuidV7, malformed, poster, safeInteger, type Endpoint, type Json } from "../../account/api";
import { validDisplayName, validTag } from "../../account/names";
import { idText } from "../../account/owner-api";
import { CENTRAL_API } from "../../config";

// ---------------------------------------------------------------- words and bounds

export const KINDS = ["bug", "improvement", "feedback", "crash", "other"] as const;
export type ReportKind = (typeof KINDS)[number];
export const STATUSES = ["new", "triaged", "in_progress", "resolved", "wont_fix", "duplicate"] as const;
export type ReportStatus = (typeof STATUSES)[number];
/** Not done: what the inbox and "open" hold. */
export const OPEN: readonly ReportStatus[] = ["new", "triaged", "in_progress"];
/** Done (`ReportStatus::done`). */
export const DONE: readonly ReportStatus[] = ["resolved", "wont_fix", "duplicate"];
export const PRIORITIES = ["critical", "high", "normal", "low"] as const;
export type Priority = (typeof PRIORITIES)[number];
export type Channel = "gateway" | "direct";
export type RecordOrigin = "gateway" | "client";
export const SORTS = ["received", "changed", "priority", "status", "kind", "reporter", "gateway"] as const;
export type ReportSort = (typeof SORTS)[number];
export type Part = "client" | "record";

/** central-api `reports::*`: the bounds a request is held to and an answer checked by. */
export const BOUNDS = {
  text: 20_000,
  excerpt: 300,
  gameId: 128,
  reporter: 128,
  said: 256,
  crashMessage: 2_000,
  crashLocation: 512,
  clientGzip: 2 * 1024 * 1024,
  recordGzip: 64 * 1024 * 1024,
  page: 50,
  maxPage: 200,
  query: 200,
  note: 4_000,
  labels: 16,
  label: 32,
  views: 50,
  viewName: 64,
  viewQuery: 2_048,
  bulk: 200,
  seen: 500,
  facetReporters: 50,
  crashGroups: 100,
  trashDays: 30,
  statsDays: 30,
  issue: 10_000_000,
} as const;

// ---------------------------------------------------------------- shapes

export interface ReportSummary {
  id: Uint8Array;
  createdAt: number;
  updatedAt: number;
  /** The authenticated gateway's name in Central. */
  gateway: string;
  /** What the gateway called itself. */
  gatewaySays: string | null;
  gatewayVersion: string | null;
  channel: Channel;
  /** The reporter's pseudonym, hex: never an account. */
  reporter: string;
  kind: ReportKind;
  status: ReportStatus;
  excerpt: string;
  build: string | null;
  crash: string | null;
  hasRecord: boolean;
  recordComplete: boolean;
  recordOrigin: RecordOrigin | null;
  recordBytes: number | null;
  issue: number | null;
  assignee: Uint8Array | null;
  priority: Priority | null;
  labels: string[];
  duplicateOf: Uint8Array | null;
  signature: string | null;
  revision: number;
  doneAt: number | null;
  deletedAt: number | null;
  /** The reading administrator opened it. */
  seen: boolean;
  notes: number;
}

export interface ReportNote {
  id: Uint8Array;
  at: number;
  author: Uint8Array;
  text: string;
}

export interface ReportAuditRow {
  at: number;
  /** None for the gateway that filed it, or the sweep. */
  actor: Uint8Array | null;
  /** Such as `report.status`. */
  action: string;
  /** The change in short, never the report's words. */
  detail: string;
}

export interface PartState {
  bytes: number;
  received: boolean;
}

export interface ReportDetail {
  summary: ReportSummary;
  text: string;
  gameId: string | null;
  gatewayUrl: string | null;
  commit: string | null;
  crashLocation: string | null;
  client: PartState | null;
  record: PartState | null;
  notes: ReportNote[];
  audit: ReportAuditRow[];
}

export interface Count<T> {
  value: T;
  count: number;
}

export interface ReportFacets {
  statuses: Count<ReportStatus>[];
  kinds: Count<ReportKind>[];
  priorities: Count<Priority | null>[];
  assignees: Count<Uint8Array | null>[];
  labels: Count<string>[];
  gateways: Count<string>[];
  channels: Count<Channel>[];
  reporters: Count<string>[];
  total: number;
}

export interface CrashGroup {
  signature: string;
  count: number;
  open: number;
  first: number;
  last: number;
  latest: Uint8Array;
  message: string;
  location: string;
}

export interface DayCount {
  day: string;
  kind: ReportKind;
  count: number;
}

export interface SavedView {
  id: Uint8Array;
  name: string;
  query: string;
}

export interface ReportAdmin {
  id: Uint8Array;
  displayName: string | null;
  tag: number | null;
}

export interface Neighbours {
  previous: Uint8Array | null;
  next: Uint8Array | null;
  /** From 1; none when the list does not hold it. */
  position: number | null;
  total: number;
}

/** Which reports a list holds (central-api `ReportFilter`): every part optional, several values of one part alternatives. */
export interface ReportFilter {
  statuses?: readonly ReportStatus[];
  kinds?: readonly ReportKind[];
  /** `null` is no priority. */
  priorities?: readonly (Priority | null)[];
  q?: string;
  from?: string;
  to?: string;
  hasRecord?: boolean;
  channel?: Channel;
  gateway?: string;
  /** An administrator's account; `unassigned` for nobody. */
  assignee?: Uint8Array;
  unassigned?: true;
  label?: string;
  reporter?: string;
  signature?: string;
  duplicateOf?: Uint8Array;
  trash?: true;
  unseen?: true;
}

/** A list: which, in what order, which page. */
export interface ListRequest {
  filter: ReportFilter;
  sort: ReportSort;
  descending: boolean;
  offset: number;
  limit: number;
}

/**
 * A change to a report's triage (central-api `ReportChange`): a part left
 * out stays, `null` clears it. One report's change sets `labels` whole; a
 * selection's adds and removes.
 */
export interface ReportChange {
  status?: ReportStatus;
  issue?: number | null;
  assignee?: Uint8Array | null;
  priority?: Priority | null;
  labels?: readonly string[];
  labelsAdd?: readonly string[];
  labelsRemove?: readonly string[];
  duplicateOf?: Uint8Array | null;
}

export type BulkAction = { do: "change"; change: ReportChange } | { do: "trash" } | { do: "restore" } | { do: "purge" };

// ---------------------------------------------------------------- checks

/** central-api's `invisible`: marks that change how the text around them reads. */
const INVISIBLE = /[\u061C\u200B\u200E\u200F\u202A-\u202E\u2028\u2029\u2060-\u2064\u2066-\u2069\uFEFF]/;
// oxlint-disable-next-line no-control-regex -- the control characters are what this finds
const CONTROL = /[\u0000-\u001F\u007F-\u009F]/;
// oxlint-disable-next-line no-control-regex -- the control characters are what this finds
const CONTROL_BUT_LINES = /[\u0000-\u0008\u000B-\u001F\u007F-\u009F]/;

const chars = (text: string): number => {
  let n = 0;
  for (const _ of text) n++;
  return n;
};

/** central-api's `shown`: within `max` characters, no control character (a tab and a newline aside where `lines`), none invisible. */
export function shown(text: string, max: number, lines = false): boolean {
  return chars(text) <= max && !(lines ? CONTROL_BUT_LINES : CONTROL).test(text) && !INVISIBLE.test(text);
}

/** central-api's `valid_label`. */
export const validLabel = (label: string): boolean => /^[a-z0-9-]{1,32}$/.test(label);
/** A crash group: 32 lowercase hex. */
export const validSignature = (s: string): boolean => /^[0-9a-f]{32}$/.test(s);
/** A reporter's pseudonym: 1 to 128 hex. */
export const validReporter = (r: string): boolean => r.length > 0 && r.length <= BOUNDS.reporter && /^[0-9a-fA-F]+$/.test(r);
/** central-api's `day`: `YYYY-MM-DD`, month 1–12, day 1–31. */
export function validDay(text: string): boolean {
  const m = /^\d{4}-(\d{2})-(\d{2})$/.exec(text);
  if (m === null) return false;
  const month = Number(m[1]);
  const day = Number(m[2]);
  return month >= 1 && month <= 12 && day >= 1 && day <= 31;
}
/** A search Central takes: words, no control or invisible character, 1 to 200 characters. */
export const validQuery = (q: string): boolean => q.trim() !== "" && shown(q, BOUNDS.query);
/** A gateway's name as a filter: 1 to 256 bytes of shown text. */
export const validGatewayName = (g: string): boolean => g !== "" && new TextEncoder().encode(g).length <= BOUNDS.said && shown(g, BOUNDS.said);
/** central-api's `valid_view`. */
export const validView = (name: string, query: string): boolean =>
  name.trim() !== "" && shown(name, BOUNDS.viewName) && query.length <= BOUNDS.viewQuery && /^[\x21-\x7e]*$/.test(query);

function oneOf<T extends string>(list: readonly T[], v: unknown): T {
  const found = list.find((x) => x === v);
  if (found === undefined) throw malformed();
  return found;
}

function id(v: unknown): Uint8Array {
  const out = bytes(v, 16);
  if (!isUuidV7(out)) throw malformed();
  return out;
}

const optional = <T>(v: unknown, read: (v: unknown) => T): T | null => (v === null ? null : read(v));

function bool(v: unknown): boolean {
  if (typeof v !== "boolean") throw malformed();
  return v;
}

function time(v: unknown): number {
  return safeInteger(v, 0);
}

/** Text of at most `max` characters, `lines` letting a tab and a newline through; `empty` allows "". */
function readText(v: unknown, max: number, { lines = false, empty = true }: { lines?: boolean; empty?: boolean } = {}): string {
  if (typeof v !== "string" || (!empty && v === "") || chars(v) > max) throw malformed();
  // what another person reads holds no control character: Central refuses one at intake
  if ((lines ? CONTROL_BUT_LINES : CONTROL).test(v)) throw malformed();
  return v;
}

function said(v: unknown): string {
  if (typeof v !== "string" || v === "" || new TextEncoder().encode(v).length > BOUNDS.said) throw malformed();
  return readText(v, BOUNDS.said);
}

function readLabel(v: unknown): string {
  if (typeof v !== "string" || !validLabel(v)) throw malformed();
  return v;
}

function readList<T>(v: unknown, max: number, read: (v: unknown) => T): T[] {
  if (!Array.isArray(v) || v.length > max) throw malformed();
  return v.map(read);
}

function readName(v: unknown): string | null {
  if (v === null) return null;
  if (typeof v !== "string" || !validDisplayName(v)) throw malformed();
  return v;
}

function tag(v: unknown): number | null {
  if (v === null) return null;
  if (typeof v !== "number" || !validTag(v)) throw malformed();
  return v;
}

const SUMMARY_KEYS = [
  "id",
  "created_at",
  "updated_at",
  "gateway",
  "gateway_says",
  "gateway_version",
  "channel",
  "reporter",
  "kind",
  "status",
  "excerpt",
  "build",
  "crash",
  "has_record",
  "record_complete",
  "record_origin",
  "record_bytes",
  "issue",
  "assignee",
  "priority",
  "labels",
  "duplicate_of",
  "signature",
  "revision",
  "done_at",
  "deleted_at",
  "seen",
  "notes",
] as const;

export function summary(v: unknown): ReportSummary {
  const o = exact(v, SUMMARY_KEYS);
  const reporter = o["reporter"];
  if (typeof reporter !== "string" || !validReporter(reporter)) throw malformed();
  const recordBytes = optional(o["record_bytes"], (b) => safeInteger(b, 1));
  if (recordBytes !== null && recordBytes > BOUNDS.recordGzip) throw malformed();
  const issue = optional(o["issue"], (n) => safeInteger(n, 1));
  if (issue !== null && issue > BOUNDS.issue) throw malformed();
  const signature = optional(o["signature"], (s) => {
    if (typeof s !== "string" || !validSignature(s)) throw malformed();
    return s;
  });
  return {
    id: id(o["id"]),
    createdAt: time(o["created_at"]),
    updatedAt: time(o["updated_at"]),
    gateway: said(o["gateway"]),
    gatewaySays: optional(o["gateway_says"], said),
    gatewayVersion: optional(o["gateway_version"], said),
    channel: oneOf(["gateway", "direct"] as const, o["channel"]),
    reporter,
    kind: oneOf(KINDS, o["kind"]),
    status: oneOf(STATUSES, o["status"]),
    excerpt: readText(o["excerpt"], BOUNDS.excerpt, { lines: true }),
    build: optional(o["build"], said),
    crash: optional(o["crash"], (c) => readText(c, BOUNDS.crashMessage, { lines: true })),
    hasRecord: bool(o["has_record"]),
    recordComplete: bool(o["record_complete"]),
    recordOrigin: optional(o["record_origin"], (r) => oneOf(["gateway", "client"] as const, r)),
    recordBytes,
    issue,
    assignee: optional(o["assignee"], id),
    priority: optional(o["priority"], (p) => oneOf(PRIORITIES, p)),
    labels: readList(o["labels"], BOUNDS.labels, readLabel),
    duplicateOf: optional(o["duplicate_of"], id),
    signature,
    revision: safeInteger(o["revision"], 0),
    doneAt: optional(o["done_at"], time),
    deletedAt: optional(o["deleted_at"], time),
    seen: bool(o["seen"]),
    notes: safeInteger(o["notes"], 0),
  };
}

function readNote(v: unknown): ReportNote {
  const o = exact(v, ["id", "at", "author", "text"]);
  return { id: id(o["id"]), at: time(o["at"]), author: id(o["author"]), text: readText(o["text"], BOUNDS.note, { lines: true, empty: false }) };
}

function auditRow(v: unknown): ReportAuditRow {
  const o = exact(v, ["at", "actor", "action", "detail"]);
  const action = o["action"];
  if (typeof action !== "string" || !/^report\.[a-z_]{1,32}$/.test(action)) throw malformed();
  return { at: time(o["at"]), actor: optional(o["actor"], id), action, detail: readText(o["detail"], 1_024) };
}

function partState(v: unknown, max: number): PartState {
  const o = exact(v, ["bytes", "received"]);
  const n = safeInteger(o["bytes"], 1);
  if (n > max) throw malformed();
  return { bytes: n, received: bool(o["received"]) };
}

/** Notes and history lines one report holds, at most: far more than a person writes. */
const MAX_ROWS = 10_000;

export function detail(v: unknown): ReportDetail {
  const o = exact(v, ["summary", "text", "game_id", "gateway_url", "commit", "crash_location", "client", "record", "notes", "audit"]);
  const commit = optional(o["commit"], (c) => {
    if (typeof c !== "string" || !/^[0-9a-fA-F]{1,64}$/.test(c)) throw malformed();
    return c;
  });
  const gameId = optional(o["game_id"], (g) => {
    if (typeof g !== "string" || g.length === 0 || g.length > BOUNDS.gameId || !/^[\x21-\x7e]+$/.test(g)) throw malformed();
    return g;
  });
  return {
    summary: summary(o["summary"]),
    text: readText(o["text"], BOUNDS.text, { lines: true }),
    gameId,
    gatewayUrl: optional(o["gateway_url"], said),
    commit,
    crashLocation: optional(o["crash_location"], (c) => readText(c, BOUNDS.crashLocation)),
    client: optional(o["client"], (p) => partState(p, BOUNDS.clientGzip)),
    record: optional(o["record"], (p) => partState(p, BOUNDS.recordGzip)),
    notes: readList(o["notes"], MAX_ROWS, readNote),
    audit: readList(o["audit"], MAX_ROWS, auditRow),
  };
}

function counts<T>(v: unknown, max: number, read: (v: unknown) => T): Count<T>[] {
  return readList(v, max, (c) => {
    const o = exact(c, ["value", "count"]);
    return { value: read(o["value"]), count: safeInteger(o["count"], 0) };
  });
}

/** Every value of a dimension Central counts, at most: the labels and gateways in use. */
const MAX_VALUES = 10_000;

export function facets(v: unknown): ReportFacets {
  const o = exact(v, ["statuses", "kinds", "priorities", "assignees", "labels", "gateways", "channels", "reporters", "total"]);
  return {
    statuses: counts(o["statuses"], STATUSES.length, (s) => oneOf(STATUSES, s)),
    kinds: counts(o["kinds"], KINDS.length, (k) => oneOf(KINDS, k)),
    priorities: counts(o["priorities"], PRIORITIES.length + 1, (p) => optional(p, (x) => oneOf(PRIORITIES, x))),
    assignees: counts(o["assignees"], MAX_VALUES, (a) => optional(a, id)),
    labels: counts(o["labels"], MAX_VALUES, readLabel),
    gateways: counts(o["gateways"], MAX_VALUES, said),
    channels: counts(o["channels"], 2, (c) => oneOf(["gateway", "direct"] as const, c)),
    reporters: counts(o["reporters"], BOUNDS.facetReporters, (r) => {
      if (typeof r !== "string" || !validReporter(r)) throw malformed();
      return r;
    }),
    total: safeInteger(o["total"], 0),
  };
}

function crashGroup(v: unknown): CrashGroup {
  const o = exact(v, ["signature", "count", "open", "first", "last", "latest", "message", "location"]);
  const signature = o["signature"];
  if (typeof signature !== "string" || !validSignature(signature)) throw malformed();
  return {
    signature,
    count: safeInteger(o["count"], 1),
    open: safeInteger(o["open"], 0),
    first: time(o["first"]),
    last: time(o["last"]),
    latest: id(o["latest"]),
    message: readText(o["message"], BOUNDS.crashMessage, { lines: true }),
    location: readText(o["location"], BOUNDS.crashLocation),
  };
}

function dayCount(v: unknown): DayCount {
  const o = exact(v, ["day", "kind", "count"]);
  const day = o["day"];
  if (typeof day !== "string" || !validDay(day)) throw malformed();
  return { day, kind: oneOf(KINDS, o["kind"]), count: safeInteger(o["count"], 0) };
}

function savedView(v: unknown): SavedView {
  const o = exact(v, ["id", "name", "query"]);
  const viewName = o["name"];
  const query = o["query"];
  if (typeof viewName !== "string" || typeof query !== "string" || !validView(viewName, query)) throw malformed();
  return { id: id(o["id"]), name: viewName, query };
}

function reportAdmin(v: unknown): ReportAdmin {
  const o = exact(v, ["id", "display_name", "tag"]);
  return { id: id(o["id"]), displayName: readName(o["display_name"]), tag: tag(o["tag"]) };
}

/** The answer of an admin read: `admin` with exactly `keys` in its `answer`. */
function admin(raw: unknown, kind: string, keys: readonly string[]): Json {
  const a = exact(answer(raw, "admin", ["answer"])["answer"], ["answer", ...keys]);
  if (a["answer"] !== kind) throw malformed();
  return a;
}

/** Administrators Central lists for assigning, at most. */
const MAX_ADMINS = 1_000;
/** Day and kind pairs of the statistics: 31 days of five kinds. */
const MAX_DAYS = 31 * KINDS.length;

export const parseReports = {
  page(raw: unknown, limit: number = BOUNDS.maxPage): { reports: ReportSummary[]; total: number } {
    const o = admin(raw, "reports", ["reports", "total"]);
    const reports = readList(o["reports"], limit, summary);
    const total = safeInteger(o["total"], 0);
    return { reports, total };
  },
  facets(raw: unknown): ReportFacets {
    return facets(admin(raw, "report_facets", ["facets"])["facets"]);
  },
  report(raw: unknown): ReportDetail {
    return detail(admin(raw, "report", ["report"])["report"]);
  },
  neighbours(raw: unknown): Neighbours {
    const o = exact(admin(raw, "neighbours", ["neighbours"])["neighbours"], ["previous", "next", "position", "total"]);
    return { previous: optional(o["previous"], id), next: optional(o["next"], id), position: optional(o["position"], (p) => safeInteger(p, 1)), total: safeInteger(o["total"], 0) };
  },
  crashGroups(raw: unknown): CrashGroup[] {
    return readList(admin(raw, "crash_groups", ["groups"])["groups"], BOUNDS.crashGroups, crashGroup);
  },
  stats(raw: unknown): DayCount[] {
    return readList(admin(raw, "report_stats", ["days"])["days"], MAX_DAYS, dayCount);
  },
  admins(raw: unknown): ReportAdmin[] {
    return readList(admin(raw, "report_admins", ["admins"])["admins"], MAX_ADMINS, reportAdmin);
  },
  views(raw: unknown): SavedView[] {
    return readList(admin(raw, "report_views", ["views"])["views"], BOUNDS.views, savedView);
  },
  changed(raw: unknown): number {
    return safeInteger(admin(raw, "changed", ["changed"])["changed"], 0);
  },
  done(raw: unknown): void {
    answer(raw, "done");
  },
};

// ---------------------------------------------------------------- requests

const arr = (b: Uint8Array): number[] => Array.from(b);

/** The filter on the wire: only the parts that are set, in Central's names. */
export function wireFilter(f: ReportFilter): Json {
  const out: Json = {};
  if (f.statuses !== undefined && f.statuses.length > 0) out["statuses"] = [...f.statuses];
  if (f.kinds !== undefined && f.kinds.length > 0) out["kinds"] = [...f.kinds];
  if (f.priorities !== undefined && f.priorities.length > 0) out["priorities"] = [...f.priorities];
  if (f.q !== undefined) out["q"] = f.q;
  if (f.from !== undefined) out["from"] = f.from;
  if (f.to !== undefined) out["to"] = f.to;
  if (f.hasRecord !== undefined) out["has_record"] = f.hasRecord;
  if (f.channel !== undefined) out["channel"] = f.channel;
  if (f.gateway !== undefined) out["gateway"] = f.gateway;
  if (f.assignee !== undefined) out["assignee"] = arr(f.assignee);
  if (f.unassigned === true) out["unassigned"] = true;
  if (f.label !== undefined) out["label"] = f.label;
  if (f.reporter !== undefined) out["reporter"] = f.reporter;
  if (f.signature !== undefined) out["signature"] = f.signature;
  if (f.duplicateOf !== undefined) out["duplicate_of"] = arr(f.duplicateOf);
  if (f.trash === true) out["trash"] = true;
  if (f.unseen === true) out["unseen"] = true;
  return out;
}

/** A change on the wire: a part the caller set, `null` where it clears; nothing else. */
export function wireChange(c: ReportChange): Json {
  const out: Json = {};
  if (c.status !== undefined) out["status"] = c.status;
  if (Object.hasOwn(c, "issue")) out["issue"] = c.issue ?? null;
  if (Object.hasOwn(c, "assignee")) out["assignee"] = c.assignee === null || c.assignee === undefined ? null : arr(c.assignee);
  if (Object.hasOwn(c, "priority")) out["priority"] = c.priority ?? null;
  if (c.labels !== undefined) out["labels"] = [...c.labels];
  if (c.labelsAdd !== undefined && c.labelsAdd.length > 0) out["labels_add"] = [...c.labelsAdd];
  if (c.labelsRemove !== undefined && c.labelsRemove.length > 0) out["labels_remove"] = [...c.labelsRemove];
  if (Object.hasOwn(c, "duplicateOf")) out["duplicate_of"] = c.duplicateOf === null || c.duplicateOf === undefined ? null : arr(c.duplicateOf);
  return out;
}

function wireBulk(b: BulkAction): Json {
  return b.do === "change" ? { do: "change", change: wireChange(b.change) } : { do: b.do };
}

export interface ReportsApi {
  reports(list: ListRequest): Promise<{ reports: ReportSummary[]; total: number }>;
  facets(filter: ReportFilter): Promise<ReportFacets>;
  report(report: Uint8Array): Promise<ReportDetail>;
  neighbours(report: Uint8Array, filter: ReportFilter, sort: ReportSort, descending: boolean): Promise<Neighbours>;
  crashGroups(filter: ReportFilter): Promise<CrashGroup[]>;
  stats(): Promise<DayCount[]>;
  admins(): Promise<ReportAdmin[]>;
  /** `expectedRevision`: the revision it was read at; a newer one answers `conflict`. */
  setReport(report: Uint8Array, change: ReportChange, expectedRevision: number | null): Promise<ReportDetail>;
  /** How many of `reports` the action changed, all in one transaction. A purge needs a fresh password. */
  bulk(reports: readonly Uint8Array[], action: BulkAction): Promise<number>;
  trash(report: Uint8Array): Promise<void>;
  restore(report: Uint8Array): Promise<void>;
  /** Deleted for good, from the trash only; needs a fresh password. */
  purge(report: Uint8Array): Promise<void>;
  addNote(report: Uint8Array, text: string): Promise<ReportDetail>;
  removeNote(report: Uint8Array, note: Uint8Array): Promise<ReportDetail>;
  views(): Promise<SavedView[]>;
  saveView(name: string, query: string): Promise<SavedView[]>;
  deleteView(view: Uint8Array): Promise<SavedView[]>;
  setSeen(reports: readonly Uint8Array[], seen: boolean): Promise<void>;
  /** A part as stored, gzip, at most `max` bytes; `null` origin for the client part. */
  part(report: Uint8Array, part: Part, max: number): Promise<{ gzip: Uint8Array; origin: RecordOrigin | null }>;
}

const request = (r: Json): Json => ({ operation: "admin", request: r });
const one = (action: string, report: Uint8Array): Json => request({ action, report: arr(report) });

/** Reads `response`'s body, refusing it past `max` bytes before or while it comes. */
export async function readBounded(response: Response, max: number): Promise<Uint8Array> {
  const declared = response.headers.get("content-length");
  if (declared !== null && Number(declared) > max) throw new ApiError("malformed");
  const reader = response.body?.getReader();
  if (reader === undefined) return new Uint8Array(0);
  const chunks: Uint8Array[] = [];
  let total = 0;
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    total += value.byteLength;
    if (total > max) {
      await reader.cancel().catch(() => {});
      throw new ApiError("malformed");
    }
    chunks.push(value);
  }
  const out = new Uint8Array(total);
  let at = 0;
  for (const chunk of chunks) {
    out.set(chunk, at);
    at += chunk.byteLength;
  }
  return out;
}

const STATUS: Readonly<Record<number, ApiError["kind"]>> = { 401: "rejected", 403: "forbidden", 409: "conflict", 429: "busy", 503: "unavailable" };

export function reportsApi({ base = CENTRAL_API, fetch: doFetch = (...a) => fetch(...a) }: Endpoint = {}): ReportsApi {
  const post = poster({ base, fetch: doFetch });
  return {
    reports: (l) =>
      post(request({ action: "reports", filter: wireFilter(l.filter), sort: l.sort, descending: l.descending, offset: l.offset, limit: l.limit }), (raw) =>
        parseReports.page(raw, l.limit),
      ),
    facets: (filter) => post(request({ action: "report_facets", filter: wireFilter(filter) }), parseReports.facets),
    report: (r) => post(one("report", r), parseReports.report),
    neighbours: (r, filter, sort, descending) =>
      post(request({ action: "report_neighbours", report: arr(r), filter: wireFilter(filter), sort, descending }), parseReports.neighbours),
    crashGroups: (filter) => post(request({ action: "crash_groups", filter: wireFilter(filter) }), parseReports.crashGroups),
    stats: () => post(request({ action: "report_stats" }), parseReports.stats),
    admins: () => post(request({ action: "report_admins" }), parseReports.admins),
    setReport: (r, change, expectedRevision) =>
      post(request({ action: "set_report", report: arr(r), change: wireChange(change), expected_revision: expectedRevision }), parseReports.report),
    bulk: (reports, action) => post(request({ action: "reports_bulk", reports: reports.map(arr), bulk: wireBulk(action) }), parseReports.changed),
    trash: (r) => post(one("trash_report", r), parseReports.done),
    restore: (r) => post(one("restore_report", r), parseReports.done),
    purge: (r) => post(one("purge_report", r), parseReports.done),
    addNote: (r, note) => post(request({ action: "add_report_note", report: arr(r), text: note }), parseReports.report),
    removeNote: (r, n) => post(request({ action: "remove_report_note", report: arr(r), note: arr(n) }), parseReports.report),
    views: () => post(request({ action: "report_views" }), parseReports.views),
    saveView: (viewName, query) => post(request({ action: "save_report_view", name: viewName, query }), parseReports.views),
    deleteView: (v) => post(request({ action: "delete_report_view", view: arr(v) }), parseReports.views),
    setSeen: (reports, seen) => post(request({ action: "set_reports_seen", reports: reports.map(arr), seen }), parseReports.done),
    // `POST {base}/v1/reports/<hex>/<part>` with the browser surface's own
    // headers and an empty body: the gzip as stored (crates/central-http
    // browser.rs `report_part`), read no further than `max`
    async part(r, part, max) {
      let response: Response;
      try {
        response = await doFetch(`${base}/v1/reports/${idText(r)}/${part}`, {
          method: "POST",
          credentials: "same-origin",
          referrerPolicy: "no-referrer",
          cache: "no-store",
          redirect: "error",
          headers: { "Content-Type": "application/json", "X-Baylee-Central": "1" },
        });
      } catch {
        throw new ApiError("network");
      }
      if (!response.ok) throw new ApiError(STATUS[response.status] ?? (response.status >= 500 ? "unavailable" : "malformed"));
      if (response.headers.get("content-type") !== "application/gzip") throw new ApiError("malformed");
      const origin = response.headers.get("x-record-origin");
      if (part === "client" ? origin !== null : origin !== "gateway" && origin !== "client") throw new ApiError("malformed");
      const gzip = await readBounded(response, max);
      // gzip's magic, which Central checked at intake
      if (gzip.length < 18 || gzip[0] !== 0x1f || gzip[1] !== 0x8b) throw new ApiError("malformed");
      return { gzip, origin: origin === "gateway" || origin === "client" ? origin : null };
    },
  };
}

/** Whether `v` is a JSON object (re-exported for the client part's readers). */
export { isObject };
