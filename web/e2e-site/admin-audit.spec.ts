import { expect, type Page, test } from "@playwright/test";

import { ID, OWN, asked, button, changes, eventOf, hex, id, open, sound, tabs, world } from "./admin-central.ts";

// The audit log and the legacy store (crates/central-web/src/islands/admin/audit.rs,
// legacy.rs) in a real browser against a stand-in Central (admin-central.ts):
// reads only, ids named by today's names where Central gives them, an
// action the page does not know shown as Central names it, and a Central
// without a legacy store said as such.

test.use({ serviceWorkers: "block" });

const rows = (page: Page) => page.locator(".admin-table tbody tr");
const end = (bytes: number[]) => `…${hex(bytes).slice(24)}`;

test("the audit log, newest first, with today's names, page by page", async ({ page }) => {
  const w = world();
  const { seen, problems } = await open(page, w, "/en/account/admin/audit");
  await expect(rows(page)).toHaveCount(3);
  expect(asked(seen)).toEqual([{ action: "audit", account: null, before: null }]);
  await expect(page.locator(".admin-lead")).toHaveText(
    "What administrators and the operator changed, newest first. The log keeps ids; the names beside them are today's.",
  );
  await expect(page.locator(".admin-table thead th")).toHaveText(["When", "What", "By", "About"]);
  await expect(rows(page).locator("th")).toHaveText(["Made administrator", "Gateway approved", "Tag changed"]);
  const first = rows(page).first();
  await expect(first.locator("td").nth(0)).toHaveText(/2026, \d\d:\d\d UTC$/);
  await expect(first.locator("td").nth(1)).toHaveText("Chef#0c4e");
  await expect(first.locator("td").nth(1).locator("a")).toHaveAttribute("href", `/en/account/admin/users/${hex(OWN)}`);
  await expect(first.locator("td").nth(2)).toHaveText("Ada#3c4d");
  await expect(first.locator("td").nth(2).locator("a")).toHaveAttribute("title", hex(ID.ada));
  // a gateway by its name, its address beside
  await expect(rows(page).nth(1).locator("td").nth(2)).toHaveText("Sun Court");
  await expect(rows(page).nth(1).locator("td span")).toHaveAttribute("title", "https://sun-court.example");
  await sound(page);
  await tabs(page, page.locator(".admin-table"), [
    first.locator("td").nth(1).locator("a"), first.locator("td").nth(2).locator("a"), rows(page).nth(1).locator("a"),
  ]);

  // the next page, the keyboard at its first event; its names join the others
  await button(page, "Show more").click();
  await expect(rows(page)).toHaveCount(6);
  await expect(rows(page).nth(3).locator("th")).toBeFocused();
  expect(asked(seen).at(-1)).toEqual({ action: "audit", account: null, before: eventOf(3, "", null, null).id });
  // an account and a gateway that are gone: the ends of their ids
  const gone = rows(page).nth(4);
  await expect(gone.locator("th")).toHaveText("Gateway token revoked");
  await expect(gone.locator("td").nth(1)).toHaveText(end(id(77)));
  await expect(gone.locator("td").nth(1).locator("a")).toHaveAttribute("href", `/en/account/admin/users/${hex(id(77))}`);
  await expect(gone.locator("td").nth(2)).toHaveText(`gw ${end(id(0x4f))}`);
  // registration's switch: no account, no gateway
  await expect(rows(page).nth(5).locator("th")).toHaveText("Registration closed");
  await expect(rows(page).nth(5).locator("td").nth(2)).toHaveText("");
  await button(page, "Show more").click();
  await expect(rows(page)).toHaveCount(8);
  // the operator, and an action this page does not know, as Central names it
  await expect(rows(page).nth(6).locator("td").nth(1)).toHaveText("The operator");
  await expect(rows(page).nth(7).locator("th code")).toHaveText("account.locked");
  await expect(button(page, "Show more")).toHaveCount(0);
  await sound(page);
  expect(changes(seen)).toEqual([]);
  expect(problems).toEqual([]);
});

