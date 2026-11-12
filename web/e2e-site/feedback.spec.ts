import { readFile } from "node:fs/promises";

import { expect, type Locator, type Page, test } from "@playwright/test";

import { HOSTILE, ME, OTHER_ADMIN, actions, asked, filter, harmless, hex, id, named, open, sound, world } from "./feedback-stand-in.ts";

// The feedback moderation's list (crates/central-web/src/islands/feedback/
// list.rs) in a real browser against the stand-in Central of
// feedback-stand-in.ts: the views and their counts, the search and the
// filters in the address and in the one request each sends, saved lists,
// a selection's actions (one request, all or none), deleting for good
// (asked first, a fresh sign-in, sent once), crash groups, the chart, the
// CSV, refusals, answers that never came, and markup in every report
// field drawn as text. At both sizes, in both schemes, at 320 px.

const LIST = "/en/account/admin/feedback";

test.use({ serviceWorkers: "block" });

const rows = (page: Page) => page.locator(".fb-results .fb-table tbody tr");
const button = (page: Page, name: string) => page.getByRole("button", { name, exact: true });
/** The newest page of the list asked for (the trash's count asks for one report). */
const lastReports = (seen: Parameters<typeof asked>[0]) => asked(seen).findLast((r) => r["action"] === "reports" && r["limit"] !== 1);
const check = (page: Page, n: number) => page.locator(`#fb-check-${hex(id(n))}`);

/** Tab leads through `stops` in this order, starting behind `from`. */
async function tabs(page: Page, from: Locator, stops: Locator[]) {
  await from.focus();
  for (const stop of stops) {
    await page.keyboard.press("Tab");
    await expect(stop).toBeFocused();
  }
}

test("without the role there is no list, no report, and nothing of the moderation is asked", async ({ page }) => {
  const w = world({ admin: false });
  const o = await open(page, w, LIST);
  await expect(page.getByText("This page is only for administrators.")).toBeVisible();
  await expect(page.locator("#fb-search, .fb-table, .fb-panel")).toHaveCount(0);
  await page.goto(`${LIST}/${hex(id(10))}`);
  await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
  await expect(page.getByText("This page is only for administrators.")).toBeVisible();
  await expect(page.locator(".fb-page, .fb-report")).toHaveCount(0);
  expect(asked(o.seen)).toEqual([]);
  expect(w.parts).toEqual([]);
  await sound(page);
});

test("the list: its views with their counts, its reports as text, and the checks every page passes", async ({ page }) => {
  const w = world();
  const o = await open(page, w, LIST);
  await expect(page.locator("h1")).toHaveText("Feedback and crash reports");
  await expect(page.getByRole("link", { name: "Administration overview" })).toHaveAttribute("href", "/en/account/admin/overview");
  await expect(rows(page)).toHaveCount(6);
  expect(lastReports(o.seen)).toEqual({ action: "reports", filter: filter(), sort: "received", descending: true, offset: 0, limit: 50 });
  // the counts: the open reports' facets and the trash's total
  expect(asked(o.seen)).toContainEqual({ action: "report_facets", filter: filter({ statuses: ["new", "triaged", "in_progress"] }) });
  expect(asked(o.seen)).toContainEqual({ action: "reports", filter: filter({ trash: true }), sort: "received", descending: true, offset: 0, limit: 1 });
  await expect(page.locator(".fb-views button")).toHaveText(["Inbox 2", "Open 4", "Mine 1", "Unassigned 2", "Done 2", "All 6", "Trash 2"]);
  await expect(button(page, "All 6")).toHaveAttribute("aria-pressed", "true");
  await expect(page.locator(".fb-countline [role=status]")).toHaveText("1 to 6 of 6 · 2 not opened");
  // a report's words, its gateway and every field are text
  const first = rows(page).first();
  await expect(first.locator("th a")).toHaveText(`${HOSTILE} first line`);
  await expect(first.locator("th a")).toHaveAttribute("href", `${LIST}/${hex(id(10))}`);
  await expect(first.getByRole("button", { name: `Only the gateway Gate ${HOSTILE}` })).toBeVisible();
  await expect(rows(page).nth(1).getByRole("link", { name: "#311 (opens in a new tab)" })).toHaveAttribute("href", "https://github.com/AceVik/baylee/issues/311");
  await expect(rows(page).nth(1).getByRole("button", { name: "Only reports looked after by Me" })).toBeVisible();
  await expect(rows(page).nth(2).getByRole("button", { name: "Only reports looked after by Ada#3c4d" })).toBeVisible();
  await expect(rows(page).nth(3).locator("td").nth(6)).toHaveText("Direct, through no gateway");
  await harmless(page, o);
  await sound(page);
  await tabs(page, page.locator("#fb-search"), [button(page, "Search"), page.locator("#fb-filters-toggle"), page.locator(".fb-menu summary")]);
  expect(o.problems).toEqual([]);
});

