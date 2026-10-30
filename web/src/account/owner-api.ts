// An account's own area (central-api `owner`, docs/web-accounts.md §"The
// account area"): its sessions, the gateways it registered and their
// tokens. Every answer is checked against its exact shape, as api.ts does;
// a token's bytes arrive once, in the answer to `create_token`, and the page
// shows them in their text form (`tokenText`) and keeps them nowhere.

import { answer, bytes, exact, isUuidV7, malformed, poster, safeInteger, type Endpoint, type Json } from "./api";

export type SessionKind = "native" | "browser";
export type GatewayState = "pending" | "approved" | "disabled";

export interface SessionInfo {
  kind: SessionKind;
  current: boolean;
  signedInAt: number;
  expiresAt: number;
  /** The gateways the session is signed in to now, by origin. */
  gateways: string[];
}

export interface Scopes {
  /** Sign players in through Central. */
  sso: boolean;
  /** Beat for the public gateway list. */
  listing: boolean;
}

export interface TokenInfo {
  id: Uint8Array;
  name: string;
  scopes: Scopes;
  createdAt: number;
  expiresAt: number;
  revokedAt: number | null;
  lastUsedAt: number | null;
}

export interface Metadata {
  name: string;
  version: string;
  website: string;
}

export interface MyGateway {
  id: Uint8Array;
  origin: string;
  metadata: Metadata;
  state: GatewayState;
  online: boolean;
  lastSeen: number | null;
  createdAt: number;
  tokens: TokenInfo[];
}

export interface CreatedToken {
  id: Uint8Array;
  /** The token's text form, `bgt_` and 64 hex digits: shown once. */
  text: string;
  expiresAt: number;
}

/** central-api's `TOKEN_PREFIX`. */
export const TOKEN_PREFIX = "bgt_";
/** central-api's `MAX_TOKEN_DAYS`. */
export const MAX_TOKEN_DAYS = 366;
/** central-api's `MAX_TOKEN_NAME`, in bytes. */
export const MAX_TOKEN_NAME = 64;

const hex = (b: Uint8Array): string => Array.from(b, (x) => x.toString(16).padStart(2, "0")).join("");

/** A token's text form, as central-api's `token_text` writes it. */
export function tokenText(token: Uint8Array): string {
  if (token.length !== 32) throw malformed();
  return `${TOKEN_PREFIX}${hex(token)}`;
}

/** central-api's `token_name`: 1 to 64 bytes, no control characters, no surrounding space. */
export function validTokenName(name: string): boolean {
  const length = new TextEncoder().encode(name).length;
  // oxlint-disable-next-line no-control-regex -- the rule is about exactly these
  return length >= 1 && length <= MAX_TOKEN_NAME && name.trim() === name && !/[\u0000-\u001f\u007f-\u009f]/.test(name);
}

/** An id as a path segment: 32 lowercase hex digits. */
export const idText = hex;

/** The id `text` names, or null. */
export function idOf(text: string): Uint8Array | null {
  if (!/^[0-9a-f]{32}$/.test(text)) return null;
  const out = new Uint8Array(16);
  for (let i = 0; i < 16; i++) out[i] = Number.parseInt(text.slice(i * 2, i * 2 + 2), 16);
  return isUuidV7(out) ? out : null;
}

function id(v: unknown): Uint8Array {
  const out = bytes(v, 16);
  if (!isUuidV7(out)) throw malformed();
  return out;
}

function str(v: unknown, max: number): string {
  if (typeof v !== "string" || v.length === 0 || new TextEncoder().encode(v).length > max) throw malformed();
  return v;
}

function time(v: unknown): number {
  return safeInteger(v, 1);
}

function optionalTime(v: unknown): number | null {
  return v === null ? null : time(v);
}

function bool(v: unknown): boolean {
  if (typeof v !== "boolean") throw malformed();
  return v;
}

/** An origin as Central keeps it: `https://host[:port]`, or loopback http in development. */
function origin(v: unknown): string {
  const o = str(v, 256);
  let url: URL;
  try {
    url = new URL(o);
  } catch {
    throw malformed();
  }
  if (url.origin !== o || (url.protocol !== "https:" && url.protocol !== "http:")) throw malformed();
  return o;
}

export function gatewayState(v: unknown): GatewayState {
  if (v !== "pending" && v !== "approved" && v !== "disabled") throw malformed();
  return v;
}

export function scopes(v: unknown): Scopes {
  const o = exact(v, ["sso", "listing"]);
  const out = { sso: bool(o["sso"]), listing: bool(o["listing"]) };
  if (!out.sso && !out.listing) throw malformed();
  return out;
}

export function metadata(v: unknown, at: string): Metadata {
  const o = exact(v, ["name", "version", "website"]);
  const website = str(o["website"], 512);
  // the website lies on the gateway's own origin, or it is not shown
  let url: URL;
  try {
    url = new URL(website);
  } catch {
    throw malformed();
  }
  if (url.origin !== at) throw malformed();
  return { name: str(o["name"], 80), version: str(o["version"], 64), website };
}

