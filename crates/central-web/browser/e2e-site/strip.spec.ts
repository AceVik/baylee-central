// The strip under the bar (docs/proposals/frontend-rewrite.md §18): Baylee
// in numbers (six cells: players, games, open tables, gateways, accounts,
// page views), nothing of GitHub's, and its cells in even, centred rows for
// every count the server can write, at every width.
import { expect, test, type Page } from "@playwright/test";

import { axe, onlyDesktop, targets } from "./check.ts";

/** The widths a reader brings: phones, a tablet, desks, and both sides of each step. */
const WIDTHS = [320, 375, 599, 600, 768, 1024, 1099, 1100, 1440, 2560];

/**
 * The list's cells as the server writes them with every source reporting:
 * the gateways' three live counts (players, games, open tables: one source,
 * so they come and go together), then the gateways, the accounts and the
 * page views, one cell each. The gateways' cell is always there (while the
 * directory is silent it only links to it); each other group is left out
 * while its source is silent.
 */
const ORDER = ["gateways", "live", "live", "live", "live", "accounts", "views"];
const OPTIONAL = ["live", "accounts", "views"];
/** Every set of silent sources: eight. */
const SILENCES = [...Array(8).keys()].map((bits) => OPTIONAL.filter((_, at) => ((bits >> at) & 1) === 1));

/**
 * Leave out what silent sources would leave out and say how many cells
 * remain, as the server would (the cells are kept aside and put back, so
 * one page serves every set).
 */
async function silence(page: Page, gone: string[]): Promise<number> {
  return page.evaluate(
    ([order, silent]) => {
      const list = document.querySelector<HTMLElement>(".strip-list");
      if (!list) throw new Error("no strip");
      const kept: unknown = Reflect.get(window, "stripCells");
      const all = Array.isArray(kept) ? (kept as Element[]) : [...list.children];
      Reflect.set(window, "stripCells", all);
      if (all.length !== order.length) throw new Error(`${all.length} cells`);
      list.replaceChildren(...all.filter((_, at) => !silent.includes(order[at] ?? "")));
      const count = list.querySelectorAll(".strip-cell").length;
      list.setAttribute("data-count", String(count));
      return count;
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

test("the strip shows Baylee in numbers and nothing of GitHub's", async ({ page }) => {
  await page.goto("/en/");
  const strip = page.getByRole("region", { name: "Baylee in numbers" });
  await expect(strip).toBeVisible();
  // each cell: its label (the term) and its value, the seven the test data has
  const cells = strip.locator(".strip-cell");
  await expect(cells.locator("dt .strip-label")).toHaveText(["Gateways", "In game", "Online", "Games", "Open tables", "Accounts", "Views"]);
  await expect(cells.locator("dd.num")).toHaveText([/^5\s*\/\s*7$/, /^34$/, /^44$/, /^12$/, /^4$/, /^1,234$/, /^48,213$/]);
  await expect(strip.locator(".strip-list")).toHaveAttribute("data-count", "7");
  // the gateways, as "5 / 7", lead to the gateways directory page
  await expect(cells.nth(0)).toHaveAttribute("title", "Gateways: 5 online, 2 offline");
  await expect(cells.nth(0).locator("a")).toHaveAttribute("href", "/en/gateways");
  // the four live counts are the gateways' own claim, said under the pointer
  for (const at of [1, 2, 3, 4]) await expect(cells.nth(at)).toHaveAttribute("title", "Right now on the gateways, as they report it");
  for (const at of [1, 2, 3]) await expect(cells.nth(at).locator("dd .dot.live")).toHaveCount(1);
  await expect(cells.nth(6)).toHaveAttribute("title", "Page views since 1 November 2026");
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
  const cells = strip.locator(".strip-cell");
  await expect(cells.first().locator(".strip-label")).toHaveText("Gateways");
  await expect(cells.nth(1).locator(".strip-label")).toHaveText("Im Spiel");
  await expect(cells.nth(1).locator("dd")).toHaveText("34");
  await expect(cells.nth(2).locator(".strip-label")).toHaveText("Online");
  await expect(cells.nth(2).locator("dd")).toHaveText("44");
  await expect(strip.locator("dd.num").nth(5)).toHaveText("1.234");
  await expect(strip.locator("dd.num").nth(6)).toHaveText("48.213");
});

for (const lang of ["en"]) {
  test(`${lang}: the strip's lines are even and centred for every set of silent sources, at every width`, async ({ page }, info) => {
    onlyDesktop(info, "sets its own window sizes, so the project's size is irrelevant");
    await page.emulateMedia({ reducedMotion: "reduce" });
    await page.goto(`/${lang}/`);
    const problems: string[] = [];
    for (const width of WIDTHS) {
      await page.setViewportSize({ width, height: 800 });
      for (const gone of SILENCES) {
        const count = await silence(page, gone);
        const silent = gone.length === 0 ? "none silent" : `${gone.join(", ")} silent`;
        for (const fault of faults(count, await layout(page))) problems.push(`${width}px, ${silent}: ${fault}`);
      }
    }
    expect(problems).toEqual([]);
  });
}