test("a view, a search, filters, an order and a page go into the address and into one request each", async ({ page }) => {
  const w = world();
  const o = await open(page, w, LIST);
  await expect(rows(page)).toHaveCount(6);
  await button(page, "Inbox 2").click();
  await expect(page).toHaveURL(`${LIST}?status=new`);
  await expect(rows(page)).toHaveCount(2);
  expect(lastReports(o.seen)).toMatchObject({ filter: filter({ statuses: ["new"] }) });
  await expect(button(page, "Inbox 2")).toBeFocused();

  // the search follows the typing once it pauses, and Enter at once
  await page.locator("#fb-search").fill("hyper link");
  await page.locator("#fb-search").press("Enter");
  await expect(page).toHaveURL(`${LIST}?status=new&q=hyper+link`);
  await expect(rows(page)).toHaveCount(0);
  expect(lastReports(o.seen)).toMatchObject({ filter: filter({ statuses: ["new"], q: "hyper link" }) });
  await page.locator("#fb-search").fill("hyperlink");
  await page.locator("#fb-search").press("Enter");
  await expect(rows(page)).toHaveCount(1);
  expect(lastReports(o.seen)).toMatchObject({ filter: filter({ statuses: ["new"], q: "hyperlink" }) });
  // a search of nothing Central reads is no search: never sent
  await page.locator("#fb-search").fill("‮‮");
  await page.locator("#fb-search").press("Enter");
  await expect(page).toHaveURL(`${LIST}?status=new`);
  expect(lastReports(o.seen)).toMatchObject({ filter: filter({ statuses: ["new"] }) });

  // the panel: chips with their counts, then the pills that take them out
  await page.locator("#fb-filters-toggle").click();
  await expect(page.locator("#fb-filters-toggle")).toHaveAttribute("aria-expanded", "true");
  await expect(page.locator("#fb-filters fieldset").nth(1).locator("button")).toHaveText(["Bug", "Improvement", "Feedback 1", "Crash 1", "Other"]);
  await page.locator("#fb-f-kind-3").click();
  await expect(page).toHaveURL(`${LIST}?status=new&kind=crash`);
  await expect(page.locator("#fb-f-kind-3")).toBeFocused();
  await expect(page.locator("#fb-filters-toggle")).toHaveText("Filters (1)");
  await page.locator("#fb-f-assignee").selectOption("none");
  await expect(page).toHaveURL(`${LIST}?status=new&kind=crash&assignee=none`);
  expect(lastReports(o.seen)).toMatchObject({ filter: filter({ statuses: ["new"], kinds: ["crash"], unassigned: true }) });
  await page.locator("#fb-f-from").fill("2026-11-01");
  await expect(page).toHaveURL(/from=2026-11-01/);
  await page.locator("#fb-f-unseen").check();
  await expect(page).toHaveURL(/unseen=1/);
  expect(lastReports(o.seen)).toMatchObject({ filter: filter({ statuses: ["new"], kinds: ["crash"], unassigned: true, from: "2026-11-01", unseen: true }) });
  // out of every view (no view is "new, unassigned"), its parts are pills too
  await expect(page.locator("#fb-pills bdi")).toHaveText(["Status: New", "Kind: Crash", "Assignee: Nobody", "From: 2026-11-01", "Only reports I have not opened"]);
  const kind = page.locator("#fb-pills li:nth-child(2) button");
  await expect(kind).toHaveAccessibleName(/Remove the filter Kind: Crash$/);
  await kind.click();
  await expect(page).toHaveURL(`${LIST}?status=new&assignee=none&from=2026-11-01&unseen=1`);
  await expect(page.locator("#fb-pills li:first-child button")).toBeFocused();
  await button(page, "Clear the filters").click();
  await expect(page).toHaveURL(LIST);
  await expect(page.locator("#fb-filters-toggle")).toBeFocused();
  await sound(page);

  // the column heads order the list; status starts from the top
  await page.locator("#fb-sort-status").click();
  await expect(page).toHaveURL(`${LIST}?sort=status&dir=asc`);
  await expect(page.locator("th[aria-sort=ascending]")).toContainText("Status");
  expect(lastReports(o.seen)).toMatchObject({ sort: "status", descending: false });
  await page.locator("#fb-sort-status").click();
  await expect(page).toHaveURL(`${LIST}?sort=status`);
  expect(lastReports(o.seen)).toMatchObject({ sort: "status", descending: true });

  // pages: 25 a page, the second after the first 25
  const base = named(w, "Report number 11");
  w.reports = Array.from({ length: 30 }, (_, i) => {
    const r = structuredClone(base);
    Object.assign(r.summary, { id: id(30 + i), status: "new", seen: false, excerpt: `Many ${i}` });
    return r;
  });
  await page.goto(`${LIST}?status=new&limit=25`);
  await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
  await expect(rows(page)).toHaveCount(25);
  await expect(page.locator(".fb-pager span.meta")).toHaveText("Page 1 of 2");
  await page.locator("#fb-page-next").click();
  await expect(page).toHaveURL(`${LIST}?status=new&limit=25&page=2`);
  await expect(rows(page)).toHaveCount(5);
  expect(lastReports(o.seen)).toMatchObject({ offset: 25, limit: 25 });
  await page.locator("#fb-page-size").selectOption("100");
  await expect(page).toHaveURL(`${LIST}?status=new&limit=100`);
  await expect(rows(page)).toHaveCount(30);
  expect(o.problems).toEqual([]);
});

