import { readFileSync } from "node:fs";
import { expect, test } from "@playwright/test";
import type { Page, Route } from "@playwright/test";
import { axe, targets, watch } from "./check.ts";

type Task = { id: number[]; description: { period: { kind: string; starts_at: number; ends_at: number }; title: { en: string; de: string } }; withdrawn: boolean };
type Feed = { now: number; before: number | null; challenges: Task[]; tower: unknown };
const feed = JSON.parse(readFileSync(new URL("../../../central-web-server/tests/data/challenges.json", import.meta.url), "utf8")) as Feed;
type Body = { operation: string; request?: { action: string; challenge?: number[]; board?: string; listed?: boolean; level?: number } };

async function setup(page: Page, signedIn = false) {
  const asked: Body[] = [];
  const state = { hints: 0, listed: false, unavailable: false, now: feed.now };
  if (signedIn) await page.addInitScript(() => localStorage.setItem("baylee-central-session", JSON.stringify({ expiresAt: 4_102_444_800 })));
  await page.route("**/api/v1", (route: Route) => {
    const body = JSON.parse(route.request().postData() ?? "{}") as Body;
    asked.push(body);
    const json = (value: unknown) => route.fulfill({ status: 200, json: value });
    const rejected = () => route.fulfill({ status: 401, body: "account request rejected\n" });
    if (body.operation === "account_summary" && signedIn) return json({
      result: "account_summary", identity: { account: feed.challenges[0].id, epoch: 1 }, username: "synthetic",
      display_name: "Synthetic player", tag: 80000, source: "registered", recovery: true, admin: false,
      signed_in_at: 1_794_000_000, expires_at: 4_102_444_800,
    });
    if (body.operation !== "challenges") return rejected();
    expect(route.request().headers()["x-baylee-central"]).toBe("1");
    expect(route.request().headers()["authorization"]).toBeUndefined();
    const answer = (response: unknown) => json({ result: "challenges", response });
    const op = body.request;
    if (!op) return rejected();
    if (state.unavailable) return route.fulfill({ status: 503, body: "unavailable" });
    const task = feed.challenges.find((c) => JSON.stringify(c.id) === JSON.stringify(op.challenge)) ?? feed.challenges[0];
    switch (op.action) {
      case "catalogue": return answer({ answer: "catalogue", challenges: feed.challenges, before: feed.before, now: state.now });
      case "tower": return answer({ answer: "tower", response: { answer: "read", now: state.now, season: feed.tower, standings: [], run: null, active: null, badges: [] } });
      case "detail": return answer({ answer: "detail", now: feed.now, challenge: task, standings: [
        { rank: 1, name: "<script>not markup</script>", tag: 80000, own: false, failures: 0, turn: 4, milliseconds: 92000 },
        { rank: 1, name: "Equal score", tag: null, own: true, failures: 0, turn: 4, milliseconds: null },
      ] });
      case "progress": return signedIn ? answer({ answer: "progress", active: [], badges: [
        { kind: "daily", count: 10, tier: "silver", next: 30 }, { kind: "weekly", count: 1, tier: "bronze", next: 4 },
        { kind: "tactician", count: 0, tier: null, next: 1 }, { kind: "first_try", count: 5, tier: "silver", next: 20 },
        { kind: "regular", count: 1, tier: "bronze", next: 4 },
      ] }) : rejected();
      case "personal": return signedIn ? answer({ answer: "personal", progress: {
        challenge: task.id, starts: 2, failures: 1, hint_level: state.hints, listed: state.listed, first_win: null,
        completed_at: feed.now - 10, live_week: null, tactician: true, first_try: false, best_turn: 4,
      } }) : rejected();
      case "hint":
        if (!signedIn) return rejected();
        state.hints = Math.max(state.hints, op.level ?? 0);
        return answer({ answer: "hint", text: { en: "Synthetic private hint", de: "Privater Testhinweis" } });
      case "visibility":
        if (!signedIn) return rejected();
        state.listed = op.listed ?? false;
        return answer({ answer: "updated" });
      default: return rejected();
    }
  });
  return { asked, state };
}

