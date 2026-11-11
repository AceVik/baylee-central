// Where a link leads, in the window's lower left corner, while the pointer
// or the keyboard is on it (owner, 11.11.2026). A browser's own status line
// is missing in the installed app, in some browsers by default, and for the
// keyboard everywhere. The address is written as text; a link that opens a
// tab of its own says so with its arrow. Purely visual: a screen reader
// already has the link's name and where it opens. Nothing here is required.

const hint = document.createElement("div");
hint.className = "link-hint";
hint.setAttribute("aria-hidden", "true");
const text = document.createElement("span");
const arrow = document.createElement("span");
arrow.className = "link-hint-out";
arrow.textContent = "↗";
hint.append(text, arrow);
const layered = typeof hint.showPopover === "function";
if (layered) hint.popover = "manual";
let shown = null;

function place(link) {
  let url;
  try {
    url = new URL(link.href, location.href);
  } catch {
    return false;
  }
  if (url.protocol !== "https:" && url.protocol !== "http:") return false;
  const path = url.pathname === "/" && !url.hash ? "" : url.pathname + url.hash;
  text.textContent = url.host + path;
  arrow.hidden = link.target !== "_blank";
  return true;
}

function show(link) {
  if (shown === link || !place(link)) return;
  shown = link;
  if (!hint.isConnected) document.body.append(hint);
  // the top layer, so it stands over an open dialog too; shown again, it
  // is the topmost
  if (layered) {
    if (hint.matches(":popover-open")) hint.hidePopover();
    hint.showPopover();
  }
  hint.classList.add("on");
}

function hide() {
  if (shown === null) return;
  shown = null;
  hint.classList.remove("on");
  if (layered && hint.matches(":popover-open")) hint.hidePopover();
}

const linkOf = (target) => (target instanceof Element ? target.closest("a[href]") : null);

document.addEventListener("pointerover", (event) => {
  if (event.pointerType !== "mouse") return;
  const link = linkOf(event.target);
  if (link) show(link);
  else hide();
});
document.addEventListener("pointerdown", hide);
document.documentElement.addEventListener("pointerleave", hide);
document.addEventListener("focusin", (event) => {
  const link = linkOf(event.target);
  if (link && link.matches(":focus-visible")) show(link);
  else hide();
});
document.addEventListener("focusout", hide);
addEventListener("pagehide", hide);
