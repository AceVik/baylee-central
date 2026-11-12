import { deflateSync, gzipSync } from "node:zlib";

import { expect, type Page, type Route } from "@playwright/test";

import { narrow, schemes, targets } from "./check.ts";

// A stand-in for Central's report moderation (central-api `AdminRequest`'s
// report actions, `docs/reports.md`), shared by feedback.spec.ts and
// feedback-report.spec.ts: answers in Central's exact shapes, refusals by
// status only, as crates/central-postgres `reports/triage.rs` decides them
// (a purge outside the trash or with a stale sign-in is the same 401). All
// of it is synthetic: the reports, the administrators, the pseudonyms.
//
// Every report's words carry markup and script, so a test can tell that
// the page draws them as text: `HOSTILE` must never run and never become
// an element.

export const id = (n: number) => Array.from({ length: 16 }, (_, i) => ({ 6: 0x70, 8: 0x80, 15: n })[i] ?? 0x10 + i);
export const hex = (bytes: number[]) => bytes.map((b) => b.toString(16).padStart(2, "0")).join("");
export const same = (a: unknown, b: number[]) => Array.isArray(a) && a.length === b.length && a.every((v, i) => v === b[i]);

/** Markup in a report: if it ever runs or becomes an element, a test sees it. */
export const HOSTILE = `<img src=x onerror="window.__pwned=1"><script>window.__pwned=1</script>`;
/** The same as one token (no space), for fields Central keeps without spaces. */
export const HOSTILE_TOKEN = `<script>window.__pwned=1</script>`;
/** A hostile report's words: markup, a card and a player the client resolved. */
export const HOSTILE_TEXT = `${HOSTILE} first line\nand [Llanowar Elves] by [@${HOSTILE_TOKEN}]`;

export const ME = id(1);
export const OTHER_ADMIN = id(2);
export const THEN = 1_794_000_000;

type Kind = "bug" | "improvement" | "feedback" | "crash" | "other";
type Status = "new" | "triaged" | "in_progress" | "resolved" | "wont_fix" | "duplicate";
type Priority = "critical" | "high" | "normal" | "low";

export type Summary = {
  id: number[];
  created_at: number;
  updated_at: number;
  gateway: string;
  gateway_says: string | null;
  gateway_version: string | null;
  channel: "gateway" | "direct";
  reporter: string;
  kind: Kind;
  status: Status;
  excerpt: string;
  build: string | null;
  crash: string | null;
  has_record: boolean;
  record_complete: boolean;
  record_origin: "gateway" | "client" | null;
  record_bytes: number | null;
  issue: number | null;
  assignee: number[] | null;
  priority: Priority | null;
  labels: string[];
  duplicate_of: number[] | null;
  signature: string | null;
  revision: number;
  done_at: number | null;
  deleted_at: number | null;
  seen: boolean;
  notes: number;
};

export type Note = { id: number[]; at: number; author: number[]; text: string };
export type Audit = { at: number; actor: number[] | null; action: string; detail: string };
export type Report = {
  summary: Summary;
  text: string;
  game_id: string | null;
  gateway_url: string | null;
  commit: string | null;
  crash_location: string | null;
  client: { bytes: number; received: boolean } | null;
  record: { bytes: number; received: boolean } | null;
  notes: Note[];
  audit: Audit[];
};

const SIGNATURE = "0123456789abcdef0123456789abcdef";

