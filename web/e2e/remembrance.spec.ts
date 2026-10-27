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
