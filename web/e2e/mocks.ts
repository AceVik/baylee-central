// Every service outside the site, answered in the browser: GitHub's API,
// Scryfall's image host, Central's directory, the feedback service (contact
// form and Impressum) and the card status. Synthetic data only; nothing
// leaves the machine. A request to any other outside host fails the test.

import { expect, type Page } from "@playwright/test";

import { page as directoryPage } from "../test/fixtures/directory.ts";
import { SYNTHETIC, encode } from "../test/fixtures/impressum.ts";

const GH = "https://api.github.com/repos/AceVik/baylee";
const DL = "https://github.com/AceVik/baylee/releases/download/v0.1.0-beta.8";

function asset(name: string, size = 80 * 1048576) {
  return { name, size, browser_download_url: `${DL}/${name}` };
}

export const RELEASE = {
  tag_name: "v0.1.0-beta.8",
  html_url: "https://github.com/AceVik/baylee/releases/tag/v0.1.0-beta.8",
  published_at: "2026-10-18T10:00:00Z",
  prerelease: true,
  draft: false,
  assets: [
    "Baylee-Setup-0.1.0-x64.exe",
    "Baylee-Setup-0.1.0-arm64.exe",
    "Baylee-0.1.0-aarch64.dmg",
    "Baylee-0.1.0-x86_64.AppImage",
    "Baylee-0.1.0-aarch64.AppImage",
    "baylee_0.1.0_amd64.deb",
    "baylee_0.1.0_arm64.deb",
    "baylee-client-0.1.0-x86_64-pc-windows-msvc.zip",
    "baylee-client-0.1.0-aarch64-apple-darwin.zip",
    "baylee-client-0.1.0-x86_64-unknown-linux-gnu.tar.gz",
  ].flatMap((n) => [asset(n), asset(`${n}.sha256`, 100), ...(n.startsWith("baylee-client") ? [asset(`${n}.sig`, 100)] : [])]),
};

export const SETS = {
  corpus: 33694,
  rev: "4e984e84a14ed7b3307144af5eb0e9b8188f8930",
  generated_at: "2026-10-14T22:15:34Z",
  totals: { total: 33694, implemented: 3219, generated: 5754, partial: 51, unimplemented: 54, absent: 24603, never: 13 },
  sets: [
    { code: "lea", name: "Limited Edition Alpha", released: "1993-08-05", position: 0, total: 290, implemented: 286, generated: 0, partial: 0, unimplemented: 0, absent: 0, never: 4 },
    { code: "arn", name: "Arabian Nights", released: "1993-12-17", position: 1, total: 77, implemented: 69, generated: 0, partial: 6, unimplemented: 0, absent: 0, never: 2 },
    { code: "atq", name: "Antiquities", released: "1994-03-04", position: 2, total: 85, implemented: 81, generated: 0, partial: 3, unimplemented: 0, absent: 0, never: 1 },
    // as many cards as cards(36) answers for every set, so the filters match
    { code: "drk", name: "The Dark", released: "1994-08-01", position: 3, total: 36, implemented: 18, generated: 0, partial: 6, unimplemented: 0, absent: 6, never: 6 },
    { code: "ice", name: "Ice Age", released: "1995-06-03", position: 4, total: 383, implemented: 0, generated: 40, partial: 0, unimplemented: 0, absent: 343, never: 0 },
  ],
};

function cards(n: number) {
  const statuses = ["implemented", "implemented", "implemented", "partial", "never", "absent"];
  return {
    cards: Array.from({ length: n }, (_, i) => ({
      name: `Synthetic Card ${i + 1}`,
      oracle_id: `00000000-0000-4000-8000-${String(i).padStart(12, "0")}`,
      status: statuses[i % statuses.length],
      ...(i % 6 === 3 ? { note: "one clause the rules language cannot say yet" } : {}),
      ...(i % 6 === 4 ? { note: "ante" } : {}),
      ...(i % 6 === 5 ? {} : { img: `0b1c2d3e-0000-4000-8000-${String(i).padStart(12, "0")}` }),
    })),
  };
}

/** A tiny PNG, for every picture the tests ask an outside host or the gitignored media for. */
const PIXEL = Buffer.from("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAIAAACQd1PeAAAADElEQVR4nGNQM3ABAAEaAJsR09CMAAAAAElFTkSuQmCC", "base64");

export interface Mocks {
  /** Requests to an outside host nobody answered: must stay empty. */
  stray: string[];
  /** What the contact form posted. */
  contact: unknown[];
  /** The directory's request bodies. */
  directory: unknown[];
  /** Why a directory request would not pass Central's browser listener: must stay empty. */
  refused: string[];
}