export function report(n: number, more: Partial<Summary> = {}, detail: Partial<Omit<Report, "summary">> = {}): Report {
  const summary: Summary = {
    id: id(n),
    created_at: THEN - n * 3600,
    updated_at: THEN - n * 3600,
    gateway: "Synthetic gateway",
    gateway_says: null,
    gateway_version: "0.1.1-beta.9b",
    channel: "gateway",
    reporter: `5bdc${n.toString(16).padStart(4, "0")}`,
    kind: "bug",
    status: "new",
    excerpt: `Report number ${n}`,
    build: "0.1.1-beta.9b",
    crash: null,
    has_record: false,
    record_complete: false,
    record_origin: null,
    record_bytes: null,
    issue: null,
    assignee: null,
    priority: null,
    labels: [],
    duplicate_of: null,
    signature: null,
    revision: 1,
    done_at: null,
    deleted_at: null,
    seen: false,
    notes: 0,
    ...more,
  };
  return {
    summary,
    text: summary.excerpt,
    game_id: null,
    gateway_url: null,
    commit: null,
    crash_location: null,
    client: null,
    record: null,
    notes: [],
    audit: [{ at: summary.created_at, actor: null, action: "report.filed", detail: "by gateway" }],
    ...detail,
  };
}

/** A hostile report: markup and script in every text field Central keeps. */
export function hostile(n: number): Report {
  return report(
    n,
    {
      excerpt: `${HOSTILE} first line`,
      gateway: `Gate ${HOSTILE}`,
      gateway_says: `Says ${HOSTILE}`,
      gateway_version: `9 ${HOSTILE}`,
      build: `build ${HOSTILE}`,
      crash: `panicked ${HOSTILE}`,
      kind: "crash",
      signature: SIGNATURE,
      labels: ["ui"],
      notes: 1,
    },
    {
      text: HOSTILE_TEXT,
      game_id: HOSTILE_TOKEN,
      gateway_url: `javascript:alert(1)//${HOSTILE}`,
      commit: "3f9a1c7e21",
      crash_location: `src/${HOSTILE_TOKEN}.rs:1:2`,
      client: { bytes: 2000, received: true },
      notes: [{ id: id(80), at: THEN, author: OTHER_ADMIN, text: `note ${HOSTILE}` }],
      audit: [
        { at: THEN - 7200, actor: null, action: "report.filed", detail: "by gateway" },
        { at: THEN - 3600, actor: OTHER_ADMIN, action: "report.labels", detail: HOSTILE },
        { at: THEN - 1800, actor: OTHER_ADMIN, action: `report.${"x".repeat(3)}`, detail: HOSTILE },
      ],
    },
  );
}

/** The reports, newest first: one of each kind of row the list draws. */
export function REPORTS(): Report[] {
  return [
    hostile(10),
    report(11, { status: "triaged", priority: "high", assignee: ME, labels: ["crash-loop", "ui"], seen: true, issue: 311 }),
    report(
      12,
      { status: "in_progress", kind: "crash", crash: "index out of bounds", signature: SIGNATURE, assignee: OTHER_ADMIN, record_bytes: 4000, has_record: true, record_origin: "client" },
      { record: { bytes: 4000, received: true }, client: { bytes: 900, received: false } },
    ),
    report(13, { status: "resolved", kind: "improvement", channel: "direct", priority: "low" }),
    report(14, { status: "duplicate", duplicate_of: id(12) }),
    report(15, { kind: "feedback", excerpt: "=HYPERLINK(\"https://x.invalid\",\"click\")" }),
    report(16, { deleted_at: THEN - 60 }),
    report(17, { deleted_at: THEN - 120, kind: "other" }),
  ];
}

export const ADMINS = [
  { id: ME, display_name: "Chef", tag: 0x0c4e },
  { id: OTHER_ADMIN, display_name: "Ada", tag: 0x3c4d },
];

type View = { id: number[]; name: string; query: string };
type Body = { operation?: string; request?: Record<string, unknown> } & Record<string, unknown>;
export type Asked = Record<string, unknown>;

export type World = {
  reports: Report[];
  views: View[];
  admin: boolean;
  stale: boolean;
  gone: boolean;
  /** How the next request of this action ends instead: a status, or no answer after it was done. */
  next: Record<string, number | "lost">;
  /** The client part's JSON, or a status for its route. */
  client: unknown;
  record: Buffer;
  /** Parts asked for, by report and part. */
  parts: string[];
};

