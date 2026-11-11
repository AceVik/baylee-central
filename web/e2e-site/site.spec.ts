import { expect, type Page, test } from "@playwright/test";

import { axe, targets } from "./check.ts";

const PAGES = [
  "",
  "download",
  "gateways",
  "faq",
  "contact",
  "impressum",
  "privacy",
  "terms",
  "fan-content",
  "signin",
  "register",
  "account",
  "account/security",
  "account/chat",
  "account/contacts",
  "account/notifications",
  "account/privacy",
  "nope",
];
const LANGS = ["en", "de"];
const WIDTHS = [320, 375, 768, 1024, 1440, 2560];

/** Fails the test on any console error, uncaught error or request to another origin. */
function watch(page: Page, origin: string, expected404 = false): string[] {
  const problems: string[] = [];
  page.on("console", (message) => {
    // The 404 page's own status is the one error a browser reports there.
    if (expected404 && message.text().includes("status of 404")) return;
    if (message.type() === "error") problems.push(`console: ${message.text()}`);
  });
  page.on("pageerror", (error) => problems.push(`error: ${error.message}`));
  page.on("request", (request) => {
    if (!request.url().startsWith(origin) && !request.url().startsWith("data:")) problems.push(`elsewhere: ${request.url()}`);
  });
  return problems;
}


for (const lang of LANGS) {
  for (const slug of PAGES) {
    for (const scheme of ["light", "dark"] as const) {
      test(`${lang}/${slug} in ${scheme}: WCAG 2.2 AA, nothing from elsewhere, no error`, async ({ page, baseURL }) => {
        const problems = watch(page, baseURL ?? "", slug === "nope");
        await page.emulateMedia({ colorScheme: scheme });
        const response = await page.goto(`/${lang}/${slug}`);
        expect(response?.status()).toBe(slug === "nope" ? 404 : 200);
        await expect(page.locator("h1")).toHaveCount(1);
        expect(await axe(page)).toEqual([]);
        expect(await targets(page)).toEqual([]);
        expect(problems).toEqual([]);
      });
    }
  }
}

test("no page needs sideways scrolling from 320 to 2560 px, portrait or landscape", async ({ page }) => {
  for (const width of WIDTHS) {
    for (const height of [width < 1024 ? 812 : 900, 360]) {
      await page.setViewportSize({ width, height });
      for (const slug of PAGES) {
        await page.goto(`/en/${slug}`);
        const wide = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
        expect(wide, `${slug} at ${width}×${height}`).toBeLessThanOrEqual(0);
      }
    }
  }
});

test("nor while signed in, where the bar names the account (German)", async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem("baylee-central-session", JSON.stringify({ expiresAt: 4_102_444_800 })));
  for (const width of WIDTHS) {
    await page.setViewportSize({ width, height: width < 1024 ? 812 : 900 });
    for (const slug of ["", "download", "account", "account/chat"]) {
      await page.goto(`/de/${slug}`);
      await expect(page.locator("a.nav-account")).toHaveAttribute("href", "/de/account");
      const wide = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
      expect(wide, `${slug} at ${width}`).toBeLessThanOrEqual(0);
    }
  }
});

test("the skip link comes first and moves to the content", async ({ page }) => {
  await page.goto("/en/download");
  await page.keyboard.press("Tab");
  const skip = page.locator("a.skip");
  await expect(skip).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page).toHaveURL(/#main$/);
});

