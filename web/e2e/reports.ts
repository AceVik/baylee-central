// Bug and crash reports for the e2e mock (`area.ts`): a few synthetic
// reports in central-api's exact wire shapes, the triage requests answered
// as Central rules them (a stale revision is 409, notes removed by their
// author only, the trash), and the two raw parts as gzip. Every request's
// fields are checked against `AdminRequest`'s (it denies unknown fields), so
// a page that sends one too many or too few fails its test. Synthetic data
// and example.invalid addresses only.

import { gzipSync } from "node:zlib";

const now = () => Math.floor(Date.now() / 1000);
const DAY = 86_400;

/** A report's id: UUIDv7-shaped, its last byte `n`. */
export const rid = (n: number): number[] => [0x01, 0x92, 0xd0, 0xc5, 0x7b, 0x50, 0x7a, 0x1f, 0x8c, 0x2d, 0x3e, 0x4f, 0x5a, 0x6b, 0x7c, n];
export const hexOf = (id: number[]): string => id.map((b) => b.toString(16).padStart(2, "0")).join("");

/** What each report action holds besides `action` (central-api `AdminRequest`). */
export const REPORT_FIELDS: Readonly<Record<string, readonly string[]>> = {
  reports: ["filter", "sort", "descending", "offset", "limit"],
  report_facets: ["filter"],
  report: ["report"],
  report_neighbours: ["report", "filter", "sort", "descending"],
  crash_groups: ["filter"],
  report_stats: [],
  report_admins: [],
  set_report: ["report", "change", "expected_revision"],
  reports_bulk: ["reports", "bulk"],
  trash_report: ["report"],
  restore_report: ["report"],
  purge_report: ["report"],
  add_report_note: ["report", "text"],
  remove_report_note: ["report", "note"],
  report_views: [],
  save_report_view: ["name", "query"],
  delete_report_view: ["view"],
  set_reports_seen: ["reports", "seen"],
};

const FILTER = new Set(["statuses", "kinds", "priorities", "q", "from", "to", "has_record", "channel", "gateway", "assignee", "unassigned", "label", "reporter", "signature", "duplicate_of", "trash", "unseen"]);
const CHANGE = new Set(["status", "issue", "assignee", "priority", "labels", "labels_add", "labels_remove", "duplicate_of"]);

/** Why `request` is not one Central reads, or null. */
export function reportShape(request: Record<string, unknown>): string | null {
  const action = String(request["action"]);
  const fields = REPORT_FIELDS[action];
  if (fields === undefined) return null;
  const keys = Object.keys(request).filter((k) => k !== "action").sort();
  if (keys.join() !== [...fields].sort().join()) return `${action} has fields ${keys.join()}`;
  const filter = request["filter"];
  if (filter !== undefined) {
    if (typeof filter !== "object" || filter === null) return `${action}.filter is not an object`;
    const unknown = Object.keys(filter).filter((k) => !FILTER.has(k));
    if (unknown.length > 0) return `${action}.filter has ${unknown.join()}`;
  }
  const change = (request["change"] ?? (request["bulk"] as { change?: unknown } | undefined)?.change) as Record<string, unknown> | undefined;
  if (change !== undefined) {
    const unknown = Object.keys(change).filter((k) => !CHANGE.has(k));
    if (unknown.length > 0 || Object.keys(change).length === 0) return `${action}.change has ${Object.keys(change).join()}`;
  }
  return null;
}

interface Row {
  summary: Record<string, unknown> & { id: number[]; status: string; kind: string; seen: boolean; revision: number; deleted_at: number | null; labels: string[]; assignee: number[] | null };
  text: string;
  notes: { id: number[]; at: number; author: number[]; text: string }[];
  audit: { at: number; actor: number[] | null; action: string; detail: string }[];
  client: boolean;
  record: boolean;
}

export interface ReportsState {
  rows: Row[];
  views: { id: number[]; name: string; query: string }[];
  /** The administrators, by id: the signed-in one first. */
  admins: { id: number[]; display_name: string | null; tag: number | null }[];
  next: number;
}

const KINDS = ["bug", "crash", "improvement", "feedback", "other"];
const STATUSES = ["new", "new", "triaged", "in_progress", "resolved", "new", "wont_fix", "new"];

