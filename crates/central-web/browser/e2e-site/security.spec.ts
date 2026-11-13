// The security page (docs/security-page.md): four diagrams in steps, drawn
// by the server and moved by the stylesheet alone. Each has a switch of six
// radio buttons (play, or one of five steps) and the same steps as a list
// beside it. What is checked here is what no Rust test can see: the page in
// a browser, by keyboard, at rest for whoever asked for stillness, and at
// every width.
import { expect, type Locator, type Page, test } from "@playwright/test";

import { axe, targets } from "./check.ts";

const TITLES = {
  en: "Passwords, keys and chats: what goes where",
  de: "Passwort, Schlüssel und Chats: was wohin geht",
} as const;
const LANGS = ["en", "de"] as const;
const FLOWS = ["signin", "memory", "chat", "pairing"];
const WIDTHS = [320, 375, 768, 1024, 1440, 2560];

/** Console errors, uncaught errors and requests to another origin. */
function watch(page: Page, origin: string): string[] {
  const problems: string[] = [];
  page.on("console", (message) => {
    if (message.type() === "error") problems.push(`console: ${message.text()}`);
  });
  page.on("pageerror", (error) => problems.push(`error: ${error.message}`));
  page.on("request", (request) => {
    if (!request.url().startsWith(origin) && !request.url().startsWith("data:")) problems.push(`elsewhere: ${request.url()}`);
  });
  return problems;
}

const animation = (part: Locator) => part.evaluate((element) => getComputedStyle(element).animationName);

for (const lang of LANGS) {
  for (const scheme of ["light", "dark"] as const) {
    test(`${lang}/security in ${scheme}: one h1, four diagrams with their steps in words, WCAG 2.2 AA`, async ({ page, baseURL }) => {
      const problems = watch(page, baseURL ?? "");
      await page.emulateMedia({ colorScheme: scheme });
      const response = await page.goto(`/${lang}/security`);
      expect(response?.status()).toBe(200);
      await expect(page.locator("h1")).toHaveCount(1);
      await expect(page.locator("h1")).toHaveText(TITLES[lang]);
      for (const flow of FLOWS) {
        const figure = page.locator(`#${flow} .sec-fig`);
        await expect(figure).toHaveCount(1);
        // the diagram is a picture of the list beside it
        await expect(figure.locator("svg.sec-chart")).toHaveAttribute("aria-hidden", "true");
        await expect(figure.locator("ol.sec-list > li")).toHaveCount(5);
        await expect(figure.getByRole("radio")).toHaveCount(6);
        // every flow says where it ends
        await expect(page.locator(`#${flow} .sec-note.sec-warn`).first()).toBeVisible();
      }
      await expect(page.locator("#limits .sec-limits > li")).toHaveCount(5);
      expect(await axe(page)).toEqual([]);
      expect(await targets(page)).toEqual([]);
      expect(problems).toEqual([]);
    });
  }
}

test("a diagram plays by itself; the keyboard chooses a step, which stops it there", async ({ page }) => {
  await page.goto("/en/security");
  const figure = page.locator("#signin .sec-fig");
  const play = figure.getByRole("radio", { name: "Play" });
  await expect(play).toBeChecked();
  expect(await animation(figure.locator(".sec-r1 .sec-art"))).toBe("sec-spot");
  expect(await animation(figure.locator(".sec-r1 .sec-token"))).toBe("sec-carry");

  await play.focus();
  await page.keyboard.press("ArrowRight");
  const first = figure.getByRole("radio", { name: "Step 1" });
  await expect(first).toBeFocused();
  await expect(first).toBeChecked();
  // the radio button itself is out of sight: the ring is around its label
  await expect(figure.locator("label.sec-l1")).toHaveCSS("outline-style", "solid");
  // the loop has stopped; the step is lit, the others stand back
  expect(await animation(figure.locator(".sec-r2 .sec-art"))).toBe("none");
  await expect(figure.locator(".sec-r1 .sec-art")).toHaveCSS("opacity", "1");
  await expect(figure.locator(".sec-r2 .sec-art")).toHaveCSS("opacity", "0.4");
  // its value travels once and stays where it arrived
  expect(await animation(figure.locator(".sec-r1 .sec-token"))).toBe("sec-once");

  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("ArrowRight");
  await expect(figure.getByRole("radio", { name: "Step 3" })).toBeChecked();
  await expect(figure.locator(".sec-r3 .sec-art")).toHaveCSS("opacity", "1");
  await expect(figure.locator(".sec-r1 .sec-art")).toHaveCSS("opacity", "0.4");

  // the other diagrams are untouched, and the first choice plays again
  await expect(page.locator("#chat .sec-fig").getByRole("radio", { name: "Play" })).toBeChecked();
  await page.keyboard.press("ArrowLeft");
  await page.keyboard.press("ArrowLeft");
  await page.keyboard.press("ArrowLeft");
  await expect(play).toBeChecked();
  expect(await animation(figure.locator(".sec-r2 .sec-art"))).toBe("sec-spot");
});

