import { describe, expect, it, vi } from "vitest";

import { directoryClient } from "../../src/central/directory";
import { idBytes, page } from "../fixtures/directory";

function answer(status: number, body: unknown, json = true): Response {
  return new Response(json ? JSON.stringify(body) : String(body), { status, headers: { "Content-Type": json ? "application/json" : "text/plain" } });
}

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
