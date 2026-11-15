import { readFileSync } from "node:fs";
import { expect, test } from "@playwright/test";
import type { Locator, Page } from "@playwright/test";
import { narrow, schemes, targets, watch } from "./check.ts";

const feed = JSON.parse(readFileSync(new URL("../../../central-web-server/tests/data/challenges.json", import.meta.url), "utf8")) as {
  now: number; before: number | null; challenges: unknown[]; tower: { id: number[] };
};

// Capture the complete section at the checked width in a taller viewport. Keeping
// the actual sticky navigation above it avoids stitching that bar over the cards.
async function frame(page: Page, section: Locator) {
  const bounds = await section.boundingBox();
  const viewport = page.viewportSize();
  expect(bounds).not.toBeNull();
  expect(viewport).not.toBeNull();
  if (!bounds || !viewport) return;
  await page.setViewportSize({ width: viewport.width, height: Math.ceil(bounds.height + 200) });
  await section.evaluate((element) => window.scrollTo(0, window.scrollY + element.getBoundingClientRect().top - 100));
}

async function setup(page: Page, signedIn = true) {
  const state = { progress: { duels: 90, victories: 80, clears: 10, flawless: 7 }, status: 200, reads: 0,
    held: false, delivered: false, pause: null as (() => Promise<void>) | null };
  if (signedIn) await page.addInitScript(() => localStorage.setItem("baylee-central-session", JSON.stringify({ expiresAt: 4_102_444_800 })));
  await page.route("**/api/v1", async (route) => {
    const body = route.request().postDataJSON() as { operation: string; request?: { action: string } };
    expect(route.request().headers()["authorization"]).toBeUndefined();
    if (body.operation === "account_summary") return route.fulfill({ json: {
      result: "account_summary", identity: { account: feed.tower.id, epoch: 1 }, username: "synthetic",
      display_name: "Synthetic player", tag: 80000, source: "registered", recovery: true, admin: false,
      signed_in_at: feed.now, expires_at: 4_102_444_800,
    } });
    if (body.operation === "tower_progress") {
      state.reads += 1;
      expect(route.request().headers()["x-baylee-central"]).toBe("1");
      const progress = structuredClone(state.progress);
      const pause = state.pause;
      state.pause = null;
      if (pause) { state.held = true; await pause(); }
      if (state.status !== 200) return route.fulfill({ status: state.status, body: "unavailable\n" });
      await route.fulfill({ json: { result: "tower_progress", progress } });
      if (pause) state.delivered = true;
      return;
    }
    if (body.operation === "challenges") {
      const json = (response: unknown) => route.fulfill({ json: { result: "challenges", response } });
      switch (body.request?.action) {
        case "catalogue": return json({ answer: "catalogue", now: feed.now, before: feed.before, challenges: feed.challenges });
        case "progress": return json({ answer: "progress", active: [], badges: [] });
        case "tower": return json({ answer: "tower", response: { answer: "read", now: feed.now, season: feed.tower,
          standings: [], run: null, active: null, badges: [] } });
      }
    }
    return route.fulfill({ status: 401, body: "rejected\n" });
  });
  return state;
}

test("permanent Echoes medals are distinct, accessible and stable across refresh", async ({ page, baseURL }, info) => {
  const problems = watch(page, baseURL ?? "");
  await page.clock.install();
  const state = await setup(page);
  await page.goto("/en/challenges");
  const echoes = page.locator(".echoes-accomplishments");
  const cards = echoes.locator("[data-echoes-badge]");
  await expect(cards).toHaveCount(3);
  await expect(cards.locator("h3")).toHaveText(["Duelmaster", "Crown conqueror", "Flawless"]);
  await expect(cards.locator(".num")).toHaveText(["80 / 200", "10 / 30", "7 / 15"]);
  await expect(cards.locator(".retention-medal.tier-3")).toHaveCount(3);
  const paths = await cards.locator(".medal-emblem").evaluateAll((nodes) => nodes.map((node) => node.getAttribute("d")));
  expect(new Set(paths).size).toBe(3);
  const preview = cards.first().getByRole("button");
  await preview.click();
  const gallery = echoes.locator(".tower-tier-gallery");
  await expect(gallery.locator(".retention-medal")).toHaveCount(5);
  await expect(gallery.locator(".medal-wreath")).toHaveCount(3);
  await expect(gallery.locator(".medal-gems")).toHaveCount(2);
  await expect(gallery.locator(".medal-crown")).toHaveCount(1);
  await preview.focus();
  const before = state.reads;
  await page.clock.fastForward(61_000);
  await expect.poll(() => state.reads).toBeGreaterThan(before);
  await expect(preview).toBeFocused();
  await expect(preview).toHaveAttribute("aria-pressed", "true");
  expect(await schemes(page)).toEqual([]);
  expect(await targets(page)).toEqual([]);
  expect(await narrow(page)).toBe(0);
  await frame(page, echoes);
  await page.emulateMedia({ colorScheme: "dark", reducedMotion: "reduce" });
  await echoes.screenshot({ path: info.outputPath("echoes-dark.png"), animations: "disabled" });
  await page.emulateMedia({ colorScheme: "light", reducedMotion: "reduce" });
  await echoes.screenshot({ path: info.outputPath("echoes-light.png"), animations: "disabled" });
  await echoes.getByRole("button", { name: "Close", exact: true }).click();
  await expect(preview).toBeFocused();
  expect(problems).toEqual([]);
});

