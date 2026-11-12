// Baylee's album as a book that opens and shuts (home.js, album.css): every
// way in and out, focus coming back, less motion meaning no motion, and
// nothing but transform and opacity moving.
import { expect, test, type Page } from "@playwright/test";

import { axe, watch } from "./check.ts";

/**
 * Presses `press` from within the page and, in the same task or at the
 * first frame its finite animations run, reads every property that any
 * animation under `selector` moves (its pseudo-elements included) and the
 * cover's first and last angle: read so, nothing has ended before it is
 * read, however slow the machine.
 */
async function moved(page: Page, press: string, selector: string): Promise<{ moved: string[]; finite: number; swing: string[] }> {
  return page.evaluate(
    async ([button, under]) => {
      const pressed = document.querySelector(button);
      if (pressed instanceof HTMLElement) pressed.click();
      const root = document.querySelector(under);
      const finite = () => (root?.getAnimations({ subtree: true }) ?? []).filter((run) => run.effect?.getComputedTiming().iterations !== Infinity);
      // the script's own animations (a print may wait a moment to be drawn)
      const scripted = () => finite().some((run) => !(run instanceof CSSAnimation) && !(run instanceof CSSTransition));
      for (let frame = 0; frame < 60 && !scripted(); frame += 1) await new Promise(requestAnimationFrame);
      const found = new Set<string>();
      for (const run of root?.getAnimations({ subtree: true }) ?? []) {
        const effect = run.effect;
        if (!(effect instanceof KeyframeEffect)) continue;
        for (const frame of effect.getKeyframes()) {
          for (const key of Object.keys(frame)) {
            if (!["offset", "computedOffset", "easing", "composite"].includes(key)) found.add(key);
          }
        }
      }
      const swing = (document.querySelector("#album .album-cover")?.getAnimations() ?? []).flatMap((run) =>
        run.effect instanceof KeyframeEffect ? run.effect.getKeyframes().map((frame) => String(frame["transform"])) : [],
      );
      return { moved: [...found].sort(), finite: finite().length, swing: swing.length > 0 ? [swing[0] ?? "", swing.at(-1) ?? ""] : [] };
    },
    [press, selector] as const,
  );
}

/** How many finite animations run under `selector` (the masthead's drifting light runs for ever). */
async function running(page: Page, selector: string): Promise<number> {
  return page.evaluate(
    (under) =>
      (document.querySelector(under)?.getAnimations({ subtree: true }) ?? []).filter(
        (run) => run.playState === "running" && run.effect?.getComputedTiming().iterations !== Infinity,
      ).length,
    selector,
  );
}

/** Waits until the book is open (`true`) or shut, its cover at rest either way. */
async function settled(page: Page, open: boolean): Promise<void> {
  await page.waitForFunction((shown) => {
    const album = document.querySelector("dialog#album");
    return album instanceof HTMLDialogElement && album.open === shown && !album.hasAttribute("data-folding");
  }, open);
}

type Folds = Window & { folds?: string[] };

/**
 * Every way the book was folding, in order, since the last call: what a
 * check after the fact could miss on a busy machine, the page writes down.
 * "" is the cover come to rest.
 */
async function folds(page: Page): Promise<string[]> {
  return page.evaluate(() => {
    const seen = window as Folds;
    if (!seen.folds) {
      seen.folds = [];
      const album = document.querySelector("dialog#album");
      if (album instanceof HTMLElement) {
        // each change's new value is the next one's old value, the last one's is the attribute now
        new MutationObserver((changes) => {
          changes.forEach((_, at) => seen.folds?.push(changes[at + 1]?.oldValue ?? (at + 1 < changes.length ? "" : (album.dataset["folding"] ?? ""))));
        }).observe(album, { attributes: true, attributeFilter: ["data-folding"], attributeOldValue: true });
      }
    }
    return seen.folds.splice(0);
  });
}

