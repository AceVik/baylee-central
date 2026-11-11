// Where a link leads, in the window's lower left corner, while the pointer
// or the keyboard is on it (owner, 11.11.2026). A browser's own status line
// is missing in the installed app, in some browsers by default, and for the
// keyboard everywhere. The address is written as text; a link that opens a
// tab of its own says so with its arrow. Purely visual: a screen reader
// already has the link's name and where it opens. Nothing here is required.
// At the end, something for those who type Baylee's name.

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


// ---- for those who type her name ---------------------------------------------

// "baylee", typed anywhere but into a field, and a cat walks across the foot
// of the window: paw prints, left and right, fading behind her. Nothing is
// stored and nothing is sent. Where less motion was asked for, one print
// shows and goes. The print is Lucide's `paw-print` (ISC, NOTICE.md).
const SVG = "http://www.w3.org/2000/svg";
const PAD = "M9 10a5 5 0 0 1 5 5v3.5a3.5 3.5 0 0 1-6.84 1.045Q6.52 17.48 4.46 16.84A3.5 3.5 0 0 1 5.5 10Z";
const TOES = [
  [11, 4],
  [18, 8],
  [20, 16],
];

function print() {
  const paw = document.createElementNS(SVG, "svg");
  paw.setAttribute("viewBox", "0 0 24 24");
  paw.setAttribute("class", "paw");
  for (const [x, y] of TOES) {
    const toe = document.createElementNS(SVG, "circle");
    toe.setAttribute("cx", String(x));
    toe.setAttribute("cy", String(y));
    toe.setAttribute("r", "2");
    paw.append(toe);
  }
  const pad = document.createElementNS(SVG, "path");
  pad.setAttribute("d", PAD);
  paw.append(pad);
  return paw;
}

let walking = false;

function walk() {
  if (walking) return;
  walking = true;
  const calm = matchMedia("(prefers-reduced-motion: reduce)").matches;
  const steps = calm ? 1 : Math.max(6, Math.min(14, Math.round(innerWidth / 110)));
  const trail = document.createElement("div");
  trail.className = "paws";
  trail.setAttribute("aria-hidden", "true");
  for (let at = 0; at < steps; at += 1) {
    const paw = print();
    paw.style.left = `${((at + 0.5) / steps) * 100}%`;
    paw.style.animationDelay = `${at * 190}ms`;
    if (at % 2 === 1) paw.classList.add("other");
    trail.append(paw);
  }
  document.body.append(trail);
  // the top layer, so she walks over an open dialog too
  if (layered) {
    trail.popover = "manual";
    trail.showPopover();
  }
  setTimeout(
    () => {
      trail.remove();
      walking = false;
    },
    steps * 190 + 2600,
  );
}

let typed = "";
document.addEventListener("keydown", (event) => {
  if (event.ctrlKey || event.metaKey || event.altKey || event.key.length !== 1) return;
  const at = event.target;
  if (at instanceof HTMLElement && (at.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(at.tagName))) {
    typed = "";
    return;
  }
  typed = (typed + event.key.toLowerCase()).slice(-6);
  if (typed === "baylee") {
    typed = "";
    walk();
  }
});
