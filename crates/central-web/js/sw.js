// GENERATED from crates/central-web/ts/sw.ts via TypeScript 7.1 — do not edit directly.
// baylee.app's service worker, at the origin's root so its scope stays "/"
// and an installed app stays the same app (docs/notifications.md,
// docs/proposals/frontend-rewrite.md §7).
//
// Text is fixed per kind, in German or English as Central says (it knows
// no other language yet); no name or message ever arrives here.
"use strict";
const sw = self;
const BITS = {
    chat: 1,
    contacts: 2,
    updates: 4,
    feedback: 8,
    gateways: 16,
};
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
function isNoticeKind(kind) {
    return typeof kind === "string" && Object.hasOwn(BITS, kind);
}
function state(write, value) {
    return new Promise((resolve, reject) => {
        const opening = indexedDB.open("baylee-push", 1);
        opening.addEventListener("upgradeneeded", () => opening.result.createObjectStore("settings"), { once: true });
        opening.addEventListener("error", () => reject(new Error("storage unavailable")), { once: true });
        opening.addEventListener("success", () => {
            const db = opening.result;
            const tx = db.transaction("settings", write ? "readwrite" : "readonly");
            const store = tx.objectStore("settings");
            const request = write ? store.put(value, "installation") : store.get("installation");
            tx.addEventListener("complete", () => {
                db.close();
                resolve(write ? value : request.result);
            }, { once: true });
            const failed = () => {
                db.close();
                reject(new Error("storage unavailable"));
            };
            tx.addEventListener("error", failed, { once: true });
            tx.addEventListener("abort", failed, { once: true });
        }, { once: true });
    });
}
const BUILD = "__BUILD__";
const CACHE = `baylee-site-${BUILD}`;
const LANGS = /* the site's languages */ ["en"];
const OFFLINE = LANGS.map((lang) => `/${lang}/offline`);
const ASSETS = /* the offline page's files */ [];
const HASHED = /^\/assets\/(?:[\w-]+\/)*[\w.-]+\.[0-9a-f]{12}\.[a-z0-9]+$/;
function fill() {
    return caches.open(CACHE).then((cache) => cache.addAll([...OFFLINE, ...ASSETS]));
}
sw.addEventListener("install", (event) => {
    event.waitUntil(fill());
});
sw.addEventListener("activate", (event) => {
    event.waitUntil((async () => {
        const keys = await caches.keys();
        const old = keys.filter((key) => mine(key) && key !== CACHE);
        await Promise.all(old.map((key) => caches.delete(key)));
        await sw.clients.claim();
    })());
});
let retired = false;
let turn = Promise.resolve();
const answering = new Set();
function mine(key) {
    return key.startsWith("baylee-site-") || key.startsWith("baylee-public-");
}
sw.addEventListener("fetch", (event) => {
    if (retired)
        return;
    const request = event.request;
    if (request.method !== "GET")
        return;
    const url = new URL(request.url);
    if (url.origin !== sw.location.origin)
        return;
    if (request.mode === "navigate") {
        const first = url.pathname.split("/")[1] ?? "";
        const offline = `/${LANGS.includes(first) ? first : "en"}/offline`;
        event.respondWith(fetch(request).catch(async () => (await caches.match(offline, { cacheName: CACHE })) ?? Response.error()));
        return;
    }
    if (!HASHED.test(url.pathname))
        return;
    const answer = (async () => {
        if (!(await caches.has(CACHE)))
            return fetch(request);
        const cache = await caches.open(CACHE);
        const kept = await cache.match(request);
        if (kept)
            return kept;
        const fresh = await fetch(request);
        if (fresh.ok && fresh.type === "basic")
            await cache.put(request, fresh.clone());
        return fresh;
    })();
    answering.add(answer);
    const done = () => {
        answering.delete(answer);
    };
    answer.then(done, done);
    event.respondWith(answer);
});
sw.addEventListener("message", (event) => {
    const sourceUrl = event.source && "url" in event.source ? event.source.url : null;
    if (!sourceUrl || new URL(sourceUrl).origin !== sw.location.origin)
        return;
    const data = (typeof event.data === "object" && event.data !== null ? event.data : null);
    if (data?.type === "SKIP_WAITING") {
        event.waitUntil(sw.skipWaiting());
        return;
    }
    if (data?.type === "RETIRE") {
        retired = true;
        turn = turn
            .then(async () => {
            retired = true;
            await Promise.allSettled(answering);
            const keys = await caches.keys();
            await Promise.all(keys.filter(mine).map((key) => caches.delete(key)));
            event.ports[0]?.postMessage({ ok: true });
        })
            .catch(() => {
            event.ports[0]?.postMessage({ ok: false });
        });
        event.waitUntil(turn);
        return;
    }
    if (data?.type === "KEEP") {
        turn = turn
            .then(async () => {
            retired = false;
            if (!(await caches.match(OFFLINE[0], { cacheName: CACHE })))
                await fill();
        })
            .catch(() => { });
        event.waitUntil(turn);
        return;
    }
    if (data?.type !== "PUSH_STATE")
        return;
    const nonce = data.nonce;
    const enabled = data.enabled;
    if ((nonce !== null && (typeof nonce !== "string" || !/^[a-f0-9]{32}$/.test(nonce))) ||
        typeof enabled !== "number" ||
        !Number.isInteger(enabled) ||
        enabled < 0 ||
        enabled > 31) {
        return;
    }
    const cleanNonce = nonce;
    event.waitUntil(state(true, { nonce: cleanNonce, enabled })
        .then(async () => {
        for (const notice of await sw.registration.getNotifications()) {
            const nKind = notice.data?.kind;
            if (cleanNonce === null || !isNoticeKind(nKind) || !(enabled & BITS[nKind]))
                notice.close();
        }
        event.ports[0]?.postMessage({ ok: true });
    })
        .catch(() => {
        event.ports[0]?.postMessage({ ok: false });
    }));
});
sw.addEventListener("push", (event) => {
    event.waitUntil((async () => {
        let payload = null;
        try {
            const raw = event.data?.json();
            if (typeof raw === "object" && raw !== null)
                payload = raw;
        }
        catch {
            return;
        }
        if (!payload || payload.v !== 1 || !isNoticeKind(payload.kind) || typeof payload.nonce !== "string")
            return;
        if (Notification.permission !== "granted")
            return;
        let current;
        try {
            current = await state(false);
        }
        catch {
            return;
        }
        if (!current || current.nonce !== payload.nonce || !(current.enabled & BITS[payload.kind]))
            return;
        const lang = payload.lang === "de" ? "de" : "en";
        await sw.registration.showNotification("Baylee", {
            body: WORDS[lang][payload.kind],
            icon: "/apple-touch-icon.png",
            tag: `baylee-${payload.kind}`,
            data: { kind: payload.kind, lang },
        });
        for (const page of await sw.clients.matchAll({ type: "window" })) {
            page.postMessage({ type: "PUSH_RECEIVED", kind: payload.kind });
        }
    })());
});
sw.addEventListener("notificationclick", (event) => {
    event.notification.close();
    const data = (typeof event.notification.data === "object" && event.notification.data !== null
        ? event.notification.data
        : {});
    const path = isNoticeKind(data.kind) ? PATHS[data.kind] : null;
    if (!path)
        return;
    event.waitUntil((async () => {
        const target = new URL(`/${data.lang === "de" ? "de" : "en"}/${path}`, sw.location.origin).href;
        const pages = await sw.clients.matchAll({ type: "window", includeUncontrolled: true });
        const existing = pages.find((page) => page.url === target);
        if (existing)
            await existing.focus();
        else
            await sw.clients.openWindow(target);
    })());
});
