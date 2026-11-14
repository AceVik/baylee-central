import { readFileSync } from "node:fs";
import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";
import { narrow, schemes, targets, watch } from "./check.ts";

type Season = { id: number[]; withdrawn: boolean; theme: { title: { en: string; de: string }; period: { starts_at: number; ends_at: number } } };
const feed = JSON.parse(readFileSync(new URL("../../../central-web-server/tests/data/challenges.json", import.meta.url), "utf8")) as {
  now: number; before: number | null; challenges: unknown[]; tower: Season;
};
type Request = { action: string; listed?: boolean; board?: string; offset?: number; request?: Request };

async function setup(page: Page) {
  const state = { listed: false, signedIn: true, boards: [] as string[], offsets: [] as number[], consent: [] as boolean[], failConsent: false,
    season: structuredClone(feed.tower), rowCount: 2, failRead: false, pause: null as (() => Promise<void>) | null, held: false, delivered: false };
  await page.addInitScript(() => localStorage.setItem("baylee-central-session", JSON.stringify({ expiresAt: 4_102_444_800 })));
  await page.route("**/api/v1", async (route) => {
    const body = route.request().postDataJSON() as { operation: string; request: Request };
    const json = (response: unknown) => route.fulfill({ json: { result: "challenges", response } });
    if (body.operation === "account_summary") return route.fulfill({ json: {
      result: "account_summary", identity: { account: feed.tower.id, epoch: 1 }, username: "synthetic",
      display_name: "Synthetic player", tag: 80000, source: "registered", recovery: true, admin: false,
      signed_in_at: feed.now, expires_at: 4_102_444_800,
    } });
    if (body.operation !== "challenges") return route.fulfill({ status: 401, body: "account request rejected\n" });
    const op = body.request;
    if (op.action === "catalogue") return json({ answer: "catalogue", now: feed.now, before: feed.before, challenges: feed.challenges });
    if (op.action === "progress") return json({ answer: "progress", active: [], badges: [] });
    expect(op.action).toBe("tower");
    const request = op.request;
    expect(request).toBeDefined();
    if (request?.action === "visibility") {
      state.consent.push(request.listed ?? false);
      if (state.failConsent) { state.failConsent = false; return route.fulfill({ status: 503, body: "unavailable\n" }); }
      state.listed = request.listed ?? false;
      return json({ answer: "tower", response: { answer: "updated" } });
    }
    expect(request?.action).toBe("read");
    if (state.failRead) { state.failRead = false; return route.fulfill({ status: 503, body: "unavailable\n" }); }
    state.boards.push(request?.board ?? "");
    const offset = request?.offset ?? 0;
    state.offsets.push(offset);
    const run = { id: [...feed.tower.id.slice(0, 15), 4], season: feed.tower, revision: 3,
      progress: { phase: "choosing", cleared: 1, lives: 3, route: [], turns: 9, milliseconds: 32000, competitive: true },
      encounter: feed.challenges[1], attempt: null, listed: state.listed };
    const examples = [{ rank: 1, name: "<script>not markup</script>", tag: null, own: false, floors: 5, losses: 0, turns: 63, milliseconds: 320000 },
      { rank: 1, name: "Equal ascent", tag: 80000, own: true, floors: 5, losses: 0, turns: 63, milliseconds: null }];
    const response = { answer: "tower", response: { answer: "read", now: feed.now, season: state.season,
      standings: Array.from({ length: state.rowCount }, (_, i) => examples[i % 2]).slice(offset, offset + 30),
      run: state.signedIn ? run : null, active: state.signedIn ? run : null,
      badges: state.signedIn ? [{ kind: "pathfinder", count: 15, tier: "silver", next: 50 }, { kind: "summit", count: 1, tier: "bronze", next: 4 }, { kind: "unbroken", count: 0, tier: null, next: 1 }] : [],
    } };
    const pause = state.pause;
    state.pause = null;
    if (pause) { state.held = true; await pause(); }
    await json(response);
    if (pause) state.delivered = true;
  });
  return state;
}

test("the expedition is server rendered and its game link carries only the public season identity", async ({ page }) => {
  await page.route("**/assets/islands.*.wasm", (route) => route.abort());
  await page.goto("/en/challenges");
  const tower = page.locator(".tower-hub");
  await expect(tower.getByRole("heading", { name: "The verdant ascent", exact: true })).toBeVisible();
  await expect(tower.locator(".tower-floors li")).toHaveCount(5);
  await expect(tower.getByRole("link", { name: "Play in Baylee" })).toHaveAttribute("href", /^baylee:\/\/tower\/[0-9a-f]{32}$/);
  const html = await tower.innerHTML();
  for (const forbidden of ["record_digest", "private-execution-package", "policy_seed", "seat_token"]) expect(html).not.toContain(forbidden);
});

test("a failed consent change resets the checkbox and never inverts the next explicit choice", async ({ page }) => {
  const state = await setup(page);
  await page.goto("/en/challenges");
  const consent = page.locator(".tower-hub").getByRole("checkbox");
  await expect(consent).not.toBeChecked();
  state.failConsent = true;
  await consent.click();
  await expect.poll(() => state.consent).toEqual([true]);
  await expect(consent).not.toBeChecked();
  await expect(consent).toBeEnabled();
  state.failRead = true;
  await consent.check();
  await expect.poll(() => state.listed).toBe(true);
  await expect(consent).toBeEnabled();
  state.failConsent = true;
  await consent.click();
  await expect.poll(() => state.consent).toEqual([true, true, false]);
  await expect(consent).toBeChecked();
  await expect(consent).toBeEnabled();
  await consent.uncheck();
  await expect.poll(() => state.consent).toEqual([true, true, false, false]);
  await expect.poll(() => state.listed).toBe(false);
});

