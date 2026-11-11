import { expect, test } from "@playwright/test";

import { axe, targets, watch } from "./check.ts";

// The nine languages a machine translated, and German beside them: every
// page holds at every width in every one of them (their words are longer
// or shorter than the English ones the other suite measures), the pages a
// visitor meets first pass WCAG 2.2 AA there too, and each says in its
// footer that a machine wrote it.
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
  "Português (Brasil)",
  "Русский",
  "日本語",
  "한국어",
  "简体中文",
  "繁體中文",
];
const PAGES = [
  "",
  "download",
  "gateways",
  "faq",
  "security",
  "contact",
  "impressum",
  "privacy",
  "terms",
  "fan-content",
  "signin",
  "register",
  "account",
  "account/chat",
  "account/notifications",
  "nope",
];
const FIRST = ["", "download", "gateways", "faq", "signin", "privacy", "terms"];
const WIDTHS = [320, 375, 768, 1440];

for (const [lang, tag] of Object.entries(TAGS)) {
  test(`${lang}: no page needs sideways scrolling from 320 to 1440 px`, async ({ page }) => {
    test.slow();
    for (const width of WIDTHS) {
      await page.setViewportSize({ width, height: width < 1024 ? 812 : 900 });
      for (const slug of PAGES) {
        await page.goto(`/${lang}/${slug}`);
        const wide = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
        expect(wide, `${slug} at ${width}`).toBeLessThanOrEqual(0);
      }
    }
  });

  if (lang === "de") continue;
  test(`${lang}: the first pages pass WCAG 2.2 AA, load nothing from elsewhere and say a machine translated them`, async ({
    page,
    baseURL,
  }) => {
    test.slow();
    const problems = watch(page, baseURL ?? "");
    for (const slug of FIRST) {
      await page.goto(`/${lang}/${slug}`);
      await expect(page.locator("html")).toHaveAttribute("lang", tag);
      await expect(page.locator("h1")).toHaveCount(1);
      await expect(page.locator(".machine-note")).toHaveCount(1);
      expect(await axe(page), slug).toEqual([]);
      expect(await targets(page), slug).toEqual([]);
    }
    expect(problems).toEqual([]);
  });
}

test("the bar's sections stand in one row beside each other from where they unfold, in every language", async ({ page }) => {
  test.slow();
  for (const lang of Object.keys(TAGS)) {
    await page.goto(`/${lang}/`);
    for (const width of [1080, 1081, 1140, 1279, 1280]) {
      await page.setViewportSize({ width, height: 800 });
      const bar = await page.evaluate(() => {
        const links = [...document.querySelectorAll<HTMLElement>(".nav > a")].filter((link) => link.offsetParent !== null);
        return {
          wide: document.documentElement.scrollWidth - document.documentElement.clientWidth,
          rows: new Set(links.map((link) => Math.round(link.getBoundingClientRect().top))).size,
          tall: Math.max(0, ...links.map((link) => link.getBoundingClientRect().height)),
        };
      });
      expect(bar.wide, `${lang} at ${width}`).toBeLessThanOrEqual(0);
      if (width > 1080) {
        expect(bar.rows, `${lang} at ${width}: rows`).toBe(1);
        expect(bar.tall, `${lang} at ${width}: a label on two lines`).toBeLessThan(60);
      }
    }
  }
});

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
