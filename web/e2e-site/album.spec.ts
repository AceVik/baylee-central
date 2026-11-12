// Baylee's album as a book that opens and shuts, and its full photograph
// turning from print to print (home.js, album.css): every way in and out,
// every photograph by every way of turning, focus coming back, less motion
// meaning no motion, and nothing but transform and opacity moving.
import { expect, test, type Locator, type Page } from "@playwright/test";

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

async function openBook(page: Page): Promise<Locator> {
  await page.locator("button.memorial-open").click();
  await settled(page, true);
  return page.locator("dialog#album");
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

/**
 * A finger's stroke over the full photograph, from and to a fraction of
 * the frame's width, made of pointer events as a touch screen sends them,
 * a frame apart; `rest` holds the finger still before it lifts, so that
 * the stroke ends without a flick.
 */
async function stroke(page: Page, from: number, to: number, rest = 0, down = 0): Promise<void> {
  await page.locator("#photo-viewer [data-viewport]").evaluate(
    async (area, [left, right, still, fall]) => {
      const box = area.getBoundingClientRect();
      const y = box.top + box.height / 2;
      const start = box.left + box.width * left;
      const end = box.left + box.width * right;
      const fire = (type: string, x: number, dy: number) =>
        area.dispatchEvent(
          new PointerEvent(type, {
            bubbles: true,
            cancelable: true,
            pointerId: 41,
            pointerType: "touch",
            isPrimary: true,
            button: type === "pointermove" ? -1 : 0,
            buttons: type === "pointerup" ? 0 : 1,
            clientX: x,
            clientY: y + dy,
          }),
        );
      fire("pointerdown", start, 0);
      for (let step = 1; step <= 10; step += 1) {
        await new Promise((done) => setTimeout(done, 16));
        fire("pointermove", start + ((end - start) * step) / 10, (fall * step) / 10);
      }
      if (still > 0) await new Promise((done) => setTimeout(done, still));
      fire("pointerup", end, fall);
    },
    [from, to, rest, down] as const,
  );
}

/** The photograph the viewer shows, by its count and its one image, once a turn has ended. */
async function showing(viewer: Locator, number: number): Promise<void> {
  await expect(viewer.locator("[data-photo-count]")).toHaveText(`${number} / 10`);
  await expect(viewer.locator("[data-viewport]")).not.toHaveAttribute("data-turning");
  await expect(viewer.locator("[data-viewer-image]")).toHaveCount(1);
  await expect(viewer.locator("[data-viewer-image]")).toHaveAttribute("src", new RegExp(`memorial/full/baylee-${number}\\.`));
  await expect(viewer.locator(".photo-print:not([hidden])")).toHaveCount(1);
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

/** The album open and its full photograph opened from the photograph's frame. */
async function openViewer(page: Page): Promise<{ album: Locator; frame: Locator; viewer: Locator }> {
  await page.goto("/en/");
  const album = await openBook(page);
  const frame = album.locator(".album-photo-open");
  await frame.click();
  const viewer = page.locator("dialog#photo-viewer");
  await expect(viewer).toBeVisible();
  await showing(viewer, 1);
  return { album, frame, viewer };
}

/** Where a box lies, to the pixel (a dialog's own way in may leave a hair). */
async function spot(locator: Locator): Promise<number[]> {
  const box = await locator.boundingBox();
  return box ? [box.x, box.y, box.width, box.height].map(Math.round) : [];
}

test("the full photograph's buttons turn to every picture both ways, only transform and opacity moving and the frame staying put", async ({ page, baseURL }) => {
  // ten turns and more, each played out
  test.slow();
  const problems = watch(page, baseURL ?? "");
  const { frame, viewer } = await openViewer(page);
  const tools = viewer.locator(".photo-viewer-tools");
  const area = viewer.locator("[data-viewport]");
  const before = [await spot(tools), await spot(area)];
  // mid-turn only transform and opacity move, and the frame and the
  // controls stay where they were
  const turn = await moved(page, '#photo-viewer [data-photo-step="1"]', "#photo-viewer");
  expect(turn.moved).toEqual(["opacity", "transform"]);
  expect(turn.finite).toBeGreaterThanOrEqual(3);
  expect([await spot(tools), await spot(area)]).toEqual(before);
  await showing(viewer, 2);
  // the next button, round all ten and to the first again
  const next = viewer.locator('[data-photo-step="1"]');
  for (let number = 3; number <= 10; number += 1) {
    await next.click();
    await showing(viewer, number);
  }
  await next.click();
  await showing(viewer, 1);
  // the previous button, back past the first to the last and on
  const previous = viewer.locator('[data-photo-step="-1"]');
  await previous.click();
  await showing(viewer, 10);
  await previous.click();
  await showing(viewer, 9);
  // the close button gives focus back to the frame that opened the viewer
  await viewer.locator(".photo-viewer-header button").click();
  await expect(viewer).toBeHidden();
  await expect(frame).toBeFocused();
  expect(problems).toEqual([]);
});

test("the arrow keys turn the full photograph to every picture, a key pressed mid-turn ends it and turns on, and Escape gives focus back", async ({ page, baseURL }) => {
  // ten turns and more, each played out
  test.slow();
  const problems = watch(page, baseURL ?? "");
  const { album, frame, viewer } = await openViewer(page);
  await viewer.locator('[data-zoom="0"]').focus();
  for (let number = 2; number <= 10; number += 1) {
    await page.keyboard.press("ArrowRight");
    await showing(viewer, number);
  }
  await page.keyboard.press("ArrowRight");
  await showing(viewer, 1);
  await page.keyboard.press("ArrowLeft");
  await page.keyboard.press("ArrowLeft");
  await showing(viewer, 9);
  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("ArrowRight");
  await showing(viewer, 1);
  // the album beneath followed
  await expect(album.locator("[data-caption]")).toHaveText(await viewer.locator("[data-viewer-caption]").innerText());
  await page.keyboard.press("Escape");
  await expect(viewer).toBeHidden();
  await expect(frame).toBeFocused();
  expect(problems).toEqual([]);
});

test("a finger turns the full photograph: far enough or flicked it turns either way, short of it or up and down it falls back, zoomed in it moves the photograph", async ({ page, baseURL }) => {
  // a dozen strokes, each turn played out
  test.slow();
  const problems = watch(page, baseURL ?? "");
  const { frame, viewer } = await openViewer(page);
  // to the left, the next photograph comes in from the right
  await stroke(page, 0.85, 0.2);
  await showing(viewer, 2);
  // to the right, the one shown is lifted off and the one before shows
  await stroke(page, 0.15, 0.8);
  await showing(viewer, 1);
  await stroke(page, 0.15, 0.8);
  await showing(viewer, 10);
  // every picture can be reached by the finger alone
  for (let number = 1; number <= 9; number += 1) {
    await stroke(page, 0.85, 0.2);
    await showing(viewer, number);
  }
  // short of the way and held still before lifting: it falls back
  await stroke(page, 0.6, 0.5, 160);
  await showing(viewer, 9);
  // up and down is no turn
  await stroke(page, 0.5, 0.52, 0, 200);
  await showing(viewer, 9);
  // zoomed in, a finger moves the photograph and does not turn it
  await viewer.locator('[data-zoom="1"]').click();
  await expect(viewer.locator("[data-zoom-level]")).toHaveText("150 %");
  await stroke(page, 0.8, 0.3);
  await showing(viewer, 9);
  await page.keyboard.press("Escape");
  await expect(viewer).toBeHidden();
  await expect(frame).toBeFocused();
  expect(problems).toEqual([]);
});


test("with less motion the book opens and shuts at once and the photographs change at once", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto("/en/");
  const logo = page.locator("button.memorial-open");
  const album = page.locator("dialog#album");
  await logo.click();
  expect(await album.evaluate((dialog) => (dialog as HTMLDialogElement).open && !dialog.hasAttribute("data-folding"))).toBe(true);
  expect(await page.evaluate(() => document.getAnimations().length)).toBe(0);
  await album.locator(".album-photo-open").click();
  const viewer = page.locator("dialog#photo-viewer");
  await expect(viewer).toBeVisible();
  await viewer.locator('[data-photo-step="1"]').click();
  const now = await viewer.evaluate((dialog) => ({
    count: dialog.querySelector("[data-photo-count]")?.textContent,
    turning: dialog.querySelector("[data-viewport]")?.hasAttribute("data-turning"),
    prints: dialog.querySelectorAll(".photo-print:not([hidden])").length,
    moving: document.getAnimations().length,
  }));
  expect(now).toEqual({ count: "2 / 10", turning: false, prints: 1, moving: 0 });
  await page.keyboard.press("ArrowRight");
  await expect(viewer.locator("[data-photo-count]")).toHaveText("3 / 10");
  expect(await page.evaluate(() => document.getAnimations().length)).toBe(0);
  // a finger turns as it lifts, and nothing followed it before
  await stroke(page, 0.85, 0.2);
  await expect(viewer.locator("[data-photo-count]")).toHaveText("4 / 10");
  expect(await viewer.locator(".photo-print:not([hidden])").count()).toBe(1);
  await page.keyboard.press("Escape");
  await expect(viewer).toBeHidden();
  await album.locator(".album-dismiss").click();
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
  const shown = await album.evaluate((dialog) => [...dialog.querySelectorAll('[role="tabpanel"]')].map((chapter) => chapter.checkVisibility()));
  expect(shown).toEqual(Array.from({ length: 8 }, () => true));
  await context.close();
});
