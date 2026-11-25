import { expect, type Locator, type Page, test } from "@playwright/test";

import { ID, PASSWORD, SECRET, asked, button, changes, hex, id, open, sound, tabs, world } from "./admin-central.ts";

// The report tokens (crates/central-web/src/islands/admin/tokens.rs) in a
// real browser against a stand-in Central (admin-central.ts): what each may
// do and where it stands, the making from a preset or from its rights, the
// token shown once, and a revocation after its confirmation, each sent once.

test.use({ serviceWorkers: "block" });

const PAGE = "/en/account/admin/feedback-tokens";
const TEXT = `brt_${hex(SECRET)}`;
const cards = (page: Page) => page.locator("#rt-tokens > li");
const card = (page: Page, name: string) => cards(page).filter({ has: page.getByRole("heading", { name, exact: true }) });
const facts = (scope: Locator) => scope.locator("dd");
const preset = (page: Page, name: string) => page.locator(".rt-making .chip-btn", { hasText: new RegExp(`^${name}$`) });
const right = (page: Page, name: string) => page.locator(`#rt-right-${name}`);
const held = async (page: Page) => {
  const found: string[] = [];
  for (const name of ["file", "read", "triage", "purge"]) if (await right(page, name).isChecked()) found.push(name);
  return found;
};

