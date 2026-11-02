import { readFileSync } from "node:fs";
import { runInNewContext } from "node:vm";
import { describe, expect, it, vi } from "vitest";

const source = readFileSync("pwa/service-worker.js", "utf8");
function worker(failStorage = false) {
  const handlers = new Map<string, (event: object) => void>();
  let installation: unknown;
  const stores = new Map<string, Map<string, unknown>>([
    ["baylee-public-oldest", new Map()],
    ["baylee-public-previous", new Map([["/assets/old.js", "old chunk"]])],
    ["baylee-public-current", new Map([["/index.html", "public shell"]])],
    ["another-app", new Map()],
  ]);
  const show = vi.fn<(...args: unknown[]) => Promise<void>>(async () => {});
  const open = vi.fn<(...args: unknown[]) => Promise<void>>(async () => {});
  const closed = vi.fn<(...args: unknown[]) => void>();
  const cache = { keys: async () => [...stores.keys()], delete: async (key: string) => stores.delete(key), open: async (key: string) => ({ match: async (path: string) => stores.get(key)?.get(path), addAll: vi.fn<(...args: unknown[]) => void>() }) };
  const permission = { permission: "granted" };
  const network = vi.fn<(...args: unknown[]) => Promise<string>>(async () => "network");
  const self = { location: { origin: "https://baylee.app" }, addEventListener: (kind: string, fn: (e: object) => void) => handlers.set(kind, fn), skipWaiting: vi.fn<(...args: unknown[]) => Promise<void>>(async () => {}), clients: { claim: vi.fn<(...args: unknown[]) => Promise<void>>(async () => {}), matchAll: async () => [], openWindow: open }, registration: { showNotification: show, getNotifications: async () => [{ data: { kind: "chat" }, close: closed }] } };
  // Minimal asynchronous IDB adapter: the production worker's transactions and
  // event handlers run unchanged. Storage errors are separately injected below.
  const db = { close() {}, transaction: () => {
    const tx = Object.assign(new EventTarget(), { objectStore: () => ({ put: (value: unknown) => { installation = value; return {}; }, get: () => ({ result: installation }) }) });
    queueMicrotask(() => tx.dispatchEvent(new Event("complete"))); return tx;
  } };
  const indexedDB = { open: () => { const request = Object.assign(new EventTarget(), { result: db }); queueMicrotask(() => request.dispatchEvent(new Event(failStorage ? "error" : "success"))); return request; } };
  runInNewContext(`const CACHE = "baylee-public-current"; const ASSETS = ["/index.html", "/assets/current.js"];\n${source}`, { self, caches: cache, indexedDB, URL, Request, Response, Notification: permission, fetch: network });
  const dispatch = async (kind: string, properties: object) => {
    const work: Promise<unknown>[] = [];
    handlers.get(kind)?.({ ...properties, waitUntil: (p: Promise<unknown>) => work.push(p), respondWith: (p: Promise<unknown>) => work.push(p) });
    return Promise.all(work);
  };
  const message = (nonce: string | null, enabled: number, origin = "https://baylee.app/account") => dispatch("message", { source: { url: origin }, data: { type: "PUSH_STATE", nonce, enabled }, ports: [{ postMessage: vi.fn<(...args: unknown[]) => void>() }] });
  const push = (data: unknown) => dispatch("push", { data: { json: () => data } });
  return { dispatch, message, push, show, open, closed, network, self, stores, permission };
}
const nonce = "0123456789abcdef0123456789abcdef";
const hint = { v: 1, kind: "chat", lang: "en", nonce };

describe("actual service worker", () => {
  it("does not display a push when consent storage is unavailable", async () => {
    const w = worker(true);
    await w.message(nonce, 31); await w.push(hint);
    expect(w.show).not.toHaveBeenCalled();
  });
  it("requires current installation and category consent, after account changes and opt-out", async () => {
    const w = worker(); await w.push(hint); expect(w.show).not.toHaveBeenCalled();
    await w.message(nonce, 2); await w.push(hint); expect(w.show).not.toHaveBeenCalled();
    await w.push({ ...hint, kind: "contacts" }); expect(w.show).toHaveBeenCalledTimes(1);
    await w.message("f".repeat(32), 31); await w.push(hint); expect(w.show).toHaveBeenCalledTimes(1);
    await w.message(nonce, 31); await w.push(hint); expect(w.show).toHaveBeenCalledTimes(2);
    w.permission.permission = "denied"; await w.push(hint); expect(w.show).toHaveBeenCalledTimes(2);
    w.permission.permission = "granted";
    await w.message(null, 0); await w.push(hint); expect(w.show).toHaveBeenCalledTimes(2);
    expect(w.closed).toHaveBeenCalled();
  });
  it("refuses foreign-origin state changes and unrecognized payloads; displays no supplied private text", async () => {
    const w = worker(); await w.message(nonce, 31, "https://evil.example/"); await w.push(hint); expect(w.show).not.toHaveBeenCalled();
    await w.message(nonce, 1);
    for (const bad of [{ ...hint, v: 2 }, { ...hint, kind: "__proto__" }, { ...hint, nonce: "old" }]) await w.push(bad);
    expect(w.show).not.toHaveBeenCalled();
    await w.push({ ...hint, lang: "__proto__", body: "private message", url: "https://evil.example/" });
    expect(JSON.stringify(w.show.mock.calls)).toContain("You have a new chat message.");
    expect(JSON.stringify(w.show.mock.calls)).not.toContain("private message");
    expect(JSON.stringify(w.show.mock.calls)).not.toContain("evil.example");
  });
  it("opens only fixed same-origin destinations, regardless of a supplied URL", async () => {
    const w = worker(); const close = vi.fn<(...args: unknown[]) => void>();
    await w.dispatch("notificationclick", { notification: { close, data: { kind: "chat", url: "https://evil.example/" } } });
    expect(w.open).toHaveBeenCalledWith("https://baylee.app/account/chat");
    await w.dispatch("notificationclick", { notification: { close, data: { kind: "__proto__" } } });
    expect(w.open).toHaveBeenCalledTimes(1);
  });
  it("never intercepts APIs, writes or foreign resources; offline navigation uses only the public shell", async () => {
    const w = worker();
    for (const [url, method] of [["https://baylee.app/api/identity", "GET"], ["https://baylee.app/site/version", "GET"], ["https://other.example/", "GET"], ["https://baylee.app/account", "POST"]]) {
      expect(await w.dispatch("fetch", { request: { url, method, mode: "navigate" } })).toEqual([]);
    }
    w.network.mockRejectedValue(new Error("offline"));
    expect(await w.dispatch("fetch", { request: { url: "https://baylee.app/account/chat", method: "GET", mode: "navigate" } })).toEqual(["public shell"]);
  });
  it("retains one previous build for other tabs and activates only on explicit request", async () => {
    const w = worker(); await w.dispatch("activate", {});
    expect([...w.stores.keys()]).toEqual(["baylee-public-previous", "baylee-public-current", "another-app"]);
    expect(await w.dispatch("fetch", { request: { url: "https://baylee.app/assets/old.js", method: "GET", mode: "cors" } })).toEqual(["old chunk"]);
    expect(w.self.skipWaiting).not.toHaveBeenCalled();
    await w.dispatch("message", { source: { url: "https://baylee.app/" }, data: { type: "ACTIVATE_UPDATE" } });
    expect(w.self.skipWaiting).toHaveBeenCalledTimes(1);
  });
});
