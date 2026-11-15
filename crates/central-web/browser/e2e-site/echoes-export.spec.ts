import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

const id = (n: number) => Array.from({ length: 16 }, (_, i) => ({ 6: 0x70, 8: 0x80, 15: n })[i] ?? 0x10 + i);
const NOW = 1_794_000_000;
const progress = { duels: 129, victories: 129, clears: 0, flawless: 0 };
const receipt = (n: number) => ({ id: id(n), event: id(n), origin: "https://tower.example", accepted_at: NOW,
  fact: { schema: 2, run: id(n), cohort: id(250), stage: 1, outcome: "victory", wins: 1, defeats: 0 } });

async function setup(page: Page) {
  const state = { pages: [] as (number[] | null)[], mode: "valid", hold: "", held: false, delivered: false,
    pause: null as (() => Promise<void>) | null };
  await page.addInitScript(() => localStorage.setItem("baylee-central-session", JSON.stringify({ expiresAt: 4_102_444_800 })));
  await page.route("**/api/v1", async (route) => {
    const body = route.request().postDataJSON() as { operation: string; after?: number[] | null; request?: { action: string } };
    const pause = body.operation === state.hold ? state.pause : null;
    if (pause) { state.pause = null; state.held = true; await pause(); }
    const json = async (value: unknown) => {
      await route.fulfill({ json: value });
      if (pause) state.delivered = true;
    };
    switch (body.operation) {
      case "account_summary": return json({ result: "account_summary", identity: { account: id(200), epoch: 1 }, username: "synthetic",
        display_name: "Synthetic player", tag: 80000, source: "registered", recovery: true, admin: false,
        signed_in_at: NOW, expires_at: 4_102_444_800 });
      case "account_export": return json({ result: "account_export", export: {
        account: id(200), username: "synthetic", display_name: "Synthetic player", display_name_at: NOW, tag: 80000,
        source: "registered", legacy_id: null, created_at: NOW, last_sign_in_at: NOW, vault_epoch: 1, document_revision: 1,
        state: "restored", admin: false, recovery: true, reset_until: null, objects: 0, object_bytes: 0, archives: 0,
        sessions: [], terms: [], gateways: [], administers: [], audit: [], complete: true,
      } });
      case "email": return json({ result: "email", state: { mail: false, address: null, confirmed_at: null, pending: null, pending_until: null } });
      case "picture": return body.request?.action === "reports"
        ? json({ result: "my_picture_reports", reports: [] })
        : json({ result: "my_picture", picture: null, notice: null, block: null });
      case "challenges": return json({ result: "challenges", response: body.request?.action === "tower"
        ? { answer: "tower", response: { answer: "export", entries: [], after: null } }
        : { answer: "export", entries: [], after: null } });
      case "tower_award_export": {
        state.pages.push(body.after ?? null);
        if (state.mode === "unavailable") return route.fulfill({ status: 503, body: "unavailable\n" });
        const receipts = body.after ? [receipt(state.mode === "repeated" ? 128 : 129)] : Array.from({ length: 128 }, (_, i) => receipt(i + 1));
        return json({ result: "tower_award_export", export: { progress, receipts, next: body.after ? null : id(128) } });
      }
      default: return route.fulfill({ status: 401, body: "rejected\n" });
    }
  });
  await page.goto("/en/account/privacy");
  await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
  return state;
}

test("the account download includes both pages of confirmed Echoes receipts and clears on sign-out", async ({ page }) => {
  const state = await setup(page);
  await page.getByRole("button", { name: "Prepare the export" }).click();
  const link = page.locator("#export-save");
  await expect(link).toBeVisible();
  const href = await link.getAttribute("href");
  const file = JSON.parse(decodeURIComponent((href ?? "").split(",").slice(1).join(","))) as {
    tower_echoes: { progress: typeof progress; receipts: unknown[]; complete: boolean; after: null };
  };
  expect(file.tower_echoes).toEqual({ progress, receipts: Array.from({ length: 129 }, (_, i) => receipt(i + 1)), complete: true, after: null });
  expect(state.pages).toEqual([null, id(128)]);
  const sibling = await page.context().newPage();
  try {
    await sibling.goto("/robots.txt");
    await sibling.evaluate(() => localStorage.removeItem("baylee-central-session"));
    await expect(link).toHaveCount(0);
  } finally { await sibling.close(); }
});

for (const mode of ["repeated", "unavailable"]) {
  test(`a ${mode} Echoes export never offers a silently incomplete download`, async ({ page }) => {
    const state = await setup(page);
    state.mode = mode;
    await page.getByRole("button", { name: "Prepare the export" }).click();
    await expect(page.locator("#export-error")).toBeVisible();
    await expect(page.locator("#export-save")).toHaveCount(0);
    state.mode = "valid";
    await page.getByRole("button", { name: "Prepare the export" }).click();
    await expect(page.locator("#export-save")).toBeVisible();
  });
}

for (const hold of ["account_export", "tower_award_export"]) {
  test(`switching accounts during ${hold} invalidates the entire assembled download`, async ({ page }) => {
    const state = await setup(page);
    let release: (() => void) | undefined;
    const pending = new Promise<void>((resolve) => { release = resolve; });
    state.hold = hold;
    state.pause = () => pending;
    await page.getByRole("button", { name: "Prepare the export" }).click();
    await expect.poll(() => state.held).toBe(true);
    await expect(page.getByRole("button", { name: "Prepare the export" })).toBeDisabled();
    const sibling = await page.context().newPage();
    try {
      await sibling.goto("/robots.txt");
      // Two distinct sessions can expire in the same second. Their local change
      // markers still invalidate old work; expiry alone is not a session identity.
      await sibling.evaluate(() => localStorage.setItem("baylee-central-session", JSON.stringify({ expiresAt: 4_102_444_800, change: Array(16).fill(1) })));
      await expect(page.getByRole("button", { name: "Prepare the export" })).toBeEnabled();
      release?.();
      await expect.poll(() => state.delivered).toBe(true);
      await expect(page.locator("#export-save")).toHaveCount(0);
      await expect(page.locator("#export-error")).toHaveCount(0);
    } finally { release?.(); await sibling.close(); }
  });
}
