// A stand-in for Central's browser surface as the administration's pages
// use it (crates/central-web/src/islands/admin/, crates/central-api/src/admin.rs),
// for the admin-*.spec.ts suites: it answers the reads and refuses the
// changes as Central's store does (central-postgres accounts/admin.rs): a
// stale sign-in, a missing role, an unknown account or gateway with 401,
// a taken or chat-bound tag with 409. All of it is synthetic.
//
// The password once more cannot be answered truthfully (OPAQUE), so that
// sign-in runs in a canned Worker, as admin.spec.ts does; the real Worker
// refusing the stand-in is the wrong password's path there.

import { expect, type Locator, type Page, type Route } from "@playwright/test";

import { narrow, schemes, targets } from "./check.ts";

export const id = (n: number) => Array.from({ length: 16 }, (_, i) => ({ 6: 0x70, 8: 0x80, 15: n })[i] ?? 0x10 + i);
export const hex = (bytes: number[]) => bytes.map((b) => b.toString(16).padStart(2, "0")).join("");

export const OWN = id(1);
export const OTHER = id(99);
export const PASSWORD = "violet lantern harbour";
export const THEN = 1_794_000_000;
/** A page of the stand-in: small, so that "Show more" is reached. */
export const PAGE = 3;

type Source = "registered" | "legacy";
type State = "pending" | "enrolled" | "restored";
export type Row = {
  id: number[];
  username: string;
  display_name: string | null;
  tag: number | null;
  source: Source;
  state: State;
  created_at: number;
  last_sign_in_at: number | null;
  admin: boolean;
  reset_until: number | null;
};
export type Account = { row: Row; recovery: boolean; chat: boolean };

export const accountOf = (n: number, username: string, more: Partial<Row> = {}, recovery = true, chat = false): Account => ({
  row: {
    id: id(n), username, display_name: null, tag: null, source: "registered", state: "restored",
    created_at: THEN - n * 3600, last_sign_in_at: THEN, admin: false, reset_until: null, ...more,
  },
  recovery,
  chat,
});

/** The accounts, newest first. */
export const ACCOUNTS = () => [
  accountOf(10, "erika", { display_name: "Erika", tag: 0x1a2b }),
  accountOf(11, "moritz", { display_name: "Moritz", tag: 0x2b3c, source: "legacy" }),
  accountOf(12, "ada", { display_name: "Ada", tag: 0x3c4d, admin: true }),
  accountOf(13, "neu", { state: "pending", last_sign_in_at: null }, false),
  // in Central chat: its tag stays
  accountOf(14, "chatty", { display_name: "Chatty", tag: 0x5e6f }, true, true),
  accountOf(15, "jonas", { display_name: "Jonas", tag: 0x4d5e, state: "enrolled", reset_until: THEN + 3600 }, false),
  accountOf(1, "chef", { display_name: "Chef", tag: 0x0c4e, admin: true }),
];
export const ID = { erika: id(10), moritz: id(11), ada: id(12), neu: id(13), chatty: id(14), jonas: id(15), chef: OWN };

export type Token = {
  id: number[];
  name: string;
  scopes: { sso: boolean; listing: boolean };
  created_at: number;
  expires_at: number;
  revoked_at: number | null;
  last_used_at: number | null;
};
export type Gateway = {
  id: number[];
  origin: string;
  metadata: { name: string; version: string; website: string };
  official: boolean;
  owner: number[] | null;
  owner_name: string | null;
  owner_tag: number | null;
  state: "pending" | "approved" | "disabled";
  online: boolean;
  last_seen: number | null;
  created_at: number;
  live_tokens: number;
  credential_pair: boolean;
  tokens: Token[];
};

const FAR = 4_102_444_800;

export const gatewayOf = (n: number, name: string, state: Gateway["state"], more: Partial<Gateway> = {}): Gateway => ({
  id: id(0x40 + n), origin: `https://${name.toLowerCase().replaceAll(" ", "-")}.example`,
  metadata: { name, version: "0.1.0", website: `https://${name.toLowerCase().replaceAll(" ", "-")}.example/` },
  official: false, owner: ID.erika, owner_name: "Erika", owner_tag: 0x1a2b, state, online: false, last_seen: null,
  created_at: THEN - n * 3600, live_tokens: 0, credential_pair: false, tokens: [], ...more,
});