test("the logo opens the album as a book, its button and Escape shut it, and focus comes back to the logo", async ({ page, baseURL }) => {
  // five times in and out, each played out
  test.slow();
  const problems = watch(page, baseURL ?? "");
  await page.goto("/en/");
  const logo = page.locator("button.memorial-open");
  const album = page.locator("dialog#album");
  expect(await folds(page)).toEqual([]);
  // pressed: the book is laid down shut and its cover swings open on the
  // spine, from shut to lying open; only transform and opacity move
  await logo.focus();
  const opening = await moved(page, "button.memorial-open", "#album");
  expect(opening.moved).toEqual(["opacity", "transform"]);
  expect(opening.swing).toEqual(["rotateY(0deg)", "rotateY(-180deg)"]);
  expect(opening.finite).toBeGreaterThanOrEqual(8);
  await expect(album.locator(".album-cover")).toHaveAttribute("aria-hidden", "true");
  // open, the cover is gone and nothing is left moving but the masthead's light
  await settled(page, true);
  expect(await folds(page)).toEqual(["open", ""]);
  await expect(album.locator(".album-cover")).toBeHidden();
  expect(await running(page, "#album")).toBe(0);
  expect(await axe(page)).toEqual([]);
  // its button: the cover comes back over the pages, then the book goes
  const shutting = await moved(page, "#album .album-dismiss", "#album");
  expect(shutting.moved).toEqual(["opacity", "transform"]);
  expect(shutting.swing).toEqual(["rotateY(-180deg)", "rotateY(0deg)"]);
  await settled(page, false);
  expect(await folds(page)).toEqual(["shut", ""]);
  await expect(logo).toBeFocused();
  // the keyboard: Enter on the logo, Escape in the book
  await page.keyboard.press("Enter");
  await settled(page, true);
  expect(await folds(page)).toEqual(["open", ""]);
  await page.keyboard.press("Escape");
  await settled(page, false);
  expect(await folds(page)).toEqual(["shut", ""]);
  await expect(logo).toBeFocused();
  // shut while it is still opening (held halfway here): the same movement
  // goes back from where it was. A reader's press in the book comes first,
  // as it would: an Escape may hold a dialog open only after one.
  await page.evaluate(() => {
    document.querySelector<HTMLElement>("button.memorial-open")?.click();
    for (const run of document.querySelector("#album")?.getAnimations({ subtree: true }) ?? []) {
      if (run.effect?.getComputedTiming().iterations === Infinity) continue;
      run.pause();
      run.currentTime = 450;
    }
  });
  await album.locator("h2").click();
  await page.keyboard.press("Escape");
  await settled(page, false);
  expect(await folds(page)).toEqual(["open", "shut", ""]);
  await expect(logo).toBeFocused();
  // and it opens again as before
  await logo.click();
  await settled(page, true);
  expect(await folds(page)).toEqual(["open", ""]);
  await expect(album.locator("h2")).toHaveText("Baylee");
  expect(problems).toEqual([]);
});

test("with less motion the book opens and shuts at once", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto("/en/");
  const logo = page.locator("button.memorial-open");
  const album = page.locator("dialog#album");
  await logo.click();
  expect(await album.evaluate((dialog) => (dialog as HTMLDialogElement).open && !dialog.hasAttribute("data-folding"))).toBe(true);
  expect(await page.evaluate(() => document.getAnimations().length)).toBe(0);
  await album.locator(".album-dismiss").click();
  expect(await album.evaluate((dialog) => (dialog as HTMLDialogElement).open)).toBe(false);
  await expect(logo).toBeFocused();
  await page.keyboard.press("Enter");
  expect(await album.evaluate((dialog) => (dialog as HTMLDialogElement).open && !dialog.hasAttribute("data-folding"))).toBe(true);
  await page.keyboard.press("Escape");
  expect(await album.evaluate((dialog) => (dialog as HTMLDialogElement).open)).toBe(false);
  await expect(logo).toBeFocused();
});

test("without script the logo still opens the album, plain and every chapter readable", async ({ browser }) => {
  const context = await browser.newContext({ javaScriptEnabled: false });
  const page = await context.newPage();
  await page.goto("/en/");
  await page.locator("button.memorial-open").click();
  const album = page.locator("dialog#album");
  await expect(album).toBeVisible();
  await expect(album.locator(".album-cover")).toHaveCount(0);
  await expect(album.locator('[role="tabpanel"]')).toHaveCount(8);
  for (const chapter of await album.locator('[role="tabpanel"]').all()) await expect(chapter).toBeVisible();
  await context.close();
});
