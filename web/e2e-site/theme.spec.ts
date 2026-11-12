// The colour switch (boot.js): one press shows the other scheme, from the
// first visit on, whatever the device asks for and whatever an earlier
// visit kept, and the scheme stands before the page is drawn
// (docs/proposals/frontend-rewrite.md §18).
import { expect, test, type Locator, type Page } from "@playwright/test";

const KEY = "baylee-site-theme";

/** The scheme the page shows: the root's computed `color-scheme`. */
const shown = (page: Page): Promise<string> =>
  page.evaluate(() => getComputedStyle(document.documentElement).colorScheme);

/** The switch a reader reaches: in the bar on a wide screen, in the menu on a narrow one. */
async function theSwitch(page: Page): Promise<Locator> {
  const inBar = page.locator(".top-tools [data-theme-switch]");
  if (await inBar.isVisible()) return inBar;
  await page.locator(".menu-btn").click();
  const inMenu = page.locator("#nav [data-theme-switch]");
  await expect(inMenu).toBeVisible();
  return inMenu;
}

/**
 * What the root said when the body began: the kept choice must already be
 * applied then (boot.js runs in the head), or the page would flash the
 * device's scheme first.
 */
async function watchFirstPaint(page: Page): Promise<void> {
  await page.addInitScript(() => {
    new MutationObserver((_, observer) => {
      if (!document.body) return;
      observer.disconnect();
      const root = document.documentElement;
      root.setAttribute("data-test-at-body", root.getAttribute("data-theme") ?? "none");
    }).observe(document, { childList: true, subtree: true });
  });
}

async function atBody(page: Page): Promise<string | null> {
  const kept = await page.evaluate((key) => localStorage.getItem(key), KEY);
  expect(await page.locator("html").getAttribute("data-test-at-body")).toBe(kept ?? "none");
  return kept;
}

for (const device of ["dark", "light"] as const) {
  test.describe(`on a ${device} device`, () => {
    test.use({ colorScheme: device });

    for (const before of [null, "light", "dark"] as const) {
      test(`one press shows the other colours, with ${before ?? "nothing"} kept from before`, async ({ page }) => {
        await watchFirstPaint(page);
        if (before !== null) {
          // kept by an earlier visit: set once, before the first page
          await page.addInitScript(
            ([key, value]) => {
              if (sessionStorage.getItem("test-seeded") !== null) return;
              localStorage.setItem(key, value);
              sessionStorage.setItem("test-seeded", "");
            },
            [KEY, before] as const,
          );
        }
        await page.goto("/en/faq");
        const first = before ?? device;
        const other = first === "dark" ? "light" : "dark";
        expect(await shown(page)).toBe(first);
        await atBody(page);

        let button = await theSwitch(page);
        await expect(button).toHaveAttribute("data-state", first);
        await button.click();
        await expect.poll(() => shown(page)).toBe(other);
        await expect(button).toHaveAttribute("data-state", other);
        await expect(button).toHaveAttribute("aria-label", other === "dark" ? "Colours: dark" : "Colours: light");

        // kept, and there before the page is drawn
        await page.reload();
        expect(await shown(page)).toBe(other);
        await atBody(page);

        // and one press goes back
        button = await theSwitch(page);
        await button.click();
        await expect.poll(() => shown(page)).toBe(first);
        await page.reload();
        expect(await shown(page)).toBe(first);
        await atBody(page);
        // the device's own scheme is not kept: the page follows the device again
        const kept = await page.evaluate((key) => localStorage.getItem(key), KEY);
        expect(kept).toBe(first === device ? null : first);
      });
    }

    test("a page left open follows the device while nothing is kept", async ({ page }) => {
      await page.goto("/en/faq");
      const button = await theSwitch(page);
      await expect(button).toHaveAttribute("data-state", device);
      const turned = device === "dark" ? "light" : "dark";
      await page.emulateMedia({ colorScheme: turned });
      await expect.poll(() => shown(page)).toBe(turned);
      await expect(button).toHaveAttribute("data-state", turned);
      // one press still shows the other
      await button.click();
      await expect.poll(() => shown(page)).toBe(device);
    });
  });
}
