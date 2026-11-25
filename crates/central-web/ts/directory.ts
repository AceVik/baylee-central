// The gateway directory's finding (the gateways page), over the cards the
// server wrote: a name or an address narrows them, the chips show only those
// online, with open tables or on the current version (together: each chip
// pressed narrows further, "All" lets go of them), the menu or a column's
// head sorts them, and the two round buttons lay them out as cards or as a
// list. Escape empties the field, Ctrl+K or ⌘K goes to it. What is chosen
// stands in the address (?q= &show= &sort= &view=), so a view can be kept or
// passed on; nothing is stored. Plain DOM, no markup strings. Without this
// script every gateway shows, in the server's order.

function cardNumber(card: HTMLElement, name: string): number {
  return Number(card.getAttribute(name) ?? -1);
}

function cardVersion(card: HTMLElement): number[] {
  return (card.getAttribute("data-version") ?? "").match(/\d+/g)?.map(Number) ?? [];
}

function newerVersion(a: HTMLElement, b: HTMLElement): number {
  const [one, other] = [cardVersion(a), cardVersion(b)];
  for (let at = 0; at < Math.max(one.length, other.length); at += 1) {
    const step = (other[at] ?? 0) - (one[at] ?? 0);
    if (step !== 0) return step;
  }
  return 0;
}