export const tokenOf = (n: number, name: string, more: Partial<Token> = {}): Token => ({
  id: id(0x60 + n), name, scopes: { sso: true, listing: true }, created_at: THEN, expires_at: FAR, revoked_at: null, last_used_at: null, ...more,
});

/** The gateways, newest first. */
export const GATEWAYS = () => [
  gatewayOf(1, "Garden Table", "pending", { live_tokens: 1, tokens: [tokenOf(1, "garden box")] }),
  gatewayOf(2, "Moon Hall", "pending", { owner: ID.moritz, owner_name: "Moritz", owner_tag: 0x2b3c }),
  gatewayOf(3, "Old Mill", "pending", { owner: ID.jonas, owner_name: null, owner_tag: null, credential_pair: true }),
  gatewayOf(4, "Night Owls", "pending"),
  gatewayOf(5, "Sun Court", "approved", {
    online: true, last_seen: THEN, live_tokens: 1,
    tokens: [tokenOf(2, "court main", { last_used_at: THEN }), tokenOf(3, "old key", { revoked_at: THEN - 60 }), tokenOf(4, "listing only", { scopes: { sso: false, listing: true }, expires_at: THEN })],
  }),
  gatewayOf(6, "Baylee", "approved", { official: true, owner: null, owner_name: null, owner_tag: null, online: true, last_seen: THEN }),
  gatewayOf(7, "Closed Door", "disabled"),
];
export const GW = { garden: id(0x41), moon: id(0x42), mill: id(0x43), owls: id(0x44), sun: id(0x45), official: id(0x46), closed: id(0x47) };

export type Event = { id: number[]; at: number; actor: number[] | null; action: string; account: number[] | null; gateway: number[] | null; beta_key: null };

export const eventOf = (n: number, action: string, actor: number[] | null, account: number[] | null, gateway: number[] | null = null): Event => ({
  id: id(0x80 + n), at: THEN - n * 60, actor, action, account, gateway, beta_key: null,
});

/** The audit log, newest first. */
export const EVENTS = () => [
  eventOf(1, "account.admin_granted", OWN, ID.ada),
  eventOf(2, "gateway.approved", OWN, null, GW.sun),
  eventOf(3, "account.tag_changed", OWN, ID.erika),
  eventOf(4, "account.reset_issued", OWN, ID.jonas),
  // an account deleted since, and a gateway too: only their ids are left
  eventOf(5, "gateway.token_revoked", id(77), null, id(0x4f)),
  eventOf(6, "registration.closed", OWN, null),
  eventOf(7, "account.admin_granted", null, OWN),
  eventOf(8, "account.locked", OWN, ID.moritz),
];

export type Legacy = {
  deadline: number | null;
  accounts: {
    legacy_id: string; username: string; display_name: string | null; tag: number | null;
    progress: "unstarted" | "reserved" | "enrolled" | "restored"; imported_at: number; failures: number;
    locked_until: number | null; must_change_password: boolean; banned: boolean; snapshot_bytes: number;
  }[];
};

export const LEGACY = (): Legacy => ({
  deadline: THEN + 30 * 86_400,
  accounts: [
    { legacy_id: "101", username: "oldtimer", display_name: "Oldtimer", tag: 0x77, progress: "unstarted", imported_at: THEN - 86_400, failures: 0, locked_until: null, must_change_password: false, banned: false, snapshot_bytes: 900 },
    { legacy_id: "102", username: "halfway", display_name: null, tag: null, progress: "reserved", imported_at: THEN - 86_400, failures: 3, locked_until: FAR, must_change_password: true, banned: true, snapshot_bytes: 1200 },
    { legacy_id: "103", username: "settler", display_name: "Settler", tag: 0x1234, progress: "restored", imported_at: THEN - 86_400, failures: 1, locked_until: THEN - 60, must_change_password: false, banned: false, snapshot_bytes: 300 },
  ],
});

