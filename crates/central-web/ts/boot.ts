// baylee.app's only blocking script, loaded in <head> so it runs before the
// first paint (WebAssembly cannot): it applies the colour scheme the
// visitor chose (localStorage "baylee-site-theme", as before), shows the
// colour switch, which turns the scheme shown into the other one, and lets
// a new scheme spread from it, names the account link while a session is remembered
// ("baylee-central-session", which holds only when it ends): with the
// account's own picture, display name and tag where a page noted them
// ("baylee-central-own"), opens dialogs in browsers without invoker commands,
// closes the dropdowns as a menu closes, copies a gateway's address, pauses
// the hero's light off screen, and marks what is on screen as the page opens.
// Nothing here is required: without it the device's scheme decides and every
// page still works.
interface StoredOwnRank {
  readonly tier?: unknown;
  readonly div?: unknown;
  readonly podium?: unknown;
}

interface StoredOwnNote {
  readonly of?: unknown;
  readonly key?: unknown;
  readonly moving?: unknown;
  readonly name?: unknown;
  readonly tag?: unknown;
  readonly rank?: StoredOwnRank | null;
}

(() => {
  "use strict";
  const root = document.documentElement;
  const KEY = "baylee-site-theme";
  const read = (key: string): string | null => {
    try {
      return localStorage.getItem(key);
    } catch {
      return null;
    }
  };
  const stored = read(KEY);
  if (stored === "light" || stored === "dark") root.dataset.theme = stored;
  const NAV_KEY = "baylee-nav-collapsed";
  if (read(NAV_KEY) === "1") root.dataset.navCollapsed = "";

  const syncNavToggle = (): void => {
    const collapsed = "navCollapsed" in root.dataset;
    for (const button of document.querySelectorAll<HTMLElement>("[data-nav-toggle]")) {
      const text = collapsed ? button.dataset.expand : button.dataset.collapse;
      button.setAttribute("aria-expanded", collapsed ? "false" : "true");
      if (text) {
        button.setAttribute("aria-label", text);
        button.setAttribute("title", text);
      }
    }
  };

  const light = matchMedia("(prefers-color-scheme: light)");
  const device = (): "light" | "dark" => (light.matches ? "light" : "dark");
  const shown = (): string => root.dataset.theme ?? device();
  const label = (button: HTMLElement): void => {
    const name = shown();
    button.dataset.state = name;
    button.setAttribute("aria-label", button.dataset[name] ?? name);
  };
  const relabel = (): void => {
    for (const button of document.querySelectorAll<HTMLElement>("[data-theme-switch]")) label(button);
  };
  light.addEventListener("change", relabel);
  const turn = (event: MouseEvent): void => {
    const next = shown() === "dark" ? "light" : "dark";
    const follow = next === device();
    try {
      if (follow) localStorage.removeItem(KEY);
      else localStorage.setItem(KEY, next);
    } catch {
      // Private windows may refuse storage; the choice then lasts this page.
    }
    const pressed = event.currentTarget instanceof HTMLElement ? event.currentTarget : null;
    const apply = (): void => {
      if (follow) delete root.dataset.theme;
      else root.dataset.theme = next;
      relabel();
    };
    if (pressed && typeof document.startViewTransition === "function" && !matchMedia("(prefers-reduced-motion: reduce)").matches) {
      const box = pressed.getBoundingClientRect();
      root.style.setProperty("--switch-x", `${box.left + box.width / 2}px`);
      root.style.setProperty("--switch-y", `${box.top + box.height / 2}px`);
      root.dataset.switching = "";
      const vt = document.startViewTransition(apply);
      vt.ready.catch(() => {});
      vt.finished
        .catch(() => {})
        .finally(() => {
          delete root.dataset.switching;
        });
    } else {
      apply();
    }
    pressed?.focus();
  };

  const REFRESH_GRACE_MS = (3 * 86400 - 900) * 1000;
  const signedIn = (): boolean => {
    try {
      const session = JSON.parse(read("baylee-central-session") ?? "null") as { expiresAt?: unknown } | null;
      return typeof session?.expiresAt === "number" && session.expiresAt * 1000 + REFRESH_GRACE_MS > Date.now();
    } catch {
      return false;
    }
  };

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
      localStorage.removeItem("baylee-central-own");
      sessionStorage.removeItem("baylee-central-own-picture");
    } catch {
      // no storage here, nothing kept
    }
  }

  const own = (): StoredOwnNote | null => {
    try {
      const held = JSON.parse(read("baylee-central-own") ?? "null") as StoredOwnNote | null;
      const session = read("baylee-central-session");
      return held && typeof held === "object" && session !== null && held.of === session ? held : null;
    } catch {
      return null;
    }
  };
  const ownFace = (link: HTMLAnchorElement, held: StoredOwnNote | null): void => {
    const face = document.createElement("span");
    face.className = "nav-face";
    face.setAttribute("aria-hidden", "true");
    if (typeof held?.key === "string" && /^[0-9a-f]{32}$/.test(held.key)) {
      const base = `/api/v1/pictures/${held.key}`;
      const picture = document.createElement("picture");
      if (held.moving) {
        const moving = document.createElement("source");
        moving.type = "image/avif";
        moving.media = "(prefers-reduced-motion: no-preference)";
        moving.srcset = `${base}.sequence.avif`;
        picture.append(moving);
      }
      const still = document.createElement("source");
      still.type = "image/avif";
      still.srcset = `${base}.avif`;
      const image = document.createElement("img");
      image.src = `${base}.png`;
      image.alt = "";
      image.width = 24;
      image.height = 24;
      image.decoding = "async";
      picture.append(still, image);
      face.append(picture);
    } else {
      face.dataset.none = "";
    }
    if (held?.rank && typeof held.rank.tier === "number") {
      const tier = Math.max(0, Math.min(11, Number(held.rank.tier) || 0));
      const scaleCls =
        tier <= 1
          ? "rank-scale-p"
          : tier <= 4
            ? "rank-scale-i"
            : tier <= 7
              ? "rank-scale-g"
              : tier <= 10
                ? "rank-scale-m"
                : "rank-scale-u";
      face.classList.add("has-rank", scaleCls);
      const frame = document.createElement("canvas");
      frame.className = "rank-frame-overlay";
      frame.width = 48;
      frame.height = 48;
      frame.dataset.rankKind = "frame";
      frame.dataset.rankTier = String(tier);
      frame.dataset.rankDiv = String(Number(held.rank.div) || 0);
      frame.dataset.rankPodium = String(Number(held.rank.podium) || 0);
      frame.dataset.rankTrack = "0";
      face.append(frame);
    }
    link.prepend(face);
  };

  const account = (): void => {
    const live = signedIn();
    const held = live ? own() : null;
    for (const link of document.querySelectorAll<HTMLAnchorElement>("a[data-account-href]")) {
      link.querySelector(".nav-face")?.remove();
      if (!live) continue;
      link.href = link.dataset.accountHref ?? "";
      const text = link.querySelector<HTMLElement>(":scope > span");
      if (text) {
        const tagStr = typeof held?.tag === "string" ? held.tag : "";
        const named = typeof held?.name === "string" && held.name !== "" && /^[0-9a-f]{4,8}$/.test(tagStr);
        if (named && typeof held?.name === "string") {
          const whose = document.createElement("span");
          whose.className = "visually-hidden";
          whose.textContent = `${link.dataset.account ?? ""}: `;
          const name = document.createElement("bdi");
          name.className = "nav-name";
          name.textContent = held.name;
          const tag = document.createElement("span");
          tag.className = "nav-tag";
          tag.textContent = `#${tagStr}`;
          text.replaceChildren(whose, name, tag);
          text.translate = false;
        } else {
          text.textContent = link.dataset.account ?? "";
          text.translate = true;
        }
      }
      ownFace(link, held);
    }
    window.dispatchEvent(new Event("baylee:badges"));
  };
  window.addEventListener("baylee:own", account);

  const ready = (): void => {
    for (const button of document.querySelectorAll<HTMLElement>("[data-theme-switch]")) {
      button.hidden = false;
      label(button);
      button.addEventListener("click", turn);
    }
    account();
    syncNavToggle();
    if (signedIn()) {
      for (const line of document.querySelectorAll<HTMLElement>("[data-signed-out]")) line.hidden = true;
    }
    const hero = document.querySelector<HTMLElement>(".hero");
    if (hero && typeof IntersectionObserver === "function") {
      new IntersectionObserver(([entry]) => {
        if (entry?.isIntersecting) delete hero.dataset.away;
        else hero.dataset.away = "";
      }).observe(hero);
    }
    if (window.isSecureContext && typeof navigator.clipboard?.writeText === "function") {
      let live: HTMLParagraphElement | null = null;
      const say = (words?: string): void => {
        if (!live) {
          live = document.createElement("p");
          live.className = "visually-hidden";
          live.setAttribute("role", "status");
          document.body.append(live);
        }
        live.textContent = live.textContent === words ? `${words}\u00a0` : (words ?? "");
      };
      for (const button of document.querySelectorAll<HTMLButtonElement>("button[data-copy]")) {
        let back = 0;
        const copyLabel = button.querySelector("span");
        const words = copyLabel?.textContent ?? "";
        button.hidden = false;
        button.addEventListener("click", () => {
          navigator.clipboard.writeText(button.dataset.copy ?? "").then(
            () => {
              button.dataset.copied = "";
              if (copyLabel) copyLabel.textContent = button.dataset.copiedLabel ?? words;
              say(button.dataset.copiedSaid);
              clearTimeout(back);
              back = window.setTimeout(() => {
                delete button.dataset.copied;
                if (copyLabel) copyLabel.textContent = words;
              }, 1600);
            },
            () => say(button.dataset.copyFailed),
          );
        });
      }
    }
    const fold = window.innerHeight + 80;
    for (const part of document.querySelectorAll<HTMLElement>(".reveal, .band > .wrap > .eyebrow, .band > .wrap > h2, .share-bar")) {
      if (part.getBoundingClientRect().top < fold) part.dataset.seen = "";
    }
  };
  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", ready, { once: true });
  else ready();
  document.addEventListener("baylee:page", ready);

  if (!("command" in HTMLButtonElement.prototype)) {
    document.addEventListener("click", (event) => {
      const button = event.target instanceof Element ? event.target.closest("button[commandfor]") : null;
      const dialog = button ? document.getElementById(button.getAttribute("commandfor") ?? "") : null;
      if (!(dialog instanceof HTMLDialogElement)) return;
      const command = button?.getAttribute("command");
      if (command === "show-modal" && !dialog.open) dialog.showModal();
      else if (command === "close") dialog.close();
    });
  }

  const calm = matchMedia("(prefers-reduced-motion: reduce)");
  const menus = (): NodeListOf<HTMLDetailsElement> => document.querySelectorAll<HTMLDetailsElement>("details[data-menu][open]");
  const gone = (menu: HTMLDetailsElement): void => {
    if (!("closing" in menu.dataset)) return;
    delete menu.dataset.closing;
    menu.open = false;
  };
  const close = (menu: HTMLDetailsElement): void => {
    if (!menu.open || "closing" in menu.dataset) return;
    if (calm.matches || !menu.querySelector(":scope > .menu")) {
      menu.open = false;
      return;
    }
    menu.dataset.closing = "";
    window.setTimeout(gone, 240, menu);
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
    const menu = event.target instanceof Element ? event.target.closest<HTMLDetailsElement>("details[data-menu][open]") : null;
    if (menu && !(event.relatedTarget instanceof Node && menu.contains(event.relatedTarget))) close(menu);
  });

  const AUTH = /^\/([A-Za-z-]+)\/(signin|register|reset)\/?$/;
  const authDialog = (which?: string): HTMLDialogElement | null => {
    const dialog = which ? document.getElementById(`auth-${which}`) : null;
    return dialog instanceof HTMLDialogElement && typeof dialog.showModal === "function" ? dialog : null;
  };
  const shutAll = (): void => {
    for (const other of document.querySelectorAll<HTMLDialogElement>("dialog.auth-dialog[open]")) {
      other.dataset.changing = "";
      other.close();
    }
  };
  let beneath = document.title;
  const show = (dialog: HTMLDialogElement): void => {
    shutAll();
    const nav = document.getElementById("nav");
    if (nav?.matches(":popover-open")) nav.hidePopover();
    const title = dialog.querySelector("h2")?.textContent;
    if (title) document.title = `${title} – Baylee`;
    dialog.showModal();
  };
  document.addEventListener(
    "close",
    (event) => {
      const closed = event.target;
      if (!(closed instanceof HTMLDialogElement) || !closed.matches(".auth-dialog")) return;
      if ("changing" in closed.dataset) {
        delete closed.dataset.changing;
        return;
      }
      const at = AUTH.exec(location.pathname);
      if (!at) return;
      document.title = beneath;
      const state = history.state as { auth?: boolean } | null;
      if (state?.auth === true) history.back();
      else location.replace(`/${at[1]}/`);
    },
    true,
  );
  const match = (): void => {
    const dialog = authDialog(AUTH.exec(location.pathname)?.[2]);
    if (dialog?.open) return;
    if (dialog) {
      show(dialog);
    } else if (document.querySelector("dialog.auth-dialog[open]")) {
      shutAll();
      document.title = beneath;
    }
  };
  const scrollActiveNav = (): void => {
    for (const [itemSel, boxSel] of [
      ['.frame-nav [aria-current="page"]', ".frame-groups"],
      ['.admin-nav [aria-current="page"]', ".admin-nav ul"],
    ] as const) {
      const active = document.querySelector(itemSel);
      const box = active?.closest(boxSel);
      if (active && box && box.scrollWidth > box.clientWidth) {
        active.scrollIntoView({ block: "nearest", inline: "center" });
      }
    }
  };
  window.addEventListener("popstate", match);
  document.addEventListener("baylee:page", () => {
    beneath = document.title;
    match();
    scrollActiveNav();
    syncNavToggle();
  });
  const syncPage = (): void => {
    match();
    scrollActiveNav();
    syncNavToggle();
  };
  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", syncPage, { once: true });
  else syncPage();
  let pressed: EventTarget | null = null;
  document.addEventListener("pointerdown", (event) => {
    pressed = event.target;
  });
  document.addEventListener("click", (event) => {
    const target = event.target instanceof Element ? event.target : null;
    if (!target || event.defaultPrevented) return;
    const navToggle = target.closest("[data-nav-toggle]");
    if (navToggle) {
      const collapse = !("navCollapsed" in root.dataset);
      if (collapse) root.dataset.navCollapsed = "";
      else delete root.dataset.navCollapsed;
      try {
        if (collapse) localStorage.setItem(NAV_KEY, "1");
        else localStorage.removeItem(NAV_KEY);
      } catch {
        // Private windows may refuse storage; the choice then lasts this page.
      }
      syncNavToggle();
      return;
    }
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
    const dialog = authDialog(AUTH.exec(link.pathname)?.[2]);
    if (!dialog || link.search) return;
    event.preventDefault();
    const address = link.pathname + link.hash;
    const state = history.state as { page?: string } | null;
    if (AUTH.test(location.pathname)) history.replaceState(history.state, "", address);
    else history.pushState({ auth: true, page: state?.page ?? location.pathname + location.search }, "", address);
    show(dialog);
  });

  document.addEventListener(
    "submit",
    (event) => {
      const island = event.target instanceof Element ? event.target.closest<HTMLElement>("leptos-island") : null;
      if (island && !("hydrated" in island.dataset)) event.preventDefault();
    },
    true,
  );
})();
