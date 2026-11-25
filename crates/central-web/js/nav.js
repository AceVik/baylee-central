// GENERATED from crates/central-web/ts/nav.ts via TypeScript 7.1 — do not edit directly.
const root = document.documentElement;
const PAGE = /^\/([a-z]{2}(?:-[a-z]+)?)(?:\/(.*))?$/;
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
const chat = (path) => /^\/[a-z]{2}(?:-[a-z]+)?\/account\/chat(\/|$)/.test(path);
const SCRIPT = /^\/assets\/[A-Za-z0-9._-]+\.js$/;
const ours = (address) => {
    if (!SCRIPT.test(address))
        throw new TypeError("not a script of this site");
    return address;
};
let parse = null;
let vouch = ours;
try {
    const types = window.trustedTypes;
    if (types?.createPolicy) {
        const policy = types.createPolicy("baylee-nav", { createHTML: (htmlText) => htmlText, createScriptURL: ours });
        parse = (htmlText) => new DOMParser().parseFromString(policy.createHTML(htmlText), "text/html");
        vouch = (address) => policy.createScriptURL(address);
    }
    else {
        parse = (htmlText) => new DOMParser().parseFromString(htmlText, "text/html");
    }
}
catch {
    // a policy the page may not make: every link loads its document
}
function reach(link) {
    if (!(link instanceof HTMLAnchorElement) || link.origin !== location.origin)
        return null;
    if (link.hasAttribute("download") || (link.target !== "" && link.target !== "_self"))
        return null;
    const to = PAGE.exec(link.pathname);
    const from = PAGE.exec(location.pathname);
    if (!to || !from || to[1] !== from[1])
        return null;
    if (chat(link.pathname) || chat(location.pathname))
        return null;
    return new URL(link.href);
}
const KEEP = 20_000;
const held = new Map();
function fetched(url) {
    const parser = parse;
    if (!parser)
        return Promise.resolve(null);
    const name = key(url);
    const have = held.get(name);
    if (have && Date.now() - have.at < KEEP)
        return have.page;
    const page = fetch(url.origin + name, { credentials: "same-origin", headers: { accept: "text/html" } })
        .then(async (response) => {
        const kind = response.headers.get("content-type") ?? "";
        if (!response.ok || !kind.startsWith("text/html"))
            return null;
        const landed = new URL(response.url);
        if (landed.origin !== location.origin)
            return null;
        return { doc: parser(await response.text()), url: landed };
    })
        .catch(() => null);
    held.set(name, { at: Date.now(), page });
    if (held.size > 8) {
        const oldest = held.keys().next().value;
        if (oldest !== undefined)
            held.delete(oldest);
    }
    return page;
}
const sparing = () => navigator.connection?.saveData === true;
let warming = 0;
const warm = (target, wait) => {
    clearTimeout(warming);
    const link = target instanceof Element ? target.closest("a[href]") : null;
    const url = link ? reach(link) : null;
    if (!url || key(url) === key(location) || sparing())
        return;
    warming = window.setTimeout(() => void fetched(url), wait);
};
const sheet = (doc) => doc.querySelector('link[rel="stylesheet"]')?.getAttribute("href") ?? "";
const modules = (doc) => [...doc.querySelectorAll('script[type="module"][src]')].map((script) => script.getAttribute("src") ?? "");
const islands = (doc) => modules(doc).find((src) => /islands\.[^/]*js$/.test(src)) ?? "";
function fits(doc) {
    if (!doc.getElementById("main") || doc.documentElement.lang !== root.lang)
        return false;
    if (sheet(doc) !== sheet(document))
        return false;
    const [mine, theirs] = [islands(document), islands(doc)];
    return mine === "" || theirs === "" || mine === theirs;
}
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
    for (const old of document.head.querySelectorAll(`${OWN},${DATA}`))
        old.remove();
    for (const fresh of doc.head.querySelectorAll(OWN))
        document.head.append(document.importNode(fresh, true));
    const form = doc.documentElement.dataset.auth;
    if (form)
        root.dataset.auth = form;
    else
        delete root.dataset.auth;
    root.dataset.swapped = "";
    document.body.replaceChildren(...[...doc.body.childNodes].map((node) => document.importNode(node, true)));
    const have = new Set(modules(document));
    for (const script of doc.querySelectorAll('script[type="module"][src]')) {
        const src = script.getAttribute("src") ?? "";
        if (have.has(src))
            continue;
        try {
            const mine = document.createElement("script");
            mine.type = "module";
            mine.integrity = script.getAttribute("integrity") ?? "";
            mine.src = vouch(src);
            document.head.append(mine);
        }
        catch {
            // not one of the site's scripts: the page stands without it
        }
    }
}
function settle(url, top) {
    const mark = url.hash.length > 1 ? document.getElementById(decodeURIComponent(url.hash.slice(1))) : null;
    if (mark && top === undefined)
        mark.scrollIntoView();
    else
        scrollTo({ top: top ?? 0, behavior: "instant" });
    document.getElementById("main")?.focus({ preventScroll: true });
    say(document.title);
}
async function go(url, { push, top }) {
    const mine = ++turn;
    const slow = window.setTimeout(() => {
        root.dataset.loading = "";
    }, 60);
    const page = parse ? await fetched(url) : null;
    clearTimeout(slow);
    if (mine !== turn)
        return;
    delete root.dataset.loading;
    held.delete(key(url));
    if (!page || !fits(page.doc)) {
        if (push)
            location.assign(url.href);
        else
            location.reload();
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
        await document.startViewTransition(apply).updateCallbackDone.catch(() => { });
    }
    else {
        apply();
    }
}
if (parse && typeof history.pushState === "function") {
    history.scrollRestoration = "manual";
    const initialState = history.state;
    const stood = initialState?.top;
    if (typeof stood === "number" && stood > 0)
        scrollTo({ top: stood, behavior: "instant" });
    history.replaceState({ ...history.state, page: shown }, "");
    addEventListener("pagehide", () => {
        history.replaceState({ ...history.state, page: shown, top: scrollY }, "");
    });
    document.addEventListener("click", (event) => {
        if (event.defaultPrevented || event.button !== 0)
            return;
        if (event.metaKey || event.ctrlKey || event.shiftKey || event.altKey)
            return;
        const link = event.target instanceof Element ? event.target.closest("a[href]") : null;
        const url = reach(link);
        if (!url)
            return;
        if (key(url) === key(location)) {
            if (url.hash)
                return;
            event.preventDefault();
            scrollTo({ top: 0 });
            return;
        }
        event.preventDefault();
        void go(url, { push: true });
    });
    addEventListener("popstate", () => {
        const currentState = history.state;
        const want = currentState?.page ?? key(location);
        if (want === shown)
            return;
        let url = null;
        try {
            url = typeof want === "string" ? new URL(want + location.hash, location.origin) : null;
        }
        catch {
            // not an address
        }
        if (!url || url.origin !== location.origin || !PAGE.test(url.pathname) || chat(url.pathname) || chat(location.pathname)) {
            location.reload();
            return;
        }
        void go(url, { push: false, top: currentState?.top ?? 0 });
    });
    document.addEventListener("pointerover", (event) => warm(event.target, 70));
    document.addEventListener("focusin", (event) => warm(event.target, 70));
    document.addEventListener("touchstart", (event) => warm(event.target, 0), { passive: true });
}
export {};
