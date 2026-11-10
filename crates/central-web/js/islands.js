// The islands' bootstrap: loads the WebAssembly (`cargo xtask site` writes
// the hashed addresses of the glue and the module into this file), prepares
// the reactive system, then hydrates every <leptos-island> in document
// order. What Leptos' inline hydration script does, as a file of its own,
// so the pages need no inline script and no nonce.
//
// It starts once the page has loaded: the WebAssembly then never competes
// with the text, the fonts and the pictures the visitor sees first, and the
// server-rendered page is whole until the islands take it over.
if (document.readyState !== "complete") {
  await new Promise((resolve) => window.addEventListener("load", resolve, { once: true }));
}
const islands = await import("./islands-glue.js");

await islands.default({ module_or_path: "./islands.wasm" });
islands.hydrate();

async function hydrate(element) {
  const run = islands[element.dataset.component ?? ""];
  if (typeof run !== "function") return;
  const done = run(element);
  if (done && typeof done.then === "function") await done;
}

async function walk(node) {
  for (const child of node.children) {
    if (child.localName === "leptos-island") await hydrate(child);
    await walk(child);
  }
}

await walk(document.body);
document.documentElement.dataset.islands = "ready";