test("Daily and Weekly render before JavaScript and link only an opaque task identity to Baylee", async ({ page }) => {
  await page.route("**/assets/islands.*.wasm", (route) => route.abort());
  await page.goto("/en/challenges");
  await expect(page.getByRole("heading", { name: "Daily fixture" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Weekly fixture" })).toBeVisible();
  const links = await page.locator('.challenge-card a[href^="baylee:"]').evaluateAll((nodes) => nodes.map((n) => n.getAttribute("href")));
  expect(links).toHaveLength(2);
  expect(links.every((href) => /^baylee:\/\/challenge\/[0-9a-f]{32}$/.test(href ?? ""))).toBe(true);
  const html = await page.locator(".challenge-hub").innerHTML();
  expect(html).not.toContain("private-execution-package");
  expect(html).not.toContain("record_digest");
});

test("public rankings escape names, preserve ties and remain accessible on a phone", async ({ page, baseURL }) => {
  const problems = watch(page, baseURL ?? "");
  const { asked, state } = await setup(page);
  await page.goto("/en/challenges");
  await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
  const open = page.getByRole("button", { name: "Task and rankings" }).first();
  await open.focus();
  await open.press("Enter");
  await expect(page.locator(".challenge-table tbody tr")).toHaveCount(2);
  await expect(page.locator("#challenge-detail-title")).toBeFocused();
  await page.locator("#challenge-back").focus();
  await page.locator("#challenge-back").press("Enter");
  await expect(open).toBeFocused();
  await open.press("Enter");
  await expect(page.locator("#challenge-detail-title")).toBeFocused();
  await expect(page.locator(".challenge-table tbody tr td:first-child")).toHaveText(["1", "1"]);
  await expect(page.getByText("<script>not markup</script>", { exact: false })).toBeVisible();
  await expect(page.locator(".challenge-table script")).toHaveCount(0);
  await page.getByRole("button", { name: "Sprint", exact: true }).click();
  await expect.poll(() => asked.filter((b) => b.request?.board === "sprint").length).toBe(1);
  await expect(page.locator(".challenge-table tbody tr")).toHaveCount(2);
  expect(await axe(page)).toEqual([]);
  expect(await targets(page)).toEqual([]);
  expect(await page.evaluate(() => document.documentElement.scrollWidth > document.documentElement.clientWidth)).toBe(false);
  expect(problems).toEqual([]);
  await page.locator("#challenge-back").click();
  state.now = feed.challenges[0].description.period.ends_at + 1;
  await page.locator("#challenge-refresh").click();
  await expect(page.locator(".challenge-grid").first().getByRole("heading")).toHaveText(["Weekly fixture"]);
  await expect(page.locator(".challenge-archive")).toHaveAttribute("open", "");
  const archived = page.locator(".challenge-archive").getByRole("button", { name: "Task and rankings" });
  await archived.focus();
  await archived.press("Enter");
  await expect(page.locator("#challenge-detail-title")).toBeFocused();
  await page.locator("#challenge-back").click();
  await expect(archived).toBeFocused();
});

test("a hint requires an explicit second action, persists assistance and never opts the player in", async ({ page }) => {
  const { asked, state } = await setup(page, true);
  await page.goto("/en/challenges");
  await expect(page.locator(".challenge-achievements .challenge-badge")).toHaveCount(5);
  await page.getByRole("button", { name: "Task and rankings" }).first().click();
  const consent = page.getByRole("checkbox", { name: "Show my public name and tag in this task's rankings." });
  await expect(consent).not.toBeChecked();
  await page.getByRole("button", { name: "Hint 1", exact: true }).click();
  expect(asked.some((b) => b.request?.action === "hint")).toBe(false);
  await page.getByRole("button", { name: "Cancel", exact: true }).click();
  expect(state.hints).toBe(0);
  await page.getByRole("button", { name: "Hint 1", exact: true }).click();
  await page.getByRole("button", { name: "Reveal hint", exact: true }).click();
  await expect(page.getByText("Synthetic private hint", { exact: true })).toBeVisible();
  await expect(page.getByText(/^Hints used:/)).toBeVisible();
  await expect(consent).not.toBeChecked();
  await consent.check();
  await expect.poll(() => state.listed).toBe(true);
  await page.reload();
  await page.getByRole("button", { name: "Task and rankings" }).first().click();
  await expect(page.getByText(/^Hints used:/)).toBeVisible();
  await expect(consent).toBeChecked();
});

test("an outage is a retryable error, never an empty success", async ({ page }) => {
  const { state } = await setup(page);
  state.unavailable = true;
  await page.goto("/en/challenges");
  await expect(page.locator("#challenge-error")).not.toBeEmpty();
  state.unavailable = false;
  await page.locator("#challenge-refresh").click();
  await expect(page.locator("#challenge-error")).toBeEmpty();
  await expect(page.getByRole("button", { name: "Task and rankings" })).toHaveCount(2);
});

test("rankings refresh automatically without interrupting hint confirmation", async ({ page }) => {
  await page.clock.install();
  const { asked, state } = await setup(page, true);
  await page.goto("/en/challenges");
  await expect(page.locator(".challenge-achievements .challenge-badge")).toHaveCount(5);
  await page.getByRole("button", { name: "Task and rankings" }).first().click();
  await expect(page.locator(".challenge-table tbody tr")).toHaveCount(2);
  await page.getByRole("button", { name: "Hint 1", exact: true }).click();
  const catalogues = () => asked.filter((body) => body.request?.action === "catalogue").length;
  const before = catalogues();
  await page.clock.fastForward(61_000);
  expect(catalogues()).toBe(before);
  await expect(page.getByRole("button", { name: "Reveal hint", exact: true })).toBeVisible();
  expect(state.hints).toBe(0);
  await page.getByRole("button", { name: "Cancel", exact: true }).click();
  await page.clock.fastForward(61_000);
  await expect.poll(catalogues).toBeGreaterThan(before);
  await expect(page.locator(".challenge-table tbody tr")).toHaveCount(2);
  await page.getByRole("button", { name: "Hint 1", exact: true }).click();
  await page.locator("#challenge-back").click();
  await expect(page.locator(".challenge-detail")).toHaveCount(0);
  const beforeBackRefresh = catalogues();
  await page.clock.fastForward(61_000);
  await expect.poll(catalogues).toBeGreaterThan(beforeBackRefresh);
  expect(state.hints).toBe(0);
  expect(asked.some((body) => ["hint", "begin"].includes(body.request?.action ?? ""))).toBe(false);
});
