import { describe, expect, it, vi } from "vitest";
import { contacts, socialApi, tagOf } from "../../src/social/api";

const person = { tag: 0x3fa2, display_name: "Public_Name" };
const wrap = (response: unknown) => ({ result: "social", response });
const list = (items: unknown[]) => wrap({ kind: "contacts", contacts: items });

describe("the Central social boundary", () => {
  it("uses the same-origin cookie surface with no gateway token", async () => {
    const doFetch = vi.fn<typeof fetch>(async () => new Response(JSON.stringify(list([{ person, relation: "incoming" }]))));
    expect(await socialApi({ fetch: doFetch }).contacts()).toEqual([{ person: { tag: 0x3fa2, displayName: "Public_Name" }, relation: "incoming" }]);
    const call = doFetch.mock.calls[0] as unknown as [string, RequestInit];
    expect(call[0]).toBe("/api/v1");
    expect(call[1].credentials).toBe("same-origin");
    expect(call[1].headers).toEqual({ "Content-Type": "application/json", "X-Baylee-Central": "1" });
    expect(JSON.parse(call[1].body as string)).toEqual({ operation: "social", request: { action: "contacts" } });
  });

  it("refuses duplicated, excessive and private-profile fields", () => {
    const entry = { person, relation: "friend" };
    for (const bad of [list([entry, entry]), list([{ ...entry, private_name: "secret" }]),
      list([{ person: { ...person, username: "secret" }, relation: "friend" }]),
      list([{ person, relation: "blocks_me" }]),
      list(Array.from({ length: 301 }, (_, i) => ({ person: { ...person, tag: i + 1 }, relation: "friend" }))),
      list([{ person: { ...person, tag: 2 ** 32 }, relation: "friend" }])]) {
      expect(() => contacts(bad)).toThrow("malformed");
    }
  });

  it("does not accept a lookup for a different tag", async () => {
    const api = socialApi({ fetch: vi.fn<typeof fetch>(async () => new Response(JSON.stringify(wrap({ kind: "person", person })))) });
    await expect(api.lookup(0x4444)).rejects.toThrow("malformed");
  });

  it("never turns a server or network failure into an empty contact list", async () => {
    await expect(socialApi({ fetch: vi.fn<typeof fetch>(async () => new Response("", { status: 503 })) }).contacts()).rejects.toThrow("unavailable");
    await expect(socialApi({ fetch: vi.fn<typeof fetch>(async () => { throw new Error("offline"); }) }).contacts()).rejects.toThrow("network");
  });

  it("parses exact public tags and handles without fuzzy name matching", () => {
    expect(tagOf(" Mira#3fa2 ")).toBe(0x3fa2);
    expect(tagOf("0007")).toBe(7);
    for (const value of ["Mira", "03fa2", "3FA2", "0000", "ffffffff", "3fa2junk", "https://invalid/3fa2"]) expect(tagOf(value)).toBeNull();
  });
});
