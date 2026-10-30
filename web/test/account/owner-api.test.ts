// An account's own area on the wire: the requests it posts, the answers it
// takes only in their exact shape, and a token's text form, which must equal
// central-api's `token_text` byte for byte.

import { describe, expect, it } from "vitest";

import { ApiError } from "../../src/account/api";
import { idOf, idText, ownerApi, parseOwner, tokenText, validTokenName } from "../../src/account/owner-api";
import { serve } from "../support/network";

const ID = [0x01, 0x92, 0xd0, 0xc5, 0x7b, 0x3e, 0x7a, 0x1f, 0x8c, 0x2d, 0x3e, 0x4f, 0x5a, 0x6b, 0x7c, 0x8d];
const ID2 = [0x01, 0x92, 0xd0, 0xc5, 0x7b, 0x3e, 0x7a, 0x1f, 0x8c, 0x2d, 0x3e, 0x4f, 0x5a, 0x6b, 0x7c, 0x8e];
const ORIGIN = "https://gateway.invalid";
const owner = (answer: unknown) => ({ result: "owner", answer });
const token = (extra: Record<string, unknown> = {}) => ({
  id: ID2,
  name: "Main",
  scopes: { sso: true, listing: true },
  created_at: 1_800_000_000,
  expires_at: 1_807_776_000,
  revoked_at: null,
  last_used_at: null,
  ...extra,
});
const gateway = (extra: Record<string, unknown> = {}) => ({
  id: ID,
  origin: ORIGIN,
  metadata: { name: "Synthetic gateway", version: "0", website: `${ORIGIN}/` },
  state: "pending",
  online: false,
  last_seen: null,
  created_at: 1_800_000_000,
  tokens: [token()],
  ...extra,
});

async function failure(p: Promise<unknown>): Promise<string> {
  try {
    await p;
  } catch (e) {
    return e instanceof ApiError ? e.kind : "other";
  }
  return "none";
}

describe("a token's text form", () => {
  it("is bgt_ and the 64 hex digits, as central-api writes it", () => {
    // crates/central-api/src/owner.rs `the_text_form_matches_the_websites_vector`
    expect(tokenText(new Uint8Array(32).fill(0xab))).toBe(`bgt_${"ab".repeat(32)}`);
    expect(() => tokenText(new Uint8Array(31))).toThrow(ApiError);
  });

  it("has a short plain name", () => {
    for (const good of ["Main", "Mein Server – Haupt", "x".repeat(64)]) expect(validTokenName(good)).toBe(true);
    for (const bad of ["", " padded", "padded ", "line\nbreak", "x".repeat(65), "ä".repeat(33)]) expect(validTokenName(bad)).toBe(false);
  });

  it("names an id by its hex, and reads back only a UUIDv7", () => {
    const id = Uint8Array.from(ID);
    expect(idOf(idText(id))).toEqual(id);
    expect(idOf("00".repeat(16))).toBeNull();
    expect(idOf("XYZ")).toBeNull();
  });
});

describe("the answers", () => {
  it("take a gateway with its tokens, and nothing more", () => {
    const [g] = parseOwner.gateways(owner({ answer: "gateways", gateways: [gateway()] }));
    expect(g?.origin).toBe(ORIGIN);
    expect(g?.tokens[0]?.scopes).toEqual({ sso: true, listing: true });
    for (const bad of [
      gateway({ note: "x" }),
      gateway({ state: "maybe" }),
      gateway({ metadata: { name: "x", version: "0", website: "https://phishing.invalid/" } }),
      gateway({ tokens: [token({ scopes: { sso: false, listing: false } })] }),
      gateway({ tokens: [token({ expires_at: 1_700_000_000 })] }),
      gateway({ tokens: [token({ digest: [1] })] }),
      gateway({ id: Array.from({ length: 16 }, () => 0) }),
    ]) {
      expect(() => parseOwner.gateways(owner({ answer: "gateways", gateways: [bad] }))).toThrow(ApiError);
    }
  });

  it("show a new token once, in its text form", () => {
    const made = parseOwner.tokenCreated(owner({ answer: "token_created", id: ID2, token: Array.from({ length: 32 }, () => 0xab), expires_at: 1_807_776_000 }));
    expect(made.text).toBe(`bgt_${"ab".repeat(32)}`);
    expect(() => parseOwner.tokenCreated(owner({ answer: "token_created", id: ID2, token: [1, 2], expires_at: 1 }))).toThrow(ApiError);
  });

  it("take the sessions, at most one of each kind and four in all", () => {
    const [s] = parseOwner.sessions(
      owner({ answer: "sessions", sessions: [{ kind: "native", current: false, signed_in_at: 1_800_000_000, expires_at: 1_800_001_800, gateways: [ORIGIN] }] }),
    );
    expect(s?.gateways).toEqual([ORIGIN]);
    expect(() => parseOwner.sessions(owner({ answer: "sessions", sessions: [{ kind: "device", current: false, signed_in_at: 1, expires_at: 2, gateways: [] }] }))).toThrow(ApiError);
  });
});

describe("the requests", () => {
  it("post the owner's actions to /api/v1", async () => {
    const calls = serve((call) => {
      const action = (call.body as { request: { action: string } }).request.action;
      if (action === "create_token") return { body: owner({ answer: "token_created", id: ID2, token: Array.from({ length: 32 }, () => 1), expires_at: 1_807_776_000 }) };
      if (action === "add_gateway") return { body: owner({ answer: "gateway_added", gateway: ID }) };
      return { body: { result: "done" } };
    });
    const api = ownerApi();
    await api.addGateway(ORIGIN, "Synthetic gateway");
    await api.createToken(Uint8Array.from(ID), "Main", { sso: true, listing: false }, 90);
    await api.revokeToken(Uint8Array.from(ID2));
    await api.endClientSession();
    expect(calls.map((c) => c.body)).toEqual([
      { operation: "owner", request: { action: "add_gateway", origin: ORIGIN, name: "Synthetic gateway" } },
      { operation: "owner", request: { action: "create_token", gateway: ID, name: "Main", scopes: { sso: true, listing: false }, days: 90 } },
      { operation: "owner", request: { action: "revoke_token", token: ID2 } },
      { operation: "owner", request: { action: "end_client_session" } },
    ]);
    expect(calls.every((c) => c.url === "/api/v1" && c.init?.credentials === "same-origin")).toBe(true);
  });

  it("names Central's refusals", async () => {
    serve(() => ({ status: 409, text: "conflict\n" }));
    expect(await failure(ownerApi().addGateway(ORIGIN, "x"))).toBe("conflict");
    serve(() => ({ status: 401, text: "no\n" }));
    expect(await failure(ownerApi().createToken(Uint8Array.from(ID), "x", { sso: true, listing: true }, 30))).toBe("rejected");
  });
});
