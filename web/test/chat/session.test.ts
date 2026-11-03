import { afterEach, describe, expect, it, vi } from "vitest";
import { ApiError, type AccountSummary } from "../../src/account/api";
import type { ChatApi, Sealed } from "../../src/chat/api";
import type { ChatCrypto, Request, Result } from "../../src/chat/crypto";
import type { GroupApi, Groups } from "../../src/chat/groups";
import { ChatSession, type Prepared } from "../../src/chat/session";
import { ACCOUNT, WRAPPED, fakeCentral } from "../support/central";

function deferred<T>() { let resolve!: (value: T) => void; const promise = new Promise<T>((done) => { resolve = done; }); return { promise, resolve }; }
const owned: ChatSession[] = [];
afterEach(() => { for (const session of owned.splice(0)) session.close(); });
const envelope: Sealed = { hdr: "a", n: "b", ct: "c", epk: "d", sig: "e", wraps: [] };

async function fixture(groups?: GroupApi, prepared: Prepared | null = null) {
  const account = fakeCentral();
  account.moved("test", { displayName: "Synthetic", tag: 7 }); account.session = { name: "test", expiresAt: 4_000_000_000 };
  const summary: AccountSummary = await account.accountSummary();
  const identity = { account: ACCOUNT, epoch: 1 };
  account.loginStart = vi.fn<typeof account.loginStart>(async () => ({ attempt: new Uint8Array(32), message: new Uint8Array(3) }));
  account.loginFinish = vi.fn<typeof account.loginFinish>(async () => ({ identity, username: "test", wrappedMaster: WRAPPED, expiresAt: 4_000_000_000 }));
  const requests: Request[] = [];
  const ask = vi.fn<(request: Request) => Promise<Result>>(async (request: Request): Promise<Result> => {
    requests.push(request);
    switch (request.op) {
      case "start": case "finish": return { op: request.op, message: new Uint8Array(3) };
      case "unlock": return { op: request.op, device: "a".repeat(16), registration: { ik: "b", dk: "c", dsig: "d", label: "Account" } };
      case "restore": return { op: request.op };
      case "observe": return { op: request.op, reviews: [], trust: new Uint8Array([11,12]) };
      case "confirm": return { op: request.op, trust: new Uint8Array([11,12]) };
      case "seal": return { op: request.op, sealed: envelope };
      case "open": return { op: request.op, messages: [] };
    }
  });
  const crypto: ChatCrypto = { ask: ask as ChatCrypto["ask"], close: vi.fn<() => void>() };
  const api: ChatApi = { register: vi.fn<ChatApi["register"]>(async () => "a".repeat(16)), keys: vi.fn<ChatApi["keys"]>(async (tags) => tags.map((tag) => ({ tag: tag.toString(16).padStart(4, "0"), handle: `Synthetic#${tag.toString(16).padStart(4, "0")}`, devices: [{ id: "a".repeat(16), ik: "b", dk: "c", dsig: "d", label: "Account", added_at: 1 }] }))), send: vi.fn<ChatApi["send"]>(async () => ({ id: "test", at: 100 })), inbox: vi.fn<ChatApi["inbox"]>(async () => ({ messages: [], more: false })) };
  const store = { read: vi.fn<() => Uint8Array | null>(() => null), write: vi.fn<(bytes: Uint8Array) => void>() };
  const session = new ChatSession(api, account, summary, crypto, store, groups, prepared); owned.push(session);
  return { session, api, account, crypto, requests, ask, store };
}

const message = (seq: number, at: number) => ({ id: `m${seq}`, conv: "dm:0007:0009", sender: 9, text: `line ${seq}`, emote: false, at });
const PREPARED: Prepared = { device: "a".repeat(16), registration: { ik: "b", dk: "c", dsig: "d", label: "Account" } };