export type World = {
  accounts: Account[];
  gateways: Gateway[];
  events: Event[];
  /** None: this Central keeps no legacy store (Central refuses the read). */
  legacy: Legacy | null;
  /** The reports' facets under "new": the open statuses and the new crashes; none where Central refuses. */
  reports: { statuses: [string, number][]; crashes: number } | null;
  /** The browser's account holds the role. */
  admin: boolean;
  /** Central counts the sign-in as older than 30 minutes; a sign-in once more makes it fresh. */
  stale: boolean;
  /** The account this browser's session is. */
  as: number[];
  /** Central no longer knows the session. */
  gone: boolean;
  /** How the next change ends instead: a status, an answer of another kind, or no answer after it was made. */
  next: number | "lost" | "other" | null;
  /** A read that is refused or unanswered instead, by its action. */
  failing: Record<string, number>;
};

export const world = (more: Partial<World> = {}): World => ({
  accounts: ACCOUNTS(), gateways: GATEWAYS(), events: EVENTS(), legacy: LEGACY(),
  reports: { statuses: [["new", 4], ["triaged", 2], ["in_progress", 1], ["resolved", 9]], crashes: 2 },
  admin: true, stale: false, as: OWN, gone: false, next: null, failing: {}, ...more,
});

export type Asked = {
  action?: string; query?: string | null; before?: number[] | null; account?: number[] | null; admin?: boolean;
  tag?: number; gateway?: number[]; enabled?: boolean; token?: number[]; state?: string | null; filter?: unknown;
};
export type Body = { operation?: string; request?: Asked; identity?: unknown } & Record<string, unknown>;

const same = (a: number[] | null | undefined, b: number[] | null | undefined) =>
  a !== null && a !== undefined && b !== null && b !== undefined && a.length === b.length && a.every((v, i) => v === b[i]);

type Answer = (value: unknown) => Promise<void>;

/** A facet's counts as Central writes them. */
const counts = (list: [string, number][]) => list.map(([value, count]) => ({ value, count }));

function named(w: World, events: Event[]) {
  const accounts = new Map<string, { id: number[]; display_name: string | null; tag: number | null }>();
  const gateways = new Map<string, { id: number[]; name: string; origin: string }>();
  for (const event of events) {
    for (const who of [event.actor, event.account]) {
      const one = w.accounts.find(({ row }) => same(row.id, who));
      if (one) accounts.set(hex(one.row.id), { id: one.row.id, display_name: one.row.display_name, tag: one.row.tag });
    }
    const gateway = w.gateways.find((g) => same(g.id, event.gateway));
    if (gateway) gateways.set(hex(gateway.id), { id: gateway.id, name: gateway.metadata.name, origin: gateway.origin });
  }
  return { accounts: [...accounts.values()], gateways: [...gateways.values()] };
}

const rowOf = (g: Gateway) => {
  const { tokens: _, ...row } = g;
  return row;
};

function overview(w: World) {
  const live = w.gateways.flatMap((g) => g.tokens).filter((t) => t.revoked_at === null && t.expires_at > Date.now() / 1000).length;
  const counted = w.gateways.filter((g) => !g.official);
  const enrolled = w.accounts.filter(({ row }) => row.state !== "pending");
  return {
    accounts: enrolled.length, accounts_week: 2, moved: enrolled.filter(({ row }) => row.source === "legacy").length,
    admins: w.accounts.filter(({ row }) => row.admin).length,
    pending: counted.filter((g) => g.state === "pending").length, approved: counted.filter((g) => g.state === "approved").length,
    disabled: counted.filter((g) => g.state === "disabled").length, online: counted.filter((g) => g.online).length, live_tokens: live,
  };
}

