// baylee.app as an installable app (docs/proposals/frontend-rewrite.md §7):
// registers the service worker at /sw.js through a Trusted Types policy of
// its own that vouches for that one address, offers a new version once it
// is waiting (never reloading over text not sent yet), and offers to
// install the app where the browser allows it. Nothing here is required:
// without it every page still works.
//
// A visitor who only reads gets nothing stored: the worker and its copy of
// the site's files are for someone signed in, for the installed app, and
// for a device that turned notifications on (which arrive through the
// worker). For anyone else a worker left from earlier is removed with its
// cache.
const root = navigator.serviceWorker;

function kept(key) {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

function signedIn() {
  try {
    const session = JSON.parse(kept("baylee-central-session") ?? "null");
    return typeof session?.expiresAt === "number" && session.expiresAt * 1000 > Date.now();
  } catch {
    return false;
  }
}

function installed() {
  return (
    navigator.standalone === true ||
    window.matchMedia?.("(display-mode: standalone), (display-mode: minimal-ui), (display-mode: window-controls-overlay)").matches === true
  );
}

if (root && window.isSecureContext) {
  if (signedIn() || installed() || kept("baylee-push-installation-v1") !== null) {
    const policy = window.trustedTypes?.createPolicy("baylee-sw", {
      createScriptURL: (url) => {
        if (url !== "/sw.js") throw new TypeError("only the service worker");
        return url;
      },
    });
    root
      .register(policy ? policy.createScriptURL("/sw.js") : "/sw.js", {
        scope: "/",
        updateViaCache: "none",
      })
      .then(watch)
      .catch(() => {});
  } else {
    root
      .getRegistration("/")
      .then(async (registration) => {
        if (!registration) return;
        await registration.unregister();
        const names = await caches.keys();
        await Promise.all(
          names.filter((name) => name.startsWith("baylee-site-") || name.startsWith("baylee-public-")).map((name) => caches.delete(name)),
        );
      })
      .catch(() => {});
  }
}

function unsent() {
  return [...document.querySelectorAll("textarea, input")].some(
    (field) => !["hidden", "checkbox", "radio", "submit", "button"].includes(field.type) && field.value.trim() !== "",
  );
}

function watch(registration) {
  let accepted = false;
  const offer = (worker) => {
    const box = document.getElementById("update");
    const said = document.getElementById("update-said");
    if (!box || !said) return;
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
  root.addEventListener("controllerchange", () => {
    if (accepted) {
      accepted = false;
      window.location.reload();
    }
  });
  if (registration.waiting && root.controller) offer(registration.waiting);
  registration.addEventListener("updatefound", () => {
    const worker = registration.installing;
    worker?.addEventListener("statechange", () => {
      if (worker.state === "installed" && root.controller) offer(worker);
    });
  });
}

// The install offer, where the browser makes one.
let deferred = null;
const boxes = () => document.querySelectorAll("[data-install-box]");
window.addEventListener("beforeinstallprompt", (event) => {
  event.preventDefault();
  deferred = event;
  for (const box of boxes()) box.hidden = false;
});
window.addEventListener("appinstalled", () => {
  deferred = null;
  for (const box of boxes()) box.hidden = true;
});
document.addEventListener("click", (event) => {
  const button = event.target instanceof Element ? event.target.closest("[data-install]") : null;
  if (!button || !deferred) return;
  const asked = deferred;
  deferred = null;
  asked.prompt();
  asked.userChoice.finally(() => {
    for (const box of boxes()) box.hidden = true;
  });
});
