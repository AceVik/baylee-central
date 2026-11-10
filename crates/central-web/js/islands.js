// The islands' bootstrap: loads the WebAssembly (`cargo xtask site` writes
// the hashed addresses of the glue and the module into this file), prepares
// the reactive system, then hydrates every <leptos-island> in document
// order. What Leptos' inline hydration script does, as a file of its own,
// so the pages need no inline script and no nonce.
import init, * as islands from "./islands-glue.js";

await init({ module_or_path: "./islands.wasm" });
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