export function reportsState(admins: ReportsState["admins"]): ReportsState {
  const t = now();
  const rows: Row[] = Array.from({ length: 8 }, (_, i) => {
    const kind = KINDS[i % KINDS.length] ?? "bug";
    const status = STATUSES[i] ?? "new";
    const crash = kind === "crash";
    return {
      summary: {
        id: rid(i + 1),
        created_at: t - (8 - i) * 3 * 3_600,
        updated_at: t - (8 - i) * 3_600,
        gateway: i === 3 ? "Moonlit Table" : "Official",
        gateway_says: i === 3 ? "Moonlit Table" : "Sanctuary",
        gateway_version: "0.1.1-beta.9b",
        channel: i === 6 ? "direct" : "gateway",
        reporter: ["5bdc0a17", "9e2f0011", "a1b2c3d4"][i % 3] ?? "5bdc",
        kind,
        status,
        excerpt:
          [
            "The stack resolved twice after I countered a [Lightning Bolt]",
            "The client closed itself while I was choosing attackers",
            "Could the hand fan out a little wider on small screens?",
            "Lovely table colours, thank you!",
            "Mana from a land tapped for black went to the wrong pool",
            "A very long report about the mulligan screen that keeps going and going to show how a long first line is cut in the list without stretching the table past the screen",
            "Direct report from a client signed in nowhere",
            "Spectator view froze for a few seconds",
          ][i] ?? "",
        build: "0.1.1-beta.9b",
        crash: crash ? "index 7 out of range for slice of length 3" : null,
        has_record: i % 2 === 0,
        record_complete: i === 0,
        record_origin: i % 2 === 0 ? (i === 6 ? "client" : "gateway") : null,
        record_bytes: i % 2 === 0 ? 180_000 + i * 1_000 : null,
        issue: i === 4 ? 311 : null,
        assignee: i === 2 || i === 3 ? (admins[0]?.id ?? null) : null,
        priority: (["high", "critical", null, "normal", "low", null, null, null] as const)[i] ?? null,
        labels: i === 0 ? ["rules", "stack"] : i === 4 ? ["mana"] : [],
        duplicate_of: null,
        signature: crash ? "5f0c3a9e1b2d4c6e8f0a1b2c3d4e5f60" : null,
        revision: 1,
        done_at: status === "resolved" || status === "wont_fix" ? t - DAY : null,
        deleted_at: null,
        seen: i > 1,
        notes: i === 2 ? 1 : 0,
      },
      text: "",
      notes: i === 2 ? [{ id: rid(0x40), at: t - 5_000, author: admins[1]?.id ?? rid(0x41), text: "Seen it on a small laptop too." }] : [],
      audit: [{ at: t - (8 - i) * 3 * 3_600, actor: null, action: "report.filed", detail: i === 3 ? "by Moonlit Table" : "by Official" }],
      client: i === 0 || i === 1,
      record: i % 2 === 0,
    };
  });
  for (const r of rows) r.text = `${String(r.summary["excerpt"])}\n\nIt happened in the second turn, right after the opponent passed.`;
  return { rows, views: [], admins, next: 0x80 };
}

const admin = (answer: string, rest: Record<string, unknown>) => ({ result: "admin", answer: { answer, ...rest } });
const DONE = { result: "done" };
const same = (a: unknown, b: number[]) => Array.isArray(a) && a.join() === b.join();

function matches(f: Record<string, unknown>, r: Row): boolean {
  const s = r.summary;
  const has = (k: string) => Object.hasOwn(f, k);
  if ((f["trash"] === true) !== (s.deleted_at !== null)) return false;
  if (has("statuses") && !(f["statuses"] as string[]).includes(s.status)) return false;
  if (has("kinds") && !(f["kinds"] as string[]).includes(s.kind)) return false;
  if (has("priorities") && !(f["priorities"] as (string | null)[]).includes(s["priority"] as string | null)) return false;
  if (has("q") && !`${r.text} ${String(s["crash"] ?? "")} ${hexOf(s.id)}`.toLowerCase().includes(String(f["q"]).toLowerCase())) return false;
  if (has("label") && !s.labels.includes(String(f["label"]))) return false;
  if (has("gateway") && s["gateway"] !== f["gateway"]) return false;
  if (has("channel") && s["channel"] !== f["channel"]) return false;
  if (has("reporter") && s["reporter"] !== f["reporter"]) return false;
  if (has("signature") && s["signature"] !== f["signature"]) return false;
  if (has("has_record") && s["has_record"] !== f["has_record"]) return false;
  if (f["unassigned"] === true && s.assignee !== null) return false;
  if (has("assignee") && (s.assignee === null || !same(f["assignee"], s.assignee))) return false;
  if (has("duplicate_of") && !same(f["duplicate_of"], (s["duplicate_of"] as number[] | null) ?? [])) return false;
  if (f["unseen"] === true && s.seen) return false;
  return true;
}