test("saved lists: read when opened, saved and removed; a name stays text", async ({ page }) => {
  const w = world();
  const o = await open(page, w, `${LIST}?status=new`);
  await expect(rows(page)).toHaveCount(2);
  const from = o.seen.length;
  await page.locator(".fb-menu summary").click();
  await expect(page.locator(".fb-view-list a")).toHaveText(`Mine ${HOSTILE}`);
  expect(actions(o.seen, from)).toEqual(["report_views"]);
  // an old administration's query string opens as it did there
  await expect(page.locator(".fb-view-list a")).toHaveAttribute("href", `${LIST}?status=new%2Ctriaged%2Cin_progress&assignee=${hex(ME)}`);
  await page.locator("#fb-view-name").fill("Inbox");
  await button(page, "Save this list").click();
  await expect(page.locator(".fb-view-list a")).toHaveText([`Mine ${HOSTILE}`, "Inbox"]);
  expect(asked(o.seen).at(-1)).toEqual({ action: "save_report_view", name: "Inbox", query: "status=new" });
  // the list shown is saved: no second form for it
  await expect(page.locator("#fb-view-name")).toHaveCount(0);
  await expect(page.locator(".fb-view-list a").nth(1)).toHaveAttribute("aria-current", "true");
  await sound(page);
  await page.getByRole("button", { name: "Remove the saved list Inbox" }).click();
  await expect(page.locator(".fb-view-list a")).toHaveCount(1);
  expect(asked(o.seen).at(-1)).toEqual({ action: "delete_report_view", view: id(92) });
  await page.locator(".fb-view-list a").click();
  await expect(page).toHaveURL(`${LIST}?status=new%2Ctriaged%2Cin_progress&assignee=${hex(ME)}`);
  await expect(button(page, "Mine 1")).toHaveAttribute("aria-pressed", "true");
  await harmless(page, o);
});

