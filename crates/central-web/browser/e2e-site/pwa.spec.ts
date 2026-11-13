// The site as an app (docs/proposals/frontend-rewrite.md §7): only someone
// signed in, the installed app or a device with notifications on gets the
// service worker; it caches the offline page per build, answers a page it cannot fetch
// with it in the address's language, and a new build waits until the
// visitor chooses to reload, which warns once about text not sent yet.
//
// A browser fetches a service worker's script past Playwright's routing,
// so the update test serves two builds itself: its own central-web on
// 127.0.0.1:28795, first with target/site, then with a copy whose sw.js
// carries another stamp. Every process it starts it stops and awaits.
import { type ChildProcessWithoutNullStreams, spawn } from "node:child_process";
import { once } from "node:events";
import { cpSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test } from "@playwright/test";

const ROOT = fileURLToPath(new URL("../../../..", import.meta.url));
const PORT = 28795;
const ORIGIN = `http://127.0.0.1:${PORT}`;

async function serve(site: string): Promise<ChildProcessWithoutNullStreams> {
  const child = spawn(join(ROOT, "target/debug/central-web"), ["--config-stdin"], { cwd: ROOT });
  child.stdin.end(
    JSON.stringify({
      listen: `127.0.0.1:${PORT}`,
      origin: ORIGIN,
      site,
      github: false,
      fallback: join(ROOT, "crates/central-web-server/tests/data"),
    }),
  );
  let said = "";
  await new Promise<void>((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error(`central-web did not start: ${said}`)), 15_000);
    child.stdout.on("data", (chunk: Buffer) => {
      said += chunk.toString();
      if (said.includes("listening")) {
        clearTimeout(timer);
        resolve();
      }
    });
    child.once("exit", () => reject(new Error(`central-web stopped: ${said}`)));
  });
  return child;
}

async function stop(child: ChildProcessWithoutNullStreams | null) {
  if (child === null || child.exitCode !== null) return;
  const exited = once(child, "exit");
  child.kill("SIGTERM");
  await exited;
}

const SIGNED_IN = () => localStorage.setItem("baylee-central-session", JSON.stringify({ expiresAt: 4_102_444_800 }));

test("a reader gets no worker, and after signing out it goes with its cache", async ({ page }) => {
  await page.goto("/en/");
  await page.waitForLoadState("networkidle");
  expect(await page.evaluate(async () => (await navigator.serviceWorker.getRegistrations()).length)).toBe(0);
  expect(await page.evaluate(async () => (await caches.keys()).length)).toBe(0);

  // signed in, the worker comes; signed out, the next page takes it away
  await page.evaluate(SIGNED_IN);
  await page.reload();
  await page.evaluate(async () => {
    await navigator.serviceWorker.ready;
  });
  await page.evaluate(() => localStorage.removeItem("baylee-central-session"));
  await expect.poll(() => page.evaluate(async () => (await caches.keys()).some((key) => key.startsWith("baylee-site-")))).toBe(true);
  await page.reload();
  await expect.poll(() => page.evaluate(async () => (await navigator.serviceWorker.getRegistrations()).length)).toBe(0);
  await expect.poll(() => page.evaluate(async () => (await caches.keys()).length)).toBe(0);
  // The removed worker still answers for this page. It was retired first,
  // so a file the page fetches later makes no cache again (12.11.2026: a
  // fetch under way while the cache went made it once more).
  expect(await page.evaluate(() => navigator.serviceWorker.controller !== null)).toBe(true);
  const sheet = await page.locator('link[rel="stylesheet"]').first().getAttribute("href");
  expect(sheet).toMatch(/^\/assets\/.+\.[0-9a-f]{12}\.css$/);
  expect(await page.evaluate(async (href) => (await fetch(href)).ok, sheet ?? "")).toBe(true);
  await page.waitForLoadState("networkidle");
  expect(await page.evaluate(async () => (await caches.keys()).length)).toBe(0);

  // signed in again, a worker comes with its cache: what was retired is not in its way
  await page.evaluate(SIGNED_IN);
  await page.reload();
  await page.evaluate(async () => {
    await navigator.serviceWorker.ready;
  });
  await expect
    .poll(() =>
      page.evaluate(async () => {
        const name = (await caches.keys()).find((key) => key.startsWith("baylee-site-"));
        if (name === undefined) return false;
        return (await (await caches.open(name)).keys()).some((request) => new URL(request.url).pathname === "/en/offline");
      }),
    )
    .toBe(true);
});