/** A change, as Central's store makes or refuses it. */
function change(w: World, asked: Asked, done: (other: boolean) => Promise<void>, refused: (status: number) => Promise<void>, abort: () => Promise<void>) {
  const next = w.next;
  w.next = null;
  if (typeof next === "number") return refused(next);
  if (w.stale) return refused(401);
  let made = false;
  switch (asked.action) {
    case "set_admin": {
      const one = w.accounts.find(({ row }) => same(row.id, asked.account));
      if (asked.admin === false && same(asked.account, w.as)) return refused(401);
      if (!one || (asked.admin && one.row.state === "pending")) return refused(401);
      one.row.admin = asked.admin === true;
      made = true;
      break;
    }
    case "set_tag": {
      const one = w.accounts.find(({ row }) => same(row.id, asked.account));
      const tag = asked.tag ?? 0;
      if (tag < 1 || tag > 0x7fffffff || !one || one.row.state === "pending") return refused(401);
      if (one.row.tag === tag) break;
      if (w.accounts.some(({ row }) => row.tag === tag) || one.chat) return refused(409);
      one.row.tag = tag;
      made = true;
      break;
    }
    case "set_gateway_approval": {
      const one = w.gateways.find((g) => same(g.id, asked.gateway));
      if (!one) return refused(401);
      one.state = asked.enabled ? "approved" : "disabled";
      made = true;
      break;
    }
    case "revoke_gateway_token": {
      const gateway = w.gateways.find((g) => g.tokens.some((t) => same(t.id, asked.token)));
      const token = gateway?.tokens.find((t) => same(t.id, asked.token));
      if (!gateway || !token) return refused(401);
      if (token.revoked_at === null) {
        token.revoked_at = THEN;
        gateway.live_tokens = Math.max(0, gateway.live_tokens - 1);
      }
      made = true;
      break;
    }
    default:
      return refused(401);
  }
  // made, and its answer lost on the way, or of another kind
  if (next === "lost" && made) return abort();
  return done(next === "other");
}

/** What Central answers an administrator. */
function admin(w: World, asked: Asked, answer: Answer, done: (other: boolean) => Promise<void>, refused: (status: number) => Promise<void>, abort: () => Promise<void>) {
  if (!w.admin) return refused(401);
  const failing = w.failing[asked.action ?? ""];
  if (failing !== undefined) return failing === 0 ? abort() : refused(failing);
  switch (asked.action) {
    case "accounts": {
      const query = asked.query?.trim().toLowerCase() ?? null;
      let found = w.accounts.filter(({ row }) => {
        if (query === null) return true;
        if (query.startsWith("#")) return row.tag === Number.parseInt(query.slice(1), 16);
        return row.username.includes(query) || (row.display_name ?? "").toLowerCase().includes(query);
      });
      if (asked.before) found = found.slice(found.findIndex(({ row }) => same(asked.before, row.id)) + 1);
      return answer({ answer: "accounts", accounts: found.slice(0, PAGE).map(({ row }) => row), more: found.length > PAGE });
    }
    case "account": {
      const one = w.accounts.find(({ row }) => same(asked.account, row.id));
      if (!one) return refused(401);
      const gateways = w.gateways.filter((g) => same(g.owner, one.row.id)).map((g) => ({ id: g.id, origin: g.origin, approved: g.state === "approved" }));
      const audit = w.events.filter((e) => same(e.account, one.row.id) || same(e.actor, one.row.id));
      return answer({ answer: "account", account: one.row, recovery: one.recovery, gateways, audit });
    }
    case "overview":
      return answer({ answer: "overview", overview: overview(w) });
    case "gateways": {
      let found = w.gateways.filter((g) => asked.state === null || asked.state === undefined || g.state === asked.state);
      if (asked.before) found = found.slice(found.findIndex((g) => same(asked.before, g.id)) + 1);
      return answer({ answer: "gateways", gateways: found.slice(0, PAGE).map(rowOf), more: found.length > PAGE });
    }
    case "gateway": {
      const one = w.gateways.find((g) => same(asked.gateway, g.id));
      if (!one) return refused(401);
      return answer({ answer: "gateway", gateway: rowOf(one), tokens: one.tokens });
    }
    case "audit": {
      let found = w.events.filter((e) => !asked.account || same(e.account, asked.account));
      if (asked.before) found = found.slice(found.findIndex((e) => same(asked.before, e.id)) + 1);
      const page = found.slice(0, PAGE);
      return answer({ answer: "audit", events: page, more: found.length > PAGE, ...named(w, page) });
    }
    case "legacy_accounts":
      if (w.legacy === null) return refused(401);
      return answer({ answer: "legacy_accounts", store: w.legacy });
    case "report_facets": {
      if (w.reports === null) return refused(401);
      return answer({
        answer: "report_facets",
        facets: {
          statuses: counts(w.reports.statuses), kinds: counts([["bug", 1], ["crash", w.reports.crashes]]), priorities: [], assignees: [],
          labels: [], gateways: [], channels: [], reporters: [], total: 4,
        },
      });
    }
    case "registration":
      return answer({ answer: "registration", open: true, changed_at: null });
    case "set_admin":
    case "set_tag":
    case "set_gateway_approval":
    case "revoke_gateway_token":
      return change(w, asked, done, refused, abort);
    default:
      return refused(401);
  }
}