test("the tokens: what each may do, who made it, and where it stands", async ({ page }) => {
  const { seen, problems } = await open(page, world(), PAGE);
  await expect(cards(page).locator("h2")).toHaveText(["Triage agent", "Nightly build", "Old helper", "Leaked", "Last year"]);
  expect(asked(seen)).toEqual([{ action: "report_tokens" }]);
  await expect(cards(page).locator(".pill")).toHaveText(["Active", "Active", "Paused", "Revoked", "Expired"]);
  const agent = card(page, "Triage agent");
  await expect(agent.locator("dt")).toHaveText(["Id", "Its rights", "Created by", "Created", "Expires", "Last used"]);
  // whole, so that the operator can revoke by it
  await expect(facts(agent).nth(0).locator("code")).toHaveText(hex(id(0xc1)));
  await expect(facts(agent).nth(1)).toHaveText("Read and edit");
  await expect(facts(agent).nth(2)).toHaveText("Chef#0c4e");
  await expect(facts(agent).nth(5)).not.toHaveText("Never");
  // rights no preset holds are named one by one
  await expect(facts(card(page, "Nightly build")).nth(1)).toHaveText("File reports · Read reports and download their parts");
  await expect(facts(card(page, "Nightly build")).nth(5)).toHaveText("Never");
  await expect(card(page, "Old helper")).toContainText("Its maker is not an administrator any more, so it does nothing.");
  await expect(facts(card(page, "Old helper")).nth(2).locator("a")).toHaveAttribute("href", `/en/account/admin/users/${hex(ID.erika)}`);
  await expect(card(page, "Leaked")).toContainText(/Revoked .+ by Ada#3c4d/);
  // a maker who is gone is named by its id's end
  await expect(facts(card(page, "Last year")).nth(2)).toHaveText(`…${hex(id(77)).slice(24)}`);
  // only what still works, or would again, is revoked
  await expect(page.getByRole("button", { name: "Revoke", exact: true })).toHaveCount(3);
  expect(changes(seen)).toEqual([]);
  await sound(page);
  expect(problems).toEqual([]);
});

test("the rights follow their needs, and a preset sets them all", async ({ page }) => {
  const { problems } = await open(page, world(), PAGE);
  // the least is where it begins
  expect(await held(page)).toEqual(["file"]);
  await expect(preset(page, "Only post")).toHaveAttribute("aria-pressed", "true");
  for (const [name, rights] of [
    ["Only read", ["read"]], ["Read and edit", ["read", "triage"]], ["Everything", ["file", "read", "triage", "purge"]], ["Only post", ["file"]],
  ] as const) {
    await preset(page, name).click();
    expect(await held(page)).toEqual(rights);
    await expect(preset(page, name)).toHaveAttribute("aria-pressed", "true");
    await expect(page.locator(".rt-making .chip-btn[aria-pressed=true]")).toHaveCount(1);
  }
  // deleting for good brings editing and reading, and reading taken takes both
  await right(page, "purge").check();
  expect(await held(page)).toEqual(["file", "read", "triage", "purge"]);
  await right(page, "triage").uncheck();
  expect(await held(page)).toEqual(["file", "read"]);
  await expect(page.locator(".rt-making .chip-btn[aria-pressed=true]")).toHaveCount(0);
  await right(page, "triage").check();
  await right(page, "read").uncheck();
  expect(await held(page)).toEqual(["file"]);
  await tabs(page, page.locator("#rt-name"), [
    preset(page, "Only post"), preset(page, "Only read"), preset(page, "Read and edit"), preset(page, "Everything"),
    right(page, "file"), right(page, "read"), right(page, "triage"), right(page, "purge"), page.locator("#rt-days"), button(page, "Create token"),
  ]);
  expect(problems).toEqual([]);
});

test("a token is made once, shown once, and gone when it is put away", async ({ page, context, browserName }) => {
  if (browserName === "chromium") await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  const w = world();
  const { seen, problems } = await open(page, w, PAGE);
  // nothing is sent without a name, or without a right
  await button(page, "Create token").click();
  await expect(page.locator("#rt-wrong")).toHaveText("Give the token a name and at least one thing it allows.");
  await expect(page.locator("#rt-name")).toBeFocused();
  await page.locator("#rt-name").fill("CI");
  await right(page, "file").uncheck();
  await button(page, "Create token").click();
  await expect(page.locator("#rt-wrong")).not.toBeEmpty();
  expect(changes(seen)).toEqual([]);
  await preset(page, "Read and edit").click();
  await page.locator("#rt-days").selectOption("180");
  await button(page, "Create token").click();
  await expect(page.locator("#rt-shown")).toBeFocused();
  expect(changes(seen)).toEqual([{ action: "create_report_token", name: "CI", scopes: ["read", "triage"], days: 180 }]);
  await expect(page.locator(".secret code")).toHaveText(TEXT);
  await expect(page.locator(".secret-box")).toContainText("Authorization: Bearer");
  // it is in no address, no storage and no field
  expect(page.url()).not.toContain(TEXT.slice(4));
  const kept = await page.evaluate(() => JSON.stringify({ ...localStorage }) + JSON.stringify({ ...sessionStorage }));
  expect(kept).not.toContain(TEXT.slice(4));
  expect(await page.locator("input").evaluateAll((all) => all.map((one) => (one as HTMLInputElement).value).join())).not.toContain(TEXT.slice(4));
  // the list holds it by its name already
  await expect(cards(page).locator("h2").first()).toHaveText("CI");
  await sound(page);
  if (browserName === "chromium") {
    await button(page.locator(".secret-box"), "Copy").click();
    await expect(page.locator(".secret-box [role=status]")).toHaveText("Copied.");
    expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(TEXT);
  }
  await button(page, "I have saved it").click();
  await expect(page.locator("#rt-new")).toBeFocused();
  await expect(page.locator(".secret-box")).toHaveCount(0);
  expect(await page.content()).not.toContain(TEXT.slice(4));
  await expect(page.locator("#rt-name")).toHaveValue("");
  expect(changes(seen)).toHaveLength(1);
  expect(problems).toEqual([]);
});

test("an old sign-in asks for the password first, and stepping back sends nothing", async ({ page }) => {
  const w = world({ stale: true });
  const { seen, problems } = await open(page, w, PAGE);
  await page.locator("#rt-name").fill("CI");
  await button(page, "Create token").click();
  await expect(page.locator("#admin-password")).toBeFocused();
  expect(changes(seen)).toEqual([]);
  await sound(page);
  await button(page, "Create token").click();
  await expect(page.getByText("Enter your password.")).toBeVisible();
  await button(page, "Back").click();
  await expect(page.locator("#rt-name")).toBeFocused();
  // what was asked for is still there
  await expect(page.locator("#rt-name")).toHaveValue("CI");
  await button(page, "Create token").click();
  await page.locator("#admin-password").fill(PASSWORD);
  await button(page, "Create token").click();
  await expect(page.locator("#rt-shown")).toBeFocused();
  expect(changes(seen)).toHaveLength(1);
  expect(JSON.stringify(seen)).not.toContain(PASSWORD);
  expect(problems).toEqual([]);
});

test("a making that fails says why, and one without an answer sends nobody looking for a token to keep", async ({ page }) => {
  for (const [next, text] of [
    [429, "Baylee Central made no token. Either 32 tokens are active, or 32 were made in the last day. Revoke one, or try again later."],
    ["lost", "No answer came back, so a token may have been made that nobody can show. Look for it in the list and revoke it."],
    ["other", "No answer came back, so a token may have been made that nobody can show. Look for it in the list and revoke it."],
  ] as const) {
    const w = world({ next });
    const { seen } = await open(page, w, PAGE);
    await page.locator("#rt-name").fill("CI");
    await button(page, "Create token").click();
    await expect(page.locator("#rt-wrong")).toHaveText(text);
    await expect(page.locator("#rt-wrong")).toBeFocused();
    await expect(page.locator("#rt-name")).toHaveValue("CI");
    await expect(page.locator(".secret")).toHaveCount(0);
    await expect(cards(page)).toHaveCount(next === 429 ? 5 : 6);
    expect(changes(seen)).toHaveLength(1);
    await page.unrouteAll();
  }
});

test("a revocation asks first, is sent once, and the list says what stands", async ({ page }) => {
  const w = world();
  const { seen, problems } = await open(page, w, PAGE);
  const agent = card(page, "Triage agent");
  await button(agent, "Revoke").click();
  await expect(page.locator("#admin-confirm-title")).toBeFocused();
  await expect(page.locator("#admin-confirm-title")).toHaveText("Revoke the token “Triage agent”?");
  await expect(page.locator(".admin-confirm")).toContainText("It stops working at once and cannot be brought back. What it filed and changed stays.");
  expect(changes(seen)).toEqual([]);
  await sound(page);
  await button(page.locator(".admin-confirm"), "Back").click();
  await expect(button(agent, "Revoke")).toBeFocused();
  expect(changes(seen)).toEqual([]);
  await button(agent, "Revoke").click();
  await button(page.locator(".admin-confirm"), "Revoke").click();
  await expect(page.locator("#admin-done")).toHaveText("The feedback token is revoked.");
  await expect(page.locator("#admin-done")).toBeFocused();
  expect(changes(seen)).toEqual([{ action: "revoke_report_token", token: id(0xc1) }]);
  await expect(agent.locator(".pill")).toHaveText("Revoked");
  await expect(agent).toContainText(/Revoked .+ by Chef#0c4e/);
  await expect(button(agent, "Revoke")).toHaveCount(0);
  expect(problems).toEqual([]);
});

test("the page holds no token after the browser put it away", async ({ page }) => {
  const { problems } = await open(page, world(), PAGE);
  await page.locator("#rt-name").fill("CI");
  await button(page, "Create token").click();
  await expect(page.locator(".secret code")).toHaveText(TEXT);
  await page.evaluate(() => window.dispatchEvent(new PageTransitionEvent("pagehide", { persisted: true })));
  await expect(page.locator(".secret")).toHaveCount(0);
  expect(await page.content()).not.toContain(TEXT.slice(4));
  expect(problems).toEqual([]);
});