test("the big logo opens Baylee's album, Escape closes it and focus returns", async ({ page }) => {
  await page.goto("/en/");
  const logo = page.locator("button.memorial-open");
  await logo.focus();
  await page.keyboard.press("Enter");
  const album = page.locator("dialog#album");
  await expect(album).toBeVisible();
  await expect(album.locator("h2")).toHaveText("Baylee");
  expect(await axe(page)).toEqual([]);
  // one chapter at a time: the tabs and the arrows turn the pages
  await expect(album.locator('[role="tabpanel"]:visible')).toHaveCount(1);
  await album.locator("#chapter-tab-3").click();
  await expect(album.locator("#chapter-3")).toBeVisible();
  await expect(album.locator("#chapter-1")).toBeHidden();
  await page.keyboard.press("ArrowRight");
  await expect(album.locator("#chapter-tab-4")).toBeFocused();
  await expect(album.locator("[data-chapter-count]")).toHaveText("4 / 8");
  // one photograph at a time, its words beneath it
  await album.locator('[data-photo-step="1"]').first().click();
  await expect(album.locator("[data-caption]")).toHaveText("Baylee looking out from the flowered duvet.");
  // the full photograph, to zoom into; Escape closes it, then the album
  await album.locator(".album-photo-open").click();
  const viewer = page.locator("dialog#photo-viewer");
  await expect(viewer).toBeVisible();
  await expect(viewer.locator("[data-viewer-image]")).toHaveAttribute("src", /memorial\/full\/baylee-2/);
  await viewer.locator('[data-zoom="1"]').click();
  await expect(viewer.locator("[data-zoom-level]")).toHaveText("150 %");
  await page.keyboard.press("Escape");
  await expect(viewer).toBeHidden();
  await page.keyboard.press("Escape");
  await expect(album).toBeHidden();
  await expect(logo).toBeFocused();
});

test("each card number explains itself by keyboard", async ({ page }) => {
  await page.goto("/en/#cards");
  const tip = page.locator("button[popovertarget=tip-generated]");
  await tip.focus();
  await page.keyboard.press("Enter");
  await expect(page.locator("#tip-generated")).toBeVisible();
  await expect(page.locator("#tip-generated")).toContainText("not yet tested by hand");
  await page.keyboard.press("Escape");
  await expect(page.locator("#tip-generated")).toBeHidden();
});

test("the colour switch cycles and is kept", async ({ page }) => {
  await page.emulateMedia({ colorScheme: "light" });
  await page.goto("/en/faq");
  const html = page.locator("html");
  const button = page.locator(".bar [data-theme-switch]");
  test.skip(!(await button.isVisible()), "the bar's switch is for wide screens");
  await button.click();
  await expect(html).toHaveAttribute("data-theme", "light");
  await button.click();
  await expect(html).toHaveAttribute("data-theme", "dark");
  await page.reload();
  await expect(html).toHaveAttribute("data-theme", "dark");
  await button.click();
  await expect(html).not.toHaveAttribute("data-theme", /./);
});

test("the contact form hydrates, checks before it sends, and says what failed", async ({ page }) => {
  await page.goto("/en/contact");
  await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
  await page.getByRole("button", { name: "Send message" }).click();
  await expect(page.getByRole("alert")).toHaveText("Write a message first.");
  await expect(page.locator("#contact-message")).toHaveAttribute("aria-invalid", "true");
  await page.locator("#contact-message").fill("Hello");
  await page.locator("#contact-email").fill("not an address");
  await page.getByRole("button", { name: "Send message" }).click();
  await expect(page.getByRole("alert")).toHaveText("This does not look like an e-mail address.");
  await page.locator("#contact-email").fill("");
  await page.getByRole("button", { name: "Send message" }).click();
  // No feedback service runs here: the form says so and keeps the text.
  await expect(page.getByRole("alert")).toContainText("could not be sent");
  await expect(page.locator("#contact-message")).toHaveValue("Hello");
  // Sent: the form goes, and its confirmation takes the focus.
  await page.route("**/site/contact", (route) => route.fulfill({ status: 201, body: "" }));
  await page.getByRole("button", { name: "Send message" }).click();
  await expect(page.getByRole("heading", { name: "Thank you" })).toBeFocused();
});

test("the Impressum's puzzle fails gracefully without the feedback service", async ({ page }) => {
  await page.goto("/de/impressum");
  await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
  await page.getByRole("button", { name: "Angaben anzeigen" }).click();
  await expect(page.getByRole("alert")).toContainText("konnten nicht geladen werden");
  // the button went while solving; back, it takes the focus again
  await expect(page.getByRole("button", { name: "Angaben anzeigen" })).toBeFocused();
});

