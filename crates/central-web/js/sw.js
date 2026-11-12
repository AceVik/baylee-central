// baylee.app's service worker, at the origin's root so its scope stays "/"
// and an installed app stays the same app (docs/notifications.md,
// docs/proposals/frontend-rewrite.md §7).
//
// It keeps one cache per build ("baylee-site-<build>", written by
// `cargo xtask site` over the placeholders below): the offline page in
// every language and the hashed files it needs, then every hashed asset as
// it is fetched. Hashed files never change, so they come from the cache
// first; pages always come from the network, and only when there is none
// the offline page answers in the address's language. /api and /site are
// never touched. A new build waits until a page says to go ahead
// (SKIP_WAITING, after the visitor chose to reload); the previous site's
// caches ("baylee-public-*") and older builds' go when it starts.
//
// It keeps the push protocol exactly: a push carries only
// {v:1, kind, nonce, lang}; it is shown only while this browser's opt-in
// (IndexedDB "baylee-push"/"settings", written by the page with
// PUSH_STATE) names the same random nonce and the kind is switched on.
// Text is fixed per kind, in German or English as Central says (it knows
// no other language yet); no name or message ever arrives here.
"use strict";

const BITS = { chat: 1, contacts: 2, updates: 4, feedback: 8, gateways: 16 };
const PATHS = {
  chat: "account/chat",
  contacts: "account/contacts",
  updates: "download",
  feedback: "admin/feedback",
  gateways: "admin/gateways",
};
const WORDS = {
  en: {
    chat: "You have a new chat message.",
    contacts: "You have a new contact request.",
    updates: "A new Baylee version is available.",
    feedback: "New feedback is ready to review.",
    gateways: "A gateway is waiting for approval.",
  },
  de: {
    chat: "Du hast eine neue Chat-Nachricht.",
    contacts: "Du hast eine neue Kontaktanfrage.",
    updates: "Eine neue Baylee-Version ist verfügbar.",
    feedback: "Eine neue Feedback-Meldung wartet auf Prüfung.",
    gateways: "Ein Gateway wartet auf Freigabe.",
  },
};

function state(write, value) {
  return new Promise((resolve, reject) => {
    const opening = indexedDB.open("baylee-push", 1);
    opening.addEventListener("upgradeneeded", () => opening.result.createObjectStore("settings"), { once: true });
    opening.addEventListener("error", () => reject(new Error("storage unavailable")), { once: true });
    opening.addEventListener(
      "success",
      () => {
        const db = opening.result;
        const tx = db.transaction("settings", write ? "readwrite" : "readonly");
        const store = tx.objectStore("settings");
        const request = write ? store.put(value, "installation") : store.get("installation");
        tx.addEventListener(
          "complete",
          () => {
            db.close();
            resolve(write ? value : request.result);
          },
          { once: true },
        );
        const failed = () => {
          db.close();
          reject(new Error("storage unavailable"));
        };
        tx.addEventListener("error", failed, { once: true });
        tx.addEventListener("abort", failed, { once: true });
      },
      { once: true },
    );
  });
}

const BUILD = "__BUILD__";
const CACHE = `baylee-site-${BUILD}`;
const LANGS = /* the site's languages */ ["en"];
const OFFLINE = LANGS.map((lang) => `/${lang}/offline`);
const ASSETS = /* the offline page's files */ [];
const HASHED = /^\/assets\/(?:[\w-]+\/)*[\w.-]+\.[0-9a-f]{12}\.[a-z0-9]+$/;

self.addEventListener("install", (event) => {
  event.waitUntil(caches.open(CACHE).then((cache) => cache.addAll([...OFFLINE, ...ASSETS])));
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    (async () => {
      const keys = await caches.keys();
      const old = keys.filter((key) => mine(key) && key !== CACHE);
      await Promise.all(old.map((key) => caches.delete(key)));
      await self.clients.claim();
    })(),
  );
});

// A page that found its visitor signed out retires this worker before it
// removes it ("RETIRE", below). The worker goes on answering for that page
// until the page goes, so from then on it leaves every request to the
// network, and what it was writing just then is waited for before the
// cache is taken away: nothing makes the cache again behind the page.
let retired = false;
const answering = new Set();

function mine(key) {
  return key.startsWith("baylee-site-") || key.startsWith("baylee-public-");
}

