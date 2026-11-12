import { expect, test } from "@playwright/test";

import { NAV, asked, button, nav, open, sound, tabs, world } from "./admin-central.ts";

// The administration's navigation and its first view
// (crates/central-web/src/islands/admin/common.rs, overview.rs) in a real
// browser against a stand-in Central (admin-central.ts): only an
// administrator sees either, and anyone else's browser asks Central for
// no administration's read at all.

// a route does not see what a service worker fetches: the canned Worker
test.use({ serviceWorkers: "block" });

const PAGES = [
  ["/en/account/admin/overview", "Administration"],
  ["/en/account/admin/users", "Accounts"],
  [`/en/account/admin/users/${"0a".repeat(16)}`, "An account"],
  ["/en/account/admin/gateways", "Gateways"],
  ["/en/account/admin/audit", "Audit log"],
  ["/en/account/admin/legacy", "Legacy accounts"],
  ["/en/account/admin", "Reset a login"],
] as const;

test("without the role every page of the administration says so, has no navigation and asks nothing", async ({ page }) => {
  const w = world({ admin: false });
  const { seen, problems } = await open(page, w, "/en/account");
  // the account's own overview leads nowhere there
  await expect(page.locator(".account-who")).toBeVisible();
  await expect(page.locator("a[href*='/account/admin']")).toHaveCount(0);
  // the reported pictures' page too (its moderation is picture.spec.ts's)
  for (const [path, title] of [...PAGES, ["/en/account/admin/pictures", "Reported profile pictures"] as const]) {
    await page.goto(path);
    await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
    await expect(page.locator("h1")).toHaveText(title);
    await expect(page.getByText("This page is only for administrators.")).toBeVisible();
    await expect(nav(page)).toHaveCount(0);
    // (the language menu names the page itself in each language)
    await expect(page.locator(".frame a[href*='/account/admin']")).toHaveCount(0);
    await expect(page.locator("table, .admin-stats, .admin-gws, form")).toHaveCount(0);
  }
  expect(seen.filter((body) => body.operation === "admin")).toEqual([]);
  await sound(page);
  expect(problems).toEqual([]);
});

test("an administrator finds the navigation on every page of the administration, the reset code's included", async ({ page }) => {
  const w = world();
  const { problems } = await open(page, w, "/en/account");
  // the account area's overview leads to the administration's
  const link = page.getByRole("link", { name: "Administration", exact: true });
  await expect(link).toHaveAttribute("href", "/en/account/admin/overview");
  await link.click();
  await expect(page).toHaveURL(/\/en\/account\/admin\/overview$/);
  for (const [path, title] of PAGES) {
    await page.goto(path);
    await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
    await expect(page.locator("h1")).toHaveText(title);
    const links = nav(page).getByRole("link");
    await expect(links).toHaveText(NAV);
    expect(await links.evaluateAll((all) => all.map((a) => a.getAttribute("href")))).toEqual([
      "/en/account/admin/overview", "/en/account/admin/users", "/en/account/admin/gateways", "/en/account/admin/audit",
      "/en/account/admin/legacy", "/en/account/admin/pictures", "/en/account/admin/feedback", "/en/account/admin",
    ]);
    // where the page is
    const current = await links.evaluateAll((all) => all.map((a) => a.getAttribute("aria-current")));
    const at = NAV.indexOf(title === "Administration" ? "Overview" : title === "An account" ? "Accounts" : title);
    expect(current).toEqual(NAV.map((_, i) => (i === at ? (title === "An account" ? "true" : "page") : null)));
    // the area's navigation is everyone's, and has no way there
    await expect(page.locator(".frame-nav a[href*='/account/admin']")).toHaveCount(0);
  }
  // the keyboard goes through it in its order
  await page.goto("/en/account/admin/overview");
  await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
  const links = nav(page).getByRole("link");
  await tabs(page, links.first(), NAV.slice(1).map((name) => nav(page).getByRole("link", { name, exact: true })));
  expect(problems).toEqual([]);
});

