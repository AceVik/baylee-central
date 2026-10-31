// The account area's and the administration's answers for the e2e mock
// (`mockCentral` with `area`): the owner's gateways, tokens and sessions,
// and an administrator's accounts, gateways, audit log and legacy store, in
// the exact shapes `src/account/owner-api.ts` and `src/admin/admin-api.ts`
// read. Synthetic names and example.invalid addresses only. Changes are
// remembered for the page's life, so a test sees what it did.

import { reportsAnswer, reportsState, type ReportsState } from "./reports.ts";

/** A UUIDv7-shaped id whose last byte is `n`. */
export const uid = (kind: number, n: number): number[] => [0x01, 0x92, 0xd0, 0xc5, 0x7b, kind, 0x7a, 0x1f, 0x8c, 0x2d, 0x3e, 0x4f, 0x5a, 0x6b, 0x7c, n];

const now = () => Math.floor(Date.now() / 1000);
const DAY = 86_400;

interface Token {
  id: number[];
  name: string;
  scopes: { sso: boolean; listing: boolean };
  created_at: number;
  expires_at: number;
  revoked_at: number | null;
  last_used_at: number | null;
}

interface Gateway {
  id: number[];
  origin: string;
  name: string;
  version: string;
  state: "pending" | "approved" | "disabled";
  online: boolean;
  last_seen: number | null;
  created_at: number;
  tokens: Token[];
  /** The owner's account index in `ACCOUNTS`, or null for the official gateway. */
  owner: number | null;
}

interface Account {
  id: number[];
  username: string;
  display_name: string | null;
  tag: number | null;
  source: "legacy" | "registered";
  admin: boolean;
  created_at: number;
  last_sign_in_at: number | null;
}

const NAMES = ["Lantern", "Moss_Keeper", "Quiet-Heron", "Tidepool", "Ember", "Saltmarsh", "Kestrel", "Juniper", "Orchard", "Wren"];

export interface AreaState {
  accounts: Account[];
  gateways: Gateway[];
  audit: { id: number[]; at: number; actor: number[] | null; action: string; account: number[] | null; gateway: number[] | null; beta_key: null }[];
  /** Index of the signed-in account in `accounts`. */
  me: number;
  /** Bug and crash reports (`reports.ts`). */
  reports: ReportsState;
  nextToken: number;
}

export function areaState(admin: boolean): AreaState {
  const t = now();
  const accounts: Account[] = NAMES.map((name, i) => ({
    id: uid(0x10, i),
    username: name.toLowerCase().replace(/[^a-z0-9_.-]/g, "_"),
    display_name: i === 7 ? null : name,
    tag: i === 7 ? null : 0x3fa2 + i * 0x111,
    source: i === 3 || i === 7 ? "legacy" : "registered",
    admin: i === 0 ? admin : i === 4,
    created_at: t - (40 - i * 3) * DAY,
    last_sign_in_at: i === 9 ? null : t - i * 3_700,
  }));
  const token = (n: number, name: string, extra: Partial<Token> = {}): Token => ({
    id: uid(0x30, n),
    name,
    scopes: { sso: true, listing: true },
    created_at: t - 20 * DAY,
    expires_at: t + 70 * DAY,
    revoked_at: null,
    last_used_at: t - 140,
    ...extra,
  });
  const gateways: Gateway[] = [
    { id: uid(0x20, 0), origin: "https://gateway.example.invalid", name: "Sanctuary", version: "0.1.1-beta.9b", state: "approved", online: true, last_seen: t - 20, created_at: t - 60 * DAY, tokens: [], owner: null },
    {
      id: uid(0x20, 1),
      origin: "https://moonlit.example.invalid",
      name: "Moonlit Table",
      version: "0.1.1-beta.9b",
      state: "approved",
      online: true,
      last_seen: t - 35,
      created_at: t - 21 * DAY,
      tokens: [token(1, "living room server"), token(2, "old laptop", { revoked_at: t - 5 * DAY, last_used_at: t - 9 * DAY }), token(3, "listing only", { scopes: { sso: false, listing: true }, expires_at: t - DAY, last_used_at: null })],
      owner: 0,
    },
    { id: uid(0x20, 2), origin: "https://garden.example.invalid", name: "Garden Table", version: "0.0.0", state: "pending", online: false, last_seen: null, created_at: t - 2 * DAY, tokens: [], owner: 0 },
    { id: uid(0x20, 3), origin: "https://harbour.example.invalid:8443", name: "Harbour Nights", version: "0.1.1-beta.9", state: "pending", online: true, last_seen: t - 50, created_at: t - DAY, tokens: [token(4, "harbour")], owner: 2 },
    { id: uid(0x20, 4), origin: "https://cellar.example.invalid", name: "The Cellar", version: "0.1.0-beta.8", state: "disabled", online: false, last_seen: t - 12 * DAY, created_at: t - 30 * DAY, tokens: [], owner: 5 },
  ];
  const audit: AreaState["audit"] = [
    ["gateway.approved", 1, null, 1],
    ["account.tag_changed", 0, 6, null],
    ["gateway.token_revoked", 0, null, 1],
    ["account.admin_granted", null, 4, null],
    ["gateway.disabled", 4, null, 4],
    ["account.reset_issued", 0, 8, null],
  ].map(([action, actor, subject, gateway], i) => ({
    id: uid(0x40, i),
    at: t - (i + 1) * 5_400,
    actor: actor === null ? null : uid(0x10, actor as number),
    action: action as string,
    account: subject === null ? null : uid(0x10, subject as number),
    gateway: gateway === null ? null : uid(0x20, gateway as number),
    beta_key: null,
  }));
  const admins = accounts.filter((a) => a.admin).map((a) => ({ id: a.id, display_name: a.display_name, tag: a.tag }));
  return { accounts, gateways, audit, me: 0, nextToken: 10, reports: reportsState(admins) };
}