describe("a chat a sign-in unlocked", () => {
  it("starts without a password: restores the trust, registers, reads its keys, then the inbox", async () => {
    const f = await fixture(undefined, PREPARED);
    f.store.read.mockReturnValue(new Uint8Array([5, 6]));
    await f.session.start(() => {});
    expect(f.account.loginStart).not.toHaveBeenCalled();
    expect(f.requests.map((r) => r.op).slice(0, 2)).toEqual(["restore", "observe"]);
    expect(f.requests[0]).toEqual({ op: "restore", trust: new Uint8Array([5, 6]) });
    expect(f.api.register).toHaveBeenCalledExactlyOnceWith(PREPARED.registration);
    await vi.waitFor(() => expect(f.api.inbox).toHaveBeenCalledWith("a".repeat(16), 0));
    expect(f.session.getSnapshot().started).toBe(true);
    // coalesced and done once
    await f.session.start(() => {});
    expect(f.api.register).toHaveBeenCalledTimes(1);
  });

  it("sends and reads nothing before it started, and a failed start goes on where it stopped", async () => {
    const f = await fixture(undefined, PREPARED);
    expect(await f.session.send("dm:0007:0009", "too early", [7, 9])).toBe(false);
    expect(await f.session.poll()).toBe(false);
    f.api.register = vi.fn<ChatApi["register"]>().mockRejectedValueOnce(new ApiError("network")).mockResolvedValue("a".repeat(16));
    await expect(f.session.start(() => {})).rejects.toBeInstanceOf(ApiError);
    expect(f.session.getSnapshot()).toMatchObject({ error: true, started: false });
    await f.session.start(() => {});
    expect(f.requests.filter((r) => r.op === "restore")).toHaveLength(0);
    expect(f.session.getSnapshot()).toMatchObject({ error: false, started: true });
    expect(f.api.inbox).not.toHaveBeenCalledWith("", 0);
  });

  it("fails closed on damaged trust: nothing registered, nothing read", async () => {
    const f = await fixture(undefined, PREPARED);
    f.store.read.mockImplementation(() => { throw new Error("trust"); });
    await expect(f.session.start(() => {})).rejects.toThrow("trust");
    expect(f.api.register).not.toHaveBeenCalled();
    expect(f.api.inbox).not.toHaveBeenCalled();
  });

  it("locks when Central refuses the session at the start", async () => {
    const f = await fixture(undefined, PREPARED);
    f.api.register = vi.fn<ChatApi["register"]>().mockRejectedValue(new ApiError("rejected"));
    const locked = vi.fn<() => void>();
    await expect(f.session.start(locked)).rejects.toBeInstanceOf(ApiError);
    expect(locked).toHaveBeenCalledTimes(1);
    expect(f.crypto.close).toHaveBeenCalledTimes(1);
  });

  it("stays unlocked in a tab hidden for long, and reads at once when it is shown again", async () => {
    const f = await fixture(undefined, PREPARED);
    const locked = vi.fn<() => void>();
    await f.session.start(locked);
    await vi.waitFor(() => expect(f.api.inbox).toHaveBeenCalledTimes(1));
    const now = Date.now();
    const clock = vi.spyOn(Date, "now");
    const visibility = vi.spyOn(document, "visibilityState", "get");
    visibility.mockReturnValue("hidden"); document.dispatchEvent(new Event("visibilitychange"));
    clock.mockReturnValue(now + 60 * 60_000);
    visibility.mockReturnValue("visible"); document.dispatchEvent(new Event("visibilitychange"));
    await vi.waitFor(() => expect(f.api.inbox).toHaveBeenCalledTimes(2));
    expect(locked).not.toHaveBeenCalled();
    expect(f.crypto.close).not.toHaveBeenCalled();
    clock.mockRestore(); visibility.mockRestore();
  });

  it("counts what the first complete read brought as seen, and what a conversation shows once it is open", async () => {
    const f = await fixture(undefined, PREPARED);
    const peer = (await f.api.keys([9]))[0]?.devices[0];
    if (peer === undefined) throw new Error("fixture peer keys");
    const delivery = (seq: number, at: number) => ({ seq, sender: { tag: 9, display_name: "Peer" }, device: peer, sealed: envelope, at });
    const original = f.ask.getMockImplementation(); if (original === undefined) throw new Error("fixture");
    f.ask.mockImplementation(async (request) => request.op === "open" ? { op: "open", messages: request.messages.map((m) => ({ seq: m.seq, message: message(m.seq, m.at) })) } : original(request));
    f.api.inbox = vi.fn<ChatApi["inbox"]>().mockResolvedValueOnce({ messages: [delivery(1, 100)], more: false }).mockResolvedValue({ messages: [], more: false });
    await f.session.start(() => {});
    await vi.waitFor(() => expect(f.session.getSnapshot().seen).not.toBeNull());
    expect(f.session.getSnapshot().seen?.get("dm:0007:0009")).toBe(100);
    f.api.inbox = vi.fn<ChatApi["inbox"]>().mockResolvedValueOnce({ messages: [delivery(2, 200)], more: false }).mockResolvedValue({ messages: [], more: false });
    await f.session.poll();
    expect(f.session.getSnapshot().seen?.get("dm:0007:0009")).toBe(100);
    f.session.markSeen("dm:0007:0009");
    expect(f.session.getSnapshot().seen?.get("dm:0007:0009")).toBe(200);
  });
});

