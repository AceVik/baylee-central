// Central's browser surface (docs/web-accounts.md §"The browser surface"):
// one route, `POST /api/v1` on this origin, taking central-api's tagged
// `Request` JSON for the operations the website may use. Every request
// carries `Content-Type: application/json` and `X-Baylee-Central: 1` (a
// custom header forces a preflight cross-origin, and Central sends no CORS
// header), goes with `credentials: same-origin` so the `__Host-central`
// session cookie travels, and never an Authorization header: the page never
// sees the session token. A refusal is plain text with a status, never read.
//
// Bytes travel as JSON arrays of numbers (serde's `Vec<u8>` and `[u8; N]`).
// Every answer is checked against its exact shape: the expected `result`,
// no other field, each byte array its exact length; anything else is
// `malformed` and nothing of it is used.

import { CENTRAL_API } from "../config";
import { canonicalName, validName } from "./names";

/** Why a request did not give its answer. */
export type Failure =
  /** 401: a bad key, a wrong proof, no live session. */
  | "rejected"
  /** 409: the name is taken, or what was accepted changed. */
  | "conflict"
  /** 429: too many attempts from here or overall. */
  | "busy"
  /** 403: refused at the door, or the terms must be accepted first. */
  | "forbidden"
  /** 5xx: Central is there but cannot answer. */
  | "unavailable"
  /** No answer at all. */
  | "network"
  /** An answer of the wrong shape, or another status. */
  | "malformed";

export class ApiError extends Error {
  readonly kind: Failure;
  constructor(kind: Failure) {
    super(`central: ${kind}`);
    this.kind = kind;
  }
}

export interface Identity {
  /** The account's UUIDv7, 16 bytes. */
  account: Uint8Array;
  /** Its vault epoch, a positive safe integer. */
  epoch: number;
}

export interface TermsVersion {
  version: string;
  digest: Uint8Array;
}

export interface TermsText {
  lang: string;
  updated: string | null;
  markdown: string;
}

export interface TermsDocument {
  version: TermsVersion;
  texts: TermsText[];
}

/** What a sign-in leaves the page: who, until when; the cookie holds the rest. */
export interface BrowserSession {
  identity: Identity;
  /** The account's canonical name, as Central keeps it. */
  username: string;
  wrappedMaster: Uint8Array;
  expiresAt: number;
}

export interface SessionStatus {
  identity: Identity;
  /** The account's canonical name, as Central keeps it. */
  username: string;
  expiresAt: number;
}

// ---------------------------------------------------------------- shapes

type Json = Record<string, unknown>;

const malformed = () => new ApiError("malformed");