test("an empty log, and a page that does not come, are said; trying again goes on where it stopped", async ({ page }) => {
  const w = world({ events: [] });
  await open(page, w, "/en/account/admin/audit");
  await expect(page.getByText("Nothing recorded yet.")).toBeVisible();
  w.events = [eventOf(1, "account.admin_granted", OWN, ID.ada), eventOf(2, "gateway.approved", OWN, null), eventOf(3, "account.tag_changed", OWN, ID.erika), eventOf(4, "account.reset_issued", OWN, ID.jonas)];
  const { seen } = await open(page, w, "/en/account/admin/audit");
  await expect(rows(page)).toHaveCount(3);
  w.failing = { audit: 503 };
  await button(page, "Show more").click();
  await expect(page.getByRole("alert")).toHaveText("Baylee Central is not answering right now. Try again in a moment.");
  await expect(rows(page)).toHaveCount(3);
  await sound(page);
  w.failing = {};
  const from = seen.length;
  await button(page, "Try again").click();
  await expect(rows(page)).toHaveCount(4);
  expect(asked(seen, from)).toEqual([{ action: "audit", account: null, before: eventOf(3, "", null, null).id }]);
  await expect(page.getByRole("alert")).toHaveCount(0);
});

test("the legacy store: its deadline, what is left and what holds each back", async ({ page }) => {
  const w = world();
  const { seen, problems } = await open(page, w, "/en/account/admin/legacy");
  await expect(rows(page)).toHaveCount(3);
  expect(asked(seen)).toEqual([{ action: "legacy_accounts" }]);
  await expect(page.locator(".admin-lead")).toContainText("Names, times and counters only: never a password hash.");
  await expect(page.locator("#admin-legacy-title")).toHaveText("Left to move");
  await expect(page.locator("section > p").first()).toHaveText(/^Left: 3\. After .+2026, the daily sweep deletes every row still here\.$/);
  await expect(page.locator(".admin-table thead th")).toHaveText(["Account", "Move", "Held back", "Failed sign-ins", "Imported"]);
  await expect(rows(page).locator("th")).toHaveText(["oldtimer Oldtimer#0077", "halfway", "settler Settler#1234"]);
  await expect(rows(page).nth(0).locator("td")).toHaveText(["Not started", "–", "0", /2026$/]);
  await expect(rows(page).nth(1).locator("td").nth(0)).toHaveText("Name reserved");
  await expect(rows(page).nth(1).locator("td").nth(1).locator(".pill")).toHaveText([
    "Banned", "New password required", /^Locked until .+ UTC$/,
  ]);
  await expect(rows(page).nth(1).locator("td").nth(2)).toHaveText("3");
  // a lockout that is over holds nothing back
  await expect(rows(page).nth(2).locator("td").nth(0)).toHaveText("Restored");
  await expect(rows(page).nth(2).locator("td").nth(1)).toHaveText("–");
  // the table scrolls in its own box, which the keyboard reaches
  await expect(page.locator(".admin-table")).toHaveAttribute("tabindex", "0");
  await expect(page.locator(".admin-table")).toHaveAccessibleName("Left to move");
  await sound(page);
  expect(problems).toEqual([]);
});

test("a Central without a legacy store says so; an empty one, and one without a deadline", async ({ page }) => {
  const w = world({ legacy: null });
  await open(page, w, "/en/account/admin/legacy");
  await expect(page.getByText("This Central keeps no legacy accounts.")).toBeVisible();
  await expect(page.locator(".admin-table")).toHaveCount(0);
  await sound(page);
  w.legacy = { deadline: null, accounts: [] };
  await page.reload();
  await expect(page.getByText("No deadline is set: the rows stay until the operator sets one.")).toBeVisible();
  await expect(page.getByText("Every account has moved. Nothing is left.")).toBeVisible();
  // Central not answering: said, and asked again
  w.failing = { legacy_accounts: 503 };
  const { seen } = await open(page, w, "/en/account/admin/legacy");
  await expect(page.getByRole("alert")).toHaveText("Baylee Central is not answering right now. Try again in a moment.");
  w.failing = {};
  const from = seen.length;
  await button(page, "Try again").click();
  await expect(page.getByText("Every account has moved. Nothing is left.")).toBeVisible();
  expect(asked(seen, from)).toEqual([{ action: "legacy_accounts" }]);
  // the role gone: said as such, not as a missing store
  w.admin = false;
  await page.reload();
  await expect(page.getByText("This page is only for administrators.")).toBeVisible();
});

test("German holds at 320 px", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 812 });
  const w = world();
  await open(page, w, "/de/account/admin/audit");
  await expect(page.locator("h1")).toHaveText("Protokoll");
  await expect(rows(page).locator("th")).toHaveText(["Zum Administrator gemacht", "Gateway freigegeben", "Tag geändert"]);
  await sound(page);
  await page.goto("/de/account/admin/legacy");
  await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
  await expect(page.locator("h1")).toHaveText("Alte Konten");
  await expect(rows(page)).toHaveCount(3);
  await expect(rows(page).nth(1).locator("td").nth(1).locator(".pill").nth(1)).toHaveText("Neues Passwort verlangt");
  await sound(page);
});
