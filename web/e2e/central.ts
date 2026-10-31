// Central's browser surface for the account pages' tests, answered in the
// browser from docs/web-accounts.md's shapes, and strict about them: a
// request without `X-Baylee-Central: 1` or JSON, with an Authorization
// header, from another origin, with an operation the surface does not take,
// with a field too many or too few, or with a byte array of the wrong length
// is refused (403) and noted, and a test fails on any note. The session is
// the mock's own (the real one is an `__Host-` cookie, which plain HTTP on
// loopback cannot carry). Synthetic data only.
//
// `stubCrypto` answers the crypto Worker's script with a stand-in that
// speaks its message protocol, for the flows' tests; the tests of the real
// WebAssembly use the built Worker and see its messages here.

import { createHash } from "node:crypto";

import type { Page, Request, Route } from "@playwright/test";

import { csp } from "../scripts/csp.ts";

import { adminAnswer, areaState, ownerAnswer, type AreaState } from "./area.ts";

export const KEY = "CENTRAL-TEST-0000-0000-0001";
export const PASSWORD = "violet lantern harbour quietly";
export const CODE = "Y06Z-NKPA-ZTZE-Y093-8NKR-KAYD-XZZD-SEMR";
/** The code the stand-in Worker makes after a sign-in, for an account without one. */
export const NEW_CODE = "7Q3K-AXWD-0P4N-9RTB-5MHE-ZC2G-8VFJ-1YS6";
export const ACCOUNT = [0x01, 0x92, 0xd0, 0xc5, 0x7b, 0x3e, 0x7a, 0x1f, 0x8c, 0x2d, 0x3e, 0x4f, 0x5a, 0x6b, 0x7c, 0x8d];
/** The initial document's 125 bytes, sealed: 1 + 24 + 125 + 16. */
export const SEALED_DOCUMENT = 166;
/** The tag the mock draws for a registration: `#3fa2`. */
export const TAG = 0x3fa2;

const n = (length: number, byte = 0) => Array.from({ length }, () => byte);

/** What each operation's request holds besides `operation`. */
const FIELDS: Readonly<Record<string, readonly string[]>> = {
  // the page always sends the name chosen with the form
  register: ["username", "beta_key", "display_name"],
  enrollment_start: ["ticket", "message"],
  enrollment_finish: ["ticket", "record", "wrapped_master", "document", "recovery"],
  discover: ["username"],
  login_start: ["identity", "message"],
  login_finish: ["attempt", "message"],
  session_status: [],
  logout: [],
  terms: [],
  terms_status: [],
  accept_terms: ["subject", "version", "digest"],
  gateway_directory: ["after"],
  account_summary: [],
  set_recovery: ["recovery"],
  set_display_name: ["display_name"],
  owner: ["request"],
  admin: ["request"],
};

/** Exact byte lengths, by operation and field. */
const LENGTHS: Readonly<Record<string, Readonly<Record<string, number>>>> = {
  enrollment_start: { ticket: 32, message: 32 },
  enrollment_finish: { ticket: 32, record: 192, wrapped_master: 73, document: SEALED_DOCUMENT, recovery: 73 },
  login_start: { message: 96 },
  login_finish: { attempt: 32, message: 64 },
  accept_terms: { digest: 32 },
  set_recovery: { recovery: 73 },
};

export interface TermsText {
  lang: string;
  updated: string | null;
  markdown: string;
}

/** central-api's `terms::digest`, computed in Node for the mock. */
export function termsDigest(texts: readonly TermsText[]): number[] {
  const hash = createHash("sha256").update("baylee-terms-v1\0");
  for (const t of texts) {
    for (const field of [t.lang, t.updated ?? "", t.markdown]) {
      const bytes = Buffer.from(field, "utf8");
      const length = Buffer.alloc(8);
      length.writeBigUInt64BE(BigInt(bytes.length));
      hash.update(length).update(bytes);
    }
  }
  return Array.from(hash.digest());
}

