// The strip under the bar (docs/proposals/frontend-rewrite.md §18): Baylee
// in numbers, nothing of GitHub's, the most telling first, and its cells in
// even, centred rows for every count the server can write, at every width.
import { expect, test, type Page } from "@playwright/test";

import { axe, targets } from "./check.ts";

/** The widths a reader brings: phones, a tablet, desks, and both sides of each step. */
const WIDTHS = [320, 375, 599, 600, 768, 1024, 1099, 1100, 1440, 2560];

/**
 * What the server writes when sources are silent, as the list's children
 * left out: the gateways' four counts (one group), then the gateways, the
 * accounts, the cards and the page views, one cell each. The cell for the
 * gateways is always there.
 */
const SILENT: Record<number, string[]> = {
  8: [],
  7: ["views"],
  6: ["cards", "views"],
  5: ["accounts", "cards", "views"],
  4: ["live"],
  3: ["live", "views"],
  2: ["live", "cards", "views"],
  1: ["live", "accounts", "cards", "views"],
};
const ORDER = ["live", "gateways", "accounts", "cards", "views"];

/** Leave out what a silent source would leave out, and say how many cells remain, as the server would. */
async function silence(page: Page, gone: string[]): Promise<void> {
  await page.evaluate(
    ([order, silent]) => {
      const list = document.querySelector<HTMLElement>(".strip-list");
      if (!list) throw new Error("no strip");
      const children = [...list.children];
      if (children.length !== order.length) throw new Error(`${children.length} groups`);
      order.forEach((name, at) => {
        if (silent.includes(name)) children[at]?.remove();
      });
      list.setAttribute("data-count", String(list.querySelectorAll(".strip-cell").length));
    },
    [ORDER, gone] as const,
  );
}

type Row = { cells: number; left: number; right: number; gaps: number[] };

/** The cells as they stand, line by line, and what does not fit. */
async function layout(page: Page): Promise<{ rows: Row[]; width: number; cut: string[]; scroll: number }> {
  return page.evaluate(() => {
    const list = document.querySelector<HTMLElement>(".strip-list");
    if (!list) throw new Error("no strip");
    const box = list.getBoundingClientRect();
    const cells = [...list.querySelectorAll<HTMLElement>(".strip-cell")].map((cell) => ({
      cell,
      rect: cell.getBoundingClientRect(),
    }));
    const rows: { top: number; rects: DOMRect[] }[] = [];
    for (const { rect } of cells) {
      const row = rows.find((r) => Math.abs(r.top + (r.rects[0]?.height ?? 0) / 2 - (rect.top + rect.height / 2)) < 4);
      if (row) row.rects.push(rect);
      else rows.push({ top: rect.top, rects: [rect] });
    }
    const cut: string[] = [];
    for (const { cell, rect } of cells) {
      // every glyph and word inside its own cell, and the cell on the screen
      if (rect.left < -0.5 || rect.right > document.documentElement.clientWidth + 0.5) cut.push(`${cell.textContent?.trim()}: off the screen`);
      for (const inner of cell.querySelectorAll<HTMLElement>("*")) {
        if (inner.closest(".visually-hidden")) continue;
        const r = inner.getBoundingClientRect();
        if (r.width === 0) continue;
        if (r.left < rect.left - 0.5 || r.right > rect.right + 0.5) cut.push(`${cell.textContent?.trim()}: ${inner.textContent?.trim() || inner.tagName} sticks out`);
      }
    }
    return {
      rows: rows.map(({ rects }) => {
        const sorted = [...rects].sort((a, b) => a.left - b.left);
        return {
          cells: sorted.length,
          left: (sorted[0]?.left ?? 0) - box.left,
          right: box.right - (sorted.at(-1)?.right ?? 0),
          gaps: sorted.slice(1).map((r, at) => r.left - (sorted[at]?.right ?? 0)),
        };
      }),
      width: box.width,
      cut,
      scroll: document.documentElement.scrollWidth - document.documentElement.clientWidth,
    };
  });
}

