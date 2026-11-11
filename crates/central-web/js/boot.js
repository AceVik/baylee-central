// baylee.app's only blocking script, loaded in <head> so it runs before the
// first paint (WebAssembly cannot): it applies the colour scheme the
// visitor chose (localStorage "baylee-site-theme", as before), shows the
// colour switch, names the account link while a session is remembered
// ("baylee-central-session", which holds only when it ends), opens
// dialogs in browsers without invoker commands, closes the dropdowns as a
// menu closes, copies a gateway's address, and pauses the hero's light
// off screen. Nothing here is
// required: without it the device's scheme decides and every page still
// works.
(() => {
  "use strict";
  const root = document.documentElement;
  const KEY = "baylee-site-theme";
  const read = (key) => {
    try {
      return localStorage.getItem(key);
    } catch {
      return null;
    }
  };
  const stored = read(KEY);
  if (stored === "light" || stored === "dark") root.dataset.theme = stored;

  const ORDER = ["system", "light", "dark"];
  const current = () => root.dataset.theme ?? "system";
  const label = (button) => {
    const name = current();
    button.dataset.state = name;
    button.setAttribute("aria-label", button.dataset[name] ?? name);
  };
  const cycle = (event) => {
    const next = ORDER[(ORDER.indexOf(current()) + 1) % ORDER.length];
    try {
      if (next === "system") localStorage.removeItem(KEY);
      else localStorage.setItem(KEY, next);
    } catch {
      // Private windows may refuse storage; the choice then lasts this page.
    }
    if (next === "system") delete root.dataset.theme;
    else root.dataset.theme = next;
    for (const button of document.querySelectorAll("[data-theme-switch]")) label(button);
    event.currentTarget.focus();
  };

  const signedIn = () => {
    try {
      const session = JSON.parse(read("baylee-central-session") ?? "null");
      return typeof session?.expiresAt === "number" && session.expiresAt * 1000 > Date.now();
    } catch {
      return false;
    }
  };

  // The chat identity kept in this browser lives only as long as the
  // sign-in (docs/central-chat.md): with no live session, it goes.
  if (!signedIn() && typeof indexedDB?.databases === "function") {
    indexedDB
      .databases()
      .then((all) => {
        if (all.some((db) => db.name === "baylee-chat")) indexedDB.deleteDatabase("baylee-chat");
      })
      .catch(() => {});
  }

  const ready = () => {
    for (const button of document.querySelectorAll("[data-theme-switch]")) {
      button.hidden = false;
      label(button);
      button.addEventListener("click", cycle);
    }
    if (signedIn()) {
      for (const link of document.querySelectorAll("a[data-account-href]")) {
        link.href = link.dataset.accountHref;
        const text = link.querySelector("span");
        if (text) text.textContent = link.dataset.account;
      }
      for (const line of document.querySelectorAll("[data-signed-out]")) line.hidden = true;
    }
    // The hero's slow light moves only while the hero is on screen.
    const hero = document.querySelector(".hero");
    if (hero && typeof IntersectionObserver === "function") {
      new IntersectionObserver(([entry]) => {
        if (entry?.isIntersecting) delete hero.dataset.away;
        else hero.dataset.away = "";
      }).observe(hero);
    }
    // Copying an address (a gateway's card), only where the browser can:
    // the button shows a check for a moment, and the live line says it.
    if (window.isSecureContext && typeof navigator.clipboard?.writeText === "function") {
      let live = null;
      const say = (words) => {
        if (!live) {
          live = document.createElement("p");
          live.className = "visually-hidden";
          live.setAttribute("role", "status");
          document.body.append(live);
        }
        // the same words twice in a row would not be read again
        live.textContent = live.textContent === words ? `${words}\u00a0` : (words ?? "");
      };
      for (const button of document.querySelectorAll("button[data-copy]")) {
        let back = 0;
        const label = button.querySelector("span");
        const words = label?.textContent ?? "";
        button.hidden = false;
        button.addEventListener("click", () => {
          navigator.clipboard.writeText(button.dataset.copy ?? "").then(
            () => {
              button.dataset.copied = "";
              if (label) label.textContent = button.dataset.copiedLabel ?? words;
              say(button.dataset.copiedSaid);
              clearTimeout(back);
              back = setTimeout(() => {
                delete button.dataset.copied;
                if (label) label.textContent = words;
              }, 1600);
            },
            () => say(button.dataset.copyFailed),
          );
        });
      }
    }
    // Invoker commands (<button commandfor command>) where the browser has none.
    if (!("command" in HTMLButtonElement.prototype)) {
      document.addEventListener("click", (event) => {
        const button = event.target instanceof Element ? event.target.closest("button[commandfor]") : null;
        const dialog = button ? document.getElementById(button.getAttribute("commandfor") ?? "") : null;
        if (!(dialog instanceof HTMLDialogElement)) return;
        const command = button.getAttribute("command");
        if (command === "show-modal" && !dialog.open) dialog.showModal();
        else if (command === "close") dialog.close();
      });
    }
  };
  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", ready, { once: true });
  else ready();

  // The dropdowns (the language menu, the download's other systems) are
  // <details data-menu>: they open without script; with it, one at a time,
  // and they close on a press outside, on Escape (the keyboard back on
  // their button) and when the keyboard leaves them.
  const menus = () => document.querySelectorAll("details[data-menu][open]");
  document.addEventListener(
    "toggle",
    (event) => {
      const opened = event.target;
      if (!(opened instanceof HTMLDetailsElement) || !opened.open || !opened.matches("[data-menu]")) return;
      for (const other of menus()) if (other !== opened) other.open = false;
    },
    true,
  );
  document.addEventListener("pointerdown", (event) => {
    for (const menu of menus()) if (!(event.target instanceof Node && menu.contains(event.target))) menu.open = false;
  });
  document.addEventListener("keydown", (event) => {
    if (event.key !== "Escape") return;
    for (const menu of menus()) {
      menu.open = false;
      menu.querySelector("summary")?.focus();
    }
  });
  document.addEventListener("focusout", (event) => {
    const menu = event.target instanceof Element ? event.target.closest("details[data-menu][open]") : null;
    if (menu && !(event.relatedTarget instanceof Node && menu.contains(event.relatedTarget))) menu.open = false;
  });

  // An island's form works only once the islands run. Until then the
  // browser's own submit would send its fields to the page's address, and
  // the sign-in's password must never leave the crypto Worker, so it is
  // held back. The islands' fields carry no names either, so even a submit
  // let through sends nothing.
  document.addEventListener(
    "submit",
    (event) => {
      const form = event.target instanceof Element ? event.target : null;
      if (root.dataset.islands !== "ready" && form?.closest("leptos-island")) event.preventDefault();
    },
    true,
  );
})();
