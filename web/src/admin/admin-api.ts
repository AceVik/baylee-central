// Central's administration (central-api `admin`, docs/administration.md):
// what an administrator reads and changes, on the browser surface, with the
// cookie of an administrator's session. A change needs a password sign-in
// less than 30 minutes old; Central refuses an older one (`rejected`), and
// the pages ask for the password before they try. Every answer is checked
// against its exact shape, as api.ts does.

import { answer, bytes, exact, isUuidV7, malformed, poster, safeInteger, type Endpoint, type Json } from "../account/api";
import { validDisplayName, validTag } from "../account/names";
import { gatewayState, metadata, tokenInfo, type GatewayState, type Metadata, type TokenInfo } from "../account/owner-api";

/** central-api's `admin::PAGE`. */
export const PAGE = 50;

export type AccountState = "pending" | "enrolled" | "restored";

export interface AdminAccount {
  id: Uint8Array;
  username: string;
  displayName: string | null;
  tag: number | null;
  source: "legacy" | "registered";
  state: AccountState;
  createdAt: number;
  lastSignInAt: number | null;
  admin: boolean;
  resetUntil: number | null;
}

export interface OwnedGateway {
  id: Uint8Array;
  origin: string;
  approved: boolean;
}

export interface AuditEvent {
  id: Uint8Array;
  at: number;
  actor: Uint8Array | null;
  action: string;
  account: Uint8Array | null;
  gateway: Uint8Array | null;
  betaKey: Uint8Array | null;
}

export interface AccountDetail {
  account: AdminAccount;
  recovery: boolean;
  gateways: OwnedGateway[];
  audit: AuditEvent[];
}

export interface AdminGateway {
  id: Uint8Array;
  origin: string;
  metadata: Metadata;
  official: boolean;
  owner: Uint8Array | null;
  ownerName: string | null;
  ownerTag: number | null;
  state: GatewayState;
  online: boolean;
  lastSeen: number | null;
  createdAt: number;
  liveTokens: number;
  credentialPair: boolean;
}

export interface Page<T> {
  items: T[];
  more: boolean;
}

function id(v: unknown): Uint8Array {
  const out = bytes(v, 16);
  if (!isUuidV7(out)) throw malformed();
  return out;
}

const optionalId = (v: unknown): Uint8Array | null => (v === null ? null : id(v));
const optionalTime = (v: unknown): number | null => (v === null ? null : safeInteger(v, 1));

function bool(v: unknown): boolean {
  if (typeof v !== "boolean") throw malformed();
  return v;
}

function name(v: unknown): string | null {
  if (v === null) return null;
  if (typeof v !== "string" || !validDisplayName(v)) throw malformed();
  return v;
}

function tag(v: unknown): number | null {
  if (v === null) return null;
  if (typeof v !== "number" || !validTag(v)) throw malformed();
  return v;
}

function origin(v: unknown): string {
  if (typeof v !== "string" || v.length === 0 || v.length > 256) throw malformed();
  let url: URL;
  try {
    url = new URL(v);
  } catch {
    throw malformed();
  }
  if (url.origin !== v) throw malformed();
  return v;
}

function account(v: unknown): AdminAccount {
  const o = exact(v, ["id", "username", "display_name", "tag", "source", "state", "created_at", "last_sign_in_at", "admin", "reset_until"]);
  const username = o["username"];
  if (typeof username !== "string" || !/^[a-z0-9][a-z0-9_.-]{2,23}$/.test(username)) throw malformed();
  const source = o["source"];
  if (source !== "legacy" && source !== "registered") throw malformed();
  const state = o["state"];
  if (state !== "pending" && state !== "enrolled" && state !== "restored") throw malformed();
  return {
    id: id(o["id"]),
    username,
    displayName: name(o["display_name"]),
    tag: tag(o["tag"]),
    source,
    state,
    createdAt: safeInteger(o["created_at"], 1),
    lastSignInAt: optionalTime(o["last_sign_in_at"]),
    admin: bool(o["admin"]),
    resetUntil: optionalTime(o["reset_until"]),
  };
}

function event(v: unknown): AuditEvent {
  const o = exact(v, ["id", "at", "actor", "action", "account", "gateway", "beta_key"]);
  const action = o["action"];
  if (typeof action !== "string" || !/^[a-z_]{1,32}\.[a-z_]{1,32}$/.test(action)) throw malformed();
  return {
    id: id(o["id"]),
    at: safeInteger(o["at"], 1),
    actor: optionalId(o["actor"]),
    action,
    account: optionalId(o["account"]),
    gateway: optionalId(o["gateway"]),
    betaKey: optionalId(o["beta_key"]),
  };
}

function ownedGateway(v: unknown): OwnedGateway {
  const o = exact(v, ["id", "origin", "approved"]);
  return { id: id(o["id"]), origin: origin(o["origin"]), approved: bool(o["approved"]) };
}

