// The administration on the wire: the requests, and the answers in their
// exact shape (the official gateway alone has no owner).

import { describe, expect, it } from "vitest";

import { ApiError } from "../../src/account/api";
import { adminApi, parseAdmin } from "../../src/admin/admin-api";
import { readTag } from "../../src/admin/UserPage";
import { serve } from "../support/network";

const ID = [0x01, 0x92, 0xd0, 0xc5, 0x7b, 0x3e, 0x7a, 0x1f, 0x8c, 0x2d, 0x3e, 0x4f, 0x5a, 0x6b, 0x7c, 0x8d];
const OWNER = [0x01, 0x92, 0xd0, 0xc5, 0x7b, 0x3e, 0x7a, 0x1f, 0x8c, 0x2d, 0x3e, 0x4f, 0x5a, 0x6b, 0x7c, 0x8e];
const admin = (answer: unknown) => ({ result: "admin", answer });
const gateway = (extra: Record<string, unknown> = {}) => ({
  id: ID,
  origin: "https://gateway.invalid",
  metadata: { name: "Synthetic gateway", version: "1.0", website: "https://gateway.invalid/" },
  official: false,
  owner: OWNER,
  owner_name: "Synthetic_Name",
  owner_tag: 0x3fa2,
  state: "pending",
  online: false,
  last_seen: null,
  created_at: 1_800_000_000,
  live_tokens: 1,
  credential_pair: false,
  ...extra,
});

const legacyRow = (extra: Record<string, unknown> = {}) => ({
  legacy_id: "0192d0c5-7b3e-7a1f-8c2d-3e4f5a6b7c01",
  username: "synthetic_moving",
  display_name: null,
  tag: 7,
  progress: "unstarted",
  imported_at: 1_800_000_000,
  failures: 0,
  locked_until: null,
  must_change_password: false,
  banned: false,
  snapshot_bytes: 2,
  ...extra,
});
const legacyStore = (accounts: unknown[], deadline: unknown = null) => admin({ answer: "legacy_accounts", store: { deadline, accounts } });

describe("the answers", () => {
  it("take a page of gateways, the official one without an owner", () => {
    const page = parseAdmin.gateways(admin({ answer: "gateways", gateways: [gateway(), gateway({ official: true, owner: null, owner_name: null, owner_tag: null })], more: false }));
    expect(page.items.map((g) => g.official)).toEqual([false, true]);
    for (const bad of [gateway({ official: true }), gateway({ owner: null }), gateway({ state: "approved", extra: 1 }), gateway({ live_tokens: -1 })]) {
      expect(() => parseAdmin.gateways(admin({ answer: "gateways", gateways: [bad], more: false }))).toThrow(ApiError);
    }
  });

  it("take what is left of the legacy store, names and counters only", () => {
    const read = parseAdmin.legacyAccounts(legacyStore([legacyRow(), legacyRow({ legacy_id: "2", progress: "restored", locked_until: 1_800_000_600 })], 1_800_864_000));
    expect(read.deadline).toBe(1_800_864_000);
    expect(read.accounts.map((a) => [a.progress, a.lockedUntil])).toEqual([["unstarted", null], ["restored", 1_800_000_600]]);
    for (const bad of [
      legacyRow({ password_hash: [1] }),
      legacyRow({ progress: "closed" }),
      legacyRow({ username: "Synthetic" }),
      legacyRow({ legacy_id: "../x" }),
      legacyRow({ failures: -1 }),
      legacyRow({ display_name: "x".repeat(65) }),
    ]) {
      expect(() => parseAdmin.legacyAccounts(legacyStore([bad]))).toThrow(ApiError);
    }
  });

  it("take a reset code only as Central prints one", () => {
    const code = `RESET-${Array.from({ length: 8 }, () => "AB12").join("-")}`;
    expect(parseAdmin.resetCode(admin({ answer: "reset_code", code, expires_at: 1_800_086_400 })).code).toBe(code);
    expect(() => parseAdmin.resetCode(admin({ answer: "reset_code", code: "Y06Z-NKPA", expires_at: 1 }))).toThrow(ApiError);
  });
});

describe("a tag as an administrator types it", () => {
  it("is one to eight hex digits, with or without #", () => {
    expect(readTag("#3fa2")).toBe(0x3fa2);
    expect(readTag(" 0007 ")).toBe(7);
    expect(readTag("7fffffff")).toBe(0x7fffffff);
    for (const bad of ["", "#", "#0", "80000000", "123456789", "zz", "#-1"]) expect(readTag(bad)).toBeNull();
  });
});

describe("the requests", () => {
  it("post the administrator's changes to /api/v1", async () => {
    const calls = serve(() => ({ body: { result: "done" } }));
    const api = adminApi();
    await api.setTag(Uint8Array.from(ID), 0x1a2b);
    await api.setGatewayApproval(Uint8Array.from(ID), true);
    await api.revokeGatewayToken(Uint8Array.from(OWNER));
    expect(calls.map((c) => c.body)).toEqual([
      { operation: "admin", request: { action: "set_tag", account: ID, tag: 0x1a2b } },
      { operation: "admin", request: { action: "set_gateway_approval", gateway: ID, enabled: true } },
      { operation: "admin", request: { action: "revoke_gateway_token", token: OWNER } },
    ]);
  });
});