function isObject(v: unknown): v is Json {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

/** `v` is an object with exactly these keys. */
function exact(v: unknown, keys: readonly string[]): Json {
  if (!isObject(v)) throw malformed();
  const own = Object.keys(v);
  if (own.length !== keys.length || !keys.every((k) => Object.hasOwn(v, k))) throw malformed();
  return v;
}

/** The answer `raw` is `result` with exactly `keys` besides. */
function answer(raw: unknown, result: string, keys: readonly string[] = []): Json {
  const o = exact(raw, ["result", ...keys]);
  if (o["result"] !== result) throw malformed();
  return o;
}

/** A JSON byte array, of exactly `length` bytes when given, else 1 to `max`. */
export function bytes(v: unknown, length?: number, max = 1_056_768): Uint8Array {
  if (!Array.isArray(v)) throw malformed();
  if (length !== undefined ? v.length !== length : v.length === 0 || v.length > max) throw malformed();
  const out = new Uint8Array(v.length);
  for (let i = 0; i < v.length; i++) {
    const b: unknown = v[i];
    if (typeof b !== "number" || !Number.isInteger(b) || b < 0 || b > 255) throw malformed();
    out[i] = b;
  }
  return out;
}

function safeInteger(v: unknown, min: number): number {
  if (typeof v !== "number" || !Number.isSafeInteger(v) || v < min) throw malformed();
  return v;
}

/** central-api's `valid_id`: version 7, RFC 9562 variant. */
export function isUuidV7(id: Uint8Array): boolean {
  return id.length === 16 && (id[6] ?? 0) >> 4 === 7 && (id[8] ?? 0) >> 6 === 2;
}

export function identity(v: unknown): Identity {
  const o = exact(v, ["account", "epoch"]);
  const account = bytes(o["account"], 16);
  if (!isUuidV7(account)) throw malformed();
  return { account, epoch: safeInteger(o["epoch"], 1) };
}

export function sameIdentity(a: Identity, b: Identity): boolean {
  return a.epoch === b.epoch && a.account.every((byte, i) => byte === b.account[i]);
}

/** A name exactly as Central keeps it: canonical (lower case) and within the rule. */
function canonicalUsername(v: unknown): string {
  if (typeof v !== "string" || v !== canonicalName(v) || !validName(v)) throw malformed();
  return v;
}

function termsVersion(v: unknown): TermsVersion {
  const o = exact(v, ["version", "digest"]);
  const version = o["version"];
  // central-api's `valid_version`: 1 to 64 printable ASCII, never `draft`
  if (typeof version !== "string" || !/^[\x21-\x7e]{1,64}$/.test(version) || version === "draft") throw malformed();
  return { version, digest: bytes(o["digest"], 32) };
}

function termsText(v: unknown): TermsText {
  const o = exact(v, ["lang", "updated", "markdown"]);
  const { lang, updated, markdown } = o;
  if (typeof lang !== "string" || !/^[a-z]{2,3}$/.test(lang)) throw malformed();
  if (updated !== null && (typeof updated !== "string" || !/^[\x21-\x7e]{1,32}$/.test(updated))) throw malformed();
  if (typeof markdown !== "string" || markdown.length === 0 || new TextEncoder().encode(markdown).length > 65_536) throw malformed();
  return { lang, updated, markdown };
}

// ---------------------------------------------------------------- answers

export const parse = {
  enrollment(raw: unknown): { identity: Identity; ticket: Uint8Array } {
    const o = answer(raw, "enrollment", ["identity", "ticket"]);
    return { identity: identity(o["identity"]), ticket: bytes(o["ticket"], 32) };
  },
  enrollmentChallenge(raw: unknown): Uint8Array {
    return bytes(answer(raw, "enrollment_challenge", ["message"])["message"], 64);
  },
  done(raw: unknown): void {
    answer(raw, "done");
  },
  /** `null` for an account Central does not know (or one still at the legacy gateway). */
  discovery(raw: unknown): Identity | null {
    const d = answer(raw, "discovery", ["discovery"])["discovery"];
    if (isObject(d) && d["kind"] === "legacy") {
      exact(d, ["kind"]);
      return null;
    }
    const o = exact(d, ["kind", "identity"]);
    if (o["kind"] !== "central") throw malformed();
    return identity(o["identity"]);
  },
  loginChallenge(raw: unknown): { attempt: Uint8Array; message: Uint8Array } {
    const o = answer(raw, "login_challenge", ["attempt", "message"]);
    return { attempt: bytes(o["attempt"], 32), message: bytes(o["message"], 320) };
  },
  // The browser surface names the account (docs/web-accounts.md): it is
  // Central's to say who is signed in, never the page's.
  browserSession(raw: unknown): BrowserSession {
    const o = answer(raw, "browser_session", ["identity", "username", "wrapped_master", "expires_at"]);
    return {
      identity: identity(o["identity"]),
      username: canonicalUsername(o["username"]),
      wrappedMaster: bytes(o["wrapped_master"], 73),
      expiresAt: safeInteger(o["expires_at"], 1),
    };
  },
  sessionStatus(raw: unknown): SessionStatus {
    const o = answer(raw, "browser_status", ["identity", "username", "expires_at"]);
    return { identity: identity(o["identity"]), username: canonicalUsername(o["username"]), expiresAt: safeInteger(o["expires_at"], 1) };
  },
  terms(raw: unknown): TermsDocument | null {
    const t = answer(raw, "terms", ["terms"])["terms"];
    if (t === null) return null;
    const o = exact(t, ["version", "texts"]);
    const texts = o["texts"];
    if (!Array.isArray(texts) || texts.length === 0 || texts.length > 4) throw malformed();
    const parsed = texts.map(termsText);
    // central-api's `validate`: strictly ascending tags, German or English among them
    if (!parsed.every((x, i) => i === 0 || (parsed[i - 1]?.lang ?? "") < x.lang)) throw malformed();
    if (!parsed.some((x) => x.lang === "de" || x.lang === "en")) throw malformed();
    return { version: termsVersion(o["version"]), texts: parsed };
  },
  termsStatus(raw: unknown): { current: TermsVersion | null; accepted: boolean } {
    const o = answer(raw, "terms_status", ["current", "accepted"]);
    if (typeof o["accepted"] !== "boolean") throw malformed();
    return { current: o["current"] === null ? null : termsVersion(o["current"]), accepted: o["accepted"] };
  },
};

// ---------------------------------------------------------------- requests

const array = (b: Uint8Array): number[] => Array.from(b);
const wireIdentity = (i: Identity) => ({ account: array(i.account), epoch: i.epoch });

export interface Api {
  register(username: string, betaKey: string): Promise<{ identity: Identity; ticket: Uint8Array }>;
  enrollmentStart(ticket: Uint8Array, message: Uint8Array): Promise<Uint8Array>;
  enrollmentFinish(body: {
    ticket: Uint8Array;
    record: Uint8Array;
    wrappedMaster: Uint8Array;
    document: Uint8Array;
    recovery: Uint8Array;
  }): Promise<void>;
  discover(username: string): Promise<Identity | null>;
  loginStart(identity: Identity, message: Uint8Array): Promise<{ attempt: Uint8Array; message: Uint8Array }>;
  loginFinish(attempt: Uint8Array, message: Uint8Array): Promise<BrowserSession>;
  sessionStatus(): Promise<SessionStatus>;
  logout(): Promise<void>;
  terms(): Promise<TermsDocument | null>;
  termsStatus(): Promise<{ current: TermsVersion | null; accepted: boolean }>;
  acceptTerms(version: TermsVersion): Promise<void>;
}

const STATUS: Readonly<Record<number, Failure>> = {
  401: "rejected",
  403: "forbidden",
  409: "conflict",
  429: "busy",
  503: "unavailable",
};

export function api({ base = CENTRAL_API, fetch: doFetch = (...a) => fetch(...a) }: { base?: string; fetch?: typeof fetch } = {}): Api {
  async function post<T>(request: Json, read: (raw: unknown) => T): Promise<T> {
    let r: Response;
    try {
      r = await doFetch(`${base}/v1`, {
        method: "POST",
        credentials: "same-origin",
        referrerPolicy: "no-referrer",
        cache: "no-store",
        redirect: "error",
        headers: { "Content-Type": "application/json", "X-Baylee-Central": "1" },
        body: JSON.stringify(request),
      });
    } catch {
      throw new ApiError("network");
    }
    if (!r.ok) throw new ApiError(STATUS[r.status] ?? (r.status >= 500 ? "unavailable" : "malformed"));
    let raw: unknown;
    try {
      raw = await r.json();
    } catch {
      throw malformed();
    }
    return read(raw);
  }

  return {
    register: (username, betaKey) => post({ operation: "register", username, beta_key: betaKey }, parse.enrollment),
    enrollmentStart: (ticket, message) =>
      post({ operation: "enrollment_start", ticket: array(ticket), message: array(message) }, parse.enrollmentChallenge),
    enrollmentFinish: (b) =>
      post(
        {
          operation: "enrollment_finish",
          ticket: array(b.ticket),
          record: array(b.record),
          wrapped_master: array(b.wrappedMaster),
          document: array(b.document),
          recovery: array(b.recovery),
        },
        parse.done,
      ),
    discover: (username) => post({ operation: "discover", username }, parse.discovery),
    loginStart: (id, message) => post({ operation: "login_start", identity: wireIdentity(id), message: array(message) }, parse.loginChallenge),
    loginFinish: (attempt, message) => post({ operation: "login_finish", attempt: array(attempt), message: array(message) }, parse.browserSession),
    sessionStatus: () => post({ operation: "session_status" }, parse.sessionStatus),
    logout: () => post({ operation: "logout" }, parse.done),
    terms: () => post({ operation: "terms" }, parse.terms),
    termsStatus: () => post({ operation: "terms_status" }, parse.termsStatus),
    acceptTerms: (v) =>
      post({ operation: "accept_terms", subject: { kind: "central" }, version: v.version, digest: array(v.digest) }, parse.done),
  };
}
