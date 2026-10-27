import { expect, test } from "@playwright/test";

import { mock } from "./mocks.ts";

for (const theme of ["light", "dark"]) {
  test(`sunlight ${theme}: album, keyboard and stillness`, async ({ page }, info) => {
    const outside = await mock(page);
    await page.emulateMedia({ reducedMotion: "reduce" });
    await page.goto(`/?lang=de&theme=${theme}`);
    const open = page.getByRole("button", { name: "An Baylee erinnern · Fotoalbum öffnen" });
    await expect(open).toBeVisible();
    await expect.poll(() => page.locator(".hero-logo").evaluate((image: HTMLImageElement) => image.naturalWidth)).toBeGreaterThan(0);
    await expect(page.locator(".baylee-cloud > img")).toHaveCSS("animation-name", "none");
    await expect(page.locator(".baylee-cloud > img")).toHaveJSProperty("naturalWidth", 1200);
    await page.locator(".baylee-cloud > img").evaluate((image: HTMLImageElement) => image.decode());
    await page.locator(".hero-logo").evaluate((image: HTMLImageElement) => image.decode());
    await page.screenshot({ path: `screens/sunlight-${info.project.name}-${theme}.png` });
    await open.click();
    const dialog = page.getByRole("dialog", { name: "Baylee", exact: true });
    await expect(dialog).toBeVisible();
    await expect(dialog.getByRole("img")).toHaveAttribute("src", "/img/memorial/baylee-1.jpg");
    await page.keyboard.press("ArrowRight");
    await expect(dialog.getByRole("img")).toHaveAttribute("src", "/img/memorial/baylee-2.jpg");
    await page.keyboard.press("ArrowLeft");
    await page.keyboard.press("ArrowLeft");
    await expect(dialog.getByRole("img")).toHaveAttribute("src", "/img/memorial/baylee-5.jpg");
    await dialog.getByRole("img").evaluate((image: HTMLImageElement) => image.decode());
    await page.screenshot({ path: `screens/album-${info.project.name}-${theme}.png` });
    const book = dialog.getByRole("tabpanel");
    const paragraphs: string[] = [];
    for (let chapter = 0; chapter < 8; chapter++) {
      await dialog.getByRole("tab").nth(chapter).click();
      paragraphs.push(...await book.locator(".story-ink > p:not(.eyebrow)").allTextContents());
    }
    expect(paragraphs).toHaveLength(37);
    expect(paragraphs[0]).toBe("Am 17. November 2025 kam Baylee zu uns nach Hause.");
    expect(paragraphs.at(-1)).toBe("Ruhe in Frieden, meine liebste Baylee.");
    await expect(dialog.getByRole("button", { name: "Nächstes Kapitel", exact: true })).toBeDisabled();
    await book.focus();
    await page.keyboard.press("End");
    await expect(book.getByText("Ruhe in Frieden, meine liebste Baylee.", { exact: true })).toBeInViewport();
    await page.screenshot({ path: `screens/story-end-${info.project.name}-${theme}.png` });
    await dialog.getByRole("tab").nth(4).click();
    expect(await book.evaluate(element => element.scrollTop)).toBe(0);
    await book.focus();
    await page.keyboard.press("End");
    await expect.poll(() => book.evaluate(element => element.scrollHeight - element.clientHeight - element.scrollTop)).toBeLessThan(2);
    const scroll = await book.evaluate(element => element.scrollTop);
    expect(scroll).toBeGreaterThan(0);
    await page.keyboard.press("ArrowRight");
    expect(await book.evaluate(element => element.scrollTop)).toBe(scroll);
    await dialog.getByRole("tab").first().click();
    await page.keyboard.press("ArrowRight");
    await expect(dialog.getByRole("tab").nth(1)).toBeFocused();
    await expect(book.getByRole("heading", { name: "Vertrauen wächst" })).toBeVisible();
    await page.keyboard.press("End");
    await expect(dialog.getByRole("tab").last()).toBeFocused();
    await page.keyboard.press("Home");
    await expect(dialog.getByRole("tab").first()).toBeFocused();
    await page.keyboard.press("Escape");
    await expect(dialog).not.toBeVisible();
    await expect(open).toBeFocused();
    await page.emulateMedia({ reducedMotion: "no-preference" });
    await expect(page.locator(".baylee-cloud > img")).toHaveCSS("animation-name", "cloud-breath");
    if (info.project.name === "desktop") {
      await open.hover();
      await expect(page.locator(".cloud-halo")).toHaveCSS("opacity", "1");
      await page.screenshot({ path: `screens/hover-${info.project.name}-${theme}.png` });
    }
    await open.click();
    await dialog.locator(".album-dismiss").click();
    await expect(dialog).not.toBeVisible();
    await expect(open).toBeFocused();
    await page.goto(`/signin?lang=de&theme=${theme}`);
    await expect(page.locator("#s-name")).toBeVisible();
    if (info.project.name === "desktop") {
      await page.locator(".world-companion").evaluate((image: HTMLImageElement) => image.decode());
    }
    await page.screenshot({ path: `screens/signin-${info.project.name}-${theme}.png` });
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    expect(outside.stray).toEqual([]);
  });
}