function page(): void {
  const leaving = new AbortController();
  document.addEventListener("baylee:leave", () => leaving.abort(), { once: true });
  const bar = document.querySelector<HTMLElement>("[data-find-bar]");
  const list = document.querySelector<HTMLElement>("[data-directory]");
  const field = bar?.querySelector("input");
  const menu = bar?.querySelector("[data-sort]");

  if (bar && list && field instanceof HTMLInputElement && menu instanceof HTMLSelectElement) {
    const cards = [...list.querySelectorAll<HTMLElement>("li[data-find]")];
    const chips = [...bar.querySelectorAll<HTMLElement>("[data-show]")];
    const views = [...bar.querySelectorAll<HTMLElement>("[data-view]")];
    const columns = document.querySelector<HTMLElement>("[data-columns]");
    const heads = [...(columns?.querySelectorAll<HTMLElement>("[data-sort-by]") ?? [])];
    const count = bar.querySelector<HTMLElement>("[data-dir-count]");
    const none = document.querySelector<HTMLElement>("[data-no-match]");
    const template = count?.getAttribute("data-shown") ?? "";
    const SHOWS = chips.map((chip) => chip.getAttribute("data-show") ?? "").filter((name) => name !== "");
    const SORTS = new Set([...menu.options].map((option) => option.value));
    const figures = new Intl.NumberFormat(document.documentElement.lang || undefined);

    const show = new Set<string>();
    let view = "";

    const names = new Intl.Collator(document.documentElement.lang || undefined, { sensitivity: "base", numeric: true });
    const ORDER: Record<string, (a: HTMLElement, b: HTMLElement) => number> = {
      name: (a, b) => names.compare(a.getAttribute("data-name") ?? "", b.getAttribute("data-name") ?? ""),
      players: (a, b) => cardNumber(b, "data-players") - cardNumber(a, "data-players"),
      tables: (a, b) => cardNumber(b, "data-tables") - cardNumber(a, "data-tables"),
      version: newerVersion,
    };
    const IS: Record<string, (card: HTMLElement) => boolean> = {
      online: (card) => card.getAttribute("data-online") === "true",
      tables: (card) => cardNumber(card, "data-tables") > 0,
      current: (card) => card.getAttribute("data-current") === "true",
    };
    const shows = (card: HTMLElement): boolean => [...show].every((name) => IS[name]?.(card) ?? true);

    const apply = (): void => {
      const words = field.value.trim().toLowerCase();
      let shown = 0;
      for (const card of cards) {
        const visible = (words === "" || (card.getAttribute("data-find") ?? "").includes(words)) && shows(card);
        card.hidden = !visible;
        if (visible) shown += 1;
      }
      const order = ORDER[menu.value];
      list.append(...(order ? [...cards].sort(order) : cards));
      for (const chip of chips) {
        const name = chip.getAttribute("data-show") ?? "";
        chip.setAttribute("aria-pressed", String(name === "" ? show.size === 0 : show.has(name)));
      }
      for (const button of views) button.setAttribute("aria-pressed", String((button.getAttribute("data-view") ?? "") === view));
      for (const head of heads) head.setAttribute("aria-pressed", String(head.getAttribute("data-sort-by") === menu.value));
      if (view === "") list.removeAttribute("data-view");
      else list.setAttribute("data-view", view);
      if (columns) columns.hidden = view !== "list" || shown === 0;
      if (count) {
        count.textContent = template.replace("{n}", figures.format(shown)).replace("{all}", figures.format(cards.length));
      }
      if (none) none.hidden = shown > 0;
      list.hidden = shown === 0;
      const query = new URLSearchParams();
      if (words !== "") query.set("q", field.value.trim());
      if (show.size > 0) query.set("show", SHOWS.filter((name) => show.has(name)).join(" "));
      if (menu.value !== "") query.set("sort", menu.value);
      if (view !== "") query.set("view", view);
      const text = query.toString();
      history.replaceState(history.state, "", `${location.pathname}${text === "" ? "" : `?${text}`}${location.hash}`);
    };
    const calm = matchMedia("(prefers-reduced-motion: reduce)");
    let gliding = 0;
    const glide = (): void => {
      if (typeof document.startViewTransition !== "function" || calm.matches || cards.length > 48) {
        apply();
        return;
      }
      const mine = (gliding += 1);
      document.documentElement.dataset.gliding = "";
      cards.forEach((card, at) => {
        card.style.viewTransitionName = `gw-${at}`;
        card.style.viewTransitionClass = "gw-card";
      });
      const turn = document.startViewTransition(apply);
      turn.ready.catch(() => {});
      turn.finished.finally(() => {
        if (mine !== gliding) return;
        delete document.documentElement.dataset.gliding;
        for (const card of cards) {
          card.style.viewTransitionName = "";
          card.style.viewTransitionClass = "";
        }
      });
    };
    const reset = (): void => {
      field.value = "";
      show.clear();
      apply();
      field.focus();
    };

    const asked = new URLSearchParams(location.search);
    field.value = (asked.get("q") ?? "").slice(0, 80);
    for (const name of (asked.get("show") ?? "").split(/[\s,]+/)) {
      if (SHOWS.includes(name)) show.add(name);
    }
    if (SORTS.has(asked.get("sort") ?? "")) menu.value = asked.get("sort") ?? "";
    if (asked.get("view") === "list") view = "list";

    const key = bar.querySelector("[data-key]");
    if (key && /Mac|iPhone|iPad/.test(navigator.platform)) key.textContent = "⌘K";

    bar.hidden = false;
    apply();
    field.addEventListener("input", apply);
    field.addEventListener("keydown", (event) => {
      if (event.key === "Escape" && field.value !== "") {
        event.preventDefault();
        field.value = "";
        apply();
      }
    });
    menu.addEventListener("change", glide);
    for (const chip of chips) {
      chip.addEventListener("click", () => {
        const asks = chip.getAttribute("data-show") ?? "";
        if (asks === "") show.clear();
        else if (!show.delete(asks)) show.add(asks);
        glide();
      });
    }
    for (const button of views) {
      button.addEventListener("click", () => {
        view = button.getAttribute("data-view") ?? "";
        glide();
      });
    }
    for (const head of heads) {
      head.addEventListener("click", () => {
        const by = head.getAttribute("data-sort-by") ?? "";
        menu.value = menu.value === by || !SORTS.has(by) ? "" : by;
        glide();
      });
    }
    none?.querySelector("button")?.addEventListener("click", reset);
    document.addEventListener(
      "keydown",
      (event) => {
        if ((event.ctrlKey || event.metaKey) && !event.altKey && event.key.toLowerCase() === "k") {
          event.preventDefault();
          field.focus();
          field.select();
        }
      },
      { signal: leaving.signal },
    );
  }
}

page();
document.addEventListener("baylee:page", page);
