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
      await expect(page.locator("a.sign-in")).toHaveAttribute("href", "/de/account");
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
  await expect(album.locator("h2")).toHaveText("For Baylee. Forever in the sunlight.");
  expect(await axe(page)).toEqual([]);
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

test("the phone's More sheet opens, holds the rest and closes with Escape", async ({ page, isMobile }) => {
  test.skip(!isMobile, "phones only");
  await page.goto("/de/");
  const more = page.locator(".dock button[popovertarget=more]");
  await more.click();
  const sheet = page.locator("#more");
  await expect(sheet).toBeVisible();
  await expect(sheet.getByRole("link", { name: "Kontakt" })).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(sheet).toBeHidden();
});
