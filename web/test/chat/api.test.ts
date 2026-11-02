import { describe, expect, it, vi } from "vitest";
import { chatApi, type Sealed } from "../../src/chat/api";
import { groupApi, groupSnapshot } from "../../src/chat/groups";

const id = "019a3d70-0000-7000-8000-000000000001";
const conv = "dm:0007:0009";
const key = { id: "a".repeat(16), ik: "A".repeat(43), dk: "B".repeat(43), dsig: "C".repeat(86), label: "Account", added_at: 1 };
const directory = { tag: "0007", handle: "Alice#0007", devices: [key] };
const envelope: Sealed = { hdr: btoa(JSON.stringify({ v: 1, id, conv, from: "0007", dev: key.id, ts: 1, kind: "text" })).replace(/=+$/, ""), n: "a".repeat(32), ct: "b".repeat(64), epk: "c".repeat(43), sig: "d".repeat(86), wraps: [{ d: key.id, k: "e".repeat(64) }] };
const delivery = { seq: 1, sender: { tag: 7, display_name: "Alice" }, device: key, sealed: envelope, at: 1 };
const fetchReply = (kind: string, response: unknown) => vi.fn<typeof fetch>(async () => new Response(JSON.stringify({ result: "social", response: { kind, response } })));
const chat = (response: unknown) => chatApi({ fetch: fetchReply("chat", response) });

describe("encrypted chat response boundaries", () => {
  it("accepts only an acknowledgement for the exact signed message", async () => {
    expect(await chat({ kind: "sent", id, at: 100 }).send(conv, envelope)).toEqual({ id, at: 100 });
    await expect(chat({ kind: "sent", id: id.replace(/1$/, "2"), at: 100 }).send(conv, envelope)).rejects.toThrow("malformed");
    await expect(chat({ kind: "sent", id, at: 100 }).send("dm:0007:0010", envelope)).rejects.toThrow("malformed");
  });
  it("rejects directory injection, duplicate devices and private fields", async () => {
    expect(await chat({ kind: "keys", keys: [directory] }).keys([7])).toEqual([directory]);
    for (const keys of [[directory, directory], [{ ...directory, tag: "0008", handle: "Alice#0008" }], [{ ...directory, devices: [key, key] }], [{ ...directory, username: "private" }]]) {
      await expect(chat({ kind: "keys", keys }).keys([7,9])).rejects.toThrow("malformed");
    }
  });
  it("refuses regressing cursors, unbounded pages and extra recipient wraps", async () => {
    expect((await chat({ kind: "inbox", messages: [delivery], more: false }).inbox(key.id, 0)).messages).toHaveLength(1);
    for (const response of [
      { kind: "inbox", messages: [delivery], more: true },
      { kind: "inbox", messages: [delivery, delivery], more: false },
      { kind: "inbox", messages: [{ ...delivery, sealed: { ...envelope, wraps: [...envelope.wraps, ...envelope.wraps] } }], more: false },
      { kind: "inbox", messages: Array.from({ length: 51 }, (_, i) => ({ ...delivery, seq: i + 1 })), more: false },
    ]) await expect(chat(response).inbox(key.id, 0)).rejects.toThrow("malformed");
    await expect(chat({ kind: "inbox", messages: [delivery], more: false }).inbox(key.id, 1)).rejects.toThrow("malformed");
  });
});

const person = { tag: 7, display_name: "Alice" };
const group = { id, name: "Friends", members: [{ person, owner: true }], invited: [] };
describe("private group boundaries", () => {
  it("keeps group operations on Central's same-origin cookie transport", async () => {
    const fetch = fetchReply("group", { kind: "created", id });
    expect(await groupApi({ fetch }).create("Friends")).toBe(id);
    const [, options] = fetch.mock.calls[0] as unknown as [string, RequestInit];
    expect(options.credentials).toBe("same-origin");
    expect(JSON.parse(options.body as string)).toEqual({ operation: "social", request: { action: "group", request: { command: "create", name: "Friends" } } });
  });
  it("rejects ambiguous membership, over-capacity snapshots and hidden names", () => {
    expect(groupSnapshot({ groups: [group], invites: [] }).groups[0]?.members[0]?.person.tag).toBe(7);
    for (const groups of [[group, group], [{ ...group, members: [{ person, owner: false }] }], [{ ...group, invited: [person] }], [{ ...group, name: "bad\u202ename" }], [{ ...group, private_id: "secret" }]]) {
      expect(() => groupSnapshot({ groups, invites: [] })).toThrow("malformed");
    }
    expect(() => groupSnapshot({ groups: [group], invites: [{ group: id, name: "Friends", inviter: person, at: 1 }] })).toThrow("malformed");
    expect(() => groupSnapshot({ groups: [], invites: Array.from({ length: 21 }, () => ({ group: id, name: "Friends", inviter: person, at: 1 })) })).toThrow("malformed");
  });
});
