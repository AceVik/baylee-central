import { expect, type Locator, type Page, test } from "@playwright/test";

import { GW, ID, PASSWORD, asked, button, changes, gatewayOf, hex, open, operations, sound, tabs, world } from "./admin-central.ts";

// The gateways (crates/central-web/src/islands/admin/gateways.rs) in a real
// browser against a stand-in Central (admin-central.ts): the queue first,
// the filters with their counts, a gateway's owner and tokens, and its
// approval, disabling, enabling and a token's revocation, each sent once
// and only after its confirmation; after a change the keyboard goes on
// down the list.

test.use({ serviceWorkers: "block" });

const cards = (page: Page) => page.locator("#admin-gws > li");
const card = (page: Page, name: string) => cards(page).filter({ has: page.getByRole("heading", { name, exact: true }) });
const heading = (page: Page, gateway: number[]) => page.locator(`#admin-gw-${hex(gateway)}`);
const chips = (page: Page) => page.locator(".admin-chips button");
const done = (page: Page) => page.locator("#admin-done");
const note = (page: Page) => page.locator("#admin-note");
const confirm = (scope: Page | Locator) => scope.locator(".admin-confirm");

test("the queue first: gateways awaiting approval, their owners and facts, and the filters", async ({ page }) => {
  const w = world();
  const { seen, problems } = await open(page, w, "/en/account/admin/gateways");
  await expect(cards(page)).toHaveCount(3);
  expect(asked(seen).map((r) => r?.action).sort()).toEqual(["gateways", "overview"]);
  expect(asked(seen).find((r) => r?.action === "gateways")).toEqual({ action: "gateways", state: "pending", before: null });
  await expect(page.locator(".admin-lead")).toContainText("Approve a gateway for the public list only after checking outside Baylee Central");
  await expect(chips(page)).toHaveText(["Awaiting approval 4", "Approved 1", "Disabled 1", "All 6"]);
  await expect(chips(page).first()).toHaveAttribute("aria-pressed", "true");
  await expect(cards(page).locator("h2")).toHaveText(["Garden Table", "Moon Hall", "Old Mill"]);
  const garden = card(page, "Garden Table");
  await expect(garden.locator(".pill")).toHaveText(["Awaiting approval", "Offline"]);
  await expect(garden.locator("code.address")).toHaveText("https://garden-table.example");
  await expect(garden.locator("dt")).toHaveText(["Owner", "Created", "Last heartbeat", "Live tokens"]);
  await expect(garden.locator("dd").nth(0)).toHaveText("Erika#1a2b");
  await expect(garden.locator("dd a")).toHaveAttribute("href", `/en/account/admin/users/${hex(ID.erika)}`);
  await expect(garden.locator("dd").nth(2)).toHaveText("–");
  await expect(garden.locator("dd").nth(3)).toHaveText("1");
  // an owner without a handle is named by its id's end; a credential pair is said
  const mill = card(page, "Old Mill");
  await expect(mill.locator("dd").nth(0)).toHaveText(`…${hex(ID.jonas).slice(24)}`);
  await expect(mill.locator("dd").nth(3)).toHaveText("0 · and a credential pair");
  await expect(garden.getByRole("button")).toHaveText(["Copy Address", "Approve", "Disable", "Show tokens"]);
  await expect(button(garden, "Approve")).toHaveAccessibleDescription("Garden Table");
  await sound(page);
  await tabs(page, chips(page).first(), [
    chips(page).nth(1), chips(page).nth(2), chips(page).nth(3), button(garden, "Copy Address"), garden.getByRole("link", { name: "Erika#1a2b" }),
    button(garden, "Approve"), button(garden, "Disable"), button(garden, "Show tokens"),
  ]);

  // the next page, the keyboard at its first
  await button(page, "Show more").click();
  await expect(cards(page)).toHaveCount(4);
  await expect(heading(page, GW.owls)).toBeFocused();
  expect(asked(seen).at(-1)).toEqual({ action: "gateways", state: "pending", before: GW.mill });

  // the approved ones: the address keeps the filter, a reload finds it again
  await chips(page).nth(1).click();
  await expect(chips(page).nth(1)).toHaveAttribute("aria-pressed", "true");
  await expect(cards(page).locator("h2")).toHaveText(["Sun Court", "Baylee"]);
  expect(asked(seen).at(-1)).toEqual({ action: "gateways", state: "approved", before: null });
  await expect(page).toHaveURL(/#state=approved$/);
  const official = card(page, "Baylee");
  await expect(official.locator(".pill")).toHaveText(["Official", "Approved", "Online"]);
  await expect(official.locator("dd").first()).toHaveText("The bridge (official)");
  await expect(official.getByRole("button")).toHaveText(["Copy Address", "Disable", "Show tokens"]);
  await sound(page);
  await page.reload();
  await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
  await expect(cards(page).locator("h2")).toHaveText(["Sun Court", "Baylee"]);
  await expect(chips(page).nth(1)).toHaveAttribute("aria-pressed", "true");
  // every gateway
  await chips(page).nth(3).click();
  await expect(cards(page)).toHaveCount(3);
  expect(asked(seen).at(-1)).toEqual({ action: "gateways", state: null, before: null });
  await expect(page).toHaveURL(/#state=all$/);
  // reading is all that was asked
  expect(changes(seen)).toEqual([]);
  expect(problems).toEqual([]);
});

test("approving asks first, is sent once, and the keyboard goes on down the queue", async ({ page }) => {
  const w = world();
  const { seen, problems } = await open(page, w, "/en/account/admin/gateways");
  const garden = card(page, "Garden Table");
  await button(garden, "Approve").click();
  await expect(page.locator("#admin-confirm-title")).toBeFocused();
  await expect(page.locator("#admin-confirm-title")).toHaveText("Approve Garden Table?");
  await expect(confirm(garden)).toContainText(
    "Approve only if you checked that the owner runs https://garden-table.example. Approved, it appears in the public list.",
  );
  await expect(button(confirm(garden), "Approve")).toHaveClass(/primary/);
  await sound(page);
  await tabs(page, page.locator("#admin-confirm-title"), [button(confirm(garden), "Approve"), button(confirm(garden), "Back")]);
  await button(confirm(garden), "Back").click();
  await expect(button(garden, "Approve")).toBeFocused();
  expect(changes(seen)).toEqual([]);

  // confirmed, twice in a hurry: one change, the list and its counts read again
  await button(garden, "Approve").click();
  let from = seen.length;
  await page.locator("#admin-confirm").dblclick();
  await expect(done(page)).toHaveText("The gateway is approved.");
  expect(changes(seen, from)).toEqual([{ action: "set_gateway_approval", gateway: GW.garden, enabled: true }]);
  expect(operations(seen, from)[0]).toBe("account_summary");
  expect(asked(seen, from).map((r) => r?.action).sort()).toEqual(["gateways", "overview", "set_gateway_approval"]);
  // it left the queue: the keyboard is at the gateway now in its place
  await expect(cards(page).locator("h2")).toHaveText(["Moon Hall", "Old Mill", "Night Owls"]);
  await expect(heading(page, GW.moon)).toBeFocused();
  await expect(chips(page).first()).toHaveText("Awaiting approval 3");
  await sound(page);

  // disabling: its own words, in the colour of what takes away
  const moon = card(page, "Moon Hall");
  await button(moon, "Disable").click();
  await expect(page.locator("#admin-confirm-title")).toHaveText("Disable Moon Hall?");
  await expect(confirm(moon)).toContainText("https://moon-hall.example leaves the public list at once, its tokens stop working");
  await expect(button(confirm(moon), "Disable")).toHaveClass(/danger/);
  from = seen.length;
  await button(confirm(moon), "Disable").click();
  await expect(done(page)).toHaveText("The gateway is disabled.");
  expect(changes(seen, from)).toEqual([{ action: "set_gateway_approval", gateway: GW.moon, enabled: false }]);
  await expect(heading(page, GW.mill)).toBeFocused();

  // a disabled one is enabled again
  await chips(page).nth(2).click();
  await expect(cards(page).locator("h2")).toHaveText(["Moon Hall", "Closed Door"]);
  const again = card(page, "Moon Hall");
  await expect(again.getByRole("button")).toHaveText(["Copy Address", "Enable again", "Show tokens"]);
  await button(again, "Enable again").click();
  await expect(page.locator("#admin-confirm-title")).toHaveText("Approve Moon Hall?");
  from = seen.length;
  await button(confirm(again), "Enable again").click();
  await expect(done(page)).toHaveText("The gateway is approved.");
  expect(changes(seen, from)).toEqual([{ action: "set_gateway_approval", gateway: GW.moon, enabled: true }]);
  await expect(heading(page, GW.closed)).toBeFocused();
  // the last one leaves its list: the keyboard goes to what was said
  await button(card(page, "Closed Door"), "Enable again").click();
  await page.locator("#admin-confirm").click();
  await expect(page.getByText("No gateway in this state.")).toBeVisible();
  await expect(done(page)).toBeFocused();
  expect(problems).toEqual([]);
});

test("a gateway's tokens are shown on request and revoked after their confirmation", async ({ page }) => {
  const w = world();
  const { seen } = await open(page, w, "/en/account/admin/gateways#state=approved");
  const sun = card(page, "Sun Court");
  await expect(sun.locator("dd").nth(2)).toHaveText(/2026, \d\d:\d\d UTC$/);
  const toggle = button(sun, "Show tokens");
  await expect(toggle).toHaveAttribute("aria-expanded", "false");
  let from = seen.length;
  await toggle.click();
  await expect(button(sun, "Hide tokens")).toHaveAttribute("aria-expanded", "true");
  expect(asked(seen, from)).toEqual([{ action: "gateway", gateway: GW.sun }]);
  const tokens = sun.locator(".tokens > li");
  await expect(tokens).toHaveCount(3);
  await expect(tokens.nth(0).locator("p").first()).toHaveText("court main · signs players in and lists the gateway");
  await expect(tokens.nth(0)).toContainText("Until ");
  await expect(tokens.nth(0)).toContainText("Last used ");
  await expect(tokens.nth(1)).toContainText("Revoked");
  await expect(tokens.nth(2).locator("p").first()).toHaveText("listing only · Lists the gateway");
  // only a live token can be revoked
  await expect(sun.getByRole("button", { name: /^Revoke/ })).toHaveCount(1);
  await expect(sun.getByRole("button", { name: "Revoke court main" })).toBeVisible();
  await sound(page);
  await sun.getByRole("button", { name: "Revoke court main" }).click();
  await expect(page.locator("#admin-confirm-title")).toHaveText("Revoke the token “court main”?");
  await expect(confirm(sun)).toContainText("The token stops working at once.");
  expect(changes(seen)).toEqual([]);
  await sound(page);
  from = seen.length;
  await button(confirm(sun), "Revoke").click();
  await expect(done(page)).toHaveText("The token is revoked.");
  expect(changes(seen, from)).toEqual([{ action: "revoke_gateway_token", token: w.gateways[4]?.tokens[0]?.id }]);
  expect(asked(seen, from).map((r) => r?.action).sort()).toEqual(["gateway", "gateways", "overview", "revoke_gateway_token"]);
  await expect(card(page, "Sun Court").locator(".tokens > li").nth(0)).toContainText("Revoked");
  await expect(card(page, "Sun Court").locator("dd").nth(3)).toHaveText("0");
  await expect(card(page, "Sun Court").getByRole("button", { name: /^Revoke/ })).toHaveCount(0);
  await button(card(page, "Sun Court"), "Hide tokens").click();
  await expect(card(page, "Sun Court").locator(".tokens")).toHaveCount(0);
  // a gateway without tokens says so
  await button(card(page, "Baylee"), "Show tokens").click();
  await expect(card(page, "Baylee")).toContainText("No token yet.");
});

test("a stale sign-in asks for the password first; refusals and a lost answer are said", async ({ page }) => {
  const w = world({ stale: true });
  const { seen } = await open(page, w, "/en/account/admin/gateways");
  const garden = card(page, "Garden Table");
  await button(garden, "Approve").click();
  let from = seen.length;
  await button(confirm(garden), "Approve").click();
  await expect(page.locator("#admin-password")).toBeFocused();
  expect(operations(seen, from)).toEqual(["account_summary"]);
  await sound(page);
  from = seen.length;
  await page.locator("#admin-password").fill(PASSWORD);
  await page.locator("#admin-password").press("Enter");
  await expect(done(page)).toHaveText("The gateway is approved.");
  expect(operations(seen, from).slice(0, 3)).toEqual(["account_summary", "login_start", "login_finish"]);
  expect(changes(seen, from)).toEqual([{ action: "set_gateway_approval", gateway: GW.garden, enabled: true }]);
  expect(JSON.stringify(seen)).not.toContain("lantern");

  // a gateway gone meanwhile
  const moon = card(page, "Moon Hall");
  await button(moon, "Approve").click();
  w.gateways = w.gateways.filter((g) => g.metadata.name !== "Moon Hall");
  await button(confirm(moon), "Approve").click();
  await expect(note(page)).toHaveText("This gateway no longer exists.");
  await expect(note(page)).toBeFocused();
  await expect(cards(page).locator("h2")).toHaveText(["Old Mill", "Night Owls"]);
  await sound(page);

  // made, and its answer lost: the list is read again and shows it
  const mill = card(page, "Old Mill");
  await button(mill, "Approve").click();
  w.next = "lost";
  from = seen.length;
  await button(confirm(mill), "Approve").click();
  await expect(note(page)).toHaveText(
    "No answer came back, so we cannot tell whether the change was made. The page has read it again: check what it shows before you try once more.",
  );
  await expect(note(page)).toBeFocused();
  await expect(cards(page).locator("h2")).toHaveText(["Night Owls"]);
  expect(changes(seen, from)).toEqual([{ action: "set_gateway_approval", gateway: GW.mill, enabled: true }]);
  w.gateways.push(gatewayOf(8, "Late Arrival", "pending"));
});

test("German holds at 320 px", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 812 });
  const w = world();
  w.gateways.unshift(gatewayOf(9, "Ein sehr langer Gatewayname für die Runde am Donnerstag", "pending", { credential_pair: true }));
  await open(page, w, "/de/account/admin/gateways");
  await expect(page.locator("h1")).toHaveText("Gateways");
  await expect(chips(page)).toHaveText(["Wartet auf Freigabe 5", "Freigegeben 1", "Abgeschaltet 1", "Alle 7"]);
  await sound(page);
  const first = cards(page).first();
  await button(first, "Abschalten").click();
  await expect(page.locator("#admin-confirm-title")).toHaveText("Ein sehr langer Gatewayname für die Runde am Donnerstag abschalten?");
  await sound(page);
  await button(confirm(first), "Zurück").click();
  await button(first, "Tokens zeigen").click();
  await expect(first).toContainText("Noch kein Token.");
  await sound(page);
});