test("a step is chosen by its label, a target of 44 px", async ({ page }) => {
  await page.goto("/de/security");
  const figure = page.locator("#chat .sec-fig");
  const label = figure.locator("label.sec-l4");
  const box = await label.boundingBox();
  expect(box?.width).toBeGreaterThanOrEqual(43.5);
  expect(box?.height).toBeGreaterThanOrEqual(43.5);
  await label.click();
  await expect(figure.getByRole("radio", { name: "Schritt 4" })).toBeChecked();
  await expect(figure.locator(".sec-r4 .sec-art")).toHaveCSS("opacity", "1");
  await expect(figure.locator(".sec-r5 .sec-art")).toHaveCSS("opacity", "0.4");
});

test("with reduced motion nothing moves: every diagram is whole and at rest", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto("/de/security");
  for (const flow of FLOWS) {
    const figure = page.locator(`#${flow} .sec-fig`);
    for (const drawing of await figure.locator(".sec-row .sec-art").all()) {
      expect(await animation(drawing)).toBe("none");
      await expect(drawing).toHaveCSS("opacity", "1");
    }
    for (const value of await figure.locator(".sec-token").all()) {
      expect(await animation(value)).toBe("none");
      await expect(value).toHaveCSS("opacity", "1");
      await expect(value).toHaveCSS("transform", "none");
    }
    // the first button says what it does now
    await expect(figure.locator("label.sec-l0 .sec-when-still")).toBeVisible();
    await expect(figure.locator("label.sec-l0 .sec-when-moving")).toBeHidden();
    await expect(figure.getByRole("radio", { name: "Alle Schritte" })).toBeChecked();
  }
  // a chosen step is lit, and still nothing moves
  const figure = page.locator("#memory .sec-fig");
  await figure.locator("label.sec-l1").click();
  await expect(figure.locator(".sec-r1 .sec-art")).toHaveCSS("opacity", "1");
  await expect(figure.locator(".sec-r2 .sec-art")).toHaveCSS("opacity", "0.4");
  expect(await animation(figure.locator(".sec-r1 .sec-token"))).toBe("none");
  await expect(figure.locator(".sec-r1 .sec-token")).toHaveCSS("transform", "none");
});

test("a diagram holds no sentence: only its actors' names, which fit, and the steps' numbers", async ({ page }) => {
  // SVG text neither wraps nor shrinks, and the site is read in many
  // languages: what a step sends and means is HTML in the list beside it
  for (const lang of LANGS) {
    await page.goto(`/${lang}/security`);
    await page.evaluate(() => document.fonts.ready);
    const wrong = await page.evaluate(() => {
      const found: string[] = [];
      for (const chart of document.querySelectorAll<SVGSVGElement>("svg.sec-chart")) {
        const width = chart.viewBox.baseVal.width;
        for (const text of chart.querySelectorAll<SVGTextElement>("text")) {
          const words = (text.textContent ?? "").trim();
          if (text.closest(".sec-num")) {
            if (!/^[1-5]$/.test(words)) found.push(`a number says "${words}"`);
            continue;
          }
          if (!text.classList.contains("sec-name")) found.push(`other text: "${words}"`);
          if (words.split(/\s+/).length > 2) found.push(`more than two words: "${words}"`);
          const box = text.getBBox();
          if (box.x < 2 || box.x + box.width > width - 2) found.push(`"${words}" leaves the chart`);
        }
        // the names stand side by side
        const names = [...chart.querySelectorAll<SVGTextElement>(".sec-name")].map((name) => name.getBBox()).sort((a, b) => a.x - b.x);
        names.forEach((name, at) => {
          const before = names[at - 1];
          if (before && name.x < before.x + before.width + 4) found.push(`names ${at} and ${at + 1} touch`);
        });
      }
      return found;
    });
    expect(wrong, lang).toEqual([]);
    // each step's sign and what it sends are in the list, as HTML
    await expect(page.locator(".sec-list .sec-what .sec-sign")).toHaveCount(20);
  }
});

test("no sideways scrolling from 320 to 2560 px, in both languages", async ({ page }) => {
  test.slow();
  for (const width of WIDTHS) {
    for (const height of [width < 1024 ? 812 : 900, 360]) {
      await page.setViewportSize({ width, height });
      for (const lang of LANGS) {
        await page.goto(`/${lang}/security`);
        const wide = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
        expect(wide, `${lang} at ${width}×${height}`).toBeLessThanOrEqual(0);
      }
    }
  }
});

test("the footer, the questions and the sign-in page lead here", async ({ page }) => {
  await page.goto("/en/");
  await expect(page.locator('footer a[href="/en/security"]')).toHaveText("Security");
  await page.goto("/en/faq");
  await expect(page.locator('main a[href="/en/security"]')).toHaveCount(1);
  await page.goto("/de/signin");
  await page.locator('main a[href="/de/security"]').click();
  await expect(page).toHaveURL(/\/de\/security$/);
  await expect(page.locator("h1")).toHaveText(TITLES.de);
});