export function world(more: Partial<World> = {}): World {
  return {
    reports: REPORTS(),
    views: [{ id: id(90), name: `Mine ${HOSTILE}`, query: "status=new%2Ctriaged%2Cin_progress&assignee=" + hex(ME) }],
    admin: true,
    stale: false,
    gone: false,
    next: {},
    client: CLIENT(),
    record: gzipSync(Buffer.from('{"seq":1}\n')),
    parts: [],
    ...more,
  };
}

// ---------------------------------------------------------------- the client part

function crc32(bytes: Uint8Array): number {
  let c = 0xffffffff;
  for (const b of bytes) {
    c ^= b;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  }
  return (c ^ 0xffffffff) >>> 0;
}

/** A real PNG, `w` × `h`, of one colour. */
export function png(w: number, h: number): Buffer {
  const chunk = (type: string, data: Buffer) => {
    const head = Buffer.alloc(8);
    head.writeUInt32BE(data.length, 0);
    head.write(type, 4, "ascii");
    const crc = Buffer.alloc(4);
    crc.writeUInt32BE(crc32(Buffer.concat([Buffer.from(type, "ascii"), data])), 0);
    return Buffer.concat([head, data, crc]);
  };
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(w, 0);
  ihdr.writeUInt32BE(h, 4);
  ihdr.set([8, 2, 0, 0, 0], 8);
  const rows = Buffer.alloc((w * 3 + 1) * h);
  for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) rows.set([0x6a, 0x5a, 0xcd], y * (w * 3 + 1) + 1 + x * 3);
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", ihdr),
    chunk("IDAT", deflateSync(rows)),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

/** Where `piece` stands in the hostile text, in characters. */
const span = (piece: string) => [HOSTILE_TEXT.indexOf(piece), HOSTILE_TEXT.indexOf(piece) + piece.length];

/** What a client sends: facts, a picture, the game, and markup in its words. */
export function CLIENT(): unknown {
  return {
    build: { version: "0.1.1-beta.9b", commit: "3f9a1c7e21aa" },
    system: { platform: `Linux ${HOSTILE}`, cpus: 8, adapter: "Synthetic GPU", backend: "vulkan", window: [1280, 720], scale: 1.5, lang: "de" },
    screenshot: { png_base64: png(64, 36).toString("base64") },
    game: { table: { when: `Combat ${HOSTILE}`, seat: 1, seq: 9 }, view: { turn: 4, active: 2, players: [{}, {}] }, pending: { ChooseAttackers: {} } },
    crash: { message: "x", location: "y", backtrace: `at main ${HOSTILE}` },
    log: { roster: [{ seat: 1, name: `Ada ${HOSTILE}`, is_ai: false }], lines: [{ turn: 1, text: `plays ${HOSTILE}` }] },
    refs: {
      cards: [{ text: "Llanowar Elves", at: span("[Llanowar Elves]"), zone: "battlefield", owner: 1 }],
      players: [{ text: HOSTILE_TOKEN, at: span(`[@${HOSTILE_TOKEN}]`), seat: 2 }],
    },
    settings: { volume: 3, name: HOSTILE },
    [HOSTILE]: { long: "y".repeat(400) },
  };
}

// ---------------------------------------------------------------- answers

const DONE: Status[] = ["resolved", "wont_fix", "duplicate"];

type Filter = {
  statuses?: Status[];
  kinds?: Kind[];
  priorities?: (Priority | null)[];
  q?: string | null;
  label?: string | null;
  assignee?: number[] | null;
  unassigned?: boolean;
  trash?: boolean;
  unseen?: boolean;
  signature?: string | null;
  reporter?: string | null;
  duplicate_of?: number[] | null;
  gateway?: string | null;
  channel?: string | null;
};