const ORDER: Record<string, (r: Row) => string | number> = {
  received: (r) => Number(r.summary["created_at"]),
  changed: (r) => Number(r.summary["updated_at"]),
  priority: (r) => ["low", "normal", "high", "critical"].indexOf(String(r.summary["priority"])),
  status: (r) => r.summary.status,
  kind: (r) => r.summary.kind,
  reporter: (r) => String(r.summary["reporter"]),
  gateway: (r) => String(r.summary["gateway"]),
};

function sorted(rows: Row[], sort: unknown, descending: unknown): Row[] {
  const key = ORDER[String(sort)] ?? ORDER["received"];
  const out = rows.toSorted((a, b) => {
    const x = key?.(a) ?? 0;
    const y = key?.(b) ?? 0;
    return x < y ? -1 : x > y ? 1 : 0;
  });
  return descending === true ? out.toReversed() : out;
}

function counted<T>(values: T[]): { value: T; count: number }[] {
  const out = new Map<string, { value: T; count: number }>();
  for (const v of values) {
    const k = JSON.stringify(v);
    const c = out.get(k) ?? { value: v, count: 0 };
    c.count += 1;
    out.set(k, c);
  }
  return [...out.values()];
}

const detail = (r: Row) => ({
  summary: r.summary,
  text: r.text,
  game_id: r.summary["channel"] === "direct" ? null : "0192d0c5-7b3e-7a1f",
  gateway_url: r.summary["channel"] === "direct" ? null : "https://gateway.example.invalid",
  commit: "3f9a1c7e21",
  crash_location: r.summary["crash"] === null ? null : "crates/engine/src/stack.rs:120:9",
  client: r.client ? { bytes: 2_048, received: true } : null,
  record: r.record ? { bytes: Number(r.summary["record_bytes"]), received: true } : null,
  notes: r.notes,
  audit: r.audit,
});

function bump(r: Row, actor: number[], action: string, detailText: string, change: Record<string, unknown> = {}) {
  Object.assign(r.summary, change);
  r.summary.revision += 1;
  r.summary["updated_at"] = now();
  r.audit.push({ at: now(), actor, action: `report.${action}`, detail: detailText });
}