test("a selection: one request for all of it, the trash with Undo, opened marks, ids and the CSV", async ({ page, context, browserName }) => {
  test.skip(browserName !== "chromium");
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  const w = world();
  const o = await open(page, w, LIST);
  await expect(rows(page)).toHaveCount(6);
  const bar = page.locator(".fb-bulk");
  await expect(bar).toBeHidden();
  await check(page, 10).check();
  await check(page, 15).check();
  await expect(bar).toBeVisible();
  await expect(bar.locator("strong")).toHaveText("Selected: 2");
  await sound(page);
  const two = [id(10), id(15)];
  const bulkAt = () => asked(o.seen).findLast((r) => r["action"] === "reports_bulk");

  await page.locator("#fb-bulk-status").selectOption("triaged");
  await expect(page.locator(".fb-notice").last()).toHaveText(/Reports changed: 2/);
  expect(bulkAt()).toEqual({ action: "reports_bulk", reports: two, bulk: { do: "change", change: { status: "triaged" } } });
  await expect(page.locator("#fb-bulk-status")).toHaveValue("");
  await page.locator("#fb-bulk-assign").selectOption(hex(OTHER_ADMIN));
  await expect.poll(() => bulkAt()).toEqual({ action: "reports_bulk", reports: two, bulk: { do: "change", change: { assignee: OTHER_ADMIN } } });
  await page.locator("#fb-bulk-priority").selectOption("none");
  await expect.poll(() => bulkAt()).toEqual({ action: "reports_bulk", reports: two, bulk: { do: "change", change: { priority: null } } });
  await page.locator("#fb-bulk-label").fill("Needs-Repro");
  await page.locator("#fb-bulk-label").press("Enter");
  await expect.poll(() => bulkAt()).toEqual({ action: "reports_bulk", reports: two, bulk: { do: "change", change: { labels_add: ["needs-repro"] } } });
  await page.locator("#fb-bulk-unlabel").selectOption("ui");
  await expect.poll(() => bulkAt()).toEqual({ action: "reports_bulk", reports: two, bulk: { do: "change", change: { labels_remove: ["ui"] } } });
  // merged into the first: the other becomes its duplicate
  await page.locator("#fb-bulk-merge").selectOption(hex(id(10)));
  await expect.poll(() => bulkAt()).toEqual({ action: "reports_bulk", reports: [id(15)], bulk: { do: "change", change: { duplicate_of: id(10) } } });

  // opened marks, the ids, the CSV of the selection
  await button(page, "Mark as opened").click();
  await expect.poll(() => asked(o.seen).at(-1)).toEqual({ action: "set_reports_seen", reports: two, seen: true });
  await button(page, "Copy the ids").click();
  await expect(page.locator(".fb-notice").last()).toHaveText(/Ids copied: 2/);
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(`${hex(id(10))}\n${hex(id(15))}`);
  const download = page.waitForEvent("download");
  await button(page, "Export as CSV (2)").click();
  const file = await download;
  expect(file.suggestedFilename()).toMatch(/^baylee-reports-\d{4}-\d\d-\d\d\.csv$/);
  const csv = await readFile((await file.path()) ?? "", "utf8");
  const lines = csv.split("\r\n");
  expect(lines[0]).toBe("id,received,changed,kind,status,priority,assignee,labels,gateway,channel,reporter,build,crash,record_bytes,issue,notes,excerpt");
  // a formula never runs: its first character is defused
  expect(csv).toContain(`"'=HYPERLINK(""https://x.invalid"",""click"")"`);
  expect(lines).toHaveLength(4);

  // into the trash, all at once, and back with Undo
  const before = o.seen.length;
  await button(page, "Move to the trash").click();
  await expect(page.locator(".fb-notice").last()).toContainText("Moved to the trash: 2");
  expect(asked(o.seen, before).find((r) => r["action"] === "reports_bulk")).toEqual({ action: "reports_bulk", reports: two, bulk: { do: "trash" } });
  await expect(bar).toBeHidden();
  await expect(rows(page)).toHaveCount(4);
  await page.locator(".fb-notice").last().getByRole("button", { name: "Undo" }).click();
  await expect(rows(page)).toHaveCount(6);
  expect(bulkAt()).toEqual({ action: "reports_bulk", reports: two, bulk: { do: "restore" } });
  await harmless(page, o);
});

