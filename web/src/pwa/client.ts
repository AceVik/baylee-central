import { notifications } from "./api";
const LOCAL = "baylee-push-installation-v1";
let registration: Promise<ServiceWorkerRegistration> | null = null;
let stateGeneration = 0;
export function supported(): boolean { return "serviceWorker" in navigator && "PushManager" in window && "Notification" in window && window.isSecureContext; }
export function register(): Promise<ServiceWorkerRegistration> {
  if (!registration) registration = navigator.serviceWorker.register("/sw.js", { scope: "/", updateViaCache: "none" }).then(async (r) => {
    if (r.active) return r;
    return navigator.serviceWorker.ready;
  }).catch((error: unknown) => { registration = null; throw error; });
  return registration;
}
export function installation(): string | null {
  try { const raw = localStorage.getItem(LOCAL); return raw !== null && /^[a-f0-9]{32}$/.test(raw) ? raw : null; } catch { return null; }
}
export function rememberInstallation(nonce: string | null) {
  if (nonce === null) localStorage.removeItem(LOCAL); else localStorage.setItem(LOCAL, nonce);
}
export async function workerState(nonce: string | null, enabled: number): Promise<void> {
  const generation = ++stateGeneration;
  const r = await register();
  // A later opt-out/account change owns the worker, even if registration was slow.
  if (generation !== stateGeneration) return;
  const worker = r.active;
  if (!worker) throw new Error("worker unavailable");
  await new Promise<void>((resolve, reject) => {
    const channel = new MessageChannel();
    const close = () => { clearTimeout(timer); channel.port1.close(); channel.port2.close(); };
    const timer = setTimeout(() => { close(); reject(new Error("worker timed out")); }, 5000);
    channel.port1.addEventListener("message", (event: MessageEvent<unknown>) => {
      close();
      if (typeof event.data === "object" && event.data !== null && "ok" in event.data && event.data.ok === true) resolve(); else reject(new Error("worker storage unavailable"));
    }, { once: true });
    channel.port1.start();
    worker.postMessage({ type: "PUSH_STATE", nonce, enabled }, [channel.port2]);
  });
}
export function keyBytes(key: string): Uint8Array<ArrayBuffer> {
  return Uint8Array.from(atob(key.replace(/-/g, "+").replace(/_/g, "/")), (c) => c.charCodeAt(0));
}
/** Clear this browser even when the network or worker storage has failed. */
export async function disablePush(): Promise<void> {
  ++stateGeneration;
  let failed = false;
  try { rememberInstallation(null); } catch { failed = true; }
  if (!supported()) return;
  const r = await navigator.serviceWorker.getRegistration("/");
  if (!r) return;
  try { await workerState(null, 0); } catch { failed = true; }
  const sub = await r.pushManager.getSubscription();
  if (sub) {
    try { await notifications({ command: "unsubscribe", endpoint: sub.endpoint }); } catch { failed = true; }
    try { if (!await sub.unsubscribe()) failed = true; } catch { failed = true; }
  }
  if (failed) throw new Error("unsubscribe not confirmed");
}