describe("one unlocked chat session", () => {
  it("never performs a follow-up request after closing during password verification", async () => {
    const f = await fixture(); const pending = deferred<{ attempt: Uint8Array; message: Uint8Array }>();
    f.account.loginStart = vi.fn<typeof f.account.loginStart>(() => pending.promise);
    const unlock = f.session.unlock("synthetic", () => {});
    await vi.waitFor(() => expect(f.account.loginStart).toHaveBeenCalledTimes(1));
    f.session.close(); pending.resolve({ attempt: new Uint8Array(32), message: new Uint8Array(3) });
    await expect(unlock).rejects.toThrow("closed");
    expect(f.account.loginFinish).not.toHaveBeenCalled(); expect(f.api.register).not.toHaveBeenCalled();
    expect(f.crypto.close).toHaveBeenCalledTimes(1);
  });

  it("coalesces overlapping inbox reads and discards replies after closing", async () => {
    const f = await fixture(); await f.session.unlock("synthetic", () => {});
    const pending = deferred<{ messages: []; more: boolean }>();
    f.api.inbox = vi.fn<ChatApi["inbox"]>(() => pending.promise);
    const first = f.session.poll(); const second = f.session.poll();
    expect(first).toBe(second); expect(f.api.inbox).toHaveBeenCalledTimes(1);
    f.session.close(); pending.resolve({ messages: [], more: false }); await first;
    expect(f.requests.filter((r) => r.op === "open")).toEqual([]);
  });

  it("retains identical ciphertext after an uncertain send and suppresses double clicks", async () => {
    const f = await fixture(); await f.session.unlock("synthetic", () => {});
    const pending = deferred<{ id: string; at: number }>();
    f.api.send = vi.fn<ChatApi["send"]>(() => pending.promise);
    const first = f.session.send("dm:0007:0009", "hello", [7,9]);
    expect(await f.session.send("dm:0007:0009", "hello", [7,9])).toBe(false);
    await vi.waitFor(() => expect(f.api.send).toHaveBeenCalledTimes(1));
    pending.resolve({ id: "test", at: 100 }); expect(await first).toBe(true);
    f.api.send = vi.fn<ChatApi["send"]>().mockRejectedValueOnce(new ApiError("network")).mockResolvedValueOnce({ id: "test", at: 100 });
    expect(await f.session.send("dm:0007:0009", "retry", [7,9])).toBe(false);
    expect(f.session.getSnapshot().pending?.failed).toBe(true);
    const seals = f.requests.filter((r) => r.op === "seal").length;
    expect(await f.session.retry()).toBe(true);
    expect(f.requests.filter((r) => r.op === "seal")).toHaveLength(seals);
    expect(f.api.send).toHaveBeenNthCalledWith(1, "dm:0007:0009", envelope);
    expect(f.api.send).toHaveBeenNthCalledWith(2, "dm:0007:0009", envelope);
  });

  it("locks and closes the crypto worker when the account session is rejected", async () => {
    const f = await fixture(); const locked = vi.fn<() => void>(); await f.session.unlock("synthetic", locked);
    f.api.inbox = vi.fn<ChatApi["inbox"]>().mockRejectedValue(new ApiError("rejected"));
    await f.session.poll();
    expect(locked).toHaveBeenCalledTimes(1); expect(f.crypto.close).toHaveBeenCalledTimes(1);
    expect(await f.session.poll()).toBe(false); expect(f.api.inbox).toHaveBeenCalledTimes(1);
  });

  it("fails closed on another account's login result before registering keys", async () => {
    const f = await fixture();
    f.account.loginFinish = vi.fn<typeof f.account.loginFinish>(async () => ({ identity: { account: new Uint8Array(16), epoch: 1 }, username: "test", wrappedMaster: WRAPPED, expiresAt: 4_000_000_000 }));
    await expect(f.session.unlock("synthetic", () => {})).rejects.toThrow("identity");
    expect(f.api.register).not.toHaveBeenCalled();
  });
  it("never seals for a recipient removed from the current directory", async () => {
    const f = await fixture(); await f.session.unlock("synthetic", () => {});
    await f.session.keys([7,9]);
    const own = (await f.api.keys([7]))[0];
    if (own === undefined) throw new Error("fixture own keys");
    f.api.keys = vi.fn<ChatApi["keys"]>(async () => [own]);
    expect(await f.session.send("dm:0007:0009", "must stay local", [7,9])).toBe(false);
    expect(f.requests.some((request) => request.op === "seal")).toBe(false);
    expect(f.api.send).not.toHaveBeenCalled();
  });

  it("does not retry an uncertain ciphertext while the recipient is unavailable", async () => {
    const f = await fixture(); await f.session.unlock("synthetic", () => {});
    f.api.send = vi.fn<ChatApi["send"]>().mockRejectedValue(new ApiError("network"));
    expect(await f.session.send("dm:0007:0009", "retry only with current recipients", [7,9])).toBe(false);
    f.api.keys = vi.fn<ChatApi["keys"]>(async () => []);
    expect(await f.session.retry()).toBe(false);
    expect(f.api.send).toHaveBeenCalledTimes(1);
    expect(f.session.getSnapshot().pending?.sealed).toBe(envelope);
  });

  it("a late inbox page cannot overwrite the replay cursor after key confirmation", async () => {
    const f = await fixture(); await f.session.unlock("synthetic", () => {});
    const peer = (await f.api.keys([9]))[0];
    const device = peer?.devices[0];
    if (peer === undefined || device === undefined) throw new Error("fixture peer keys");
    const open = deferred<Extract<Result, { op: "open" }>>();
    const original = f.ask.getMockImplementation(); if (original === undefined) throw new Error("fixture");
    f.ask.mockImplementation((request) => request.op === "open" ? open.promise : original(request));
    f.api.inbox = vi.fn<ChatApi["inbox"]>(async () => ({ messages: [{ seq: 89, sender: { tag: 9, display_name: "Peer" }, device, sealed: envelope, at: 100 }], more: false }));
    const reading = f.session.poll();
    await vi.waitFor(() => expect(f.ask).toHaveBeenCalledWith(expect.objectContaining({ op: "open" })));
    await f.session.confirm(peer);
    open.resolve({ op: "open", messages: [{ seq: 89, message: null }] });
    await reading;
    expect(f.session.getSnapshot().refused).toBe(0);
    f.ask.mockImplementation(original);
    await f.session.poll();
    expect(f.api.inbox).toHaveBeenLastCalledWith("a".repeat(16), 0);
  });

});

