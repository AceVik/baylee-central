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
 *
 * The page is read at rest: cards that rise and brighten as they scroll
 * into view (a scroll-driven animation) would otherwise be measured half
 * faded at the window's edge. Contrast is owed to the text as it stays.
 */
export async function axe(page: Page): Promise<string[]> {
  await page.emulateMedia({ reducedMotion: "reduce" });
  const result = await new AxeBuilder({ page })
    .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa", "best-practice"])
    .disableRules(["target-size"])
    .analyze()
    .finally(() => page.emulateMedia({ reducedMotion: null }));
  return result.violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target.join(" ")).join(", ")}`);
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
        small.push(`${element.tagName.toLowerCase()} "${(element.textContent ?? "").trim().slice(0, 30)}" ${Math.round(rect.width)}×${Math.round(rect.height)}`);
      }
    }
    return small;
  });
}