test("on a phone the menu button opens the sections, and Escape closes them", async ({ page, isMobile }) => {
  test.skip(!isMobile, "phones only");
  await page.goto("/de/");
  const nav = page.locator("#nav");
  await expect(nav).toBeHidden();
  const button = page.locator(".menu-btn");
  await button.click();
  await expect(nav).toBeVisible();
  await expect(nav.getByRole("link", { name: "Kontakt" })).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(nav).toBeHidden();
  await expect(button).toBeFocused();
});

test("the language menu opens from the bar, offers this page in German, and closes with Escape", async ({ page }) => {
  await page.goto("/en/download");
  const menu = page.locator("details.lang");
  await menu.locator("summary").click();
  const german = menu.getByRole("link", { name: /Deutsch/ });
  await expect(german).toBeVisible();
  await expect(german).toHaveAttribute("href", "/de/download");
  await page.keyboard.press("Escape");
  await expect(german).toBeHidden();
  await expect(menu.locator("summary")).toBeFocused();
});

test("the download offers every other system from its menu", async ({ page }) => {
  await page.goto("/en/");
  const more = page.locator(".dl-more");
  await more.locator("summary").click();
  await expect(more.locator(".menu-item")).not.toHaveCount(0);
  for (const href of await more.locator(".menu-item").evaluateAll((all) => all.map((a) => a.getAttribute("href")))) {
    expect(href).toMatch(/^https:\/\/github\.com\//);
  }
  // a press outside closes it
  await page.locator("h1").click();
  await expect(more.locator(".menu")).toBeHidden();
});

test("a screenshot opens large, the arrows step, and Escape closes it", async ({ page }) => {
  await page.goto("/en/");
  const shots = page.locator(".shot-open");
  // the screenshots come from a Baylee checkout (BAYLEE_SRC); a build without one has none
  test.skip((await shots.count()) === 0, "built without Baylee's screenshots");
  await shots.nth(1).scrollIntoViewIfNeeded();
  await shots.nth(1).click();
  const viewer = page.locator("dialog#shot-viewer");
  await expect(viewer).toBeVisible();
  await expect(viewer.locator("[data-shot-image]")).toHaveAttribute("src", /shot-game-log-1200/);
  await expect(viewer.locator("[data-shot-caption] strong")).toHaveText("The game log");
  await page.keyboard.press("ArrowRight");
  await expect(viewer.locator("[data-shot-image]")).toHaveAttribute("src", /shot-table-ring-1200/);
  await viewer.locator('[data-shot-step="-1"]').click();
  await page.keyboard.press("ArrowLeft");
  await expect(viewer.locator("[data-shot-image]")).toHaveAttribute("src", /shot-table-duel-1200/);
  expect(await axe(page)).toEqual([]);
  await page.keyboard.press("Escape");
  await expect(viewer).toBeHidden();
  await expect(shots.nth(1)).toBeFocused();
});

test("a gateway's address is copied from its card, and the page says so", async ({ page, context }) => {
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  await page.goto("/en/");
  const card = page.locator(".gw").first();
  const copy = card.locator("button[data-copy]");
  await expect(copy).toBeVisible();
  await copy.click();
  await expect(copy).toHaveAttribute("data-copied", "");
  await expect(copy).toHaveText("Copied");
  await expect(page.getByRole("status").filter({ hasText: "Address copied." })).toHaveCount(1);
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(await copy.getAttribute("data-copy"));
  await expect(copy).not.toHaveAttribute("data-copied", "", { timeout: 4000 });
  await expect(copy).toHaveText("Copy address");
});

test("the start page fetches no WebAssembly until its form comes near, then the form takes over", async ({ page }) => {
  const wasm: string[] = [];
  page.on("request", (request) => {
    if (request.url().endsWith(".wasm")) wasm.push(request.url());
  });
  await page.goto("/en/");
  await page.waitForLoadState("load");
  await page.waitForTimeout(1200);
  expect(wasm).toEqual([]);
  await page.locator("#contact").scrollIntoViewIfNeeded();
  await expect(page.locator("html")).toHaveAttribute("data-islands", "ready", { timeout: 15_000 });
  expect(wasm).toHaveLength(1);
  // hydrated: an empty message is refused in place, nothing is sent
  await page.locator("#contact button[type=submit]").click();
  await expect(page.locator("#contact-error")).not.toBeEmpty();
});