/** The answer to a report action, a status to refuse it with, or undefined for another action. */
export function reportsAnswer(s: ReportsState, request: Record<string, unknown>): unknown {
  const me = s.admins[0]?.id ?? rid(0);
  const find = (v: unknown) => s.rows.find((r) => same(v, r.summary.id));
  const filter = (request["filter"] ?? {}) as Record<string, unknown>;
  switch (request["action"]) {
    case "reports": {
      const all = sorted(s.rows.filter((r) => matches(filter, r)), request["sort"], request["descending"]);
      const offset = Number(request["offset"]);
      const limit = Number(request["limit"]);
      if (!(limit >= 1 && limit <= 200)) return 401;
      return admin("reports", { reports: all.slice(offset, offset + limit).map((r) => r.summary), total: all.length });
    }
    case "report_facets": {
      const under = (without: string) => {
        const { [without]: _own, ...rest } = filter;
        return s.rows.filter((r) => matches(rest, r)).map((r) => r.summary);
      };
      return admin("report_facets", {
        facets: {
          statuses: counted(under("statuses").map((r) => r.status)),
          kinds: counted(under("kinds").map((r) => r.kind)),
          priorities: counted(under("priorities").map((r) => r["priority"] ?? null)),
          assignees: counted(under("assignee").map((r) => r.assignee)),
          labels: counted(under("label").flatMap((r) => r.labels)),
          gateways: counted(under("gateway").map((r) => r["gateway"])),
          channels: counted(under("channel").map((r) => r["channel"])),
          reporters: counted(under("reporter").map((r) => r["reporter"])),
          total: s.rows.filter((r) => matches(filter, r)).length,
        },
      });
    }
    case "report": {
      const r = find(request["report"]);
      return r === undefined ? 401 : admin("report", { report: detail(r) });
    }
    case "report_neighbours": {
      const all = sorted(s.rows.filter((r) => matches(filter, r)), request["sort"], request["descending"]);
      const at = all.findIndex((r) => same(request["report"], r.summary.id));
      return admin("neighbours", {
        neighbours: { previous: all[at - 1]?.summary.id ?? null, next: at < 0 ? null : (all[at + 1]?.summary.id ?? null), position: at < 0 ? null : at + 1, total: all.length },
      });
    }
    case "crash_groups": {
      const crashes = s.rows.filter((r) => matches(filter, r) && r.summary["signature"] !== null);
      const first = crashes[0];
      return admin("crash_groups", {
        groups:
          first === undefined
            ? []
            : [
                {
                  signature: first.summary["signature"],
                  count: crashes.length,
                  open: crashes.filter((r) => ["new", "triaged", "in_progress"].includes(r.summary.status)).length,
                  first: first.summary["created_at"],
                  last: crashes.at(-1)?.summary["created_at"],
                  latest: crashes.at(-1)?.summary.id,
                  message: first.summary["crash"],
                  location: "crates/engine/src/stack.rs",
                },
              ],
      });
    }
    case "report_stats": {
      const days = Array.from({ length: 30 }, (_, i) => new Date((now() - (29 - i) * DAY) * 1000).toISOString().slice(0, 10));
      return admin("report_stats", {
        days: days.flatMap((day, i) => [
          ...(i % 3 === 0 ? [{ day, kind: "bug", count: 1 + (i % 4) }] : []),
          ...(i % 5 === 0 ? [{ day, kind: "crash", count: 1 }] : []),
          ...(i % 7 === 2 ? [{ day, kind: "feedback", count: 2 }] : []),
        ]),
      });
    }
    case "report_admins":
      return admin("report_admins", { admins: s.admins });
    case "set_report": {
      const r = find(request["report"]);
      if (r === undefined) return 401;
      const expected = request["expected_revision"];
      if (expected !== null && expected !== r.summary.revision) return 409;
      const c = request["change"] as Record<string, unknown>;
      const next: Record<string, unknown> = {};
      for (const k of ["status", "issue", "assignee", "priority", "labels", "duplicate_of"]) if (Object.hasOwn(c, k)) next[k] = c[k];
      if (Object.hasOwn(c, "duplicate_of")) next["status"] = c["duplicate_of"] === null ? "triaged" : "duplicate";
      if (next["status"] !== undefined && next["status"] !== "duplicate") next["duplicate_of"] = null;
      bump(r, me, Object.keys(c)[0] ?? "status", "", next);
      return admin("report", { report: detail(r) });
    }
    case "reports_bulk": {
      const ids = request["reports"] as number[][];
      const bulk = request["bulk"] as { do: string; change?: Record<string, unknown> };
      let changed = 0;
      for (const id of ids) {
        const r = find(id);
        if (r === undefined) return 401;
        if (bulk.do === "trash" && r.summary.deleted_at === null) bump(r, me, "trashed", "", { deleted_at: now() });
        else if (bulk.do === "restore" && r.summary.deleted_at !== null) bump(r, me, "restored", "", { deleted_at: null });
        else if (bulk.do === "purge" && r.summary.deleted_at !== null) s.rows.splice(s.rows.indexOf(r), 1);
        else if (bulk.do === "change" && bulk.change !== undefined) {
          const c = bulk.change;
          const next: Record<string, unknown> = {};
          for (const k of ["status", "issue", "assignee", "priority", "duplicate_of"]) if (Object.hasOwn(c, k)) next[k] = c[k];
          next["labels"] = [...new Set([...r.summary.labels, ...((c["labels_add"] as string[] | undefined) ?? [])])].filter((l) => !((c["labels_remove"] as string[] | undefined) ?? []).includes(l)).sort();
          bump(r, me, "status", "", next);
        } else continue;
        changed += 1;
      }
      return admin("changed", { changed });
    }
    case "trash_report":
    case "restore_report":
    case "purge_report": {
      const r = find(request["report"]);
      if (r === undefined) return 401;
      if (request["action"] === "trash_report" && r.summary.deleted_at === null) bump(r, me, "trashed", "", { deleted_at: now() });
      if (request["action"] === "restore_report" && r.summary.deleted_at !== null) bump(r, me, "restored", "", { deleted_at: null });
      if (request["action"] === "purge_report") {
        if (r.summary.deleted_at === null) return 401;
        s.rows.splice(s.rows.indexOf(r), 1);
      }
      return DONE;
    }
    case "add_report_note": {
      const r = find(request["report"]);
      const text = String(request["text"]);
      if (r === undefined || text.trim() === "" || [...text].length > 4_000) return 401;
      r.notes.push({ id: rid(s.next++), at: now(), author: me, text });
      bump(r, me, "note", "added", { notes: r.notes.length });
      return admin("report", { report: detail(r) });
    }
    case "remove_report_note": {
      const r = find(request["report"]);
      const n = r?.notes.find((x) => same(request["note"], x.id));
      if (r === undefined || n === undefined || !same(n.author, me)) return 401;
      r.notes.splice(r.notes.indexOf(n), 1);
      bump(r, me, "note", "removed", { notes: r.notes.length });
      return admin("report", { report: detail(r) });
    }
    case "report_views":
      return admin("report_views", { views: s.views });
    case "save_report_view": {
      const name = String(request["name"]);
      const query = String(request["query"]);
      if (!/^[\x21-\x7e]*$/.test(query) || name.trim() === "" || s.views.length >= 50) return 401;
      const named = s.views.find((v) => v.name === name);
      if (named !== undefined) named.query = query;
      else s.views.push({ id: rid(s.next++), name, query });
      return admin("report_views", { views: s.views });
    }
    case "delete_report_view":
      s.views = s.views.filter((v) => !same(request["view"], v.id));
      return admin("report_views", { views: s.views });
    case "set_reports_seen":
      for (const id of request["reports"] as number[][]) {
        const r = find(id);
        if (r !== undefined) r.summary.seen = request["seen"] === true;
      }
      return DONE;
  }
  return undefined;
}