export interface CentralMock {
  /** Every operation asked, in order. */
  operations: string[];
  /** Requests refused for their shape: must stay empty. */
  violations: string[];
  /** The bodies, by operation, last first. */
  bodies: Map<string, Record<string, unknown>>;
  /** An account moved from the official gateway has no recovery wrapper until it stores one. */
  accounts: Map<string, { wrapped: number[]; recovery: number[] | null; displayName: string | null; tag: number | null }>;
  signedIn: string | null;
  accepted: { version: string; digest: number[] } | null;
}

interface Options {
  /** Central's terms in force, or none (as today: `draft`). */
  terms?: { version: string; texts: TermsText[]; digest?: number[] } | null;
  /** Accounts that exist already, registered without a display name: name → password stand-in. */
  accounts?: string[];
  /** Registered accounts that exist already with a display name: name → display name. */
  named?: Record<string, string>;
  /** Accounts moved from the official gateway, without a recovery code. */
  moved?: string[];
  /** Answer the crypto's first message with bytes the real module refuses. */
  refuseCrypto?: boolean;
  /** Signed in already as this account (one of `accounts` or `named`), its sign-in ten minutes old. */
  signedIn?: string;
  /** Answer the account area's and the administration's requests (`area.ts`), as an administrator or not. */
  area?: { admin: boolean };
}

function bad(m: CentralMock, why: string, route: Route) {
  m.violations.push(why);
  return route.fulfill({ status: 403, contentType: "text/plain", body: "account request rejected\n" });
}

function shape(op: string, body: Record<string, unknown>): string | null {
  const fields = FIELDS[op];
  if (fields === undefined) return `operation ${op} is not on the browser surface`;
  const keys = Object.keys(body).filter((k) => k !== "operation").sort();
  if (keys.join() !== [...fields].sort().join()) return `${op} has fields ${keys.join()}`;
  for (const [field, length] of Object.entries(LENGTHS[op] ?? {})) {
    const v = body[field];
    if (!Array.isArray(v) || v.length !== length || !v.every((b) => Number.isInteger(b) && b >= 0 && b <= 255)) return `${op}.${field} is not ${length} bytes`;
  }
  return null;
}

function headersOk(request: Request, origin: string, headers: Record<string, string>): string | null {
  if (request.method() !== "POST") return "not a POST";
  if (headers["content-type"] !== "application/json") return "Content-Type is not application/json";
  if (headers["x-baylee-central"] !== "1") return "no X-Baylee-Central: 1";
  if (headers["authorization"] !== undefined) return "an Authorization header";
  if (headers["origin"] !== origin) return `Origin ${headers["origin"]}`;
  const site = headers["sec-fetch-site"];
  if (site !== undefined && site !== "same-origin") return `Sec-Fetch-Site ${site}`;
  return null;
}

const json = (route: Route, body: unknown) => route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(body) });
const refuse = (route: Route, status: number) => route.fulfill({ status, contentType: "text/plain", body: "account request rejected\n" });

