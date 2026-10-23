// The browser surface's client: what goes over the wire (docs/web-accounts.md
// §"The browser surface") and that every answer is held to its exact shape.

import { describe, expect, it } from "vitest";

import { ApiError, api, parse } from "../../src/account/api";
import { serve } from "../support/network";

const ACCOUNT = [0x01, 0x92, 0xd0, 0xc5, 0x7b, 0x3e, 0x7a, 0x1f, 0x8c, 0x2d, 0x3e, 0x4f, 0x5a, 0x6b, 0x7c, 0x8d];
const IDENTITY = { account: ACCOUNT, epoch: 1 };
const n = (length: number, byte = 1) => Array.from({ length }, () => byte);

const session = (identity: unknown) => ({ result: "browser_session", identity, username: "synthetic_name", wrapped_master: n(73), expires_at: 4_000_000_000 });
const doc = (texts: unknown, version: unknown = { version: "2026-10-21", digest: n(32) }) => ({ result: "terms", terms: { version, texts } });
const text = (lang: string) => ({ lang, updated: "2026-10-21", markdown: "# Terms" });

async function failure(p: Promise<unknown>): Promise<string> {
  try {
    await p;
  } catch (e) {
    return e instanceof ApiError ? e.kind : "other";
  }
  return "none";
}

describe("the request", () => {
  it("posts the tagged operation to /api/v1 with the two headers, the cookie and nothing else", async () => {
    const calls = serve(() => ({ body: { result: "enrollment", identity: IDENTITY, ticket: n(32) } }));
    await api().register("synthetic_name", "CENTRAL-TEST-0000-0000-0001", "Synthetic_Name");
    await api().register("synthetic_name", "CENTRAL-TEST-0000-0000-0001", null);
    expect(calls).toHaveLength(2);
    const call = calls[0];
    expect(call?.url).toBe("/api/v1");
    expect(call?.method).toBe("POST");
    expect(call?.body).toEqual({ operation: "register", username: "synthetic_name", beta_key: "CENTRAL-TEST-0000-0000-0001", display_name: "Synthetic_Name" });
    // without a name, the request is the one from before display names
    expect(calls[1]?.body).toEqual({ operation: "register", username: "synthetic_name", beta_key: "CENTRAL-TEST-0000-0000-0001" });
    expect(call?.init?.credentials).toBe("same-origin");
    expect(call?.init?.redirect).toBe("error");
    expect(call?.init?.headers).toEqual({ "Content-Type": "application/json", "X-Baylee-Central": "1" });
  });

  it("sends bytes as number arrays and the identity as Central reads it", async () => {
    const calls = serve(() => ({ body: { result: "done" } }));
    const client = api();
    await client.enrollmentFinish({
      ticket: new Uint8Array(n(32, 2)),
      record: new Uint8Array(n(192, 3)),
      wrappedMaster: new Uint8Array(n(73, 4)),
      document: new Uint8Array(n(5, 5)),
      recovery: new Uint8Array(n(73, 6)),
    });
    await client.acceptTerms({ version: "2026-10-21", digest: new Uint8Array(n(32, 7)) });
    await client.setRecovery(new Uint8Array(n(73, 8)));
    await client.setDisplayName("Synthetic_Name");
    await client.logout();
    expect(calls.map((c) => c.body)).toEqual([
      { operation: "enrollment_finish", ticket: n(32, 2), record: n(192, 3), wrapped_master: n(73, 4), document: n(5, 5), recovery: n(73, 6) },
      { operation: "accept_terms", subject: { kind: "central" }, version: "2026-10-21", digest: n(32, 7) },
      { operation: "set_recovery", recovery: n(73, 8) },
      { operation: "set_display_name", display_name: "Synthetic_Name" },
      { operation: "logout" },
    ]);
  });

  it("names each refusal by its status and never reads its text", async () => {
    for (const [status, kind] of [
      [401, "rejected"],
      [403, "forbidden"],
      [409, "conflict"],
      [429, "busy"],
      [503, "unavailable"],
      [500, "unavailable"],
      [400, "malformed"],
      [413, "malformed"],
    ] as const) {
      serve(() => ({ status, text: '{"result":"done"}' }));
      expect({ status, kind: await failure(api().logout()) }).toEqual({ status, kind });
    }
  });
});