const same = (a: unknown, b: number[]) => Array.isArray(a) && a.join() === b.join();

const metadata = (g: Gateway) => ({ name: g.name, version: g.version, website: `${g.origin}/` });

function myGateway(g: Gateway) {
  return { id: g.id, origin: g.origin, metadata: metadata(g), state: g.state, online: g.online, last_seen: g.last_seen, created_at: g.created_at, tokens: g.tokens };
}

function account(a: Account) {
  return { ...a, state: a.source === "legacy" ? "restored" : "enrolled", reset_until: null };
}

function adminGateway(s: AreaState, g: Gateway) {
  const owner = g.owner === null ? null : s.accounts[g.owner];
  return {
    id: g.id,
    origin: g.origin,
    metadata: metadata(g),
    official: owner === null,
    owner: owner?.id ?? null,
    owner_name: owner?.display_name ?? null,
    owner_tag: owner?.tag ?? null,
    state: g.state,
    online: g.online,
    last_seen: g.last_seen,
    created_at: g.created_at,
    live_tokens: g.tokens.filter((t) => t.revoked_at === null && t.expires_at > now()).length,
    credential_pair: owner === null,
  };
}

const owner = (answer: string, rest: Record<string, unknown>) => ({ result: "owner", answer: { answer, ...rest } });
const admin = (answer: string, rest: Record<string, unknown>) => ({ result: "admin", answer: { answer, ...rest } });
const DONE = { result: "done" };

/** The answer to an `owner` request, or a status to refuse it with. */
export function ownerAnswer(s: AreaState, request: Record<string, unknown>): unknown {
  const mine = s.gateways.filter((g) => g.owner === s.me);
  switch (request["action"]) {
    case "sessions":
      return owner("sessions", {
        sessions: [
          { kind: "browser", current: true, signed_in_at: now() - 600, expires_at: now() + 1200, gateways: [] },
          { kind: "native", current: false, signed_in_at: now() - 7_200, expires_at: now() + 36_000, gateways: ["https://moonlit.example.invalid"] },
        ],
      });
    case "end_client_session":
      return DONE;
    case "gateways":
      return owner("gateways", { gateways: mine.map(myGateway) });
    case "add_gateway": {
      const origin = String(request["origin"]);
      if (s.gateways.some((g) => g.origin === origin)) return 409;
      const g: Gateway = { id: uid(0x20, 0x80 + s.gateways.length), origin, name: String(request["name"]), version: "0.0.0", state: "pending", online: false, last_seen: null, created_at: now(), tokens: [], owner: s.me };
      s.gateways.push(g);
      return owner("gateway_added", { gateway: g.id });
    }
    case "create_token": {
      const g = mine.find((x) => same(request["gateway"], x.id));
      if (g === undefined) return 401;
      const id = uid(0x30, s.nextToken++);
      const expires = now() + Number(request["days"]) * DAY;
      g.tokens.unshift({ id, name: String(request["name"]), scopes: request["scopes"] as Token["scopes"], created_at: now(), expires_at: expires, revoked_at: null, last_used_at: null });
      return owner("token_created", { id, token: Array.from({ length: 32 }, (_, i) => (i * 37 + 11) & 0xff), expires_at: expires });
    }
    case "revoke_token": {
      const token = mine.flatMap((g) => g.tokens).find((x) => same(request["token"], x.id));
      if (token === undefined) return 401;
      token.revoked_at = now();
      return DONE;
    }
  }
  return 400;
}