export async function mockCentral(page: Page, origin: string, opts: Options = {}): Promise<CentralMock> {
  const m: CentralMock = { operations: [], violations: [], bodies: new Map(), accounts: new Map(), signedIn: null, accepted: null };
  for (const name of opts.accounts ?? []) m.accounts.set(name, { wrapped: n(73, 2), recovery: n(73, 4), displayName: null, tag: TAG });
  for (const [name, displayName] of Object.entries(opts.named ?? {})) m.accounts.set(name, { wrapped: n(73, 2), recovery: n(73, 4), displayName, tag: TAG });
  // moved before its name and tag were carried
  for (const name of opts.moved ?? []) m.accounts.set(name, { wrapped: n(73, 2), recovery: null, displayName: null, tag: null });
  let signedInAt = 0;
  if (opts.signedIn !== undefined) {
    m.signedIn = opts.signedIn;
    signedInAt = Math.floor(Date.now() / 1000) - 600;
  }
  const area: AreaState | null = opts.area === undefined ? null : areaState(opts.area.admin);
  const tickets = new Map<string, { name: string; displayName: string }>();
  let attempt: string | null = null;
  const terms = opts.terms ?? null;
  const version = terms === null ? null : { version: terms.version, digest: terms.digest ?? termsDigest(terms.texts) };
  const identity = { account: ACCOUNT, epoch: 1 };

  await page.route("**/api/v1", async (route) => {
    const request = route.request();
    const wrongHeaders = headersOk(request, origin, await request.allHeaders());
    if (wrongHeaders !== null) return bad(m, wrongHeaders, route);
    let body: Record<string, unknown>;
    try {
      body = request.postDataJSON() as Record<string, unknown>;
    } catch {
      return bad(m, "not JSON", route);
    }
    const op = String(body["operation"]);
    m.operations.push(op);
    m.bodies.set(op, body);
    const wrongShape = shape(op, body);
    if (wrongShape !== null) return bad(m, wrongShape, route);

    switch (op) {
      case "register": {
        if (body["beta_key"] !== KEY) return refuse(route, 401);
        const name = String(body["username"]);
        if (!/^[A-Za-z0-9][A-Za-z0-9_-]{1,14}[A-Za-z0-9]$/.test(String(body["display_name"]))) return refuse(route, 401);
        if (m.accounts.has(name)) return refuse(route, 409);
        const ticket = n(32, tickets.size + 1);
        tickets.set(ticket.join(), { name, displayName: String(body["display_name"]) });
        return json(route, { result: "enrollment", identity, ticket });
      }
      case "enrollment_start":
        if (!tickets.has(String(body["ticket"]))) return refuse(route, 401);
        // the real module refuses a zero point; the stand-in takes it
        return json(route, { result: "enrollment_challenge", message: n(64, opts.refuseCrypto ? 0 : 5) });
      case "enrollment_finish": {
        const reserved = tickets.get(String(body["ticket"]));
        if (reserved === undefined) return refuse(route, 401);
        tickets.delete(String(body["ticket"]));
        m.accounts.set(reserved.name, {
          wrapped: body["wrapped_master"] as number[],
          recovery: body["recovery"] as number[],
          displayName: reserved.displayName,
          tag: TAG,
        });
        return json(route, { result: "done" });
      }
      case "discover":
        return json(route, {
          result: "discovery",
          discovery: m.accounts.has(String(body["username"])) ? { kind: "central", identity } : { kind: "legacy" },
        });
      case "login_start":
        attempt = "pending";
        return json(route, { result: "login_challenge", attempt: n(32, 9), message: n(320, opts.refuseCrypto ? 0 : 7) });
      case "login_finish": {
        const name = [...m.accounts.keys()][0];
        if (attempt === null || name === undefined) return refuse(route, 401);
        attempt = null;
        m.signedIn = name;
        signedInAt = Math.floor(Date.now() / 1000);
        return json(route, {
          result: "browser_session",
          identity,
          username: name,
          wrapped_master: m.accounts.get(name)?.wrapped,
          expires_at: Math.floor(Date.now() / 1000) + 1800,
        });
      }
      case "session_status":
        if (m.signedIn === null) return refuse(route, 401);
        return json(route, { result: "browser_status", identity, username: m.signedIn, expires_at: Math.floor(Date.now() / 1000) + 1800 });
      case "logout":
        if (m.signedIn === null) return refuse(route, 401);
        m.signedIn = null;
        return json(route, { result: "done" });
      case "terms":
        return json(route, { result: "terms", terms: terms === null || version === null ? null : { version, texts: terms.texts } });
      case "terms_status": {
        if (m.signedIn === null) return refuse(route, 401);
        const accepted = version === null || (m.accepted?.version === version.version && m.accepted.digest.join() === version.digest.join());
        return json(route, { result: "terms_status", current: version, accepted });
      }
      case "account_summary": {
        const account = m.signedIn === null ? undefined : m.accounts.get(m.signedIn);
        if (m.signedIn === null || account === undefined) return refuse(route, 401);
        return json(route, {
          result: "account_summary",
          identity,
          username: m.signedIn,
          display_name: account.displayName,
          tag: account.tag,
          source: account.recovery === null || opts.moved?.includes(m.signedIn) ? "legacy" : "registered",
          recovery: account.recovery !== null,
          admin: area?.accounts[area.me]?.admin ?? false,
          signed_in_at: signedInAt,
          expires_at: signedInAt + 1800,
        });
      }
      case "set_recovery": {
        const account = m.signedIn === null ? undefined : m.accounts.get(m.signedIn);
        if (m.signedIn === null || account === undefined) return refuse(route, 401);
        const recovery = body["recovery"] as number[];
        if (account.recovery !== null) return account.recovery.join() === recovery.join() ? json(route, { result: "done" }) : refuse(route, 409);
        account.recovery = recovery;
        return json(route, { result: "done" });
      }
      case "set_display_name": {
        const account = m.signedIn === null ? undefined : m.accounts.get(m.signedIn);
        if (m.signedIn === null || account === undefined) return refuse(route, 401);
        if (version !== null && m.accepted?.version !== version.version) return refuse(route, 403);
        const name = body["display_name"];
        if (typeof name !== "string" || !/^[A-Za-z0-9][A-Za-z0-9_-]{1,14}[A-Za-z0-9]$/.test(name) || account.tag === null) return refuse(route, 401);
        if (account.displayName !== null) return account.displayName === name ? json(route, { result: "done" }) : refuse(route, 409);
        account.displayName = name;
        return json(route, { result: "done" });
      }
      case "accept_terms": {
        if (m.signedIn === null) return refuse(route, 401);
        const subject = body["subject"] as Record<string, unknown>;
        if (JSON.stringify(subject) !== '{"kind":"central"}') return bad(m, "accept_terms for another subject", route);
        if (version === null || body["version"] !== version.version || (body["digest"] as number[]).join() !== version.digest.join()) return refuse(route, 409);
        m.accepted = { version: version.version, digest: body["digest"] as number[] };
        return json(route, { result: "done" });
      }
      case "owner":
      case "admin": {
        if (m.signedIn === null || area === null) return refuse(route, 401);
        const asked = body["request"] as Record<string, unknown>;
        if (op === "admin" && area.accounts[area.me]?.admin !== true) return refuse(route, 401);
        const answer = op === "owner" ? ownerAnswer(area, asked) : adminAnswer(area, asked);
        return typeof answer === "number" ? refuse(route, answer) : json(route, answer);
      }
      case "gateway_directory":
        // Checked like every request here, then answered by `mock`'s directory.
        return route.fallback();
    }
    return bad(m, `unhandled ${op}`, route);
  });
  return m;
}

