import { afterEach, beforeEach, expect, it, vi } from "vitest";

const api = vi.hoisted(() => vi.fn<(request: unknown) => Promise<unknown>>(async () => ({})));
vi.mock("../../src/pwa/api", () => ({ notifications: api }));
const LOCAL = "baylee-push-installation-v1";
beforeEach(() => { vi.resetModules(); localStorage.clear(); api.mockReset().mockResolvedValue({}); });
afterEach(() => vi.unstubAllGlobals());

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => { resolve = done; });
  return { promise, resolve };
}
function browser(ready: Promise<unknown>, messages: unknown[]) {
  vi.stubGlobal("isSecureContext", true);
  vi.stubGlobal("PushManager", {});
  vi.stubGlobal("Notification", { permission: "granted" });
  vi.stubGlobal("navigator", { serviceWorker: { register: () => ready, getRegistration: async () => undefined } });
  vi.stubGlobal("MessageChannel", class {
    callback: ((event: { data: unknown }) => void) | undefined;
    port1 = { start() {}, close() {}, addEventListener: (_: string, fn: (event: { data: unknown }) => void) => { this.callback = fn; } };
    port2 = { close() {}, acknowledge: () => this.callback?.({ data: { ok: true } }) };
  });
  return { active: { postMessage: (data: unknown, ports: { acknowledge(): void }[]) => { messages.push(data); ports[0]?.acknowledge(); } } };
}
it("a delayed registration cannot restore consent after a newer opt-out", async () => {
  const ready = deferred<unknown>(); const messages: unknown[] = [];
  const registration = browser(ready.promise, messages);
  const { workerState } = await import("../../src/pwa/client");
  const old = workerState("a".repeat(32), 7);
  const optOut = workerState(null, 0);
  ready.resolve(registration);
  await Promise.all([old, optOut]);
  expect(messages).toEqual([{ type: "PUSH_STATE", nonce: null, enabled: 0 }]);
});
it("logout invalidates pending writes and forgets the installation before browser lookup fails", async () => {
  const ready = deferred<unknown>(); const messages: unknown[] = [];
  const registration = browser(ready.promise, messages);
  navigator.serviceWorker.getRegistration = vi.fn<() => Promise<ServiceWorkerRegistration | undefined>>(async () => { throw new Error("browser unavailable"); });
  const { workerState, disablePush } = await import("../../src/pwa/client");
  localStorage.setItem(LOCAL, "a".repeat(32));
  const old = workerState("a".repeat(32), 7);
  await expect(disablePush()).rejects.toThrow("browser unavailable");
  expect(localStorage.getItem(LOCAL)).toBeNull();
  ready.resolve(registration); await old;
  expect(messages).toEqual([]);
});

it("retires the same account's previous endpoint instead of leaking device quota on reauthentication", async () => {
  const messages: unknown[] = [];
  const ready = deferred<unknown>(); const registration = browser(ready.promise, messages);
  const sub = { endpoint: "https://fcm.googleapis.com/fcm/send/owned", unsubscribe: vi.fn<() => Promise<boolean>>(async () => true) };
  const active = { ...registration, pushManager: { getSubscription: async () => sub } };
  navigator.serviceWorker.getRegistration = vi.fn<() => Promise<ServiceWorkerRegistration>>(async () => active as unknown as ServiceWorkerRegistration);
  ready.resolve(active);
  const { disablePush } = await import("../../src/pwa/client");
  await disablePush();
  expect(api).toHaveBeenCalledWith({ command: "unsubscribe", endpoint: sub.endpoint });
  expect(sub.unsubscribe).toHaveBeenCalledTimes(1);
  expect(messages).toEqual([{ type: "PUSH_STATE", nonce: null, enabled: 0 }]);
  api.mockRejectedValueOnce(new Error("offline"));
  await expect(disablePush()).rejects.toThrow("unsubscribe not confirmed");
  expect(sub.unsubscribe).toHaveBeenCalledTimes(2);
});
