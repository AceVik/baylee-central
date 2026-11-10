// baylee.app's service worker, at the origin's root so its scope stays "/"
// and an installed app stays the same app (docs/notifications.md).
//
// It caches nothing yet (the offline page and the update prompt come with
// the chat; docs/proposals/frontend-rewrite.md §7) and removes the previous
// site's caches ("baylee-public-*"). It keeps the push protocol exactly:
// a push carries only {v:1, kind, nonce, lang}; it is shown only while this
// browser's opt-in (IndexedDB "baylee-push"/"settings", written by the page
// with PUSH_STATE) names the same random nonce and the kind is switched on.
// Text is fixed per kind; no name or message ever arrives here.
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

self.addEventListener("install", (event) => {
  event.waitUntil(self.skipWaiting());
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    (async () => {
      const keys = await caches.keys();
      await Promise.all(keys.filter((key) => key.startsWith("baylee-public-")).map((key) => caches.delete(key)));
      await self.clients.claim();
    })(),
  );
});

self.addEventListener("message", (event) => {
  if (!event.source || new URL(event.source.url).origin !== self.location.origin) return;
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