export function tokenInfo(v: unknown): TokenInfo {
  const o = exact(v, ["id", "name", "scopes", "created_at", "expires_at", "revoked_at", "last_used_at"]);
  const name = str(o["name"], MAX_TOKEN_NAME);
  if (!validTokenName(name)) throw malformed();
  const createdAt = time(o["created_at"]);
  const expiresAt = time(o["expires_at"]);
  if (expiresAt <= createdAt) throw malformed();
  return {
    id: id(o["id"]),
    name,
    scopes: scopes(o["scopes"]),
    createdAt,
    expiresAt,
    revokedAt: optionalTime(o["revoked_at"]),
    lastUsedAt: optionalTime(o["last_used_at"]),
  };
}

function myGateway(v: unknown): MyGateway {
  const o = exact(v, ["id", "origin", "metadata", "state", "online", "last_seen", "created_at", "tokens"]);
  const at = origin(o["origin"]);
  const tokens = o["tokens"];
  if (!Array.isArray(tokens) || tokens.length > 8 * 32) throw malformed();
  return {
    id: id(o["id"]),
    origin: at,
    metadata: metadata(o["metadata"], at),
    state: gatewayState(o["state"]),
    online: bool(o["online"]),
    lastSeen: optionalTime(o["last_seen"]),
    createdAt: time(o["created_at"]),
    tokens: tokens.map(tokenInfo),
  };
}

function sessionInfo(v: unknown): SessionInfo {
  const o = exact(v, ["kind", "current", "signed_in_at", "expires_at", "gateways"]);
  const kind = o["kind"];
  if (kind !== "native" && kind !== "browser") throw malformed();
  const gateways = o["gateways"];
  if (!Array.isArray(gateways) || gateways.length > 32) throw malformed();
  return {
    kind,
    current: bool(o["current"]),
    signedInAt: time(o["signed_in_at"]),
    expiresAt: time(o["expires_at"]),
    gateways: gateways.map((g) => str(g, 256)),
  };
}

/** The answer of an owner request: `owner` with exactly `keys` in its `answer`. */
function owned(raw: unknown, kind: string, keys: readonly string[]): Json {
  const a = exact(answer(raw, "owner", ["answer"])["answer"], ["answer", ...keys]);
  if (a["answer"] !== kind) throw malformed();
  return a;
}

export const parseOwner = {
  sessions(raw: unknown): SessionInfo[] {
    const s = owned(raw, "sessions", ["sessions"])["sessions"];
    if (!Array.isArray(s) || s.length > 4) throw malformed();
    return s.map(sessionInfo);
  },
  gateways(raw: unknown): MyGateway[] {
    const g = owned(raw, "gateways", ["gateways"])["gateways"];
    if (!Array.isArray(g) || g.length > 8) throw malformed();
    return g.map(myGateway);
  },
  gatewayAdded(raw: unknown): Uint8Array {
    return id(owned(raw, "gateway_added", ["gateway"])["gateway"]);
  },
  tokenCreated(raw: unknown): CreatedToken {
    const o = owned(raw, "token_created", ["id", "token", "expires_at"]);
    return { id: id(o["id"]), text: tokenText(bytes(o["token"], 32)), expiresAt: time(o["expires_at"]) };
  },
  done(raw: unknown): void {
    answer(raw, "done");
  },
};

export interface OwnerApi {
  sessions(): Promise<SessionInfo[]>;
  /** Ends the client's session, and with it its sign-ins at gateways. */
  endClientSession(): Promise<void>;
  gateways(): Promise<MyGateway[]>;
  addGateway(origin: string, name: string): Promise<Uint8Array>;
  /** Needs a password sign-in less than 30 minutes old. */
  createToken(gateway: Uint8Array, name: string, scopes: Scopes, days: number): Promise<CreatedToken>;
  revokeToken(token: Uint8Array): Promise<void>;
}

const owner = (request: Json): Json => ({ operation: "owner", request });

export function ownerApi(endpoint: Endpoint = {}): OwnerApi {
  const post = poster(endpoint);
  return {
    sessions: () => post(owner({ action: "sessions" }), parseOwner.sessions),
    endClientSession: () => post(owner({ action: "end_client_session" }), parseOwner.done),
    gateways: () => post(owner({ action: "gateways" }), parseOwner.gateways),
    addGateway: (at, name) => post(owner({ action: "add_gateway", origin: at, name }), parseOwner.gatewayAdded),
    createToken: (gateway, name, s, days) =>
      post(
        owner({ action: "create_token", gateway: Array.from(gateway), name, scopes: { sso: s.sso, listing: s.listing }, days }),
        parseOwner.tokenCreated,
      ),
    revokeToken: (token) => post(owner({ action: "revoke_token", token: Array.from(token) }), parseOwner.done),
  };
}
