// The gateway directory's finding (the gateways page): a name or an
// address, and online only, narrow the cards the server wrote; Escape
// empties the field, Ctrl+K or ⌘K goes to it. Plain DOM, no markup
// strings. Without it every gateway shows.

const bar = document.querySelector("[data-find-bar]");
const list = document.querySelector("[data-directory]");
const field = bar?.querySelector("input");
const online = bar?.querySelector("[data-online-only]");

if (bar && list && field instanceof HTMLInputElement && online) {
  const cards = [...list.querySelectorAll("li[data-find]")];
  const count = document.querySelector("[data-dir-count]");
  const none = document.querySelector("[data-no-match]");
  const every = count?.textContent ?? "";
  const template = count?.getAttribute("data-shown") ?? "";

  const apply = () => {
    const words = field.value.trim().toLowerCase();
    const onlineOnly = online.getAttribute("aria-pressed") === "true";
    let shown = 0;
    for (const card of cards) {
      const show =
        (words === "" || (card.getAttribute("data-find") ?? "").includes(words)) &&
        (!onlineOnly || card.getAttribute("data-online") === "true");
      card.hidden = !show;
      if (show) shown += 1;
    }
    if (count) {
      count.textContent =
        words === "" && !onlineOnly
          ? every
          : template.replace("{n}", String(shown)).replace("{all}", String(cards.length));
    }
    if (none) none.hidden = shown > 0;
    list.hidden = shown === 0;
  };
  const reset = () => {
    field.value = "";
    online.setAttribute("aria-pressed", "false");
    apply();
    field.focus();
  };

  bar.hidden = false;
  field.addEventListener("input", apply);
  field.addEventListener("keydown", (event) => {
    if (event.key === "Escape" && field.value !== "") {
      event.preventDefault();
      field.value = "";
      apply();
    }
  });
  online.addEventListener("click", () => {
    online.setAttribute("aria-pressed", String(online.getAttribute("aria-pressed") !== "true"));
    apply();
  });
  none?.querySelector("button")?.addEventListener("click", reset);
  document.addEventListener("keydown", (event) => {
    if ((event.ctrlKey || event.metaKey) && !event.altKey && event.key.toLowerCase() === "k") {
      event.preventDefault();
      field.focus();
      field.select();
    }
  });
}