describe("the answers", () => {
  it("take exactly their fields", () => {
    expect(() => parse.done({ result: "done", extra: 1 })).toThrow(ApiError);
    expect(() => parse.done({ result: "enrollment" })).toThrow(ApiError);
    expect(() => parse.done([])).toThrow(ApiError);
    expect(() => parse.enrollmentChallenge({ result: "enrollment_challenge", message: n(63) })).toThrow(ApiError);
    expect(() => parse.enrollmentChallenge({ result: "enrollment_challenge", message: [...n(63), 256] })).toThrow(ApiError);
    expect(() => parse.enrollmentChallenge({ result: "enrollment_challenge", message: [...n(63), 1.5] })).toThrow(ApiError);
    expect(parse.enrollmentChallenge({ result: "enrollment_challenge", message: n(64, 255) })).toHaveLength(64);
  });

  it("hold an identity to a UUIDv7 and a positive safe epoch", () => {
    expect(parse.browserSession(session(IDENTITY)).identity.epoch).toBe(1);
    const v4 = [...ACCOUNT];
    v4[6] = 0x4a;
    for (const bad of [
      { account: v4, epoch: 1 },
      { account: ACCOUNT, epoch: 0 },
      { account: ACCOUNT, epoch: 2 ** 53 },
      { account: ACCOUNT, epoch: "1" },
      { account: ACCOUNT.slice(1), epoch: 1 },
      { account: ACCOUNT, epoch: 1, legacy_id: "x" },
    ]) {
      expect(() => parse.browserSession(session(bad))).toThrow(ApiError);
    }
    expect(() => parse.browserSession({ ...session(IDENTITY), token: n(32) })).toThrow(ApiError);
    expect(() => parse.browserSession({ ...session(IDENTITY), wrapped_master: n(72) })).toThrow(ApiError);
  });

  it("take the session's named shapes only, the name canonical", () => {
    const named = { result: "browser_session", identity: IDENTITY, username: "synthetic_name", wrapped_master: n(73), expires_at: 4_000_000_000 };
    expect(parse.browserSession(named).username).toBe("synthetic_name");
    // the unnamed shape of the transition (#28) is gone
    expect(() => parse.browserSession({ result: "browser_session", identity: IDENTITY, wrapped_master: n(73), expires_at: 4_000_000_000 })).toThrow(ApiError);
    for (const name of ["Synthetic_Name", "ab", "a..b", "_abc", 7, null, ""]) {
      expect(() => parse.browserSession({ ...named, username: name })).toThrow(ApiError);
    }
    expect(() => parse.browserSession({ ...named, token: n(32) })).toThrow(ApiError);

    const status = { result: "browser_status", identity: IDENTITY, username: "synthetic_name", expires_at: 4_000_000_000 };
    expect(parse.sessionStatus(status)).toMatchObject({ username: "synthetic_name", expiresAt: 4_000_000_000 });
    const native = { result: "session_status", identity: IDENTITY, expires_at: 4_000_000_000, migration_closed: false };
    for (const bad of [
      native,
      { ...status, migration_closed: false },
      { result: "browser_status", identity: IDENTITY, expires_at: 4_000_000_000 },
      { ...status, username: "SYNTHETIC" },
      { ...native, username: "synthetic_name" },
      { ...status, result: "session_status" },
    ]) {
      expect(() => parse.sessionStatus(bad)).toThrow(ApiError);
    }
  });

  it("read a renewal as its new end and nothing else", () => {
    expect(parse.renewed({ result: "renewed", expires_at: 4_000_001_800 })).toBe(4_000_001_800);
    for (const bad of [
      { result: "renewed" },
      { result: "renewed", expires_at: 0 },
      { result: "renewed", expires_at: "4000001800" },
      { result: "renewed", expires_at: 4_000_001_800, username: "synthetic_name" },
      { result: "browser_status", expires_at: 4_000_001_800 },
    ]) {
      expect(() => parse.renewed(bad)).toThrow(ApiError);
    }
  });

  it("take the account summary's exact shape", async () => {
    const summary = {
      result: "account_summary",
      identity: IDENTITY,
      username: "synthetic_name",
      display_name: null,
      tag: null,
      source: "legacy",
      recovery: false,
      admin: false,
      signed_in_at: 3_999_998_200,
      expires_at: 4_000_000_000,
    };
    expect(parse.accountSummary(summary)).toEqual({
      identity: { account: Uint8Array.from(ACCOUNT), epoch: 1 },
      username: "synthetic_name",
      displayName: null,
      tag: null,
      source: "legacy",
      recovery: false,
      admin: false,
      signedInAt: 3_999_998_200,
      expiresAt: 4_000_000_000,
    });
    // the fields later steps fill, within their rules
    expect(parse.accountSummary({ ...summary, display_name: "Alice_7", tag: 7, admin: true })).toMatchObject({ displayName: "Alice_7", tag: 7, admin: true });
    expect(parse.accountSummary({ ...summary, source: "registered", recovery: true }).source).toBe("registered");
    const withoutAdmin = Object.fromEntries(Object.entries(summary).filter(([key]) => key !== "admin"));
    for (const bad of [
      withoutAdmin,
      { ...summary, role: "admin" },
      { ...summary, result: "browser_status" },
      { ...summary, source: "official" },
      { ...summary, source: null },
      { ...summary, recovery: "no" },
      { ...summary, admin: 0 },
      { ...summary, username: "Synthetic_Name" },
      { ...summary, display_name: "ab" },
      { ...summary, display_name: "a".repeat(17) },
      { ...summary, display_name: "_alice" },
      { ...summary, display_name: "alice!" },
      { ...summary, tag: 0 },
      { ...summary, tag: 2 ** 31 },
      { ...summary, tag: 1.5 },
      { ...summary, tag: "7" },
      { ...summary, signed_in_at: 0 },
      { ...summary, signed_in_at: 4_000_000_000 },
      { ...summary, expires_at: "4000000000" },
      // every registered account holds the code of its registration
      { ...summary, source: "registered", recovery: false },
    ]) {
      expect(() => parse.accountSummary(bad)).toThrow(ApiError);
    }
    const calls = serve(() => ({ body: summary }));
    expect((await api().accountSummary()).recovery).toBe(false);
    expect(calls.map((c) => c.body)).toEqual([{ operation: "account_summary" }]);
  });

  it("read discovery's two kinds and nothing else", () => {
    expect(parse.discovery({ result: "discovery", discovery: { kind: "legacy" } })).toBeNull();
    expect(parse.discovery({ result: "discovery", discovery: { kind: "central", identity: IDENTITY } })?.epoch).toBe(1);
    expect(() => parse.discovery({ result: "discovery", discovery: { kind: "legacy", identity: IDENTITY } })).toThrow(ApiError);
    expect(() => parse.discovery({ result: "discovery", discovery: { kind: "other", identity: IDENTITY } })).toThrow(ApiError);
  });

  it("hold terms to central-api's rules", () => {
    expect(parse.terms({ result: "terms", terms: null })).toBeNull();
    expect(parse.terms(doc([text("de"), text("en")]))?.texts).toHaveLength(2);
    for (const bad of [
      doc([]),
      doc([text("en"), text("de")]),
      doc([text("fr")]),
      doc([text("EN")]),
      doc([{ ...text("en"), markdown: "" }]),
      doc([text("en")], { version: "draft", digest: n(32) }),
      doc([text("en")], { version: "a b", digest: n(32) }),
      doc([text("en")], { version: "v1", digest: n(31) }),
    ]) {
      expect(() => parse.terms(bad)).toThrow(ApiError);
    }
    expect(parse.termsStatus({ result: "terms_status", current: null, accepted: true })).toEqual({ current: null, accepted: true });
    expect(() => parse.termsStatus({ result: "terms_status", current: null, accepted: "yes" })).toThrow(ApiError);
  });
});