function adminGateway(v: unknown): AdminGateway {
  const o = exact(v, [
    "id",
    "origin",
    "metadata",
    "official",
    "owner",
    "owner_name",
    "owner_tag",
    "state",
    "online",
    "last_seen",
    "created_at",
    "live_tokens",
    "credential_pair",
  ]);
  const at = origin(o["origin"]);
  const official = bool(o["official"]);
  const owner = optionalId(o["owner"]);
  // the official gateway has no owner, and every other one has
  if (official !== (owner === null)) throw malformed();
  return {
    id: id(o["id"]),
    origin: at,
    metadata: metadata(o["metadata"], at),
    official,
    owner,
    ownerName: name(o["owner_name"]),
    ownerTag: tag(o["owner_tag"]),
    state: gatewayState(o["state"]),
    online: bool(o["online"]),
    lastSeen: optionalTime(o["last_seen"]),
    createdAt: safeInteger(o["created_at"], 1),
    liveTokens: safeInteger(o["live_tokens"], 0),
    credentialPair: bool(o["credential_pair"]),
  };
}

function list<T>(v: unknown, max: number, read: (v: unknown) => T): T[] {
  if (!Array.isArray(v) || v.length > max) throw malformed();
  return v.map(read);
}

/** The answer of an admin read: `admin` with exactly `keys` in its `answer`. */
function admin(raw: unknown, kind: string, keys: readonly string[]): Json {
  const a = exact(answer(raw, "admin", ["answer"])["answer"], ["answer", ...keys]);
  if (a["answer"] !== kind) throw malformed();
  return a;
}

/** central-api's reset code as printed: `RESET` and eight groups of four Crockford symbols. */
const RESET_CODE = /^RESET(-[0-9A-HJKMNP-TV-Z]{4}){8}$/;

export const parseAdmin = {
  accounts(raw: unknown): Page<AdminAccount> {
    const o = admin(raw, "accounts", ["accounts", "more"]);
    return { items: list(o["accounts"], PAGE, account), more: bool(o["more"]) };
  },
  account(raw: unknown): AccountDetail {
    const o = admin(raw, "account", ["account", "recovery", "gateways", "audit"]);
    return {
      account: account(o["account"]),
      recovery: bool(o["recovery"]),
      gateways: list(o["gateways"], 20, ownedGateway),
      audit: list(o["audit"], 20, event),
    };
  },
  audit(raw: unknown): Page<AuditEvent> {
    const o = admin(raw, "audit", ["events", "more"]);
    return { items: list(o["events"], PAGE, event), more: bool(o["more"]) };
  },
  gateways(raw: unknown): Page<AdminGateway> {
    const o = admin(raw, "gateways", ["gateways", "more"]);
    return { items: list(o["gateways"], PAGE, adminGateway), more: bool(o["more"]) };
  },
  gateway(raw: unknown): { gateway: AdminGateway; tokens: TokenInfo[] } {
    const o = admin(raw, "gateway", ["gateway", "tokens"]);
    return { gateway: adminGateway(o["gateway"]), tokens: list(o["tokens"], 20, tokenInfo) };
  },
  resetCode(raw: unknown): { code: string; expiresAt: number } {
    const o = admin(raw, "reset_code", ["code", "expires_at"]);
    const code = o["code"];
    if (typeof code !== "string" || !RESET_CODE.test(code)) throw malformed();
    return { code, expiresAt: safeInteger(o["expires_at"], 1) };
  },
  done(raw: unknown): void {
    answer(raw, "done");
  },
};

export interface AdminApi {
  accounts(query: string | null, before: Uint8Array | null): Promise<Page<AdminAccount>>;
  account(id: Uint8Array): Promise<AccountDetail>;
  audit(account: Uint8Array | null, before: Uint8Array | null): Promise<Page<AuditEvent>>;
  gateways(state: GatewayState | null, before: Uint8Array | null): Promise<Page<AdminGateway>>;
  gateway(id: Uint8Array): Promise<{ gateway: AdminGateway; tokens: TokenInfo[] }>;
  setAdmin(id: Uint8Array, admin: boolean): Promise<void>;
  resetLogin(id: Uint8Array): Promise<{ code: string; expiresAt: number }>;
  setTag(id: Uint8Array, tag: number): Promise<void>;
  setGatewayApproval(id: Uint8Array, enabled: boolean): Promise<void>;
  revokeGatewayToken(id: Uint8Array): Promise<void>;
}

const arr = (b: Uint8Array | null) => (b === null ? null : Array.from(b));
const request = (r: Json): Json => ({ operation: "admin", request: r });

export function adminApi(endpoint: Endpoint = {}): AdminApi {
  const post = poster(endpoint);
  return {
    accounts: (query, before) => post(request({ action: "accounts", query, before: arr(before) }), parseAdmin.accounts),
    account: (a) => post(request({ action: "account", account: arr(a) }), parseAdmin.account),
    audit: (a, before) => post(request({ action: "audit", account: arr(a), before: arr(before) }), parseAdmin.audit),
    gateways: (state, before) => post(request({ action: "gateways", state, before: arr(before) }), parseAdmin.gateways),
    gateway: (g) => post(request({ action: "gateway", gateway: arr(g) }), parseAdmin.gateway),
    setAdmin: (a, isAdmin) => post(request({ action: "set_admin", account: arr(a), admin: isAdmin }), parseAdmin.done),
    resetLogin: (a) => post(request({ action: "reset_login", account: arr(a) }), parseAdmin.resetCode),
    setTag: (a, t) => post(request({ action: "set_tag", account: arr(a), tag: t }), parseAdmin.done),
    setGatewayApproval: (g, enabled) => post(request({ action: "set_gateway_approval", gateway: arr(g), enabled }), parseAdmin.done),
    revokeGatewayToken: (t) => post(request({ action: "revoke_gateway_token", token: arr(t) }), parseAdmin.done),
  };
}