test("signing in beside a signed-out page that stays open brings the worker's cache back", async ({ page, context }) => {
  const stored = (of: typeof page) =>
    of.evaluate(async () => {
      const name = (await caches.keys()).find((key) => key.startsWith("baylee-site-"));
      if (name === undefined) return [];
      return (await (await caches.open(name)).keys()).map((request) => new URL(request.url).pathname);
    });

  // the first page: signed in, then signed out, and left open with the worker it retired
  await page.goto("/en/");
  await page.evaluate(SIGNED_IN);
  await page.reload();
  await page.evaluate(async () => {
    await navigator.serviceWorker.ready;
  });
  await page.evaluate(() => localStorage.removeItem("baylee-central-session"));
  await expect.poll(async () => (await stored(page)).length).toBeGreaterThan(0);
  await page.reload();
  await expect.poll(() => page.evaluate(async () => (await navigator.serviceWorker.getRegistrations()).length)).toBe(0);
  await expect.poll(() => page.evaluate(async () => (await caches.keys()).length)).toBe(0);

  // A second page signs in. The browser hands it the worker the first page
  // still holds, retired and with no cache: the page keeps it, and the
  // worker fills its cache and caches again.
  await page.evaluate(SIGNED_IN);
  const second = await context.newPage();
  await second.goto("/en/download");
  await second.evaluate(async () => {
    await navigator.serviceWorker.ready;
  });
  await expect.poll(async () => (await stored(second)).includes("/de/offline")).toBe(true);
  const filled = (await stored(second)).length;
  await second.reload();
  await expect.poll(async () => (await stored(second)).length).toBeGreaterThan(filled);
  await context.setOffline(true);
  await second.goto("/de/download");
  await expect(second.getByRole("heading", { level: 1 })).toHaveText("Du bist offline");
  await context.setOffline(false);
});

test("a device with notifications on keeps its worker past the sign-in", async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem("baylee-push-installation-v1", "0123456789abcdef0123456789abcdef"));
  await page.goto("/en/");
  await expect.poll(() => page.evaluate(async () => Boolean((await navigator.serviceWorker.getRegistration("/"))?.active))).toBe(true);
});

test("works offline", async ({ page, context }) => {
  await page.addInitScript(SIGNED_IN);
  await page.goto("/en/");
  await page.evaluate(async () => {
    await navigator.serviceWorker.ready;
  });
  await page.reload();
  expect(await page.evaluate(() => navigator.serviceWorker.controller !== null)).toBe(true);
  const cached = await page.evaluate(async () => {
    const name = (await caches.keys()).find((key) => key.startsWith("baylee-site-"));
    if (name === undefined) return [];
    return (await (await caches.open(name)).keys()).map((request) => new URL(request.url).pathname);
  });
  expect(cached).toEqual(expect.arrayContaining(["/en/offline", "/de/offline", "/ja/offline", "/zh-hant/offline"]));
  expect(cached.every((path) => path.endsWith("/offline") || path.startsWith("/assets/"))).toBe(true);

  // the page asked for answers with the offline page, in its language
  await context.setOffline(true);
  await page.goto("/de/download");
  await expect(page.getByRole("heading", { level: 1 })).toHaveText("Du bist offline");
  await expect(page.getByRole("link", { name: "Erneut versuchen" })).toBeVisible();
  await context.setOffline(false);
});

test("a new version waits, and warns once about unsent text before reloading", async ({ page }, info) => {
  test.skip(info.project.name !== "desktop", "one server of its own at a time");
  const copy = mkdtempSync(join(tmpdir(), "baylee-site-"));
  let server: ChildProcessWithoutNullStreams | null = null;
  try {
    cpSync(join(ROOT, "target/site"), copy, { recursive: true });
    server = await serve(copy);
    await page.addInitScript(SIGNED_IN);
    await page.goto(`${ORIGIN}/en/contact`);
    await page.evaluate(async () => {
      await navigator.serviceWorker.ready;
    });
    await page.reload();
    await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");

    // the next build: only the stamp differs
    await stop(server);
    server = null;
    const worker = join(copy, "sw.js");
    writeFileSync(worker, readFileSync(worker, "utf8").replace(/const BUILD = "[0-9a-f]+";/, 'const BUILD = "0000000000000000";'));
    rmSync(`${worker}.br`, { force: true });
    rmSync(`${worker}.gz`, { force: true });
    server = await serve(copy);

    await page.getByLabel("Your message").fill("Words not sent yet");
    await page.evaluate(async () => {
      await (await navigator.serviceWorker.getRegistration())?.update();
    });
    const offer = page.locator("#update");
    await expect(offer).toBeVisible({ timeout: 15_000 });
    await expect(page.getByRole("status").filter({ hasText: "A new version of baylee.app is ready." })).toHaveCount(1);
    await offer.getByRole("button", { name: "Reload" }).click();
    await expect(offer.getByText("You have text here that is not sent yet. Reload anyway?")).toBeVisible();
    await expect(page.getByLabel("Your message")).toHaveValue("Words not sent yet");
    const reloaded = page.waitForEvent("load");
    await offer.getByRole("button", { name: "Reload" }).click();
    await reloaded;
    await expect
      .poll(() => page.evaluate(async () => (await caches.keys()).filter((key) => key.startsWith("baylee-site-"))))
      .toEqual(["baylee-site-0000000000000000"]);
  } finally {
    await stop(server);
    rmSync(copy, { recursive: true, force: true });
  }
});

test("offers no install without the browser's offer", async ({ page }) => {
  await page.goto("/en/account");
  await expect(page.locator("[data-install-box]")).toBeHidden();
});