test("the overview counts, leads to the work, and says what waits", async ({ page }) => {
  const w = world();
  const { seen, problems } = await open(page, w, "/en/account/admin/overview");
  const tiles = page.locator(".admin-stats > li");
  await expect(tiles).toHaveCount(6);
  await expect(tiles.locator("h2")).toHaveText([
    "Accounts", "Awaiting approval", "Open reports", "Gateways in the list", "Administrators", "Left to move",
  ]);
  await expect(tiles.locator(".admin-stat-n")).toHaveText(["6", "4", "7", "1", "2", "3"]);
  await expect(tiles.nth(0).locator(".admin-stat-sub")).toHaveText("2 new this week, 1 moved from the official gateway.");
  await expect(tiles.nth(1).locator(".admin-stat-sub")).toHaveText("Approve each only after checking who runs its address.");
  await expect(tiles.nth(2).locator(".admin-stat-sub")).toHaveText("New crashes waiting for you: 2.");
  await expect(tiles.nth(3).locator(".admin-stat-sub")).toHaveText("1 online now, 1 disabled, 2 live tokens.");
  await expect(tiles.nth(4).locator(".admin-stat-sub")).toHaveText("Every change they make is in the audit log.");
  await expect(tiles.nth(5).locator(".admin-stat-sub")).toHaveText(/^Accounts of the official gateway that have not moved yet, until .+2026\.$/);
  // what asks for attention says so in its colour too
  await expect(tiles.nth(1)).toHaveClass(/admin-stat-warn/);
  await expect(tiles.nth(2)).toHaveClass(/admin-stat-warn/);
  await expect(tiles.nth(0)).not.toHaveClass(/admin-stat-warn/);
  const go = tiles.locator("a");
  await expect(go).toHaveText(["All accounts", "Review gateways", "Triage reports", "Approved gateways", "Audit log", "Legacy accounts"]);
  expect(await go.evaluateAll((all) => all.map((a) => a.getAttribute("href")))).toEqual([
    "/en/account/admin/users", "/en/account/admin/gateways#state=pending", "/en/account/admin/feedback",
    "/en/account/admin/gateways#state=approved", "/en/account/admin/audit", "/en/account/admin/legacy",
  ]);
  // three reads, and nothing that changes
  expect(asked(seen).map((r) => r?.action).sort()).toEqual(["legacy_accounts", "overview", "report_facets"]);
  expect(asked(seen).find((r) => r?.action === "report_facets")?.filter).toMatchObject({ statuses: ["new"] });
  await sound(page);
  await tabs(page, nav(page).getByRole("link", { name: "Reset a login" }), [...(await go.all())]);
  // the approved gateways' link opens their filter
  await go.nth(3).click();
  await expect(page).toHaveURL(/\/en\/account\/admin\/gateways#state=approved$/);
  await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
  await expect(button(page, "Approved 1")).toHaveAttribute("aria-pressed", "true");
  expect(problems).toEqual([]);
});

test("a calm overview, and the tiles whose read gives nothing are left out", async ({ page }) => {
  const w = world({ legacy: null, reports: null });
  w.gateways = w.gateways.filter((g) => g.state !== "pending");
  await open(page, w, "/en/account/admin/overview");
  const tiles = page.locator(".admin-stats > li");
  await expect(tiles.locator("h2")).toHaveText(["Accounts", "Awaiting approval", "Gateways in the list", "Administrators"]);
  await expect(tiles.nth(1).locator(".admin-stat-n")).toHaveText("0");
  await expect(tiles.nth(1).locator(".admin-stat-sub")).toHaveText("Nothing waits for you.");
  await expect(tiles.nth(1)).not.toHaveClass(/admin-stat-warn/);
  await sound(page);
  // no new crash: the reports' tile is calm
  w.reports = { statuses: [["new", 1]], crashes: 0 };
  w.legacy = { deadline: null, accounts: [] };
  await page.reload();
  await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
  await expect(tiles).toHaveCount(6);
  await expect(tiles.nth(2).locator(".admin-stat-sub")).toHaveText("No new crash. Bugs, ideas and feedback from the players.");
  await expect(tiles.nth(2)).not.toHaveClass(/admin-stat-warn/);
  await expect(tiles.nth(5).locator(".admin-stat-sub")).toHaveText("No deadline is set: the rows stay until the operator sets one.");
});

test("counts that do not come are said, and asked for again", async ({ page }) => {
  const w = world({ failing: { overview: 503 } });
  const { seen } = await open(page, w, "/en/account/admin/overview");
  await expect(page.getByRole("alert")).toHaveText("Baylee Central did not say how many there are.");
  await expect(page.locator(".admin-stats")).toHaveCount(0);
  await sound(page);
  w.failing = {};
  const from = seen.length;
  await button(page, "Try again").click();
  await expect(page.locator(".admin-stats > li")).toHaveCount(6);
  expect(asked(seen, from).map((r) => r?.action)).toEqual(["overview"]);
  // the role gone meanwhile: said as such
  w.admin = false;
  await page.reload();
  await expect(page.getByText("This page is only for administrators.")).toBeVisible();
});

test("German holds at 320 px", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 812 });
  const w = world();
  await open(page, w, "/de/account/admin/overview");
  await expect(page.locator("h1")).toHaveText("Administration");
  const links = page.getByRole("navigation", { name: "Administration" }).getByRole("link");
  await expect(links).toHaveText(["Übersicht", "Accounts", "Gateways", "Protokoll", "Alte Konten", "Gemeldete Profilbilder", "Feedback", "Anmeldung zurücksetzen"]);
  await expect(page.locator(".admin-stats > li h2")).toHaveText([
    "Accounts", "Wartet auf Freigabe", "Offene Meldungen", "Gateways in der Liste", "Administratoren", "Noch umzuziehen",
  ]);
  await sound(page);
});