function central(seen: Body[], w: World) {
  return (route: Route) => {
    const request = route.request();
    expect(request.headers()["x-baylee-central"]).toBe("1");
    // nothing typed or shown travels in an address
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
          result: "account_summary", identity: { account: w.as, epoch: 1 }, username: "chef", display_name: "Chef", tag: 0x0c4e,
          source: "registered", recovery: true, admin: w.admin, signed_in_at: w.stale ? now - 3600 : now - 60, expires_at: FAR,
        });
      case "admin":
        return admin(
          w, body.request ?? {}, (value) => json({ result: "admin", answer: value }),
          (other) => json(other ? { result: "renewed", expires_at: FAR } : { result: "done" }), refused, () => route.abort(),
        );
      case "login_start":
        return json({ result: "login_challenge", attempt: Array(32).fill(7), message: Array.from({ length: 320 }, (_, i) => (i * 37) % 251) });
      case "login_finish":
        // a sign-in once more: the session's password is fresh again
        w.stale = false;
        return json({
          result: "browser_session", identity: { account: w.as, epoch: 1 }, username: "chef",
          wrapped_master: Array(73).fill(9), expires_at: FAR,
        });
      default:
        return refused(401);
    }
  };
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

export type Opened = { seen: Body[]; problems: string[] };

/** Opens `path` signed in, against the stand-in `w`. */
export async function open(page: Page, w: World, path: string): Promise<Opened> {
  const seen: Body[] = [];
  const problems: string[] = [];
  page.on("pageerror", (error) => problems.push(error.message));
  await page.addInitScript(() => {
    if (!sessionStorage.getItem("seeded")) {
      sessionStorage.setItem("seeded", "1");
      localStorage.setItem("baylee-central-session", JSON.stringify({ expiresAt: 4_102_444_800 }));
    }
  });
  await page.route("**/api/v1", central(seen, w));
  await page.route("**/assets/crypto-worker.*.js", (route) => route.fulfill({ contentType: "text/javascript", body: WORKER }));
  await page.goto(path);
  await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
  return { seen, problems };
}

/** The administration's requests since `from`. */
export const asked = (seen: Body[], from = 0) => seen.slice(from).filter((body) => body.operation === "admin").map((body) => body.request);
/** The operations since `from`. */
export const operations = (seen: Body[], from: number) => seen.slice(from).map((body) => body.operation);
/** The changes since `from`. */
export const changes = (seen: Body[], from = 0) =>
  asked(seen, from).filter((r) => ["set_admin", "set_tag", "set_gateway_approval", "revoke_gateway_token"].includes(r?.action ?? ""));

export const button = (scope: Page | Locator, name: string) => scope.getByRole("button", { name, exact: true });

/** The page as it stands passes the checks every page passes: axe in both schemes, the targets, 320 px. */
export async function sound(page: Page) {
  expect(await schemes(page)).toEqual([]);
  expect(await targets(page)).toEqual([]);
  expect(await narrow(page)).toBe(0);
}

/** Tab leads through `stops` in this order, starting behind `from`. */
export async function tabs(page: Page, from: Locator, stops: Locator[]) {
  await from.focus();
  for (const stop of stops) {
    await page.keyboard.press("Tab");
    await expect(stop).toBeFocused();
  }
}

/** The administration's navigation, in its order. */
export const NAV = ["Overview", "Accounts", "Gateways", "Audit log", "Legacy accounts", "Reported profile pictures", "Feedback", "Reset a login"];
export const nav = (page: Page) => page.getByRole("navigation", { name: "Administration" });
