import { expect, type Locator, type Page, test } from "@playwright/test";

import { HOSTILE, HOSTILE_TOKEN, ME, OTHER_ADMIN, actions, asked, filter, harmless, hex, id, named, open, sound, world } from "./feedback-stand-in.ts";

// One report of the feedback moderation (crates/central-web/src/islands/
// feedback/report.rs) in a real browser against the stand-in Central of
// feedback-stand-in.ts: opened and marked opened, what was written and
// what the client sent drawn as text (its picture through a canvas, its
// whole object as a tree), each triage change one request at the revision
// read, notes, the trash, deleting for good, the record, the neighbours,
// refusals and answers that never came. At both sizes, both schemes, 320 px.

const LIST = "/en/account/admin/feedback";
const at = (n: number, query = "") => `${LIST}/${hex(id(n))}${query}`;

test.use({ serviceWorkers: "block" });

const button = (page: Page, name: string) => page.getByRole("button", { name, exact: true });
const section = (page: Page, name: string) => page.getByRole("region", { name, exact: true });
const lastChange = (seen: Parameters<typeof asked>[0]) => asked(seen).findLast((r) => r["action"] === "set_report");
/** No change is on its way: a field's Enter sends again (a click on a control waits for this by itself). */
const settled = (page: Page) => expect(button(page, "Add a label")).not.toHaveAttribute("aria-disabled", "true");

/** Tab leads through `stops` in this order, starting behind `from`. */
async function tabs(page: Page, from: Locator, stops: Locator[]) {
  await from.focus();
  for (const stop of stops) {
    await page.keyboard.press("Tab");
    await expect(stop).toBeFocused();
  }
}

test("a report opens, is marked opened, and everything written and sent is text", async ({ page }) => {
  const w = world();
  const o = await open(page, w, at(10, "?status=new"));
  await expect(page.locator("h1")).toHaveText("Report");
  const title = page.locator("#fb-report-title");
  await expect(title).toHaveText(`${HOSTILE} first line`);
  // read, marked opened, placed in the list it came from
  await expect.poll(() => actions(o.seen)).toEqual(expect.arrayContaining(["report", "report_neighbours", "set_reports_seen", "report_admins"]));
  expect(asked(o.seen)).toContainEqual({ action: "report", report: id(10) });
  expect(asked(o.seen)).toContainEqual({ action: "set_reports_seen", reports: [id(10)], seen: true });
  expect(asked(o.seen)).toContainEqual({ action: "report_neighbours", report: id(10), filter: filter({ statuses: ["new"] }), sort: "received", descending: true });
  await expect(page.locator(".fb-steps .meta")).toHaveText("1 of 2");
  await expect(page.locator(".fb-steps").getByRole("link", { name: "Next report" })).toHaveAttribute("href", at(15, "?status=new"));
  await expect(page.locator(".fb-steps").getByRole("link", { name: "Back to the list" })).toHaveAttribute("href", `${LIST}?status=new`);

  // what was written: the words as written, the client's references in their place
  const wrote = section(page, "What they wrote");
  await expect(wrote.locator(".fb-text")).toHaveText(`${HOSTILE} first line\nand Llanowar Elves by @${HOSTILE_TOKEN}`);
  await expect(wrote.locator(".fb-chip").first()).toHaveAttribute("title", "Llanowar Elves · battlefield, seat 1");
  await expect(wrote.locator("pre")).toHaveText(`panicked ${HOSTILE}`);
  await expect(wrote.locator("code")).toHaveText(`src/${HOSTILE_TOKEN}.rs:1:2`);
  // the facts: the gateway's address as text, never a link
  const facts = section(page, "Facts");
  await expect(facts).toContainText(`Gate ${HOSTILE} · calls itself Says ${HOSTILE}`);
  await expect(facts.locator("code").filter({ hasText: "javascript:" })).toHaveText(`javascript:alert(1)//${HOSTILE}`);
  await expect(page.locator("a[href^='javascript']")).toHaveCount(0);
  await expect(facts.getByText(HOSTILE_TOKEN, { exact: true })).toBeVisible();

  // what the client sent: read once, inflated in the browser, drawn as text
  await expect.poll(() => w.parts).toEqual([`${hex(id(10))}/client`]);
  const sent = section(page, "What the client sent");
  const shot = sent.locator("canvas");
  await expect(shot).toHaveAttribute("aria-label", "The client's window when the report was written, 64 × 36 pixels");
  await expect(shot).toHaveAttribute("width", "64");
  // drawn from its own bytes, not through an address
  await expect.poll(() => shot.evaluate((c: HTMLCanvasElement) => c.getContext("2d")?.getImageData(1, 1, 1, 1).data[2])).toBe(0xcd);
  await expect(sent.locator("dl.facts").first()).toContainText(`Combat ${HOSTILE}`);
  await expect(sent.locator("dl.facts").first()).toContainText("Choose attackers");
  await sent.getByText("Everything, as a tree").click();
  const tree = sent.locator(".fb-json").last();
  await expect(tree.locator(".fb-j-key").filter({ hasText: HOSTILE })).toHaveCount(1);
  await tree.locator("summary").filter({ hasText: "screenshot:" }).click();
  await expect(tree).toContainText(/png_base64: [\d,]+ characters of base64/);
  await tree.locator("summary").filter({ hasText: HOSTILE }).click();
  await expect(tree.getByRole("button", { name: "Show all 400 characters" })).toBeVisible();
  await sound(page);
  await harmless(page, o);
  await tabs(page, page.locator(".fb-steps").getByRole("link", { name: "Back to the list" }), [
    page.locator(".fb-steps").getByRole("link", { name: "Next report" }),
    button(page, "Copy the link"),
  ]);
});

