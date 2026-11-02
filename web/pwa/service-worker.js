// Generated build constants precede this file. Cache only the public build;
// cookie APIs, chat ciphertext and account responses never enter CacheStorage.
const PATHS = { chat: "/account/chat", contacts: "/account/contacts", updates: "/download", feedback: "/admin/reports", gateways: "/admin/gateways" };
const BITS = { chat: 1, contacts: 2, updates: 4, feedback: 8, gateways: 16 };
const WORDS = {
  en: { chat: "You have a new chat message.", contacts: "You have a new contact request.", updates: "A new Baylee version is available.", feedback: "New feedback is ready to review.", gateways: "A gateway is waiting for approval." },
  de: { chat: "Du hast eine neue Chat-Nachricht.", contacts: "Du hast eine neue Kontaktanfrage.", updates: "Eine neue Baylee-Version ist verfügbar.", feedback: "Eine neue Feedback-Meldung wartet auf Prüfung.", gateways: "Ein Gateway wartet auf Freigabe." },
};
function state(write, value) {
  return new Promise((resolve, reject) => {
    const opening = indexedDB.open("baylee-push", 1);
    opening.addEventListener("upgradeneeded", () => opening.result.createObjectStore("settings"), { once: true });
    opening.addEventListener("error", () => reject(new Error("storage unavailable")), { once: true });
    opening.addEventListener("success", () => {
      const db = opening.result;
      const tx = db.transaction("settings", write ? "readwrite" : "readonly");
      const request = write ? tx.objectStore("settings").put(value, "installation") : tx.objectStore("settings").get("installation");
      tx.addEventListener("complete", () => { db.close(); resolve(write ? value : request.result); }, { once: true });
      const failed = () => { db.close(); reject(new Error("storage unavailable")); };
      tx.addEventListener("error", failed, { once: true });
      tx.addEventListener("abort", failed, { once: true });
    }, { once: true });
  });
}
self.addEventListener("install", (event) => {
  event.waitUntil(caches.open(CACHE).then((cache) => cache.addAll(ASSETS.map((path) => new Request(path, { credentials: "omit", cache: "reload" })))));
});
self.addEventListener("activate", (event) => {
  event.waitUntil((async () => {
    // Keep one prior build for lazy chunks in tabs still using it. Two builds
    // bound storage; activating an update never forces another tab to reload.
    const keys = await caches.keys();
    await Promise.all(keys.filter((key) => key.startsWith("baylee-public-") && key !== CACHE).slice(0, -1).map((key) => caches.delete(key)));
    await self.clients.claim();
  })());
});
self.addEventListener("fetch", (event) => {
  const url = new URL(event.request.url);
  if (event.request.method !== "GET" || url.origin !== self.location.origin || url.pathname.startsWith("/api/") || url.pathname.startsWith("/site/")) return;
  if (event.request.mode === "navigate") {
    event.respondWith(fetch(event.request).catch(async () => (await (await caches.open(CACHE)).match("/index.html")) ?? new Response("Offline", { status: 503 })));
  } else if (url.search === "" && ASSETS.includes(url.pathname)) {
    event.respondWith(caches.open(CACHE).then(async (cache) => (await cache.match(url.pathname)) ?? fetch(event.request)));
  } else if (url.search === "" && /^\/assets\/[^/]+$/.test(url.pathname)) {
    event.respondWith((async () => {
      for (const key of (await caches.keys()).filter((name) => name.startsWith("baylee-public-")).toReversed()) {
        const old = await (await caches.open(key)).match(url.pathname);
        if (old) return old;
      }
      return fetch(event.request);
    })());
  }
});
self.addEventListener("message", (event) => {
  if (!event.source || new URL(event.source.url).origin !== self.location.origin) return;
  if (event.data?.type === "ACTIVATE_UPDATE") { event.waitUntil(self.skipWaiting()); return; }
  if (event.data?.type !== "PUSH_STATE") return;
  const { nonce, enabled } = event.data;
  if ((nonce !== null && (typeof nonce !== "string" || !/^[a-f0-9]{32}$/.test(nonce))) || !Number.isInteger(enabled) || enabled < 0 || enabled > 31) return;
  event.waitUntil(state(true, { nonce, enabled }).then(async () => {
    const notifications = await self.registration.getNotifications();
    for (const notice of notifications) {
      if (nonce === null || !(enabled & BITS[notice.data?.kind])) notice.close();
    }
    event.ports[0]?.postMessage({ ok: true });
  }).catch(() => { event.ports[0]?.postMessage({ ok: false }); }));
});
self.addEventListener("push", (event) => {
  event.waitUntil((async () => {
    let payload;
    try { payload = event.data?.json(); } catch { return; }
    if (payload?.v !== 1 || !Object.hasOwn(BITS, payload.kind) || typeof payload.nonce !== "string") return;
    if (Notification.permission !== "granted") return;
    let current;
    try { current = await state(false); } catch { return; }
    if (!current || current.nonce !== payload.nonce || !(current.enabled & BITS[payload.kind])) return;
    const words = payload.lang === "de" ? WORDS.de : WORDS.en;
    await self.registration.showNotification("Baylee", { body: words[payload.kind], icon: "/apple-touch-icon.png", badge: "/favicon-32.png", tag: `baylee-${payload.kind}`, data: { kind: payload.kind }, renotify: false });
    const pages = await self.clients.matchAll({ type: "window" });
    for (const page of pages) {
      // oxlint-disable-next-line unicorn/require-post-message-target-origin -- ServiceWorker Client API has no targetOrigin parameter
      page.postMessage({ type: "PUSH_RECEIVED", kind: payload.kind });
    }
  })());
});
self.addEventListener("notificationclick", (event) => {
  event.notification.close();
  const kind = event.notification.data?.kind;
  const path = Object.hasOwn(PATHS, kind) ? PATHS[kind] : null;
  if (!path) return;
  event.waitUntil((async () => {
    const pages = await self.clients.matchAll({ type: "window", includeUncontrolled: true });
    const target = new URL(path, self.location.origin).href;
    const existing = pages.find((page) => page.url === target);
    if (existing) await existing.focus();
    else await self.clients.openWindow(target);
  })());
});