function matches(r: Report, f: Filter, without?: keyof Filter): boolean {
  const s = r.summary;
  if (without !== "statuses" && f.statuses?.length && !f.statuses.includes(s.status)) return false;
  if (without !== "kinds" && f.kinds?.length && !f.kinds.includes(s.kind)) return false;
  if (without !== "priorities" && f.priorities?.length && !f.priorities.includes(s.priority)) return false;
  if (f.q && !`${r.text} ${s.crash ?? ""} ${hex(s.id)}`.toLowerCase().includes(f.q.toLowerCase())) return false;
  if (without !== "label" && f.label && !s.labels.includes(f.label)) return false;
  if (without !== "assignee" && f.assignee && !same(f.assignee, s.assignee ?? [])) return false;
  if (without !== "assignee" && f.unassigned && s.assignee !== null) return false;
  if (f.signature && s.signature !== f.signature) return false;
  if (f.reporter && s.reporter !== f.reporter) return false;
  if (f.duplicate_of && !same(f.duplicate_of, s.duplicate_of ?? [])) return false;
  if (without !== "gateway" && f.gateway && s.gateway !== f.gateway) return false;
  if (without !== "channel" && f.channel && s.channel !== f.channel) return false;
  if (f.unseen && s.seen) return false;
  return (s.deleted_at !== null) === (f.trash === true);
}

function counts<T>(rows: Report[], of: (r: Report) => T[]): { value: T; count: number }[] {
  const out: { value: T; count: number }[] = [];
  for (const r of rows) {
    for (const v of of(r)) {
      const found = out.find((c) => JSON.stringify(c.value) === JSON.stringify(v));
      if (found) found.count += 1;
      else out.push({ value: v, count: 1 });
    }
  }
  return out;
}

function facets(w: World, f: Filter) {
  const under = (without: keyof Filter) => w.reports.filter((r) => matches(r, f, without));
  return {
    statuses: counts(under("statuses"), (r) => [r.summary.status]),
    kinds: counts(under("kinds"), (r) => [r.summary.kind]),
    priorities: counts(under("priorities"), (r) => [r.summary.priority]),
    assignees: counts(under("assignee"), (r) => [r.summary.assignee]),
    labels: counts(under("label"), (r) => r.summary.labels),
    gateways: counts(under("gateway"), (r) => (r.summary.channel === "gateway" ? [r.summary.gateway] : [])),
    channels: counts(under("channel"), (r) => [r.summary.channel]),
    reporters: counts(w.reports.filter((r) => matches(r, f)), (r) => [r.summary.reporter]),
    total: w.reports.filter((r) => matches(r, f)).length,
  };
}

function find(w: World, report: unknown): Report | undefined {
  return w.reports.find((r) => same(report, r.summary.id));
}

type Change = { status?: Status; issue?: number | null; assignee?: number[] | null; priority?: Priority | null; labels?: string[]; labels_add?: string[]; labels_remove?: string[]; duplicate_of?: number[] | null };

function apply(r: Report, c: Change, actor: number[]) {
  const s = r.summary;
  const line = (action: string, detail: string) => r.audit.push({ at: THEN, actor, action: `report.${action}`, detail });
  if (c.status !== undefined) {
    line("status", `${s.status} → ${c.status}`);
    s.status = c.status;
    s.done_at = DONE.includes(c.status) ? THEN : null;
  }
  if ("issue" in c) s.issue = c.issue ?? null;
  if ("assignee" in c) s.assignee = c.assignee ?? null;
  if ("priority" in c) s.priority = c.priority ?? null;
  if (c.labels) s.labels = [...c.labels].sort();
  for (const l of c.labels_add ?? []) if (!s.labels.includes(l)) s.labels = [...s.labels, l].sort();
  for (const l of c.labels_remove ?? []) s.labels = s.labels.filter((x) => x !== l);
  if ("duplicate_of" in c) {
    s.duplicate_of = c.duplicate_of ?? null;
    if (c.duplicate_of) s.status = "duplicate";
  }
  s.revision += 1;
  s.updated_at = THEN;
}