export async function mock(page: Page, opts: { gateways?: number; ownContent?: boolean } = {}): Promise<Mocks> {
  const m: Mocks = { stray: [], contact: [], directory: [], refused: [] };
  const blob = await encode(SYNTHETIC);
  const total = opts.gateways ?? 130;

  await page.route(
    (url) => url.hostname !== "127.0.0.1",
    async (route) => {
      const url = new URL(route.request().url());
      if (url.href.startsWith(`${GH}/releases`)) return route.fulfill({ json: [RELEASE] });
      if (url.href.startsWith(`${GH}/actions/`)) {
        return route.fulfill({
          json: { workflow_runs: [{ conclusion: "success", html_url: `https://github.com/AceVik/baylee/actions/runs/1`, head_sha: "4e984e84a14ed7b3307144af5eb0e9b8188f8930", updated_at: "2026-10-20T08:00:00Z" }] },
        });
      }
      if (url.href.startsWith(`${GH}/commits`)) {
        return route.fulfill({
          json: [{ sha: "4e984e84a14ed7b3307144af5eb0e9b8188f8930", html_url: "https://github.com/AceVik/baylee/commit/4e984e8", commit: { message: "Synthetic commit", committer: { date: "2026-10-20T07:30:00Z" } } }],
        });
      }
      if (url.href === GH) return route.fulfill({ json: { stargazers_count: 42, open_issues_count: 7 } });
      if (url.hostname === "cards.scryfall.io") return route.fulfill({ body: PIXEL, contentType: "image/png" });
      m.stray.push(url.href);
      return route.abort();
    },
  );
  // the card status and the screenshots: synthetic, unless the pictures for a
  // review may show what `npm run content` copied into the build
  if (opts.ownContent !== true) {
    await page.route("**/data/sets.json", (route) => route.fulfill({ json: SETS }));
    await page.route("**/data/sets/set-*.json", (route) => route.fulfill({ json: cards(36) }));
    await page.route("**/media/**", (route) => route.fulfill({ body: PIXEL, contentType: "image/png" }));
  }
  // Central's browser listener (docs/web-accounts.md): the directory with the
  // page's own marks and no credential; every other operation is the account
  // mock's (e2e/central.ts) or nobody's.
  await page.route("**/api/v1", async (route) => {
    const request = route.request();
    const body = request.postDataJSON() as { operation?: string; after: number[] | null };
    if (body.operation !== "gateway_directory") return route.fallback();
    const headers = await request.allHeaders();
    if (headers["x-baylee-central"] !== "1") m.refused.push("no X-Baylee-Central: 1");
    if (headers["content-type"] !== "application/json") m.refused.push("Content-Type is not application/json");
    if (headers["authorization"] !== undefined || headers["cookie"] !== undefined) m.refused.push("a credential");
    m.directory.push(body);
    // the second page starts after the first page's last id (gateway 100)
    return route.fulfill({ json: directoryPage(body.after === null ? 0 : 1, total) });
  });
  await page.route("http://127.0.0.1:28780/site/contact", async (route) => {
    m.contact.push(route.request().postDataJSON());
    return route.fulfill({ status: 201, json: { ok: true } });
  });
  await page.route("http://127.0.0.1:28780/site/impressum/challenge*", (route) =>
    route.fulfill({ json: { nonce: "AAAAAAAAAAAAAAAAAAAAAA", parts: 2, difficulty: 4, expires: 4_000_000_000, sig: "synthetic" } }),
  );
  await page.route("http://127.0.0.1:28780/site/impressum", (route) => route.fulfill({ body: blob, contentType: "text/plain" }));
  return m;
}

/** No error in the console (a CSP refusal is one) and no uncaught exception. */
export function watchErrors(page: Page): string[] {
  const errors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") errors.push(msg.text());
  });
  page.on("pageerror", (e) => errors.push(e.message));
  return errors;
}

/**
 * The page fits its width: the layout is no wider than the device (a phone
 * browser widens its layout viewport, and with it `innerWidth`, to fit a
 * too-wide element), and no element reaches past the sides except inside
 * something that scrolls or clips it (a code block, the hero's painting) or
 * that is hidden on purpose (.vh, the honeypot).
 */
export async function expectNoSideScroll(page: Page): Promise<void> {
  const device = page.viewportSize()?.width ?? 0;
  const seen = await page.evaluate((width) => {
    const found: string[] = [];
    for (const el of Array.from(document.querySelectorAll<HTMLElement>("body *"))) {
      const r = el.getBoundingClientRect();
      if (r.width === 0 || (r.right <= width + 1 && r.left >= -1)) continue;
      if (el.closest(".vh, .trap, .skip, .cpreview")) continue;
      let clipped = false;
      for (let p = el.parentElement; p && p !== document.body; p = p.parentElement) {
        if (getComputedStyle(p).overflowX !== "visible") {
          clipped = true;
          break;
        }
      }
      if (!clipped) found.push(`${el.tagName.toLowerCase()}.${el.className}`);
    }
    return { layout: Math.max(window.innerWidth, document.documentElement.scrollWidth), found: found.slice(0, 5) };
  }, device);
  expect(seen).toEqual({ layout: device, found: [] });
}
