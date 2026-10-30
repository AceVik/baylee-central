// The main pages at a phone's and a desktop's width, against the build and
// its CSP: they draw, they work, nothing scrolls sideways, and nothing is
// refused or thrown in the console.

import { expect, test } from "@playwright/test";

import { SYNTHETIC } from "../test/fixtures/impressum.ts";
import { expectNoSideScroll, mock, watchErrors } from "./mocks.ts";

test.describe("smoke", () => {
  test("the start page offers the download and the way around", async ({ page }, info) => {
    const errors = watchErrors(page);
    const m = await mock(page);
    await page.goto("/?lang=en");
    await expect(page.getByRole("heading", { level: 1 })).toHaveText("A table that knows the rules.");
    const download = page.locator(".hero .dl .button-big");
    await expect(download).toHaveAttribute("href", /^https:\/\/github\.com\/AceVik\/baylee\/releases\//);
    await expect(page.locator(".hero .dl-meta")).toContainText("v0.1.0-beta.8");
    await expect(page.getByText("passing").first()).toBeVisible();
    await expect(page.locator(".features > li")).toHaveCount(9);
    await expectNoSideScroll(page);

    // the sections, through the menu on a phone
    if (info.project.name === "phone") {
      await page.getByRole("button", { name: "Menu" }).click();
    }
    await page.getByRole("navigation", { name: "Sections" }).getByRole("link", { name: "Download" }).click();
    await expect(page.getByRole("heading", { level: 1, name: "Download" })).toBeVisible();
    await expect(page.locator(".platforms > li")).toHaveCount(6);
    await expectNoSideScroll(page);
    expect(m.stray).toEqual([]);
    expect(errors).toEqual([]);
  });

  test("the card status opens a set and shows a card large", async ({ page }, info) => {
    const errors = watchErrors(page);
    const m = await mock(page);
    await page.goto("/cards?lang=en");
    await expect(page.getByRole("heading", { level: 1, name: "The cards, honestly" })).toBeVisible();
    await expect(page.locator(".number dt .vh").first()).toHaveText("3,219");
    await page.getByText("The Dark", { exact: true }).click();
    const grid = page.getByRole("list", { name: "Cards of The Dark" });
    await expect(grid.getByRole("listitem")).toHaveCount(36);
    await page.getByRole("button", { name: /^Partial/ }).click();
    await expect(grid.getByRole("listitem")).toHaveCount(6);
    if (info.project.name === "phone") {
      // a tap shows the card large first, with the way to Scryfall
      await grid.getByRole("link").first().click();
      const dialog = page.getByRole("dialog");
      await expect(dialog).toContainText("Partly implemented");
      await expect(dialog.getByRole("link", { name: "Artist and printing on Scryfall" })).toHaveAttribute("href", /^https:\/\/scryfall\.com\//);
      await dialog.getByRole("button", { name: "Close" }).click();
      await expect(dialog).toHaveCount(0);
    } else {
      // the keyboard: one stop, Space shows the card large
      await grid.getByRole("link").first().focus();
      await page.keyboard.press("Space");
      const dialog = page.getByRole("dialog");
      await expect(dialog).toContainText("Card image provided by Scryfall, unaltered.");
      await page.keyboard.press("Escape");
      await expect(dialog).toHaveCount(0);
    }
    await expectNoSideScroll(page);
    expect(m.stray).toEqual([]);
    expect(errors).toEqual([]);
  });

  test("the gateway directory lists Central's gateways page by page", async ({ page }) => {
    const errors = watchErrors(page);
    const m = await mock(page, { gateways: 130 });
    await page.goto("/gateways?lang=en");
    await expect(page.getByText("100 gateways listed")).toBeVisible();
    await page.getByRole("button", { name: "Show the next 100" }).click();
    await expect(page.getByText("130 gateways listed")).toBeVisible();
    await expect(page.locator(".gateways > li")).toHaveCount(130);
    // each card opens the client by its deep link and keeps the address to copy
    const first = page.locator(".gateways > li").first();
    await expect(first.getByRole("link", { name: "Open in Baylee" })).toHaveAttribute("href", /^baylee:\/\/gateway\/add\?origin=https%3A%2F%2F/);
    await expect(first.getByRole("button", { name: "Copy address" })).toBeVisible();
    // every control of a card is 44 px high at least (docs/website-design.md §"Accessibility")
    const heights = await first.locator("a, button").evaluateAll((els) => els.map((e) => e.getBoundingClientRect().height));
    expect(heights.length).toBeGreaterThan(2);
    for (const height of heights) expect(height).toBeGreaterThanOrEqual(44);
    expect(m.directory).toHaveLength(2);
    expect(m.directory[0]).toEqual({ operation: "gateway_directory", after: null });
    expect(m.refused).toEqual([]);
    await expectNoSideScroll(page);
    expect(m.stray).toEqual([]);
    expect(errors).toEqual([]);
  });

  test("the Impressum shows the details only after the puzzle, readable but scrambled", async ({ page }) => {
    const errors = watchErrors(page);
    const m = await mock(page);
    await page.goto("/impressum?lang=en");
    await expect(page.locator('meta[name="robots"]')).toHaveAttribute("content", "noindex, noarchive, nosnippet");
    await expect(page.locator("body")).not.toContainText("Mustermann");
    await page.getByRole("button", { name: "Show the details" }).click();
    const address = page.locator(".imp-address");
    await expect(address).toBeVisible();
    // the DOM's text is a jumble ...
    const text = await page.locator("main").evaluate((el) => el.textContent ?? "");
    for (const value of Object.values(SYNTHETIC)) expect(text).not.toContain(value);
    // ... while the screen reads right: the real characters, left to right
    const name = await address.locator(".imp-line").first().evaluate((line) =>
      Array.from(line.querySelectorAll(".scr > span:not(.scr-x)"))
        .map((s) => ({ x: s.getBoundingClientRect().left, c: s.textContent ?? "" }))
        .sort((a, b) => a.x - b.x)
        .map((s) => s.c)
        .join(""),
    );
    expect(name).toBe(SYNTHETIC.name);
    await expect(page.locator("a[href^='mailto:'], a[href^='tel:']")).toHaveCount(0);
    await expectNoSideScroll(page);
    expect(m.stray).toEqual([]);
    expect(errors).toEqual([]);
  });

  test("the contact form sends and thanks; the language stays chosen", async ({ page }) => {
    const errors = watchErrors(page);
    const m = await mock(page);
    await page.goto("/contact?lang=en");
    await page.getByRole("button", { name: "Deutsch" }).click();
    await expect(page.getByRole("heading", { level: 1 })).toHaveText("Schreib uns");
    await page.getByLabel("Nachricht").fill("Eine synthetische Nachricht.");
    await page.getByRole("button", { name: "Senden" }).click();
    await expect(page.getByText("Danke, sie ist unterwegs.")).toBeVisible();
    expect(m.contact).toEqual([{ name: null, email: null, message: "Eine synthetische Nachricht.", language: "de", website: "" }]);
    // remembered without ?lang=
    await page.goto("/privacy");
    await expect(page.getByRole("heading", { level: 1 })).toHaveText("Datenschutzerklärung dieser Website");
    await expect(page.locator("html")).toHaveAttribute("lang", "de");
    await expectNoSideScroll(page);
    expect(m.stray).toEqual([]);
    expect(errors).toEqual([]);
  });

  test("light and dark both draw, and reduced motion hides nothing", async ({ page }) => {
    const errors = watchErrors(page);
    await mock(page);
    await page.emulateMedia({ colorScheme: "light", reducedMotion: "reduce" });
    await page.goto("/?lang=en");
    const bg = await page.evaluate(() => getComputedStyle(document.body).backgroundColor);
    expect(bg).toBe("rgb(245, 242, 233)");
    await page.locator("#source").scrollIntoViewIfNeeded();
    await expect(page.locator(".repo-card")).toBeVisible();
    expect(await page.locator(".reveal").evaluateAll((els) => els.every((e) => getComputedStyle(e).opacity === "1"))).toBe(true);
    await page.emulateMedia({ colorScheme: "dark" });
    expect(await page.evaluate(() => getComputedStyle(document.body).backgroundColor)).toBe("rgb(15, 27, 45)");
    expect(errors).toEqual([]);
  });
});