test("the complete English story stays readable without WebGL", async ({ page }) => {
  await mock(page);
  await page.setViewportSize({ width: 320, height: 640 });
  await page.addInitScript(() => {
    const original = HTMLCanvasElement.prototype.getContext;
    HTMLCanvasElement.prototype.getContext = function (this: HTMLCanvasElement, ...args: Parameters<typeof original>) {
      if (String(args[0]).startsWith("webgl")) return null;
      return Reflect.apply(original, this, args);
    } as typeof original;
  });
  await page.goto("/?lang=en&theme=light");
  await page.getByRole("button", { name: "Remember Baylee · open her photo album" }).click();
  await expect(page.getByRole("tabpanel")).toContainText("Baylee came home to us on 17 November 2025.");
  await page.getByRole("tab").last().click();
  await expect(page.getByRole("tabpanel")).toContainText("Rest in peace, my dearest Baylee.");
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await page.getByRole("button", { name: "Back to her world" }).click();
  await expect(page.getByRole("dialog")).not.toBeVisible();
});


test("the album shader stops for reduced motion, restores its context and stops on close", async ({ page }) => {
  await mock(page);
  await page.addInitScript(() => {
    let draws = 0;
    Object.defineProperty(window, "memorialDraws", { get: () => draws });
    const original = WebGLRenderingContext.prototype.drawArrays;
    WebGLRenderingContext.prototype.drawArrays = function (...args) {
      draws++;
      return Reflect.apply(original, this, args);
    };
  });
  const draws = () => page.evaluate(() => (window as unknown as { memorialDraws: number }).memorialDraws);
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto("/?lang=de&theme=dark");
  await page.getByRole("button", { name: "An Baylee erinnern · Fotoalbum öffnen" }).click();
  const canvas = page.locator(".album-light");
  await expect(canvas).toHaveAttribute("data-renderer", "webgl");
  await expect.poll(draws).toBeGreaterThan(0);
  await page.waitForTimeout(250); // Let ResizeObserver's initial static draw settle.
  const still = await draws();
  await page.waitForTimeout(200);
  expect(await draws()).toBe(still);
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await expect.poll(draws).toBeGreaterThan(still + 2);
  await canvas.evaluate((element: HTMLCanvasElement) => {
    const extension = element.getContext("webgl")?.getExtension("WEBGL_lose_context");
    if (!extension) throw new Error("WebGL context loss extension unavailable");
    element.addEventListener("webglcontextlost", () => setTimeout(() => extension.restoreContext(), 100), { once: true });
    extension.loseContext();
  });
  await expect(canvas).toHaveAttribute("data-generation", "1");
  await expect(canvas).toHaveAttribute("data-renderer", "webgl");
  await page.keyboard.press("Escape");
  const closed = await draws();
  await page.waitForTimeout(200);
  expect(await draws()).toBe(closed);
});
