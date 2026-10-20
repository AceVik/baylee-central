import { describe, expect, it, vi } from "vitest";

import { DirectoryError, directoryClient, isUuidV7, parsePage, uuidBytes, uuidText, validMetadata, validOrigin } from "./directory";
import { idBytes, listing, page } from "./fixture";

function answer(status: number, body: unknown, json = true): Response {
  return new Response(json ? JSON.stringify(body) : String(body), { status, headers: { "Content-Type": json ? "application/json" : "text/plain" } });
}

describe("ids", () => {
  it("round-trips a UUIDv7 between Central's bytes and text", () => {
    const bytes = idBytes(7);
    const text = uuidText(bytes);
    expect(text).toBe("019a2b3c-4d5e-7000-8000-000000000007");
    expect(uuidBytes(text)).toEqual(bytes);
    expect(isUuidV7(bytes)).toBe(true);
  });

  it("refuses another version or variant, as central-client does", () => {
    const v4 = idBytes(1);
    v4[6] = 0x40;
    expect(isUuidV7(v4)).toBe(false);
    const ncs = idBytes(1);
    ncs[8] = 0x00;
    expect(isUuidV7(ncs)).toBe(false);
    expect(uuidBytes("019A2B3C-4D5E-7000-8000-000000000007")).toBeNull();
  });
});

describe("origins and metadata", () => {
  it("takes only canonical HTTPS origins", () => {
    expect(validOrigin("https://gateway.invalid")).toBe(true);
    for (const bad of [
      "http://gateway.invalid",
      "https://u@gateway.invalid",
      "https://gateway.invalid/",
      "https://GATEWAY.invalid",
      "https://gateway.invalid/path",
      "https://gateway.invalid:443",
      "https://gateway.invalid?x",
      "javascript:alert(1)",
      "",
    ]) {
      expect({ bad, valid: validOrigin(bad) }).toEqual({ bad, valid: false });
    }
  });

  it("takes numeric loopback only on a page on loopback", () => {
    expect(validOrigin("http://127.0.0.1:9876")).toBe(false);
    expect(validOrigin("http://127.0.0.1:9876", true)).toBe(true);
    expect(validOrigin("http://[::1]:9876", true)).toBe(true);
    expect(validOrigin("http://localhost:9876", true)).toBe(false);
    expect(validOrigin("http://127.0.0.1", true)).toBe(false);
    expect(validOrigin("http://127.0.0.1:0", true)).toBe(false);
    expect(validOrigin("http://10.0.0.1:9876", true)).toBe(false);
    expect(validOrigin("https://gateway.invalid", true)).toBe(true);
  });

  it("keeps a website on its gateway's origin, without query or credentials", () => {
    const meta = { name: "Synthetic", version: "1.2.3-test", website: "https://gateway.invalid/status" };
    expect(validMetadata(meta, "https://gateway.invalid")).toBe(true);
    for (const website of [
      "https://foreign.invalid/",
      "https://u@gateway.invalid/",
      "https://gateway.invalid/?next=elsewhere",
      "https://gateway.invalid/#x",
      "https://gateway.invalid",
      "javascript:alert(1)",
    ]) {
      expect({ website, valid: validMetadata({ ...meta, website }, "https://gateway.invalid") }).toEqual({ website, valid: false });
    }
  });

  it("bounds the name and the version", () => {
    const ok = { name: "Synthetic", version: "1.2.3", website: "https://g.invalid/" };
    expect(validMetadata({ ...ok, name: "" }, "https://g.invalid")).toBe(false);
    expect(validMetadata({ ...ok, name: " padded" }, "https://g.invalid")).toBe(false);
    expect(validMetadata({ ...ok, name: "bell\u0007" }, "https://g.invalid")).toBe(false);
    expect(validMetadata({ ...ok, name: "é".repeat(41) }, "https://g.invalid")).toBe(false);
    expect(validMetadata({ ...ok, name: "é".repeat(40) }, "https://g.invalid")).toBe(true);
    expect(validMetadata({ ...ok, version: "1.0 beta" }, "https://g.invalid")).toBe(false);
    expect(validMetadata({ ...ok, version: "x".repeat(65) }, "https://g.invalid")).toBe(false);
  });
});

