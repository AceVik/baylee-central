// Going from page to page without loading the document anew: a link to
// another page of the site is fetched, and the page's own part of the
// document (its head's words, its body) takes the place of the one on
// screen. The style sheet, the fonts, the scripts and the WebAssembly stay
// as they are, so the next page is there as fast as its HTML. Every page is
// still a complete document at its own address: a first visit, a search
// engine, a browser without script or without what this needs, and every
// case below that is not plainly safe get that document from the server,
// as before.
//
// What is swapped is exactly what the server renders for that address (the
// same request a navigation makes, the same response). It is parsed into a
// document that runs nothing, and what is taken from it is put in place as
// nodes; a script in it does not run, the page's own scripts are loaded by
// their address as the server names them. The site's Content Security
// Policy holds for it as for any other part of the page.
//
// The document is loaded anew instead when
// - the link leaves the language, or enters or leaves the signed-in area
//   (its islands hold a session, Workers and connections that end with
//   their document);
// - an island on screen was taken over (its state would be dropped without
//   its being told);
// - the response is not a page of this build (another style sheet, another
//   islands module: a release went out in between), not HTML, or failed.
//
// The scripts of a page start over with each page: `baylee:leave` goes out
// before a page is taken away, `baylee:page` once the next one stands.
const root = document.documentElement;
const PAGE = /^\/([a-z]{2}(?:-[a-z]+)?)(?:\/(.*))?$/;
// What a page's head says of that page alone. Its structured data is for
// a reader of the document the server sends, and is a script element,
// which a page may not fill from a string: the one on screen goes with
// its page, and none is put in its place.
const OWN = [
  'meta[name="description"]',
  'meta[name="robots"]',
  'link[rel="canonical"]',
  'link[rel="alternate"]',
  'meta[property^="og:"]',
  'meta[name^="twitter:"]',
].join(",");
const DATA = 'script[type="application/ld+json"]';
const key = (url) => url.pathname + url.search;
const area = (path) => /^\/[a-z-]+\/account(\/|$)/.test(path);

// Trusted Types holds back two things this needs: a string for the parser,
// and a script's address. The policy vouches for the one use of the first,
// a response of this site parsed into a document that runs no script, and
// for no address but one of the site's own hashed scripts
// (`/assets/<name>.js`). A browser without Trusted Types has no such rule
// to satisfy, and the same check on the address.
const SCRIPT = /^\/assets\/[A-Za-z0-9._-]+\.js$/;
const ours = (address) => {
  if (!SCRIPT.test(address)) throw new TypeError("not a script of this site");
  return address;
};
let parse = null;
let vouch = ours;
try {
  const types = window.trustedTypes;
  if (types?.createPolicy) {
    const policy = types.createPolicy("baylee-nav", { createHTML: (text) => text, createScriptURL: ours });
    parse = (text) => new DOMParser().parseFromString(policy.createHTML(text), "text/html");
    vouch = (address) => policy.createScriptURL(address);
  } else {
    parse = (text) => new DOMParser().parseFromString(text, "text/html");
  }
} catch {
  // a policy the page may not make: every link loads its document
}

/** Whether a link leads to a page this script may put in place. */
function reach(link) {
  if (!(link instanceof HTMLAnchorElement) || link.origin !== location.origin) return null;
  if (link.hasAttribute("download") || (link.target !== "" && link.target !== "_self")) return null;
  const to = PAGE.exec(link.pathname);
  const from = PAGE.exec(location.pathname);
  if (!to || !from || to[1] !== from[1]) return null;
  if (area(link.pathname) || area(location.pathname)) return null;
  return new URL(link.href);
}

// ---- fetching, a moment ahead of the press ---------------------------------

const KEEP = 20_000;
const held = new Map();
function fetched(url) {
  const name = key(url);
  const have = held.get(name);
  if (have && Date.now() - have.at < KEEP) return have.page;
  const page = fetch(url.origin + name, { credentials: "same-origin", headers: { accept: "text/html" } })
    .then(async (response) => {
      const kind = response.headers.get("content-type") ?? "";
      if (!response.ok || !kind.startsWith("text/html")) return null;
      const landed = new URL(response.url);
      if (landed.origin !== location.origin) return null;
      return { doc: parse(await response.text()), url: landed };
    })
    .catch(() => null);
  held.set(name, { at: Date.now(), page });
  if (held.size > 8) held.delete(held.keys().next().value);
  return page;
}

const sparing = () => navigator.connection?.saveData === true;
let warming = 0;
const warm = (target, wait) => {
  clearTimeout(warming);
  const link = target instanceof Element ? target.closest("a[href]") : null;
  const url = link ? reach(link) : null;
  if (!url || key(url) === key(location) || sparing()) return;
  warming = setTimeout(() => void fetched(url), wait);
};

// ---- putting a page in place ------------------------------------------------

const sheet = (doc) => doc.querySelector('link[rel="stylesheet"]')?.getAttribute("href") ?? "";
const modules = (doc) => [...doc.querySelectorAll('script[type="module"][src]')].map((script) => script.getAttribute("src"));
const islands = (doc) => modules(doc).find((src) => /islands\.[^/]*js$/.test(src)) ?? "";