self.addEventListener("fetch", (event) => {
  if (retired) return;
  const request = event.request;
  if (request.method !== "GET") return;
  const url = new URL(request.url);
  if (url.origin !== self.location.origin) return;
  if (request.mode === "navigate") {
    const first = url.pathname.split("/")[1] ?? "";
    const offline = `/${LANGS.includes(first) ? first : "en"}/offline`;
    event.respondWith(fetch(request).catch(async () => (await caches.match(offline, { cacheName: CACHE })) ?? Response.error()));
    return;
  }
  if (!HASHED.test(url.pathname)) return;
  const answer = (async () => {
    // A worker started again after its cache was taken away (it forgot that
    // it was retired) finds no cache, and does not make one.
    if (!(await caches.has(CACHE))) return fetch(request);
    const cache = await caches.open(CACHE);
    const kept = await cache.match(request);
    if (kept) return kept;
    const fresh = await fetch(request);
    if (fresh.ok && fresh.type === "basic") await cache.put(request, fresh.clone());
    return fresh;
  })();
  answering.add(answer);
  const done = () => answering.delete(answer);
  answer.then(done, done);
  event.respondWith(answer);
});

self.addEventListener("message", (event) => {
  if (!event.source || new URL(event.source.url).origin !== self.location.origin) return;
  if (event.data?.type === "SKIP_WAITING") {
    event.waitUntil(self.skipWaiting());
    return;
  }
  if (event.data?.type === "RETIRE") {
    retired = true;
    event.waitUntil(
      (async () => {
        await Promise.allSettled([...answering]);
        const keys = await caches.keys();
        await Promise.all(keys.filter(mine).map((key) => caches.delete(key)));
        event.ports[0]?.postMessage({ ok: true });
      })().catch(() => {
        event.ports[0]?.postMessage({ ok: false });
      }),
    );
    return;
  }
  if (event.data?.type !== "PUSH_STATE") return;
  const { nonce, enabled } = event.data;
  if (
    (nonce !== null && (typeof nonce !== "string" || !/^[a-f0-9]{32}$/.test(nonce))) ||
    !Number.isInteger(enabled) ||
    enabled < 0 ||
    enabled > 31
  ) {
    return;
  }
  event.waitUntil(
    state(true, { nonce, enabled })
      .then(async () => {
        for (const notice of await self.registration.getNotifications()) {
          if (nonce === null || !(enabled & BITS[notice.data?.kind])) notice.close();
        }
        event.ports[0]?.postMessage({ ok: true });
      })
      .catch(() => {
        event.ports[0]?.postMessage({ ok: false });
      }),
  );
});

self.addEventListener("push", (event) => {
  event.waitUntil(
    (async () => {
      let payload;
      try {
        payload = event.data?.json();
      } catch {
        return;
      }
      if (payload?.v !== 1 || !Object.hasOwn(BITS, payload.kind) || typeof payload.nonce !== "string") return;
      if (Notification.permission !== "granted") return;
      let current;
      try {
        current = await state(false);
      } catch {
        return;
      }
      if (!current || current.nonce !== payload.nonce || !(current.enabled & BITS[payload.kind])) return;
      const lang = payload.lang === "de" ? "de" : "en";
      await self.registration.showNotification("Baylee", {
        body: WORDS[lang][payload.kind],
        icon: "/apple-touch-icon.png",
        tag: `baylee-${payload.kind}`,
        data: { kind: payload.kind, lang },
        renotify: false,
      });
      for (const page of await self.clients.matchAll({ type: "window" })) {
        page.postMessage({ type: "PUSH_RECEIVED", kind: payload.kind });
      }
    })(),
  );
});

self.addEventListener("notificationclick", (event) => {
  event.notification.close();
  const { kind, lang } = event.notification.data ?? {};
  const path = Object.hasOwn(PATHS, kind) ? PATHS[kind] : null;
  if (!path) return;
  event.waitUntil(
    (async () => {
      const target = new URL(`/${lang === "de" ? "de" : "en"}/${path}`, self.location.origin).href;
      const pages = await self.clients.matchAll({ type: "window", includeUncontrolled: true });
      const existing = pages.find((page) => page.url === target);
      if (existing) await existing.focus();
      else await self.clients.openWindow(target);
    })(),
  );
});