describe("parsePage", () => {
  it("reads a page in UUID order", () => {
    const listings = parsePage(page(0, 3), null);
    expect(listings.map((g) => g.origin)).toEqual([
      "https://gw1.example.invalid",
      "https://gw2.example.invalid",
      "https://gw3.example.invalid",
    ]);
    expect(listings[0]?.id).toBe(uuidText(idBytes(1)));
  });

  it("refuses an unordered page, an id at or before the cursor, and unknown fields", () => {
    const swapped = { result: "gateway_directory", gateways: [listing(2), listing(1)] };
    expect(() => parsePage(swapped, null)).toThrow(DirectoryError);
    expect(() => parsePage(page(0, 2), idBytes(1))).toThrow(DirectoryError);
    expect(() => parsePage({ ...page(0, 1), extra: 1 }, null)).toThrow(DirectoryError);
    const g = { ...listing(1), owner: "someone" };
    expect(() => parsePage({ result: "gateway_directory", gateways: [g] }, null)).toThrow(DirectoryError);
    expect(() => parsePage({ result: "object", gateways: [] }, null)).toThrow(DirectoryError);
  });

  it("refuses more than a page and a listing that fails its checks", () => {
    const big = { result: "gateway_directory", gateways: Array.from({ length: 101 }, (_, i) => listing(i + 1)) };
    expect(() => parsePage(big, null)).toThrow(DirectoryError);
    const foreign = listing(1);
    foreign.metadata.website = "https://elsewhere.invalid/";
    expect(() => parsePage({ result: "gateway_directory", gateways: [foreign] }, null)).toThrow(DirectoryError);
  });

  it("leaves markup in a name as text for the page to escape", () => {
    const g = listing(1, { name: "<img src=x onerror=alert(1)>" });
    expect(parsePage({ result: "gateway_directory", gateways: [g] }, null)[0]?.metadata.name).toBe("<img src=x onerror=alert(1)>");
  });
});

describe("directoryClient", () => {
  it("posts the exact native request, without credentials, and follows the cursor", async () => {
    const fetch = vi.fn<(url: string, init?: RequestInit) => Promise<Response>>(async () => answer(200, page(0, 2)));
    const client = directoryClient({ api: "/api", fetch: fetch as unknown as typeof globalThis.fetch });
    const first = await client.page(null);
    expect(first).toHaveLength(2);
    const [url, init] = fetch.mock.calls[0] ?? [];
    expect(url).toBe("/api/v1/native");
    expect(init?.method).toBe("POST");
    expect(init?.credentials).toBe("omit");
    expect(init?.headers).toEqual({ "Content-Type": "application/json", Accept: "application/json" });
    expect(JSON.parse(String(init?.body))).toEqual({ operation: "gateway_directory", after: null });

    fetch.mockResolvedValueOnce(answer(200, { result: "gateway_directory", gateways: [] }));
    await client.page(first[1]?.id ?? null);
    expect(JSON.parse(String(fetch.mock.calls[1]?.[1]?.body))).toEqual({ operation: "gateway_directory", after: idBytes(2) });
  });

  it("names each refusal and never parses its plain text as JSON", async () => {
    const cases: [number, string][] = [
      [429, "busy"],
      [503, "unavailable"],
      [401, "refused"],
      [403, "refused"],
      [500, "unavailable"],
    ];
    for (const [status, kind] of cases) {
      const client = directoryClient({ api: "/api", fetch: (async () => answer(status, "account request rejected\n", false)) as typeof fetch });
      await expect(client.page(null)).rejects.toMatchObject({ kind });
    }
    const down = directoryClient({ api: "/api", fetch: (async () => Promise.reject(new TypeError("offline"))) as typeof fetch });
    await expect(down.page(null)).rejects.toMatchObject({ kind: "network" });
  });

  it("refuses a cursor that is no UUIDv7 before asking", async () => {
    const fetch = vi.fn<typeof globalThis.fetch>();
    const client = directoryClient({ api: "/api", fetch: fetch as unknown as typeof globalThis.fetch });
    await expect(client.page("not-an-id")).rejects.toMatchObject({ kind: "malformed" });
    expect(fetch).not.toHaveBeenCalled();
  });
});