test("a new season resets a later ranking page and preserves the old active expedition", async ({ page }) => {
  const state = await setup(page);
  state.rowCount = 31;
  await page.goto("/en/challenges");
  const tower = page.locator(".tower-hub");
  await expect(tower.locator(".challenge-table tbody tr")).toHaveCount(30);
  await tower.getByRole("button", { name: "Next", exact: true }).click();
  await expect(tower.locator(".challenge-table tbody tr")).toHaveCount(1);
  expect(state.offsets.at(-1)).toBe(30);
  state.season = structuredClone(feed.tower);
  state.season.id[15] = 9;
  state.season.theme.title.en = "Next week's ascent";
  state.season.theme.period.starts_at += 604800;
  state.season.theme.period.ends_at += 604800;
  state.rowCount = 1;
  await tower.getByRole("button", { name: "Refresh", exact: true }).click();
  await expect.poll(() => state.offsets.slice(-2)).toEqual([30, 0]);
  await expect(tower.locator(".challenge-table tbody tr")).toHaveCount(1);
  await expect(tower.getByRole("button", { name: "Previous", exact: true })).toBeDisabled();
  await expect(tower.locator(".tower-owned")).toHaveCount(1);
});

test("a delayed response from the old account cannot restore private progress after another tab signs out", async ({ page }) => {
  const state = await setup(page);
  await page.goto("/en/challenges");
  const tower = page.locator(".tower-hub");
  await expect(tower.locator(".tower-owned")).toHaveCount(1);
  let release: (() => void) | undefined;
  const pending = new Promise<void>((resolve) => { release = resolve; });
  state.pause = () => pending;
  await tower.getByRole("button", { name: "Refresh", exact: true }).click();
  await expect.poll(() => state.held).toBe(true);
  state.signedIn = false;
  const sibling = await page.context().newPage();
  try {
    await sibling.goto("/robots.txt");
    await sibling.evaluate(() => localStorage.removeItem("baylee-central-session"));
    await expect(tower.locator(".tower-owned")).toHaveCount(0);
    release?.();
    await expect.poll(() => state.delivered).toBe(true);
    await expect(tower.locator(".tower-owned")).toHaveCount(0);
    await expect(tower.locator(".tower-relics")).toHaveCount(0);
  } finally {
    release?.();
    await sibling.close();
  }
});

test("owned ascent, reversible ranking consent, ornate tiers and privacy survive refresh without losing focus", async ({ page, baseURL }, info) => {
  const problems = watch(page, baseURL ?? "");
  await page.clock.install();
  const state = await setup(page);
  await page.goto("/en/challenges");
  await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
  const tower = page.locator(".tower-hub");
  await expect(tower.locator(".tower-upgrades article")).toHaveCount(3);
  await expect(tower.locator(".tower-floors [aria-current=step]")).toContainText("Ember gallery");
  const consent = tower.getByRole("checkbox");
  await expect(consent).not.toBeChecked();
  await consent.check();
  await expect.poll(() => state.listed).toBe(true);
  await expect(consent).toBeChecked();
  await consent.uncheck();
  await expect.poll(() => state.listed).toBe(false);
  await tower.getByRole("button", { name: "Flawless sprint", exact: true }).click();
  await expect.poll(() => state.boards.at(-1)).toBe("sprint");
  await expect(tower.getByRole("button", { name: "Flawless sprint", exact: true })).toHaveAttribute("aria-pressed", "true");
  await expect(tower.locator(".challenge-table tbody tr td:first-child")).toHaveText(["1", "1"]);
  await expect(tower.locator(".challenge-table tbody th").last()).toHaveText("Equal ascent#13880");
  await expect(tower.locator(".challenge-table script")).toHaveCount(0);
  const preview = tower.locator(".tower-relics .button").first();
  await preview.click();
  await expect(tower.locator(".tower-tier-gallery").first().locator(".retention-medal")).toHaveCount(5);
  await preview.focus();
  const before = state.boards.length;
  await page.clock.fastForward(61_000);
  await expect.poll(() => state.boards.length).toBeGreaterThan(before);
  await expect(preview).toBeFocused();
  await expect(preview).toHaveAttribute("aria-pressed", "true");
  expect(await schemes(page)).toEqual([]);
  expect(await targets(page)).toEqual([]);
  expect(await narrow(page)).toBe(0);
  await page.emulateMedia({ colorScheme: "dark", reducedMotion: "reduce" });
  await tower.screenshot({ path: info.outputPath("tower-dark.png"), animations: "disabled" });
  await tower.locator(".medal-preview").getByRole("button", { name: "Close", exact: true }).click();
  await expect(preview).toBeFocused();
  await expect(preview).toHaveAttribute("aria-pressed", "false");
  state.signedIn = false;
  const sibling = await page.context().newPage();
  await sibling.goto("/robots.txt");
  await sibling.evaluate(() => localStorage.removeItem("baylee-central-session"));
  await expect(tower.locator(".tower-owned")).toHaveCount(0);
  await expect(tower.locator(".tower-relics")).toHaveCount(0);
  await sibling.close();
  expect(problems).toEqual([]);
});