/** Whether `doc` is a page of the build on screen, in its language. */
function fits(doc) {
  if (!doc.getElementById("main") || doc.documentElement.lang !== root.lang) return false;
  if (sheet(doc) !== sheet(document)) return false;
  const [mine, theirs] = [islands(document), islands(doc)];
  return mine === "" || theirs === "" || mine === theirs;
}

// The page on screen, by its address. An entry of the history says which
// page it shows (`page`); a form's own entry (`boot.js`) names the page it
// stands over, which is not its address.
let shown = key(location);
let turn = 0;
let said = null;

function say(words) {
  if (!said?.isConnected) {
    said = document.createElement("p");
    said.className = "visually-hidden";
    said.setAttribute("role", "status");
    document.body.append(said);
  }
  said.textContent = words;
}

function place(doc) {
  document.dispatchEvent(new Event("baylee:leave"));
  document.title = doc.title;
  for (const old of document.head.querySelectorAll(`${OWN},${DATA}`)) old.remove();
  for (const fresh of doc.head.querySelectorAll(OWN)) document.head.append(document.importNode(fresh, true));
  const form = doc.documentElement.dataset.auth;
  if (form) root.dataset.auth = form;
  else delete root.dataset.auth;
  root.dataset.swapped = "";
  document.body.replaceChildren(...[...doc.body.childNodes].map((node) => document.importNode(node, true)));
  // A page's own scripts the document has not run yet, each made here from
  // its address and its integrity value (one that came with the parsed
  // page would not run); those it has start over at `baylee:page`.
  const have = new Set(modules(document));
  for (const script of doc.querySelectorAll('script[type="module"][src]')) {
    const src = script.getAttribute("src") ?? "";
    if (have.has(src)) continue;
    try {
      const mine = document.createElement("script");
      mine.type = "module";
      mine.integrity = script.getAttribute("integrity") ?? "";
      mine.src = vouch(src);
      document.head.append(mine);
    } catch {
      // not one of the site's scripts: the page stands without it
    }
  }
}

function settle(url, top) {
  const mark = url.hash.length > 1 ? document.getElementById(decodeURIComponent(url.hash.slice(1))) : null;
  if (mark && top === undefined) mark.scrollIntoView();
  else scrollTo({ top: top ?? 0, behavior: "instant" });
  document.getElementById("main")?.focus({ preventScroll: true });
  say(document.title);
}

/** Shows the page at `url`: `push` adds it to the history, `top` is where it stood. */
async function go(url, { push, top }) {
  const mine = ++turn;
  const slow = setTimeout(() => {
    root.dataset.loading = "";
  }, 140);
  const page = parse ? await fetched(url) : null;
  clearTimeout(slow);
  if (mine !== turn) return;
  delete root.dataset.loading;
  held.delete(key(url));
  if (!page || !fits(page.doc) || document.querySelector("leptos-island[data-hydrated]")) {
    if (push) location.assign(url.href);
    else location.reload();
    return;
  }
  const landed = page.url;
  landed.hash = url.hash;
  const apply = () => {
    if (push) {
      history.replaceState({ ...history.state, page: shown, top: scrollY }, "");
      history.pushState({ page: key(landed) }, "", landed.href);
    }
    place(page.doc);
    shown = key(landed);
    settle(landed, top);
    document.dispatchEvent(new Event("baylee:page"));
  };
  const calm = matchMedia("(prefers-reduced-motion: reduce)").matches;
  if (typeof document.startViewTransition === "function" && !calm) {
    await document.startViewTransition(apply).updateCallbackDone.catch(() => {});
  } else {
    apply();
  }
}

if (parse && typeof history.pushState === "function") {
  history.scrollRestoration = "manual";
  // a document loaded anew (a reload, Back into one) stands where it stood,
  // and its entry names the page the server made for it
  const stood = history.state?.top;
  if (typeof stood === "number" && stood > 0) scrollTo({ top: stood, behavior: "instant" });
  history.replaceState({ ...history.state, page: shown }, "");
  addEventListener("pagehide", () => {
    history.replaceState({ ...history.state, page: shown, top: scrollY }, "");
  });

  document.addEventListener("click", (event) => {
    if (event.defaultPrevented || event.button !== 0) return;
    if (event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
    const link = event.target instanceof Element ? event.target.closest("a[href]") : null;
    const url = link ? reach(link) : null;
    if (!url) return;
    if (key(url) === key(location)) {
      // the page on screen: its own places are the browser's to go to
      if (url.hash) return;
      event.preventDefault();
      scrollTo({ top: 0 });
      return;
    }
    event.preventDefault();
    void go(url, { push: true });
  });

  addEventListener("popstate", () => {
    const want = history.state?.page ?? key(location);
    if (want === shown) return;
    // an entry names its page by a path of this site, and nothing else is
    // fetched for one: anything else gets the document at the address
    let url = null;
    try {
      url = typeof want === "string" ? new URL(want + location.hash, location.origin) : null;
    } catch {
      // not an address
    }
    if (!url || url.origin !== location.origin || !PAGE.test(url.pathname)) {
      location.reload();
      return;
    }
    void go(url, { push: false, top: history.state?.top ?? 0 });
  });

  document.addEventListener("pointerover", (event) => warm(event.target, 70));
  document.addEventListener("focusin", (event) => warm(event.target, 70));
  document.addEventListener("touchstart", (event) => warm(event.target, 0), { passive: true });
}
