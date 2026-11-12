// baylee.app's only blocking script, loaded in <head> so it runs before the
// first paint (WebAssembly cannot): it applies the colour scheme the
// visitor chose (localStorage "baylee-site-theme", as before), shows the
// colour switch, which turns the scheme shown into the other one, and lets
// a new scheme spread from it, names the account link while a session is remembered
// ("baylee-central-session", which holds only when it ends), opens
// dialogs in browsers without invoker commands, closes the dropdowns as a
// menu closes, copies a gateway's address, pauses the hero's light off
// screen, and marks what is on screen as the page opens. Nothing here is
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

  // The scheme on screen: the one chosen, else the device's as the
  // stylesheet reads it (light only where the device asks for light).
  // One press always shows the other one. A choice the device would make
  // anyway is not kept, so the page follows the device again; before, the
  // switch went from the device's scheme to light and dark, and on a light
  // device its first press changed nothing.
  const light = matchMedia("(prefers-color-scheme: light)");
  const device = () => (light.matches ? "light" : "dark");
  const shown = () => root.dataset.theme ?? device();
  const label = (button) => {
    const name = shown();
    button.dataset.state = name;
    button.setAttribute("aria-label", button.dataset[name] ?? name);
  };
  const relabel = () => {
    for (const button of document.querySelectorAll("[data-theme-switch]")) label(button);
  };
  // the device changing while nothing is chosen changes what is shown
  light.addEventListener("change", relabel);
  const turn = (event) => {
    const next = shown() === "dark" ? "light" : "dark";
    const follow = next === device();
    try {
      if (follow) localStorage.removeItem(KEY);
      else localStorage.setItem(KEY, next);
    } catch {
      // Private windows may refuse storage; the choice then lasts this page.
    }
    const pressed = event.currentTarget;
    const apply = () => {
      if (follow) delete root.dataset.theme;
      else root.dataset.theme = next;
      relabel();
    };
    // The new colours spread from the switch where the browser can show
    // one state giving way to another (a view transition), and where
    // nobody asked for less motion; anywhere else they are simply there.
    if (typeof document.startViewTransition === "function" && !matchMedia("(prefers-reduced-motion: reduce)").matches) {
      const box = pressed.getBoundingClientRect();
      root.style.setProperty("--switch-x", `${box.left + box.width / 2}px`);
      root.style.setProperty("--switch-y", `${box.top + box.height / 2}px`);
      root.dataset.switching = "";
      document.startViewTransition(apply).finished.finally(() => {
        delete root.dataset.switching;
      });
    } else {
      apply();
    }
    pressed.focus();
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
  // A browser that cannot list its databases is told to delete it all the
  // same: deleting what is not there does nothing.
  if (!signedIn()) {
    try {
      if (typeof indexedDB.databases === "function") {
        indexedDB
          .databases()
          .then((all) => {
            if (all.some((db) => db.name === "baylee-chat")) indexedDB.deleteDatabase("baylee-chat");
          })
          .catch(() => {});
      } else {
        indexedDB.deleteDatabase("baylee-chat");
      }
    } catch {
      // no storage here, nothing kept
    }
  }

  const ready = () => {
    for (const button of document.querySelectorAll("[data-theme-switch]")) {
      button.hidden = false;
      label(button);
      button.addEventListener("click", turn);
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
    // What stands on screen as the page opens is there at once: rising and
    // brightening on the way into view (home.css) is for what comes after.
    // A card that started half in view would otherwise wait half faded.
    const fold = window.innerHeight + 80;
    for (const part of document.querySelectorAll(".reveal, .band > .wrap > .eyebrow, .band > .wrap > h2, .share-bar")) {
      if (part.getBoundingClientRect().top < fold) part.dataset.seen = "";
    }
  };
  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", ready, { once: true });
  else ready();

  // The dropdowns (the language menu, the download's other systems) are
  // <details data-menu>: they open without script; with it, one at a time,
  // and they close on a press outside, on Escape (the keyboard back on
  // their button) and when the keyboard leaves them. A menu leaves the way
  // it came: `data-closing` plays its way out, then `open` falls.
  const calm = matchMedia("(prefers-reduced-motion: reduce)");
  const menus = () => document.querySelectorAll("details[data-menu][open]");
  const gone = (menu) => {
    if (!("closing" in menu.dataset)) return;
    delete menu.dataset.closing;
    menu.open = false;
  };
  const close = (menu) => {
    if (!menu.open || "closing" in menu.dataset) return;
    if (calm.matches || !menu.querySelector(":scope > .menu")) {
      menu.open = false;
      return;
    }
    menu.dataset.closing = "";
    setTimeout(gone, 240, menu);
  };
  document.addEventListener("animationend", (event) => {
    const menu = event.target instanceof Element && event.target.matches(".menu") ? event.target.parentElement : null;
    if (menu instanceof HTMLDetailsElement) gone(menu);
  });
  document.addEventListener(
    "toggle",
    (event) => {
      const opened = event.target;
      if (!(opened instanceof HTMLDetailsElement) || !opened.open || !opened.matches("[data-menu]")) return;
      for (const other of menus()) if (other !== opened) close(other);
    },
    true,
  );
  document.addEventListener("click", (event) => {
    const summary = event.target instanceof Element ? event.target.closest("details[data-menu] > summary") : null;
    const menu = summary?.parentElement;
    if (!(menu instanceof HTMLDetailsElement) || !menu.open) return;
    // its own button closes an open menu; a second press while it leaves does nothing
    event.preventDefault();
    close(menu);
  });
  document.addEventListener("pointerdown", (event) => {
    for (const menu of menus()) if (!(event.target instanceof Node && menu.contains(event.target))) close(menu);
  });
  document.addEventListener("keydown", (event) => {
    if (event.key !== "Escape") return;
    for (const menu of menus()) {
      menu.querySelector("summary")?.focus();
      close(menu);
    }
  });
  document.addEventListener("focusout", (event) => {
    const menu = event.target instanceof Element ? event.target.closest("details[data-menu][open]") : null;
    if (menu && !(event.relatedTarget instanceof Node && menu.contains(event.relatedTarget))) close(menu);
  });

  // Signing in and registering open over the page that asked for them:
  // every link to either shows its dialog where the page has one. A press
  // with a modifier key, a browser without <dialog>, or a page without
  // them (the two pages themselves) follows the link as ever. The dialog
  // closes by its button, by Escape, and by a press on the veil around it
  // that also began there (a text selection dragged out of a field is not
  // one).
  let pressed = null;
  document.addEventListener("pointerdown", (event) => {
    pressed = event.target;
  });
  document.addEventListener("click", (event) => {
    const target = event.target instanceof Element ? event.target : null;
    if (!target || event.defaultPrevented) return;
    const shut = target.closest("[data-auth-close]");
    if (shut) {
      shut.closest("dialog")?.close();
      return;
    }
    if (target instanceof HTMLDialogElement && target.matches(".auth-dialog")) {
      if (pressed === target) target.close();
      return;
    }
    if (event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
    const link = target.closest("a[href]");
    if (!(link instanceof HTMLAnchorElement) || link.origin !== location.origin || link.target === "_blank") return;
    const which = /^\/[A-Za-z-]+\/(signin|register)\/?$/.exec(link.pathname)?.[1];
    const dialog = which ? document.getElementById(`auth-${which}`) : null;
    if (!(dialog instanceof HTMLDialogElement) || typeof dialog.showModal !== "function") return;
    event.preventDefault();
    for (const other of document.querySelectorAll("dialog.auth-dialog[open]")) other.close();
    const nav = document.getElementById("nav");
    if (nav?.matches(":popover-open")) nav.hidePopover();
    dialog.showModal();
  });

  // An island's form works only once its island runs. Until then the
  // browser's own submit would send its fields to the page's address, and
  // the sign-in's password must never leave the crypto Worker, so it is
  // held back. The islands' fields carry no names either, so even a submit
  // let through sends nothing.
  document.addEventListener(
    "submit",
    (event) => {
      const island = event.target instanceof Element ? event.target.closest("leptos-island") : null;
      if (island && !("hydrated" in island.dataset)) event.preventDefault();
    },
    true,
  );
})();
