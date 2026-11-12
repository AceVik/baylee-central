// The gateway directory's finding (the gateways page), over the cards the
// server wrote: a name or an address narrows them, a chip shows only those
// online or with open tables, the menu sorts them, and the two
// round buttons lay them out as cards or as a list. Escape empties the
// field, Ctrl+K or ⌘K goes to it. What is chosen stands in the address
// (?q= &show= &sort= &view=), so a view can be kept or passed on; nothing
// is stored. Plain DOM, no markup strings. Without this script every
// gateway shows, in the server's order.

const bar = document.querySelector("[data-find-bar]");
const list = document.querySelector("[data-directory]");
const field = bar?.querySelector("input");
const menu = bar?.querySelector("[data-sort]");

if (bar && list && field instanceof HTMLInputElement && menu instanceof HTMLSelectElement) {
  const cards = [...list.querySelectorAll("li[data-find]")];
  const chips = [...bar.querySelectorAll("[data-show]")];
  const views = [...bar.querySelectorAll("[data-view]")];
  const count = document.querySelector("[data-dir-count]");
  const none = document.querySelector("[data-no-match]");
  const every = count?.textContent ?? "";
  const template = count?.getAttribute("data-shown") ?? "";
  const SHOWS = new Set(chips.map((chip) => chip.getAttribute("data-show") ?? ""));
  const SORTS = new Set([...menu.options].map((option) => option.value));

  let show = "";
  let view = "";

  const number = (card, name) => Number(card.getAttribute(name) ?? -1);
  // a version's numbers, one after the other: 0.2.0-beta.o1 → [0, 2, 0, 1]
  const version = (card) => (card.getAttribute("data-version") ?? "").match(/\d+/g)?.map(Number) ?? [];
  const newer = (a, b) => {
    const [one, other] = [version(a), version(b)];
    for (let at = 0; at < Math.max(one.length, other.length); at += 1) {
      const step = (other[at] ?? 0) - (one[at] ?? 0);
      if (step !== 0) return step;
    }
    return 0;
  };
  const names = new Intl.Collator(document.documentElement.lang || undefined, { sensitivity: "base", numeric: true });
  const ORDER = {
    name: (a, b) => names.compare(a.getAttribute("data-name") ?? "", b.getAttribute("data-name") ?? ""),
    players: (a, b) => number(b, "data-players") - number(a, "data-players"),
    tables: (a, b) => number(b, "data-tables") - number(a, "data-tables"),
    version: newer,
  };
  const shows = (card) =>
    show === "" ||
    (show === "online" && card.getAttribute("data-online") === "true") ||
    (show === "tables" && number(card, "data-tables") > 0);

  const apply = () => {
    const words = field.value.trim().toLowerCase();
    let shown = 0;
    for (const card of cards) {
      const visible = (words === "" || (card.getAttribute("data-find") ?? "").includes(words)) && shows(card);
      card.hidden = !visible;
      if (visible) shown += 1;
    }
    // the server's order is the cards' own; a sort is stable upon it
    const order = ORDER[menu.value];
    list.append(...(order ? [...cards].sort(order) : cards));
    for (const chip of chips) chip.setAttribute("aria-pressed", String((chip.getAttribute("data-show") ?? "") === show));
    for (const button of views) button.setAttribute("aria-pressed", String((button.getAttribute("data-view") ?? "") === view));
    if (view === "") list.removeAttribute("data-view");
    else list.setAttribute("data-view", view);
    if (count) {
      count.textContent =
        words === "" && show === ""
          ? every
          : template.replace("{n}", String(shown)).replace("{all}", String(cards.length));
    }
    if (none) none.hidden = shown > 0;
    list.hidden = shown === 0;
    // the address says what is shown
    const query = new URLSearchParams();
    if (words !== "") query.set("q", field.value.trim());
    if (show !== "") query.set("show", show);
    if (menu.value !== "") query.set("sort", menu.value);
    if (view !== "") query.set("view", view);
    const text = query.toString();
    history.replaceState(history.state, "", `${location.pathname}${text === "" ? "" : `?${text}`}${location.hash}`);
  };
  // A press on a chip, the sort or the layout: the cards glide to their new
  // places where the browser can show one state giving way to another (a
  // view transition) and nobody asked for less motion. Typing does not wait
  // for that: each letter narrows at once.
  const calm = matchMedia("(prefers-reduced-motion: reduce)");
  let gliding = 0;
  const glide = () => {
    if (typeof document.startViewTransition !== "function" || calm.matches) {
      apply();
      return;
    }
    const mine = (gliding += 1);
    document.documentElement.dataset.gliding = "";
    cards.forEach((card, at) => {
      card.style.viewTransitionName = `gw-${at}`;
      card.style.viewTransitionClass = "gw-card";
    });
    document.startViewTransition(apply).finished.finally(() => {
      // a later press has its own names on the cards
      if (mine !== gliding) return;
      delete document.documentElement.dataset.gliding;
      for (const card of cards) {
        card.style.viewTransitionName = "";
        card.style.viewTransitionClass = "";
      }
    });
  };
  const reset = () => {
    field.value = "";
    show = "";
    apply();
    field.focus();
  };

  // what the address asked for, where it is something this page offers
  const asked = new URLSearchParams(location.search);
  field.value = (asked.get("q") ?? "").slice(0, 80);
  if (SHOWS.has(asked.get("show") ?? "")) show = asked.get("show") ?? "";
  if (SORTS.has(asked.get("sort") ?? "")) menu.value = asked.get("sort") ?? "";
  if (asked.get("view") === "list") view = "list";

  // ⌘K where the keyboard has that key
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
      // a pressed chip pressed again shows all
      show = asks === show ? "" : asks;
      glide();
    });
  }
  for (const button of views) {
    button.addEventListener("click", () => {
      view = button.getAttribute("data-view") ?? "";
      glide();
    });
  }
  none?.querySelector("button")?.addEventListener("click", reset);
  document.addEventListener("keydown", (event) => {
    if ((event.ctrlKey || event.metaKey) && !event.altKey && event.key.toLowerCase() === "k") {
      event.preventDefault();
      field.focus();
      field.select();
    }
  });
}