const detail = (r: Report) => ({ ...r, summary: { ...r.summary }, notes: r.notes.map((n) => ({ ...n })), audit: r.audit.map((a) => ({ ...a })) });

/** Central's answer to an administrator's report action. */
function answer(w: World, asked: Asked): { status: number } | { json: unknown } | "lost" {
  if (!w.admin) return { status: 401 };
  const ok = (value: unknown) => ({ json: { result: "admin", answer: value } });
  const done = { json: { result: "done" } };
  const action = String(asked["action"]);
  const next = w.next[action];
  if (typeof next === "number") {
    delete w.next[action];
    return { status: next };
  }
  const lostAfter = next === "lost";
  if (lostAfter) delete w.next[action];
  const result = ((): { status: number } | { json: unknown } => {
    switch (action) {
      case "reports": {
        const rows = w.reports.filter((r) => matches(r, asked["filter"] as Filter));
        const offset = Number(asked["offset"]);
        const limit = Number(asked["limit"]);
        return ok({ answer: "reports", reports: rows.slice(offset, offset + limit).map((r) => r.summary), total: rows.length });
      }
      case "report_facets":
        return ok({ answer: "report_facets", facets: facets(w, asked["filter"] as Filter) });
      case "report": {
        const r = find(w, asked["report"]);
        return r ? ok({ answer: "report", report: detail(r) }) : { status: 401 };
      }
      case "report_neighbours": {
        const rows = w.reports.filter((r) => matches(r, asked["filter"] as Filter));
        const at = rows.findIndex((r) => same(asked["report"], r.summary.id));
        return ok({
          answer: "neighbours",
          neighbours: {
            previous: at > 0 ? rows[at - 1]?.summary.id : null,
            next: at >= 0 && at < rows.length - 1 ? rows[at + 1]?.summary.id : null,
            position: at >= 0 ? at + 1 : null,
            total: rows.length,
          },
        });
      }
      case "crash_groups": {
        const crashes = w.reports.filter((r) => r.summary.signature !== null && matches(r, asked["filter"] as Filter));
        const groups = crashes.length === 0 ? [] : [{
          signature: SIGNATURE, count: crashes.length, open: crashes.filter((r) => !DONE.includes(r.summary.status)).length,
          first: THEN - 7200, last: THEN, latest: crashes[0]?.summary.id, message: `panicked ${HOSTILE}`, location: `src/${HOSTILE_TOKEN}.rs:1:2`,
        }];
        return ok({ answer: "crash_groups", groups });
      }
      case "report_stats": {
        const today = new Date().toISOString().slice(0, 10);
        return ok({ answer: "report_stats", days: [{ day: today, kind: "crash", count: 2 }, { day: today, kind: "bug", count: 3 }] });
      }
      case "report_admins":
        return ok({ answer: "report_admins", admins: ADMINS });
      case "set_report": {
        const r = find(w, asked["report"]);
        if (!r) return { status: 401 };
        if (asked["expected_revision"] !== r.summary.revision) return { status: 409 };
        apply(r, asked["change"] as Change, ME);
        return ok({ answer: "report", report: detail(r) });
      }
      case "reports_bulk": {
        const ids = asked["reports"] as number[][];
        const bulk = asked["bulk"] as { do: string; change?: Change };
        const rows = ids.map((i) => find(w, i));
        if (rows.some((r) => !r)) return { status: 401 };
        if (bulk.do === "purge" && (w.stale || rows.some((r) => r?.summary.deleted_at === null))) return { status: 401 };
        let changed = 0;
        for (const r of rows as Report[]) {
          if (bulk.do === "change" && bulk.change) apply(r, bulk.change, ME);
          if (bulk.do === "trash" && r.summary.deleted_at === null) r.summary.deleted_at = THEN;
          if (bulk.do === "restore" && r.summary.deleted_at !== null) r.summary.deleted_at = null;
          if (bulk.do === "purge") w.reports = w.reports.filter((x) => x !== r);
          changed += 1;
        }
        return ok({ answer: "changed", changed });
      }
      case "trash_report":
      case "restore_report":
      case "purge_report": {
        const r = find(w, asked["report"]);
        if (!r) return { status: 401 };
        if (action === "purge_report") {
          if (w.stale || r.summary.deleted_at === null) return { status: 401 };
          w.reports = w.reports.filter((x) => x !== r);
        }
        if (action === "trash_report") r.summary.deleted_at = THEN;
        if (action === "restore_report") r.summary.deleted_at = null;
        return done;
      }
      case "add_report_note": {
        const r = find(w, asked["report"]);
        if (!r) return { status: 401 };
        r.notes.push({ id: id(100 + r.notes.length), at: THEN, author: ME, text: String(asked["text"]) });
        r.summary.notes = r.notes.length;
        return ok({ answer: "report", report: detail(r) });
      }
      case "remove_report_note": {
        const r = find(w, asked["report"]);
        const note = r?.notes.find((n) => same(asked["note"], n.id));
        if (!r || !note || !same(note.author, ME)) return { status: 401 };
        r.notes = r.notes.filter((n) => n !== note);
        r.summary.notes = r.notes.length;
        return ok({ answer: "report", report: detail(r) });
      }
      case "report_views":
        return ok({ answer: "report_views", views: w.views });
      case "save_report_view": {
        const name = String(asked["name"]);
        const existing = w.views.find((v) => v.name === name);
        if (existing) existing.query = String(asked["query"]);
        else w.views.push({ id: id(91 + w.views.length), name, query: String(asked["query"]) });
        return ok({ answer: "report_views", views: w.views });
      }
      case "delete_report_view":
        w.views = w.views.filter((v) => !same(asked["view"], v.id));
        return ok({ answer: "report_views", views: w.views });
      case "set_reports_seen":
        for (const i of asked["reports"] as number[][]) {
          const r = find(w, i);
          if (r) r.summary.seen = asked["seen"] === true;
        }
        return done;
      default:
        return { status: 401 };
    }
  })();
  return lostAfter ? "lost" : result;
}

