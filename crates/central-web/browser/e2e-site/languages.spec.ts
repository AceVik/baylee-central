import { expect, test } from "@playwright/test";

import { axe, onlyDesktop, targets, watch } from "./check.ts";

// Full accessibility, layout and per-page matrices run in English (`site.spec.ts`).
// Here every non-English locale gets a lean live-render smoke check on `/` (SSR
// metadata, heading, machine-translation note, single-row header bar, zero console/network
// errors) plus a compact mobile + WCAG spot-check on the hardest scripts (`de`, `ja`, `ru`).
const TAGS: Record<string, string> = {
  de: "de",
  fr: "fr",
  it: "it",
  es: "es",
  "pt-br": "pt-BR",
  ru: "ru",
  ja: "ja",
  ko: "ko",
  "zh-hans": "zh-Hans",
  "zh-hant": "zh-Hant",
};
const NAMES = [
  "English",
  "Deutsch",
  "Français",
  "Italiano",
  "Español",
  "Português",
  "Русский",
  "日本語",
  "한국어",
  "简体中文",
  "繁體中文",
];
const SPOT_SCRIPTS = ["de", "ja", "ru"] as const;
const SPOT_SLUGS = ["", "signin"] as const;

for (const [lang, tag] of Object.entries(TAGS)) {
  test(`${lang}: landing renders cleanly with localized html lang, single-row bar and no overflow`, async ({
    page,
    baseURL,
  }, info) => {
    onlyDesktop(info, "sets its own desktop width for bar geometry");
    const problems = watch(page, baseURL ?? "");
    await page.setViewportSize({ width: 1280, height: 800 });
    const response = await page.goto(`/${lang}/`);
    expect(response?.status()).toBe(200);
    await expect(page.locator("html")).toHaveAttribute("lang", tag);
    await expect(page.locator("h1")).toHaveCount(1);
    expect(((await page.locator("h1").textContent()) ?? "").trim().length).toBeGreaterThan(0);
    await expect(page.locator(".machine-note")).toHaveCount(lang === "de" ? 0 : 1);
    const bar = await page.evaluate(() => {
      const links = [...document.querySelectorAll<HTMLElement>(".nav > a")].filter((link) => link.offsetParent !== null);
      return {
        wide: document.documentElement.scrollWidth - document.documentElement.clientWidth,
        rows: new Set(links.map((link) => Math.round(link.getBoundingClientRect().top))).size,
        tall: Math.max(0, ...links.map((link) => link.getBoundingClientRect().height)),
      };
    });
    expect(bar.wide, `${lang} overflow`).toBeLessThanOrEqual(0);
    expect(bar.rows, `${lang} nav rows`).toBe(1);
    expect(bar.tall, `${lang} wrapped nav label`).toBeLessThan(60);
    expect(problems).toEqual([]);
  });
}

for (const lang of SPOT_SCRIPTS) {
  test(`${lang}: compact mobile WCAG 2.2 AA and tap-target spot check`, async ({ page, baseURL }, info) => {
    onlyDesktop(info, "uses an explicit 375 px phone viewport");
    const problems = watch(page, baseURL ?? "");
    await page.setViewportSize({ width: 375, height: 812 });
    for (const slug of SPOT_SLUGS) {
      await page.goto(`/${lang}/${slug}`);
      const wide = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
      expect(wide, `${lang}/${slug} mobile overflow`).toBeLessThanOrEqual(0);
      expect(await axe(page), `${lang}/${slug}`).toEqual([]);
      expect(await targets(page), `${lang}/${slug}`).toEqual([]);
    }
    expect(problems).toEqual([]);
  });
}

test("the language menu names the eleven languages, each in its own, and each leads to this page", async ({ page }) => {
  await page.goto("/en/faq");
  const menu = page.locator("details.lang");
  await menu.locator("summary").click();
  const links = menu.locator("a.menu-item");
  await expect(links).toHaveCount(NAMES.length);
  for (const name of NAMES) await expect(menu.getByRole("link", { name })).toBeVisible();
  const japanese = menu.getByRole("link", { name: "日本語" });
  await expect(japanese).toHaveAttribute("href", "/ja/faq");
  await expect(japanese).toHaveAttribute("lang", "ja");
  await expect(menu.getByRole("link", { name: "繁體中文" })).toHaveAttribute("hreflang", "zh-Hant");
  // the open menu stays within the window
  const box = await menu.locator(".menu").boundingBox();
  const width = page.viewportSize()?.width ?? 0;
  expect(box && box.x >= 0 && box.x + box.width <= width).toBe(true);
  await japanese.click();
  await expect(page).toHaveURL(/\/ja\/faq$/);
  await expect(page.locator("html")).toHaveAttribute("lang", "ja");
});

test("a browser that asks for Traditional Chinese, Brazilian Portuguese or Russian is sent to it", async ({ browser }) => {
  for (const [locale, lang] of [
    ["zh-TW", "zh-hant"],
    ["zh-CN", "zh-hans"],
    ["pt-PT", "pt-br"],
    ["ru-RU", "ru"],
    ["nl-NL", "en"],
  ]) {
    const context = await browser.newContext({ locale });
    const page = await context.newPage();
    await page.goto("/download");
    await expect(page).toHaveURL(new RegExp(`/${lang}/download$`));
    await context.close();
  }
});