/** A PNG of 2 × 2 pixels, for the screenshot inside a client part. */
const PNG = "iVBORw0KGgoAAAANSUhEUgAAAAIAAAACCAIAAAD91JpzAAAAFklEQVR4nGNgYPj/n4GBgYGJAQoAADsABgFpOvMAAAAASUVORK5CYII=";

/** A report's part as Central keeps it: gzip, and the record's origin; null when it has none. */
export function reportPart(s: ReportsState, hex: string, part: string): { gzip: Buffer; origin: string | null } | null {
  const r = s.rows.find((x) => hexOf(x.summary.id) === hex);
  if (r === undefined) return null;
  if (part === "record" && r.record) {
    const lines = [{ seq: 1, event: "game_started" }, { seq: 2, event: "turn", n: 1 }].map((l) => JSON.stringify(l)).join("\n");
    return { gzip: gzipSync(Buffer.from(lines)), origin: r.summary["record_origin"] === "client" ? "client" : "gateway" };
  }
  if (part === "client" && r.client) {
    const client = {
      build: { version: "0.1.1-beta.9b", commit: "3f9a1c7e21" },
      system: { platform: "macOS 15", cpus: 8, adapter: "Synthetic GPU", backend: "Metal", window: [1280, 800], scale: 2, lang: "en" },
      screenshot: { width: 2, height: 2, png_base64: PNG },
      refs: {
        cards: [{ text: "Lightning Bolt", at: [43, 59], card: 17, zone: "stack", owner: 1, print: { scryfall_id: "77c6fa74-5543-42ac-9ead-0e890b188e99" } }],
        players: [],
      },
      game: { table: { seat: 0, seq: 214, when: "Turn 2 · Main 1" }, view: { turn: 2, active: 1, players: [{}, {}] }, pending: { ChooseAttackers: { count: 2 } } },
      log: { seat: 0, roster: [{ seat: 0, name: "Lantern", is_ai: false }, { seat: 1, name: "House AI", is_ai: true }], lines: [{ turn: 2, at: 0, text: "Lightning Bolt was countered." }] },
      settings: { reduce_motion: false, volume: 0.4 },
    };
    return { gzip: gzipSync(Buffer.from(JSON.stringify(client))), origin: null };
  }
  return null;
}
