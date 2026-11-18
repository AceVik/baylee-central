// The islands' bootstrap: loads the WebAssembly (`cargo xtask site` writes
// the hashed addresses of the glue and the module into this file), prepares
// the reactive system, then hydrates every <leptos-island> in document
// order. What Leptos' inline hydration script does, as a file of its own,
// so the pages need no inline script and no nonce.
//
// It starts once the page has loaded and its first frame is drawn: the
// WebAssembly then never competes with the text, the fonts and the pictures
// the visitor sees first, and the server-rendered page is whole until the
// islands take it over. A hidden tab draws no frames; there the half second
// starts it, and after the largest paint. Islands far down the page (the start page's contact form)
// wait until the reader comes within a screen of one, or reaches into one
// with the keyboard or a pointer: a page read only at its top never
// fetches the half megabyte. An island without a box of its own cannot be
// watched, so it starts at once; one in a dialog that is not open (the
// sign-in and the registration every page can show over itself) has none
// until the dialog opens, and waits for that. Such an island is taken over
// when its dialog first opens, not before: its form counts its time from
// the moment a person can see it.
//
// A page put in place without a new document (`nav.js`) is watched the
// same way from the moment it stands; once the WebAssembly is here, its
// islands are taken over at once.
const root = document.documentElement;
if (document.readyState !== "complete") {
  await new Promise((resolve) => window.addEventListener("load", resolve, { once: true }));
}
await new Promise((resolve) => {
  requestAnimationFrame(() => setTimeout(resolve, 0));
  setTimeout(resolve, 500);
});
// ... and once the page's largest paint is on screen, so the half
// megabyte never holds it back; a browser that does not report it (or a
// page that never paints one) waits a second at most.
await new Promise((resolve) => {
  const types = typeof PerformanceObserver === "function" ? PerformanceObserver.supportedEntryTypes : undefined;
  if (!types?.includes("largest-contentful-paint")) {
    resolve();
    return;
  }
  const painted = new PerformanceObserver(() => {
    painted.disconnect();
    requestAnimationFrame(() => setTimeout(resolve, 0));
  });
  painted.observe({ type: "largest-contentful-paint", buffered: true });
  setTimeout(resolve, 1000);
});

// the WebAssembly, fetched once for the document
let loading = null;
let islands = null;
const load = () =>
  (loading ??= (async () => {
    const glue = await import("./islands-glue.js");
    await glue.default({ module_or_path: "./islands.wasm" });
    glue.hydrate();
    islands = glue;
  })());

async function hydrate(element) {
  if ("hydrated" in element.dataset) return;
  const run = islands?.[element.dataset.component ?? ""];
  if (typeof run !== "function") return;
  const done = run(element);
  if (done && typeof done.then === "function") await done;
  // `boot.js` holds a form's submit back until its island says this
  element.dataset.hydrated = "";
}

// a closed dialog's islands wait for it to open
const waiting = new MutationObserver((changes) => {
  for (const change of changes) {
    if (change.target instanceof HTMLDialogElement && change.target.open) void walk(change.target);
  }
});

async function walk(node) {
  for (const child of node.children) {
    if (child instanceof HTMLDialogElement && !child.open) {
      if (child.querySelector("leptos-island:not([data-hydrated])")) {
        waiting.observe(child, { attributes: true, attributeFilter: ["open"] });
      }
      continue;
    }
    if (child.localName === "leptos-island") await hydrate(child);
    await walk(child);
  }
}

// what watches the page on screen for a reader coming near an island
let unwatch = () => {};

/** Resolves once the page's islands are wanted; never for a page that left first. */
function wanted() {
  return new Promise((resolve) => {
    const all = [...document.querySelectorAll("leptos-island")];
    if (
      islands ||
      all.length === 0 ||
      typeof IntersectionObserver !== "function" ||
      all.some((island) => island.getClientRects().length === 0 && !island.closest("dialog:not([open])"))
    ) {
      resolve();
      return;
    }
    const watch = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) start();
      },
      { rootMargin: "100% 0px" },
    );
    const reach = (event) => {
      if (event.target instanceof Element && event.target.closest("leptos-island")) start();
    };
    function stop() {
      watch.disconnect();
      document.removeEventListener("focusin", reach, true);
      document.removeEventListener("pointerdown", reach, true);
    }
    function start() {
      stop();
      resolve();
    }
    unwatch = stop;
    document.addEventListener("focusin", reach, true);
    document.addEventListener("pointerdown", reach, true);
    for (const island of all) watch.observe(island);
  });
}

let pages = 0;
async function page() {
  const mine = ++pages;
  delete root.dataset.islands;
  await wanted();
  if (mine !== pages) return;
  await load();
  if (mine !== pages) return;
  await walk(document.body);
  if (mine === pages) root.dataset.islands = "ready";
}

document.addEventListener("baylee:leave", () => {
  pages += 1;
  unwatch();
  unwatch = () => {};
  waiting.disconnect();
});
document.addEventListener("baylee:page", () => void page());
await page();