function groupFixture(tags: number[]) {
  const id = "019a3456-7890-7123-8123-123456789abc";
  const data: Groups = { groups: [{ id, name: "Synthetic", invited: [], members: tags.map((tag) => ({ owner: tag === 7, person: { tag, displayName: `Person${tag}` } })) }], invites: [] };
  const groups: GroupApi = { list: vi.fn<GroupApi["list"]>(async () => data), create: vi.fn<GroupApi["create"]>(async () => id), change: vi.fn<GroupApi["change"]>(async () => {}) };
  return { data, groups, conv: `grp:${id}` };
}
it("encrypts to fresh group membership even when the page still shows an older list", async () => {
  const g = groupFixture([7,9,11]); const f = await fixture(g.groups);
  await f.session.unlock("synthetic", () => {});
  expect(await f.session.send(g.conv, "joined meanwhile", [7,9])).toBe(true);
  expect(f.api.keys).toHaveBeenLastCalledWith([7,9,11]);
});
it("does not retry an uncertain group envelope after a recipient leaves", async () => {
  const g = groupFixture([7,9,11]); const f = await fixture(g.groups);
  await f.session.unlock("synthetic", () => {});
  f.api.send = vi.fn<ChatApi["send"]>().mockRejectedValue(new Error("network"));
  expect(await f.session.send(g.conv, "uncertain", [7,9,11])).toBe(false);
  g.data.groups[0]?.members.pop();
  expect(await f.session.retry()).toBe(false);
  expect(f.api.send).toHaveBeenCalledTimes(1);
  expect(f.session.getSnapshot().pending?.sealed).toBe(envelope);
});
it("a definitive membership conflict releases the envelope but reports failure to preserve the draft", async () => {
  const f = await fixture(); await f.session.unlock("synthetic", () => {});
  f.api.send = vi.fn<ChatApi["send"]>().mockRejectedValue(new ApiError("conflict"));
  expect(await f.session.send("dm:0007:0009", "keep this draft", [7,9])).toBe(false);
  expect(f.session.getSnapshot().pending).toBeNull();
  expect(f.session.getSnapshot().error).toBe(true);
});
