// GENERATED from crates/central-web/ts/pwa.ts via TypeScript 7.1 — do not edit directly.
// baylee.app as an installable app (docs/proposals/frontend-rewrite.md §7):
// registers the service worker at /sw.js through a Trusted Types policy of
// its own that vouches for that one address, offers a new version once it
// is waiting (never reloading over text not sent yet), and offers to
// install the app where the browser allows it. Nothing here is required:
// without it every page still works.
const swRoot = navigator.serviceWorker;
function kept(key) {
    try {
        return localStorage.getItem(key);
    }
    catch {
        return null;
    }
}
const REFRESH_GRACE_MS = (3 * 86400 - 900) * 1000;
function signedIn() {
    try {
        const session = JSON.parse(kept("baylee-central-session") ?? "null");
        return typeof session?.expiresAt === "number" && session.expiresAt * 1000 + REFRESH_GRACE_MS > Date.now();
    }
    catch {
        return false;
    }
}
function installed() {
    return (navigator.standalone === true ||
        window.matchMedia?.("(display-mode: standalone), (display-mode: minimal-ui), (display-mode: window-controls-overlay)").matches === true);
}
if (swRoot && window.isSecureContext) {
    if (signedIn() || installed() || kept("baylee-push-installation-v1") !== null) {
        const policy = window.trustedTypes?.createPolicy("baylee-sw", {
            createScriptURL: (url) => {
                if (url !== "/sw.js")
                    throw new TypeError("only the service worker");
                return url;
            },
        });
        const swUrl = policy ? policy.createScriptURL("/sw.js") : "/sw.js";
        swRoot
            .register(swUrl, {
            scope: "/",
            updateViaCache: "none",
        })
            .then((registration) => {
            registration.active?.postMessage({ type: "KEEP" });
            watch(registration);
        })
            .catch(() => { });
    }
    else {
        swRoot
            .getRegistration("/")
            .then(async (registration) => {
            const retired = retire(registration?.active ?? null);
            await registration?.unregister();
            await retired;
            const names = await caches.keys();
            await Promise.all(names.filter((name) => name.startsWith("baylee-site-") || name.startsWith("baylee-public-")).map((name) => caches.delete(name)));
        })
            .catch(() => { });
    }
}
function retire(worker) {
    if (!worker)
        return Promise.resolve();
    return new Promise((resolve) => {
        const channel = new MessageChannel();
        const over = () => {
            channel.port1.close();
            resolve();
        };
        channel.port1.addEventListener("message", over, { once: true });
        channel.port1.start();
        setTimeout(over, 3000);
        try {
            worker.postMessage({ type: "RETIRE" }, [channel.port2]);
        }
        catch {
            over();
        }
    });
}
function unsent() {
    return [...document.querySelectorAll("textarea, input")].some((field) => {
        if (!(field instanceof HTMLInputElement || field instanceof HTMLTextAreaElement))
            return false;
        if (["hidden", "checkbox", "radio", "submit", "button"].includes(field.type))
            return false;
        if (field.closest("dialog:not([open])"))
            return false;
        return field.value !== field.defaultValue && field.value.trim() !== "";
    });
}
function watch(registration) {
    let accepted = false;
    let hadActive = registration.active !== null;
    const offer = (worker) => {
        const box = document.getElementById("update");
        const said = document.getElementById("update-said");
        if (!box || !said)
            return;
        box.hidden = false;
        said.textContent = said.dataset.said ?? "";
        let warned = false;
        box.querySelector("[data-update]")?.addEventListener("click", () => {
            if (!warned && unsent()) {
                warned = true;
                box.querySelector(".update-unsent")?.removeAttribute("hidden");
                return;
            }
            accepted = true;
            worker.postMessage({ type: "SKIP_WAITING" });
        });
    };
    swRoot?.addEventListener("controllerchange", () => {
        if (accepted) {
            accepted = false;
            window.location.reload();
        }
    });
    if (registration.waiting && hadActive && swRoot?.controller)
        offer(registration.waiting);
    registration.addEventListener("updatefound", () => {
        const isUpdate = hadActive || registration.active !== null;
        const worker = registration.installing;
        worker?.addEventListener("statechange", () => {
            if (worker.state === "activated") {
                hadActive = true;
                return;
            }
            if (worker.state === "installed" && swRoot?.controller) {
                if (isUpdate)
                    offer(worker);
                else
                    worker.postMessage({ type: "SKIP_WAITING" });
            }
        });
    });
}
let deferred = null;
const docRoot = document.documentElement;
let lastCanInstall = null;
let lastInstalled = null;
function installState() {
    return {
        canInstall: deferred !== null,
        installed: installed(),
    };
}
function syncInstall(forceEvent = false) {
    const current = installState();
    docRoot.dataset.pwa = current.canInstall ? "installable" : current.installed ? "installed" : "";
    for (const box of document.querySelectorAll("[data-install-box]")) {
        if (box.hidden === current.canInstall)
            box.hidden = !current.canInstall;
    }
    for (const button of document.querySelectorAll("[data-pwa-install]")) {
        if (button.hidden === current.canInstall)
            button.hidden = !current.canInstall;
    }
    if (forceEvent || current.canInstall !== lastCanInstall || current.installed !== lastInstalled) {
        lastCanInstall = current.canInstall;
        lastInstalled = current.installed;
        window.dispatchEvent(new CustomEvent("baylee:pwa", { detail: current }));
    }
}
syncInstall(true);
window.addEventListener("beforeinstallprompt", (event) => {
    event.preventDefault();
    deferred = event;
    syncInstall(true);
});
window.addEventListener("appinstalled", () => {
    deferred = null;
    syncInstall(true);
});
const standaloneMedia = window.matchMedia?.("(display-mode: standalone), (display-mode: minimal-ui), (display-mode: window-controls-overlay)");
standaloneMedia?.addEventListener?.("change", () => syncInstall(true));
document.addEventListener("click", (event) => {
    const button = event.target instanceof Element
        ? event.target.closest("[data-pwa-install], [data-install]")
        : null;
    if (!button || !deferred)
        return;
    const asked = deferred;
    deferred = null;
    void asked.prompt();
    void asked.userChoice.finally(() => {
        syncInstall(true);
    });
});
document.addEventListener("baylee:page", () => {
    syncInstall(true);
});
if (document.body) {
    new MutationObserver((records) => {
        for (const record of records) {
            if (record.addedNodes.length > 0) {
                syncInstall(false);
                return;
            }
        }
    }).observe(document.body, { childList: true, subtree: true });
}
export {};