export type Opened = { seen: Body[]; problems: string[]; consoled: string[] };

/** Every request the page makes to Central's surface, answered by the stand-in. */
export async function open(page: Page, w: World, path: string): Promise<Opened> {
  const seen: Body[] = [];
  const problems: string[] = [];
  const consoled: string[] = [];
  page.on("pageerror", (error) => problems.push(error.message));
  page.on("console", (message) => consoled.push(message.text()));
  await page.addInitScript(() => {
    if (!sessionStorage.getItem("seeded")) {
      sessionStorage.setItem("seeded", "1");
      localStorage.setItem("baylee-central-session", JSON.stringify({ expiresAt: 4_102_444_800 }));
    }
  });
  await page.route("**/api/v1", (route: Route) => {
    const request = route.request();
    expect(request.headers()["x-baylee-central"]).toBe("1");
    // nothing of a report travels in an address
    expect(new URL(request.url()).search).toBe("");
    const body = JSON.parse(request.postData() ?? "{}") as Body;
    seen.push(body);
    const json = (value: unknown) => route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(value) });
    const refused = (status: number) => route.fulfill({ status, body: "account request rejected\n" });
    if (w.gone) return refused(401);
    const now = Math.floor(Date.now() / 1000);
    switch (body.operation) {
      case "account_summary":
        return json({
          result: "account_summary", identity: { account: ME, epoch: 1 }, username: "chef", display_name: "Chef", tag: 0x0c4e,
          source: "registered", recovery: true, admin: w.admin, signed_in_at: w.stale ? now - 3600 : now - 60, expires_at: 4_102_444_800,
        });
      case "admin": {
        const result = answer(w, body.request ?? {});
        if (result === "lost") return route.abort();
        return "status" in result ? refused(result.status) : json(result.json);
      }
      case "login_start":
        return json({ result: "login_challenge", attempt: Array(32).fill(7), message: Array.from({ length: 320 }, (_, i) => (i * 37) % 251) });
      case "login_finish":
        w.stale = false;
        return json({ result: "browser_session", identity: { account: ME, epoch: 1 }, username: "chef", wrapped_master: Array(73).fill(9), expires_at: 4_102_444_800 });
      default:
        return refused(401);
    }
  });
  await page.route("**/api/v1/reports/*/*", (route: Route) => {
    const request = route.request();
    expect(request.method()).toBe("POST");
    expect(request.headers()["x-baylee-central"]).toBe("1");
    expect(request.headers()["content-type"]).toBe("application/json");
    const [, , , , report, part] = new URL(request.url()).pathname.split("/");
    w.parts.push(`${report}/${part}`);
    if (!w.admin) return route.fulfill({ status: 401, body: "account request rejected\n" });
    if (typeof w.client === "number" && part === "client") return route.fulfill({ status: w.client, body: "" });
    if (part === "client") {
      return route.fulfill({ status: 200, contentType: "application/gzip", body: gzipSync(Buffer.from(JSON.stringify(w.client))) });
    }
    return route.fulfill({ status: 200, headers: { "content-type": "application/gzip", "x-record-origin": "client" }, body: w.record });
  });
  await page.route("**/assets/crypto-worker.*.js", (route) => route.fulfill({ contentType: "text/javascript", body: WORKER }));
  await page.goto(path);
  await expect(page.locator("html")).toHaveAttribute("data-islands", "ready", { timeout: 20_000 });
  return { seen, problems, consoled };
}