/** The answer to an `admin` request, or a status to refuse it with. */
export function adminAnswer(s: AreaState, request: Record<string, unknown>): unknown {
  const byId = (v: unknown) => s.accounts.find((a) => same(v, a.id));
  const gatewayById = (v: unknown) => s.gateways.find((g) => same(v, g.id));
  switch (request["action"]) {
    case "accounts": {
      const q = typeof request["query"] === "string" ? request["query"].toLowerCase().replace(/^#/, "") : "";
      const hits = s.accounts.filter((a) => q === "" || a.username.includes(q) || (a.display_name ?? "").toLowerCase().includes(q) || (a.tag !== null && a.tag.toString(16) === q));
      return admin("accounts", { accounts: hits.toReversed().map((a) => account(a)), more: false });
    }
    case "account": {
      const a = byId(request["account"]);
      if (a === undefined) return 404;
      const index = s.accounts.indexOf(a);
      return admin("account", {
        account: account(a),
        recovery: a.source === "registered",
        gateways: s.gateways.filter((g) => g.owner === index).map((g) => ({ id: g.id, origin: g.origin, approved: g.state === "approved" })),
        audit: s.audit.filter((e) => same(e.account, a.id) || same(e.actor, a.id)),
      });
    }
    case "audit": {
      const events = request["account"] === null ? s.audit : s.audit.filter((e) => same(e.account, request["account"] as number[]));
      // today's names of the accounts and gateways the page names, each once
      const ids = new Set(events.flatMap((e) => [e.actor, e.account]).filter((x) => x !== null).map((x) => x.join()));
      const gws = new Set(events.map((e) => e.gateway).filter((x) => x !== null).map((x) => x.join()));
      return admin("audit", {
        events,
        more: false,
        accounts: s.accounts.filter((a) => ids.has(a.id.join())).map((a) => ({ id: a.id, display_name: a.display_name, tag: a.tag })),
        gateways: s.gateways.filter((g) => gws.has(g.id.join())).map((g) => ({ id: g.id, name: g.name, origin: g.origin })),
      });
    }
    case "overview": {
      const registered = s.gateways.filter((g) => g.owner !== null);
      return admin("overview", {
        overview: {
          accounts: s.accounts.length,
          accounts_week: s.accounts.filter((a) => a.created_at > now() - 7 * DAY).length,
          moved: s.accounts.filter((a) => a.source === "legacy").length,
          admins: s.accounts.filter((a) => a.admin).length,
          pending: registered.filter((g) => g.state === "pending").length,
          approved: registered.filter((g) => g.state === "approved").length,
          disabled: registered.filter((g) => g.state === "disabled").length,
          online: registered.filter((g) => g.online).length,
          live_tokens: s.gateways.flatMap((g) => g.tokens).filter((t) => t.revoked_at === null && t.expires_at > now()).length,
        },
      });
    }
    case "gateways": {
      const state = request["state"];
      return admin("gateways", { gateways: s.gateways.filter((g) => state === null || g.state === state).map((g) => adminGateway(s, g)), more: false });
    }
    case "gateway": {
      const g = gatewayById(request["gateway"]);
      if (g === undefined) return 404;
      return admin("gateway", { gateway: adminGateway(s, g), tokens: g.tokens });
    }
    case "set_gateway_approval": {
      const g = gatewayById(request["gateway"]);
      if (g === undefined) return 404;
      g.state = request["enabled"] === true ? "approved" : "disabled";
      return DONE;
    }
    case "revoke_gateway_token": {
      const token = s.gateways.flatMap((g) => g.tokens).find((x) => same(request["token"], x.id));
      if (token === undefined) return 404;
      token.revoked_at = now();
      return DONE;
    }
    case "set_admin": {
      const a = byId(request["account"]);
      if (a === undefined) return 404;
      a.admin = request["admin"] === true;
      return DONE;
    }
    case "set_tag": {
      const a = byId(request["account"]);
      const tag = Number(request["tag"]);
      if (a === undefined) return 404;
      if (s.accounts.some((x) => x !== a && x.tag === tag)) return 409;
      a.tag = tag;
      return DONE;
    }
    case "reset_login":
      return admin("reset_code", { code: "RESET-7Q3K-AXWD-0P4N-9RTB-5MHE-ZC2G-8VFJ-1YS6", expires_at: now() + DAY });
    case "legacy_accounts":
      return admin("legacy_accounts", {
        store: {
          deadline: now() + 45 * DAY,
          accounts: ["oldfriend", "night_owl", "tablemaster"].map((username, i) => ({
            legacy_id: `legacy-${i + 1}`,
            username,
            display_name: i === 1 ? null : `${username[0]?.toUpperCase()}${username.slice(1)}`,
            tag: i === 1 ? null : 0x1a2b + i,
            progress: (["unstarted", "reserved", "restored"] as const)[i],
            imported_at: now() - 10 * DAY,
            failures: i,
            locked_until: null,
            must_change_password: i === 0,
            banned: false,
            snapshot_bytes: 2_048 * (i + 1),
          })),
        },
      });
  }
  const report = reportsAnswer(s.reports, request);
  if (report !== undefined) return report;
  return 400;
}