test("deleting for good is asked first, wants a fresh sign-in, and is sent once", async ({ page }) => {
  const w = world({ stale: true });
  const o = await open(page, w, `${LIST}?trash=1`);
  await expect(rows(page)).toHaveCount(2);
  await expect(page.getByText("Reports in the trash are deleted for good after 30 days.")).toBeVisible();
  // restored from the trash, both in one request
  await check(page, 16).check();
  await check(page, 17).check();
  const bar = page.locator(".fb-bulk");
  await bar.getByRole("button", { name: "Restore" }).click();
  await expect(rows(page)).toHaveCount(0);
  expect(asked(o.seen).find((r) => r["action"] === "reports_bulk")).toEqual({ action: "reports_bulk", reports: [id(16), id(17)], bulk: { do: "restore" } });
  for (const n of [16, 17]) named(w, `Report number ${n}`).summary.deleted_at = 1;
  await button(page, "Refresh").click();
  await expect(rows(page)).toHaveCount(2);
  await check(page, 16).check();
  await check(page, 17).check();
  const from = o.seen.length;
  await page.locator("#fb-bulk-purge").click();
  const confirm = page.locator(".fb-confirm");
  await expect(page.locator("#fb-purge-title")).toBeFocused();
  await expect(confirm).toContainText("Reports to delete: 2. They leave Baylee Central for good, with their notes, history and parts.");
  await sound(page);
  // the question sends nothing; cancelled, nothing was asked
  await confirm.getByRole("button", { name: "Cancel" }).click();
  await expect(confirm).toHaveCount(0);
  await expect(page.locator("#fb-bulk-purge")).toBeFocused();
  expect(o.seen.slice(from)).toEqual([]);

  // a stale sign-in: the password once more before the one request
  await page.locator("#fb-bulk-purge").click();
  await page.locator("#fb-purge-go").dblclick();
  await expect(page.locator("#fb-purge-password")).toBeFocused();
  expect(o.seen.slice(from).map((b) => b.operation)).toEqual(["account_summary"]);
  await expect(confirm).toContainText("Deleting for good needs a sign-in less than 30 minutes old");
  await sound(page);
  await confirm.getByRole("button", { name: "Delete them for good" }).click();
  await expect(confirm.locator("[role=alert]")).toHaveText("Enter your password.");
  expect(actions(o.seen, from)).toEqual([]);
  await page.locator("#fb-purge-password").fill("violet lantern harbour");
  await page.locator("#fb-purge-password").press("Enter");
  await expect(page.locator(".fb-notice").last()).toContainText("Deleted for good: 2");
  await expect(rows(page)).toHaveCount(0);
  await expect(page.locator(".fb-countline [role=status]")).toHaveText("No report matches this list.");
  const purges = asked(o.seen, from).filter((r) => r["action"] === "reports_bulk");
  expect(purges).toEqual([{ action: "reports_bulk", reports: [id(16), id(17)], bulk: { do: "purge" } }]);
  expect(JSON.stringify(o.seen)).not.toContain("lantern");
  expect(o.problems).toEqual([]);
});

test("deleting for good: Central's refusal and an answer that never came are said as they are", async ({ page }) => {
  const w = world();
  const o = await open(page, w, `${LIST}?trash=1`);
  await expect(rows(page)).toHaveCount(2);
  await check(page, 16).check();
  // a report restored meanwhile: Central deletes nothing
  named(w, "Report number 16").summary.deleted_at = null;
  await page.locator("#fb-bulk-purge").click();
  await page.locator("#fb-purge-go").click();
  await expect(page.locator(".fb-notice.bad")).toContainText("Baylee Central deleted nothing: a report is no longer in the trash, or no longer exists.");
  await expect(rows(page)).toHaveCount(1);
  // Central counts the sign-in as stale though this browser's clock does not:
  // the reports still in the trash tell it, and the password is asked
  await check(page, 17).check();
  w.staleUnseen = true;
  let from = o.seen.length;
  await page.locator("#fb-bulk-purge").click();
  await page.locator("#fb-purge-go").click();
  await expect(page.locator("#fb-purge-password")).toBeFocused();
  await expect(page.locator(".fb-confirm [role=alert]")).toHaveText(
    "Baylee Central counts your sign-in as older than 30 minutes. Try again; you enter your password once more.",
  );
  expect(actions(o.seen, from)).toEqual(["reports_bulk", "report"]);
  await page.locator(".fb-confirm").getByRole("button", { name: "Cancel" }).click();
  w.staleUnseen = false;
  // no answer: perhaps deleted, so the list is read again
  w.next["reports_bulk"] = "lost";
  from = o.seen.length;
  await page.locator("#fb-bulk-purge").click();
  await page.locator("#fb-purge-go").click();
  await expect(page.locator(".fb-notice.bad").last()).toContainText("No answer came back, so this page cannot tell whether the reports were deleted.");
  await expect(rows(page)).toHaveCount(0);
  expect(actions(o.seen, from)).toContain("reports");
  await sound(page);
});

