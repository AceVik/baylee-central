// The hero's clouds (src/home/HeroShader.tsx): drawn on WebGL 2 where the
// browser has it, the painting alone where not, with no complaint either
// way; the pointer stirs them; under reduced motion they hold still. The
// test browser renders in software, where the page draws a still picture
// unless asked for the live air (`baylee-clouds`).

import { expect, test, type Page } from "@playwright/test";

import { mock } from "./mocks.ts";

function watchErrors(page: Page): string[] {
  const errors: string[] = [];
  page.on("console", (m) => { if (m.type() === "error") errors.push(m.text()); });
  page.on("pageerror", (e) => errors.push(e.message));
  return errors;
}

/** Only the clouds left to see: the painting, its light and the words hidden (through the CSSOM, which the CSP allows). */
async function onlyClouds(page: Page) {
  await page.evaluate(() => {
    for (const el of document.querySelectorAll<HTMLElement>(".sky-painting, .sky-veil, .sky-light, .sky-mist, .shader-beams, .shader-motes, .hero-inner")) el.style.visibility = "hidden";
    const sky = document.querySelector<HTMLElement>(".sky");
    if (sky !== null) sky.style.background = "rgb(111, 159, 216)";
  });
}

async function sweep(page: Page) {
  const box = await page.locator(".hero").boundingBox();
  if (box === null) throw new Error("no hero");
  await page.mouse.move(box.x + box.width * 0.04, box.y + box.height * 0.75);
  for (let i = 1; i <= 24; i++) await page.mouse.move(box.x + box.width * (0.04 + 0.4 * (i / 24)), box.y + box.height * (0.75 - 0.1 * Math.sin((i / 24) * Math.PI)), { steps: 2 });
}

test("the clouds draw, and the pointer stirs them", async ({ page }) => {
  const errors = watchErrors(page);
  const outside = await mock(page);
  await page.addInitScript(() => localStorage.setItem("baylee-clouds", "live"));
  await page.goto("/?lang=en&theme=light");
  const canvas = page.locator(".shader-canvas");
  await expect(canvas).toHaveAttribute("data-renderer", /^(live|none)$/);
  if ((await canvas.getAttribute("data-renderer")) === "live") {
    const sky = page.locator(".sky");
    await onlyClouds(page);
    await page.waitForTimeout(300);
    const before = await sky.screenshot();
    await sweep(page);
    await page.waitForTimeout(250);
    expect((await sky.screenshot()).equals(before)).toBe(false);
  }
  expect(errors).toEqual([]);
  expect(outside.stray).toEqual([]);
});

test("under reduced motion the clouds hold still, also under the pointer", async ({ page }) => {
  const errors = watchErrors(page);
  await mock(page);
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.addInitScript(() => localStorage.setItem("baylee-clouds", "live"));
  await page.goto("/?lang=en&theme=dark");
  const canvas = page.locator(".shader-canvas");
  await expect(canvas).toHaveAttribute("data-renderer", /^(still|none)$/);
  await onlyClouds(page);
  await page.waitForTimeout(300);
  const sky = page.locator(".sky");
  const before = await sky.screenshot({ animations: "disabled" });
  await sweep(page);
  await page.waitForTimeout(600);
  expect((await sky.screenshot({ animations: "disabled" })).equals(before)).toBe(true);
  expect(errors).toEqual([]);
});

test("a software renderer draws one still picture", async ({ page }) => {
  await mock(page);
  await page.goto("/?lang=en&theme=light");
  await expect(page.locator(".shader-canvas")).toHaveAttribute("data-renderer", /^(still|none)$/);
});