/** What is wrong with the lines for this many cells: uneven, alone, off centre, gaps that differ, cut or scrolling. */
function faults(count: number, { rows, cut, scroll }: Awaited<ReturnType<typeof layout>>): string[] {
  const found: string[] = [...cut];
  const counts = rows.map((row) => row.cells);
  if (counts.reduce((a, b) => a + b, 0) !== count) found.push(`${counts.join("+")} cells, not ${count}`);
  if (Math.max(...counts) - Math.min(...counts) > 1) found.push(`lines of ${counts.join(", ")}`);
  if (count > 1 && counts.includes(1)) found.push(`a cell alone on a line: ${counts.join(", ")}`);
  for (const row of rows) {
    if (Math.abs(row.left - row.right) > 1.5) found.push(`a line of ${row.cells} off centre: ${row.left.toFixed(1)} | ${row.right.toFixed(1)}`);
    if (row.gaps.length > 0 && Math.max(...row.gaps) - Math.min(...row.gaps) > 1) found.push(`gaps of ${row.gaps.map((g) => g.toFixed(1)).join(", ")}`);
  }
  const gaps = rows.flatMap((row) => row.gaps);
  if (gaps.length > 0 && Math.max(...gaps) - Math.min(...gaps) > 1) found.push(`gaps differ between lines: ${gaps.map((g) => g.toFixed(1)).join(", ")}`);
  if (scroll > 0) found.push(`the page scrolls sideways by ${scroll}px`);
  return found;
}

test("the strip shows Baylee in numbers, the most telling first, and nothing of GitHub's", async ({ page }) => {
  await page.goto("/en/");
  const strip = page.getByRole("region", { name: "Baylee in numbers" });
  await expect(strip).toBeVisible();
  // each cell's label (for a screen reader) comes before its value
  await expect(strip.locator(".strip-cell")).toHaveText([
    /^\s*12 players$/,
    /^\s*8 in a game$/,
    /^\s*3 open tables$/,
    /^\s*4 games$/,
    /^\s*Gateways\s*1 online\s*1 offline$/,
    /^\s*Accounts at Baylee Central\s*1,234 accounts$/,
    /^\s*Playable cards\s*8,348\s*cards$/,
    /^\s*Page views since 1 November 2026\s*48,213 views$/,
  ]);
  await expect(strip.locator(".strip-list")).toHaveAttribute("data-count", "8");
  // the gateways' counts are one term of the list, as they report them
  await expect(strip.locator(".strip-live dt")).toHaveText("Right now on the gateways, as they report it");
  // nothing of GitHub's: no release, no checks, no commit, no stars, no link there
  await expect(strip).not.toContainText(/v0\.2\.0|passing|failing|Build|Commit|Stars|GitHub/);
  await expect(strip.locator("a[href*='github.com']")).toHaveCount(0);
  await expect(strip.locator("code")).toHaveCount(0);
  expect(await axe(page)).toEqual([]);
  expect(await targets(page)).toEqual([]);
});

test("in German the strip is named and counts as German does", async ({ page }) => {
  await page.goto("/de/");
  const strip = page.getByRole("region", { name: "Baylee in Zahlen" });
  await expect(strip.locator(".strip-cell").first()).toHaveText("12 Spieler");
  await expect(strip).toContainText("1.234 Konten");
});

for (const lang of ["en", "de", "fr", "ru"]) {
  test(`${lang}: the strip's lines are even and centred for every count, at every width`, async ({ page }, info) => {
    // a phone has one width; the desk goes through them all
    const widths = info.project.name === "phone" ? [page.viewportSize()?.width ?? 412] : WIDTHS;
    test.setTimeout(120_000);
    await page.emulateMedia({ reducedMotion: "reduce" });
    const problems: string[] = [];
    for (const [count, gone] of Object.entries(SILENT)) {
      await page.goto(`/${lang}/`);
      await silence(page, gone);
      await expect(page.locator(".strip-list")).toHaveAttribute("data-count", count);
      for (const width of widths) {
        if (info.project.name !== "phone") await page.setViewportSize({ width, height: 800 });
        for (const fault of faults(Number(count), await layout(page))) problems.push(`${width}px, ${count} cells: ${fault}`);
      }
    }
    expect(problems).toEqual([]);
  });
}