test("triage: each change is one request at the revision read; a conflict and a lost answer read it again", async ({ page }) => {
  const w = world();
  const o = await open(page, w, at(15));
  await expect(page.locator("#fb-report-title")).toHaveText(`=HYPERLINK("https://x.invalid","click")`);
  const status = page.locator("#fb-status-triaged");
  await status.click();
  await expect(status).toHaveAttribute("aria-pressed", "true");
  await expect(status).toBeFocused();
  expect(lastChange(o.seen)).toEqual({ action: "set_report", report: id(15), change: { status: "triaged" }, expected_revision: 1 });
  await page.locator("#fb-priority-high").click();
  await expect(page.locator("#fb-priority-high")).toHaveAttribute("aria-pressed", "true");
  expect(lastChange(o.seen)).toEqual({ action: "set_report", report: id(15), change: { priority: "high" }, expected_revision: 2 });
  await page.locator("#fb-priority-none").click();
  await expect.poll(() => lastChange(o.seen)).toEqual({ action: "set_report", report: id(15), change: { priority: null }, expected_revision: 3 });
  await page.locator("#fb-assignee").selectOption(hex(OTHER_ADMIN));
  await expect.poll(() => lastChange(o.seen)).toEqual({ action: "set_report", report: id(15), change: { assignee: OTHER_ADMIN }, expected_revision: 4 });
  await button(page, "Assign to me").click();
  await expect.poll(() => lastChange(o.seen)).toEqual({ action: "set_report", report: id(15), change: { assignee: ME }, expected_revision: 5 });
  await expect(button(page, "Assign to me")).toHaveCount(0);
  await page.locator("#fb-assignee").selectOption("");
  await expect.poll(() => lastChange(o.seen)).toEqual({ action: "set_report", report: id(15), change: { assignee: null }, expected_revision: 6 });
  await settled(page);

  // labels: the whole set; a label Central would refuse is not sent
  const before = o.seen.length;
  await page.locator("#fb-label-new").fill("Bad Label!");
  await page.locator("#fb-label-new").press("Enter");
  await expect(page.locator("#fb-label-bad")).toHaveText("A label is 1 to 32 lowercase letters, digits and dashes.");
  // the field, first reached, read the labels in use; no change was sent
  expect(actions(o.seen, before)).toEqual(["report_facets"]);
  await page.locator("#fb-label-new").fill("Needs-Repro");
  await page.locator("#fb-label-new").press("Enter");
  await expect.poll(() => lastChange(o.seen)).toEqual({ action: "set_report", report: id(15), change: { labels: ["needs-repro"] }, expected_revision: 7 });
  await expect(page.locator(".fb-label-list a")).toHaveText("needs-repro");
  await expect(page.locator(".fb-label-list a")).toHaveAttribute("href", `${LIST}?label=needs-repro`);
  // its answer held back: the change is on its way while a duplicate is pasted
  const { promise: held, resolve: release } = Promise.withResolvers<void>();
  w.hold = held;
  await page.getByRole("button", { name: "Remove the label needs-repro" }).click();
  await expect.poll(() => lastChange(o.seen)).toEqual({ action: "set_report", report: id(15), change: { labels: [] }, expected_revision: 8 });
  await expect(button(page, "Mark")).toHaveAttribute("aria-disabled", "true");

  // a duplicate, pasted as an address: an Enter meanwhile sends nothing and keeps what was typed
  const address = `https://baylee.app${at(12)}`;
  await page.locator("#fb-duplicate").fill(address);
  await page.locator("#fb-duplicate").press("Enter");
  await expect(page.locator("#fb-duplicate")).toHaveValue(address);
  expect(lastChange(o.seen)).toEqual({ action: "set_report", report: id(15), change: { labels: [] }, expected_revision: 8 });
  release();
  await expect(page.locator(".fb-label-list a")).toHaveCount(0);
  await settled(page);
  await page.locator("#fb-duplicate").press("Enter");
  await expect.poll(() => lastChange(o.seen)).toEqual({ action: "set_report", report: id(15), change: { duplicate_of: id(12) }, expected_revision: 9 });
  // and not one
  await button(page, "Not a duplicate").click();
  await expect.poll(() => lastChange(o.seen)).toEqual({ action: "set_report", report: id(15), change: { duplicate_of: null }, expected_revision: 10 });
  await page.locator("#fb-duplicate").fill(hex(id(15)));
  await page.locator("#fb-duplicate").press("Enter");
  await expect(page.getByText("That is not another report's id.")).toBeVisible();

  // a GitHub issue: by its number only, linked in Baylee's repository
  const before2 = o.seen.length;
  await page.locator("#fb-issue").fill("javascript:alert(1)");
  await page.locator("#fb-issue").press("Enter");
  await expect(page.getByText("That is no issue of Baylee's repository.")).toBeVisible();
  expect(o.seen.slice(before2)).toEqual([]);
  await page.locator("#fb-new-issue, details:has(#fb-issue-summary) summary").first().click();
  await page.locator("#fb-issue-summary").fill("The stack overflows\nwhen attacking");
  const form = page.getByRole("link", { name: /Open GitHub's form/ });
  const href = (await form.getAttribute("href")) ?? "";
  expect(href).toMatch(/^https:\/\/github\.com\/AceVik\/baylee\/issues\/new\?title=Feedback%3A\+The\+stack\+overflows&body=/);
  // nothing of the report reaches it but its category and a build that reads as a version
  expect(href).not.toContain("HYPERLINK");
  expect(href).toContain("Build%3A+0.1.1-beta.9b");
  await page.locator("#fb-issue").fill("#311");
  await page.locator("#fb-issue").press("Enter");
  await expect.poll(() => lastChange(o.seen)).toEqual({ action: "set_report", report: id(15), change: { issue: 311 }, expected_revision: 11 });
  await expect(page.locator(".fb-issue")).toHaveAttribute("href", "https://github.com/AceVik/baylee/issues/311");
  await expect(page.locator(".fb-issue")).toHaveAttribute("rel", "noopener noreferrer");
  await button(page, "Unlink").click();
  await expect.poll(() => lastChange(o.seen)).toEqual({ action: "set_report", report: id(15), change: { issue: null }, expected_revision: 12 });
  await sound(page);

  // another administrator changed it meanwhile: read again, said, decided again
  named(w, "=HYPERLINK").summary.revision += 5;
  let from = o.seen.length;
  await page.locator("#fb-status-resolved").click();
  await expect(page.locator("#fb-report-error")).toHaveText("Another administrator changed this report meanwhile. It shows the report as it is now: decide again.");
  expect(actions(o.seen, from)).toEqual(["set_report", "report"]);
  // no answer: perhaps made, so read again before anything else
  w.next["set_report"] = "lost";
  from = o.seen.length;
  await page.locator("#fb-status-resolved").click();
  await expect(page.locator("#fb-report-error")).toHaveText(/^No answer came back, so this page cannot tell whether that was done\./);
  expect(actions(o.seen, from)).toEqual(["set_report", "report"]);
  await expect(page.locator("#fb-status-resolved")).toHaveAttribute("aria-pressed", "true");
  await sound(page);
});

test("notes: added and removed by their author; a draft whose answer was lost stays", async ({ page }) => {
  const w = world();
  const o = await open(page, w, at(10));
  const notes = page.locator(".fb-notes li");
  await expect(notes).toHaveCount(1);
  await expect(notes.first().locator(".fb-note-text")).toHaveText(`note ${HOSTILE}`);
  // another administrator's note is theirs to remove
  await expect(notes.first().getByRole("button")).toHaveCount(0);
  const mine = `mine ${HOSTILE}`;
  await page.locator("#fb-note").fill(mine);
  await expect(page.locator(".fb-note-foot")).toContainText(`Characters left: ${(4000 - [...mine].length).toLocaleString("en")}`);
  await button(page, "Save the note").click();
  await expect(notes).toHaveCount(2);
  expect(asked(o.seen).at(-1)).toEqual({ action: "add_report_note", report: id(10), text: `mine ${HOSTILE}` });
  await expect(page.locator("#fb-note")).toHaveValue("");
  await expect(page.locator("#fb-note")).toBeFocused();
  await notes.last().getByRole("button", { name: /^Remove your note of / }).click();
  await expect(notes).toHaveCount(1);
  expect(asked(o.seen).at(-1)).toEqual({ action: "remove_report_note", report: id(10), note: id(101) });

  w.next["add_report_note"] = "lost";
  const from = o.seen.length;
  await page.locator("#fb-note").fill("kept");
  await page.locator("#fb-note").press("Control+Enter");
  await expect(page.locator("#fb-report-error")).toHaveText(/^No answer came back/);
  expect(actions(o.seen, from)).toEqual(["add_report_note", "report"]);
  await expect(page.locator("#fb-note")).toHaveValue("kept");
  await harmless(page, o);
});

test("the trash: moved with Undo, and deleting for good is asked first, sent once, and leads to the trash", async ({ page }) => {
  const w = world();
  const o = await open(page, w, at(13, "?status=resolved"));
  await expect(page.locator("#fb-report-title")).toHaveText("Report number 13");
  await button(page, "Move to the trash").click();
  await expect(page.locator(".fb-trashed")).toContainText("In the trash since ");
  expect(asked(o.seen)).toContainEqual({ action: "trash_report", report: id(13) });
  await expect(page.getByText("A report in the trash is not triaged. Restore it first.")).toBeVisible();
  await page.locator(".fb-notice").getByRole("button", { name: "Undo" }).click();
  await expect(page.locator(".fb-trashed")).toHaveCount(0);
  expect(asked(o.seen)).toContainEqual({ action: "restore_report", report: id(13) });
  await button(page, "Move to the trash").click();
  await expect(page.locator(".fb-trashed")).toBeVisible();
  await page.locator("#fb-restore").click();
  await expect(page.locator(".fb-trashed")).toHaveCount(0);
  expect(asked(o.seen).filter((x) => x["action"] === "restore_report")).toHaveLength(2);
  await button(page, "Move to the trash").click();
  await expect(page.locator(".fb-trashed")).toBeVisible();
  await sound(page);

  const from = o.seen.length;
  await page.locator("#fb-purge").click();
  await expect(page.locator("#fb-purge-title")).toBeFocused();
  await expect(page.locator(".fb-confirm")).toContainText("Reports to delete: 1.");
  expect(o.seen.slice(from)).toEqual([]);
  await sound(page);
  await page.locator("#fb-purge-go").dblclick();
  await expect(page).toHaveURL(`${LIST}?trash=1&status=resolved`);
  expect(o.seen.slice(from).filter((b) => b.operation === "admin" && (b.request as Record<string, unknown>)["action"] === "purge_report")).toHaveLength(1);
  expect(w.reports.some((r) => r.summary.excerpt === "Report number 13")).toBe(false);
});

test("the record is handed over as Central keeps it, and a client part not arrived is said", async ({ page }) => {
  const w = world();
  await open(page, w, at(12));
  await expect(section(page, "What the client sent")).toContainText("The client's details have not arrived yet.");
  const facts = section(page, "Facts");
  await expect(facts).toContainText("3.9 KiB, written by the client, unverified, the game's end is not in it");
  const download = page.waitForEvent("download");
  await facts.getByRole("button", { name: "Download the record (.gz)" }).click();
  const file = await download;
  expect(file.suggestedFilename()).toBe(`baylee-report-${hex(id(12)).slice(-8)}-record.jsonl.gz`);
  expect(w.parts).toEqual([`${hex(id(12))}/record`]);
  await sound(page);
});

test("refusals in clear words; an address no report has asks nothing", async ({ page }) => {
  const w = world();
  const o = await open(page, w, `${LIST}/${"00".repeat(16)}`);
  // not a report's id: said, and nothing asked
  await expect(page.locator(".fb-page, .fb-report")).toHaveCount(0);
  await expect(page.getByRole("alert")).toHaveText("This report does not exist, or no longer exists.");
  expect(asked(o.seen).filter((r) => r["action"] !== "report_admins")).toEqual([]);
  await page.goto(at(99));
  await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
  await expect(page.locator(".fb-page [role=alert]").first()).toHaveText("This report does not exist, or no longer exists.");
  await sound(page);
  await page.goto(at(11));
  await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
  await expect(page.locator("#fb-report-title")).toHaveText("Report number 11");
  w.admin = false;
  await page.locator("#fb-status-resolved").click();
  await expect(page.locator("#fb-report-error")).toHaveText("Your account no longer holds the administrator role.");
  // the client's details refused, or unreadable, say so
  w.admin = true;
  w.client = { not: "a client" };
  await page.goto(at(10));
  await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
  await expect(section(page, "What the client sent")).toContainText("No screenshot.");
  w.client = 503;
  await page.goto(at(10));
  await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
  await expect(section(page, "What the client sent").getByRole("alert")).toHaveText("Baylee Central is not answering right now. Try again in a moment.");
});

test("German, and the longest words, hold at 320 px", async ({ page }) => {
  const w = world();
  await open(page, w, `/de/account/admin/feedback/${hex(id(10))}`);
  await expect(page.locator("h1")).toHaveText("Bericht");
  await expect(page.locator("#fb-report-title")).toBeVisible();
  await expect(section(page, "Was der Client mitgeschickt hat").locator("canvas")).toBeVisible();
  await sound(page);
  await page.goto(`/de/account/admin/feedback/${hex(id(16))}`);
  await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
  await page.locator("#fb-purge").click();
  await expect(page.locator("#fb-purge-title")).toHaveText("Endgültig löschen?");
  await sound(page);
});