test("crash groups and the chart of the last 30 days", async ({ page }) => {
  const w = world();
  const o = await open(page, w, LIST);
  await expect(rows(page)).toHaveCount(6);
  await button(page, "Crash groups").click();
  const groups = page.getByRole("region", { name: "Crash groups" }).locator("tbody tr");
  await expect(groups).toHaveCount(1);
  expect(asked(o.seen).at(-1)).toEqual({ action: "crash_groups", filter: filter() });
  await expect(groups.locator("th code").first()).toHaveText(`panicked ${HOSTILE}`);
  await expect(groups.locator("td.num")).toHaveText(["2", "2"]);
  await sound(page);
  await button(page, "Show its reports").click();
  await expect(page).toHaveURL(`${LIST}?signature=0123456789abcdef0123456789abcdef`);
  await expect(rows(page)).toHaveCount(2);
  await expect(page.locator("#fb-mode-list")).toBeFocused();

  const chart = page.locator(".fb-chart");
  const from = o.seen.length;
  await chart.locator("summary").click();
  await expect(chart.locator(".fb-chart-legend")).toHaveText(/Reports: 3\s+Crashes: 2/);
  expect(actions(o.seen, from)).toEqual(["report_stats"]);
  await expect(chart.locator("svg rect.fb-chart-reports")).toHaveCount(1);
  await expect(chart.locator("svg rect.fb-chart-crashes")).toHaveCount(1);
  await chart.locator("#fb-chart-days summary").click();
  await expect(chart.locator("#fb-chart-days tbody tr td")).toHaveText(["3", "2"]);
  await sound(page);
  await harmless(page, o);
});

test("refusals in clear words, and an answer that never came is no refusal", async ({ page }) => {
  const w = world();
  const o = await open(page, w, LIST);
  await expect(rows(page)).toHaveCount(6);
  // the role taken away meanwhile
  w.admin = false;
  await button(page, "Refresh").click();
  await expect(page.locator(".fb-results [role=alert]")).toHaveText("Your account no longer holds the administrator role.");
  w.admin = true;
  await button(page, "Try again").click();
  await expect(rows(page)).toHaveCount(6);
  // Central unavailable, and no answer at all
  w.next["reports"] = 503;
  await button(page, "Refresh").click();
  await expect(page.locator(".fb-results [role=alert]")).toHaveText("Baylee Central is not answering right now. Try again in a moment.");
  await button(page, "Try again").click();
  await expect(rows(page)).toHaveCount(6);
  // a change whose answer was lost: perhaps made, so the list is read again
  await check(page, 13).check();
  w.next["reports_bulk"] = "lost";
  const from = o.seen.length;
  await page.locator("#fb-bulk-status").selectOption("resolved");
  await expect(page.locator(".fb-notice.bad")).toHaveText(/No answer came back, so this page cannot tell whether that was done/);
  expect(actions(o.seen, from)).toEqual(expect.arrayContaining(["reports_bulk", "reports"]));
  // a session that is gone is said by the area
  w.gone = true;
  await button(page, "Refresh").click();
  await expect(page.getByText("You are not signed in.")).toBeVisible();
  expect(await page.evaluate(() => localStorage.getItem("baylee-central-session"))).toBeNull();
});

test("German, and the longest words, hold at 320 px in every state", async ({ page }) => {
  const w = world();
  await open(page, w, "/de/account/admin/feedback?trash=1");
  await expect(page.locator("h1")).toHaveText("Feedback und Absturzberichte");
  await expect(rows(page)).toHaveCount(2);
  await sound(page);
  await page.locator("#fb-filters-toggle").click();
  await check(page, 16).check();
  await page.locator(".fb-menu summary").click();
  await sound(page);
  await page.locator("#fb-bulk-purge").click();
  await expect(page.locator("#fb-purge-title")).toHaveText("Endgültig löschen?");
  await sound(page);
  await page.goto("/de/account/admin/feedback");
  await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
  await check(page, 10).check();
  await check(page, 11).check();
  await page.locator("#fb-filters-toggle").click();
  await sound(page);
});
