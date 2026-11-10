// baylee.app's only blocking script, loaded in <head> so it runs before the
// first paint (WebAssembly cannot): it applies the colour scheme the
// visitor chose (localStorage "baylee-site-theme", as before), shows the
// colour switch, names the account link while a session is remembered
// ("baylee-central-session", which holds only when it ends), and opens
// dialogs in browsers without invoker commands. Nothing here is required:
// without it the device's scheme decides and every page still works.
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
})();