test("an outage retains confirmed totals, while corrupt totals never replace them", async ({ page }) => {
  const state = await setup(page);
  await page.goto("/en/challenges");
  const echoes = page.locator(".echoes-accomplishments");
  await expect(echoes.locator(".num").first()).toHaveText("80 / 200");
  state.status = 503;
  await echoes.getByRole("button", { name: "Refresh", exact: true }).click();
  await expect(echoes.getByRole("alert")).not.toBeEmpty();
  await expect(echoes.locator(".num").first()).toHaveText("80 / 200");
  state.status = 200;
  state.progress.victories = 91;
  await echoes.getByRole("button", { name: "Refresh", exact: true }).click();
  await expect(echoes.getByRole("alert")).not.toBeEmpty();
  await expect(echoes.locator(".num").first()).toHaveText("80 / 200");
  state.progress.victories = 81;
  await echoes.getByRole("button", { name: "Refresh", exact: true }).click();
  await expect(echoes.locator(".num").first()).toHaveText("81 / 200");
  await expect(echoes.locator('[role="alert"]')).toBeEmpty();
});

test("an initial outage does not fabricate zero progress, and rejection clears earned previews", async ({ page }) => {
  const state = await setup(page);
  state.status = 503;
  await page.goto("/en/challenges");
  const echoes = page.locator(".echoes-accomplishments");
  await expect(echoes.getByRole("alert")).not.toBeEmpty();
  await expect(echoes.locator("[data-echoes-badge]")).toHaveCount(0);
  state.status = 200;
  await echoes.getByRole("button", { name: "Refresh", exact: true }).click();
  await echoes.locator("[data-echoes-badge]").first().getByRole("button").click();
  await expect(echoes.locator(".medal-preview")).toHaveCount(1);
  state.status = 401;
  await echoes.getByRole("button", { name: "Refresh", exact: true }).click();
  await expect(echoes.locator("[data-echoes-badge]")).toHaveCount(0);
  await expect(echoes.locator(".medal-preview")).toHaveCount(0);
  await expect(echoes.getByRole("link", { name: "Sign in", exact: true })).toBeVisible();
});

test("a late old-account answer cannot restore medals after another tab switches accounts", async ({ page }) => {
  const state = await setup(page);
  await page.goto("/en/challenges");
  const echoes = page.locator(".echoes-accomplishments");
  await echoes.locator("[data-echoes-badge]").first().getByRole("button").click();
  let release: (() => void) | undefined;
  const pending = new Promise<void>((resolve) => { release = resolve; });
  state.pause = () => pending;
  await echoes.getByRole("button", { name: "Refresh", exact: true }).click();
  await expect.poll(() => state.held).toBe(true);
  const sibling = await page.context().newPage();
  try {
    await sibling.goto("/robots.txt");
    await sibling.evaluate(() => localStorage.removeItem("baylee-central-session"));
    await expect(echoes.locator("[data-echoes-badge]")).toHaveCount(0);
    await expect(echoes.locator(".medal-preview")).toHaveCount(0);
    state.progress = { duels: 0, victories: 0, clears: 0, flawless: 0 };
    await sibling.evaluate(() => localStorage.setItem("baylee-central-session", JSON.stringify({ expiresAt: 4_102_444_801 })));
    await expect(echoes.locator(".num")).toHaveText(["0 / 1", "0 / 1", "0 / 1"]);
    release?.();
    await expect.poll(() => state.delivered).toBe(true);
    await expect(echoes.locator(".num")).toHaveText(["0 / 1", "0 / 1", "0 / 1"]);
    await expect(echoes.locator(".medal-preview")).toHaveCount(0);
  } finally {
    release?.();
    await sibling.close();
  }
});

test("anonymous visitors see a sign-in invitation and request no private collection", async ({ page }) => {
  const state = await setup(page, false);
  await page.goto("/en/challenges");
  await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
  const echoes = page.locator(".echoes-accomplishments");
  await expect(echoes.getByRole("link", { name: "Sign in", exact: true })).toBeVisible();
  await expect(echoes.locator("[data-echoes-badge]")).toHaveCount(0);
  expect(state.reads).toBe(0);
});

test("the German collection and its highest tiers fit a narrow viewport", async ({ page }, info) => {
  const state = await setup(page);
  state.progress = { duels: 1200, victories: 1000, clears: 100, flawless: 30 };
  await page.emulateMedia({ colorScheme: "dark", reducedMotion: "reduce" });
  await page.goto("/de/challenges");
  const echoes = page.locator(".echoes-accomplishments");
  await expect(echoes.locator("h3")).toHaveText(["Duellmeister", "Kronenbezwinger", "Makellos"]);
  await expect(echoes.locator(".tier-5")).toHaveCount(3);
  await expect(echoes.locator(".num")).toHaveText(["1.000", "100", "30"]);
  expect(await narrow(page)).toBe(0);
  await frame(page, echoes);
  await echoes.screenshot({ path: info.outputPath("echoes-german.png"), animations: "disabled" });
});