/** The crypto Worker's stand-in: the protocol of src/account/crypto.ts, knowing only `PASSWORD`. */
const STUB = `
const fill = (n, b) => new Uint8Array(n).fill(b);
let password = null;
self.addEventListener("message", (e) => {
  const m = e.data;
  const reply = (r) => self.postMessage(r);
  if (m.op === "enrollment_start") { password = m.password; reply({ op: m.op, message: fill(32, 1) }); }
  else if (m.op === "enrollment_finish") reply({ op: m.op, record: fill(192, 3), wrappedMaster: fill(73, 2), recovery: fill(73, 4), document: fill(${SEALED_DOCUMENT}, 6), recoveryCode: ${JSON.stringify(CODE)} });
  else if (m.op === "login_start") { password = m.password; reply({ op: m.op, message: fill(96, 8) }); }
  else if (m.op === "login_finish") reply(password === ${JSON.stringify(PASSWORD)} ? { op: m.op, message: fill(64, 10) } : { op: "error", error: "rejected" });
  else if (m.op === "login_confirm") reply(m.wrappedMaster.every((b) => b === 2) ? { op: m.op } : { op: "error", error: "rejected" });
  else if (m.op === "recovery_create") reply(m.wrappedMaster.every((b) => b === 2) ? { op: m.op, recovery: fill(73, 11), recoveryCode: ${JSON.stringify(NEW_CODE)} } : { op: "error", error: "rejected" });
});
`;

export async function stubCrypto(page: Page): Promise<void> {
  await page.route("**/assets/crypto-worker-*.js", (route) =>
    route.fulfill({ status: 200, contentType: "text/javascript; charset=utf-8", headers: { "Content-Security-Policy": csp() }, body: STUB }),
  );
}