/** The sign-in once more, canned: the stand-in's OPAQUE answers are none. */
const WORKER = `
self.addEventListener("message", (event) => {
  const m = JSON.parse(event.data);
  const ok = (value) => self.postMessage(JSON.stringify({ id: m.id, ok: value }));
  switch (m.op) {
    case "login_start": return ok({ message: Array(96).fill(1) });
    case "login_finish": return ok({ message: Array(64).fill(2) });
    default: return self.postMessage(JSON.stringify({ id: m.id, error: "failed" }));
  }
});`;

/** The administration's report requests since `from`. */
export const asked = (seen: Body[], from = 0) => seen.slice(from).filter((b) => b.operation === "admin").map((b) => b.request as Asked);
export const actions = (seen: Body[], from = 0) => asked(seen, from).map((r) => r["action"]);

/** A filter as Central's serializer writes every part of it. */
export function filter(more: Record<string, unknown> = {}) {
  return {
    statuses: [], kinds: [], priorities: [], q: null, from: null, to: null, has_record: null, channel: null, gateway: null,
    assignee: null, unassigned: false, label: null, reporter: null, signature: null, duplicate_of: null, trash: false, unseen: false,
    ...more,
  };
}

/** The page as it stands passes the checks every page passes: axe in both schemes, the targets, 320 px. */
export async function sound(page: Page) {
  expect(await schemes(page)).toEqual([]);
  expect(await targets(page)).toEqual([]);
  expect(await narrow(page)).toBe(0);
}

/** Markup in a report never ran and never became an element; nothing of it reached the console. */
export async function harmless(page: Page, opened: Opened) {
  expect(await page.evaluate(() => (window as unknown as { __pwned?: number }).__pwned)).toBeUndefined();
  expect(await page.locator("img[src='x'], [onerror], body script:not([src])").count()).toBe(0);
  expect(opened.consoled.filter((m) => m.includes("__pwned") || m.includes("first line"))).toEqual([]);
  expect(opened.problems).toEqual([]);
}
