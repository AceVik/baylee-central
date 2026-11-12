// The checks every page of the site passes, shared by the suites.
import { AxeBuilder } from "@axe-core/playwright";
import type { Page } from "@playwright/test";

/**
 * axe-core with every WCAG 2.2 A/AA rule and its best practices. Its
 * `target-size` rule is replaced by `targets` below: axe counts a target as
 * too small while the phone's sticky bottom bar covers it, which only lasts
 * until the page scrolls (focus is never hidden: `scroll-padding`); the
 * owner's rule, 44 × 44 px for every control, is stricter than WCAG's 24 px
 * and is measured without that overlap.
 */
export async function axe(page: Page): Promise<string[]> {
  const result = await new AxeBuilder({ page })
    .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa", "best-practice"])
    .disableRules(["target-size"])
    .analyze();
  return result.violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target.join(" ")).join(", ")}`);
}

/**
 * `axe` in the dark and in the light scheme, for a page in a state no
 * address leads to (a form an answer changed, a message that came back):
 * the scheme is switched under the page as it stands. Playwright's own
 * scheme is light, so a check that sets none never sees the dark one. The
 * colours cross-fade when the scheme changes, so they are measured with
 * motion reduced, where nothing fades (`base.css`); the page is left light
 * and moving, as a test found it.
 */
export async function schemes(page: Page): Promise<string[]> {
  const found: string[] = [];
  const inks: string[] = [];
  for (const scheme of ["dark", "light"] as const) {
    await page.emulateMedia({ colorScheme: scheme, reducedMotion: "reduce" });
    // a colour still fading would be measured in between: with reduced motion nothing fades
    const fading = await page.evaluate(() => document.getAnimations().length);
    if (fading > 0) found.push(`${scheme}: ${fading} animations run with reduced motion asked for`);
    inks.push(await page.evaluate(() => getComputedStyle(document.body).color));
    found.push(...(await axe(page)).map((violation) => `${scheme}: ${violation}`));
  }
  await page.emulateMedia({ colorScheme: "light", reducedMotion: null });
  if (inks[0] === inks[1]) found.push(`the page's ink is ${inks[0]} in both schemes: it did not follow them`);
  return found;
}

/** Every visible control smaller than 44 × 44 CSS px, except links inside running text. */
export async function targets(page: Page): Promise<string[]> {
  return page.evaluate(() => {
    const small: string[] = [];
    for (const element of document.querySelectorAll<HTMLElement>("a[href], button, input, textarea, select, summary")) {
      const rect = element.getBoundingClientRect();
      if (rect.width === 0 || rect.height === 0 || getComputedStyle(element).visibility === "hidden") continue;
      // Content of a closed <details> keeps a layout size but is never drawn or reached.
      if (!element.checkVisibility({ contentVisibilityAuto: true, visibilityProperty: true })) continue;
      // WCAG 2.5.8's inline exception: a link in a sentence.
      const sentence = "p, li, dd, td, figcaption, blockquote, label";
      if (element.matches("a") && element.closest(sentence) && !element.matches(".button, .tool")) {
        const block = element.closest(sentence);
        if (block && block.textContent && block.textContent.trim() !== element.textContent?.trim()) continue;
      }
      // A checkbox's target is the box and its label together.
      if (element.matches("input[type=checkbox], input[type=radio]") && element.id) {
        const label = document.querySelector<HTMLElement>(`label[for="${element.id}"]`);
        const area = label?.getBoundingClientRect();
        if (area && area.height >= 44 - 0.5) continue;
      }
      if (element.closest(".trap")) continue;
      if (rect.width < 44 - 0.5 || rect.height < 44 - 0.5) {
        small.push(
          `${element.tagName.toLowerCase()} "${(element.textContent ?? "").trim().slice(0, 30)}" ${Math.round(rect.width)}×${Math.round(rect.height)}`,
        );
      }
    }
    return small;
  });
}

/**
 * How far the page as it stands reaches past a window 320 px wide, the
 * narrowest WCAG 1.4.10 asks for; nothing, where no sideways scrolling is
 * needed. For a page in a state no address leads to. The window is put
 * back as it was.
 */
export async function narrow(page: Page): Promise<number> {
  const before = page.viewportSize();
  await page.setViewportSize({ width: 320, height: 812 });
  const wide = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
  if (before) await page.setViewportSize(before);
  return Math.max(0, wide);
}

/** Fails the test on any console error, uncaught error or request to another origin. */
export function watch(page: Page, origin: string, expected404 = false): string[] {
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
