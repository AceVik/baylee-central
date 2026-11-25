// The islands' bootstrap: loads the WebAssembly (`cargo xtask site` writes
// the hashed addresses of the glue and the module into this file), prepares
// the reactive system, then hydrates every <leptos-island> in document
// order. What Leptos' inline hydration script does, as a file of its own,
// so the pages need no inline script and no nonce.

const root = document.documentElement;
if (document.readyState !== "complete") {
  await new Promise<void>((resolve) => window.addEventListener("load", () => resolve(), { once: true }));
}
await new Promise<void>((resolve) => {
  requestAnimationFrame(() => setTimeout(resolve, 0));
  setTimeout(resolve, 500);
});
// ... and once the page's largest paint is on screen, so the half
// megabyte never holds it back; a browser that does not report it (or a
// page that never paints one) waits a second at most.
await new Promise<void>((resolve) => {
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
interface LoadedIslands {
  readonly [component: string]: unknown;
}

let loading: Promise<void> | null = null;
let islands: LoadedIslands | null = null;
const load = (): Promise<void> =>
  (loading ??= (async () => {
    const glue = await import("./islands-glue.js");
    await glue.default({ module_or_path: "./islands.wasm" });
    glue.hydrate();
    islands = glue as unknown as LoadedIslands;
  })());

async function hydrate(element: HTMLElement): Promise<void> {
  if ("hydrated" in element.dataset) return;
  const run = islands?.[element.dataset.component ?? ""];
  if (typeof run !== "function") return;
  const done = (run as (el: HTMLElement) => unknown)(element);
  if (done !== null && typeof done === "object" && "then" in done && typeof (done as PromiseLike<unknown>).then === "function") {
    await (done as PromiseLike<unknown>);
  }
  // `boot.js` holds a form's submit back until its island says this
  element.dataset.hydrated = "";
}

// a closed dialog's islands wait for it to open
const waiting = new MutationObserver((changes) => {
  for (const change of changes) {
    if (change.target instanceof HTMLDialogElement && change.target.open) void walk(change.target);
  }
});

async function walk(node: Element): Promise<void> {
  for (const child of node.children) {
    if (child instanceof HTMLDialogElement && !child.open) {
      if (child.querySelector("leptos-island:not([data-hydrated])")) {
        waiting.observe(child, { attributes: true, attributeFilter: ["open"] });
      }
      continue;
    }
    if (child.localName === "leptos-island" && child instanceof HTMLElement) await hydrate(child);
    await walk(child);
  }
}

// what watches the page on screen for a reader coming near an island
let unwatch = (): void => {};

/** Resolves once the page's islands are wanted; never for a page that left first. */
function wanted(): Promise<void> {
  return new Promise<void>((resolve) => {
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
    const reach = (event: Event): void => {
      if (event.target instanceof Element && event.target.closest("leptos-island")) start();
    };
    function stop(): void {
      watch.disconnect();
      document.removeEventListener("focusin", reach, true);
      document.removeEventListener("pointerdown", reach, true);
    }
    function start(): void {
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
async function page(): Promise<void> {
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
  unwatch = (): void => {};
  waiting.disconnect();
});
document.addEventListener("baylee:page", () => void page());
await page();
