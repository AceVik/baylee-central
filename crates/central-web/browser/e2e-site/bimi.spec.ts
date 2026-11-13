import { readFile } from "node:fs/promises";
import { expect, test } from "@playwright/test";

// The BIMI mark (docs/proposals/frontend-rewrite.md §23): mail providers
// fetch it at its fixed address, so it answers there as it is, and every
// page shows the same file, since a mark certificate asks that it was
// public on the domain for a year.

const SOURCE = new URL("../../assets/bimi/baylee.svg", import.meta.url);

test("the mark answers at /bimi/baylee.svg as an SVG with the approved bytes, without a redirect", async ({ request }) => {
  const answer = await request.get("/bimi/baylee.svg", { maxRedirects: 0 });
  expect(answer.status()).toBe(200);
  expect(answer.headers()["content-type"]).toBe("image/svg+xml");
  expect(answer.headers()["set-cookie"]).toBeUndefined();
  expect(Buffer.compare(await answer.body(), await readFile(SOURCE))).toBe(0);
  for (const encoding of ["br", "gzip"]) {
    const compressed = await request.get("/bimi/baylee.svg", { maxRedirects: 0, headers: { "accept-encoding": encoding } });
    expect(compressed.status()).toBe(200);
    // the client decodes it: the bytes are the same
    expect(Buffer.compare(await compressed.body(), await readFile(SOURCE))).toBe(0);
  }
  // no language before it
  expect((await request.get("/en/bimi/baylee.svg", { maxRedirects: 0 })).status()).not.toBe(200);
});

for (const path of ["/en/", "/de/", "/ja/download", "/fr/faq", "/en/account"]) {
  test(`the footer shows the mark, loaded and at least 64 px: ${path}`, async ({ page }) => {
    await page.goto(path);
    const mark = page.locator("footer img.foot-mark");
    await expect(mark).toHaveCount(1);
    await expect(mark).toHaveAttribute("src", "/bimi/baylee.svg");
    await expect(mark).toHaveAttribute("alt", "Baylee");
    await mark.scrollIntoViewIfNeeded();
    await expect(mark).toBeVisible();
    await expect.poll(() => mark.evaluate((img: HTMLImageElement) => img.complete && img.naturalWidth > 0)).toBe(true);
    const box = await mark.boundingBox();
    expect(box?.width ?? 0).toBeGreaterThanOrEqual(64);
    expect(box?.height ?? 0).toBeGreaterThanOrEqual(64);
    // the sunlight wordmark stays beside it
    await expect(page.locator("footer img.foot-logo")).toBeVisible();
  });
}
