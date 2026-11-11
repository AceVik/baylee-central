// Real browser workers, OPAQUE, cookie sessions and Central/PostgreSQL.
// Uses the isolated fixture described in README.md; never real accounts.
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { expect, test, type Page } from "@playwright/test";

const password = "violet lantern harbour quietly";
const key = readFileSync(process.env["REAL_KEYS"] ?? "", "utf8").split("\n")[0]?.trim() ?? "";
const stamp = String(Date.now() % 1_000_000);
async function register(page: Page, username: string, display: string): Promise<string> {
  await page.goto("/register?lang=en");
  await page.getByLabel("Beta key").fill(key);
  await page.getByLabel("Username").fill(username);
  await page.getByLabel("Display name").fill(display);
  await page.getByLabel("Password", { exact: true }).fill(password);
  await page.getByLabel("Repeat password").fill(password);
  await page.getByLabel("I am at least 16 years old.").check();
  await page.getByRole("checkbox", { name: /I accept the terms of use/ }).check();
  await page.getByRole("button", { name: "Create account" }).click();
  const recovery = page.getByRole("region", { name: "Your recovery code" });
  await expect(recovery).toBeVisible({ timeout: 60_000 });
  await recovery.getByLabel("I have kept my recovery code in a safe place.").check();
  await recovery.getByRole("button", { name: "Continue" }).click();
  await page.getByRole("button", { name: "I accept", exact: true }).click();
  await expect(page).toHaveURL(/\/account$/);
  const name = page.locator(".account-name bdi");
  await expect(name).toHaveText(new RegExp(`^${display}#[a-f0-9]{4,8}$`));
  return (await name.textContent()) ?? "";
}
async function unlock(page: Page) {
  await page.goto("/account/chat?lang=en");
  await page.getByLabel("Account password").fill(password);
  await page.getByRole("button", { name: "Unlock", exact: true }).click();
  await expect(page.getByRole("button", { name: "Lock chat" })).toBeVisible({ timeout: 60_000 });
}
async function send(page: Page, text: string) {
  await page.getByLabel("Write a message").fill(text);
  await page.getByRole("button", { name: "Send", exact: true }).click();
  await expect(page.getByRole("log").getByText(text, { exact: true })).toBeVisible();
  await expect(page.locator(".chat-pending")).toHaveCount(0);
}

test("two accounts exchange private messages, verify keys and manage a group", async ({ browser, page, context }, info) => {
  test.setTimeout(180_000);
  const phone = info.project.name === "phone";
  const peer = await browser.newContext({ baseURL: "https://baylee.app", ignoreHTTPSErrors: true, viewport: phone ? { width: 412, height: 915 } : { width: 1366, height: 900 } });
  const other = await peer.newPage();
  const failures: string[] = [];
  for (const tab of [page, other]) tab.on("pageerror", (error) => failures.push(error.message));
  for (const c of [context, peer]) await c.route("**/*", (route) => new URL(route.request().url()).hostname === "baylee.app" ? route.continue() : route.fulfill({ status: 503, body: "synthetic test: external service disabled" }));
  try {
    const alice = await register(page, `ca_${info.project.name}_${stamp}`, "Chat_Alice");
    const contactToggle = page.getByRole("switch", { name: "Contact requests", exact: true });
    await expect(contactToggle).toBeChecked();
    await contactToggle.click(); await expect(contactToggle).not.toBeChecked();
    await page.reload(); await expect(contactToggle).not.toBeChecked();
    await contactToggle.click(); await expect(contactToggle).toBeChecked();
    if (process.env["REAL_OUT"]) await page.screenshot({ path: join(process.env["REAL_OUT"], `notifications-${info.project.name}.png`), fullPage: true });
    const bob = await register(other, `cb_${info.project.name}_${stamp}`, "Chat_Bob");
    await page.goto("/account/contacts?lang=en");
    await page.getByLabel("Public handle or tag").fill(bob);
    await page.getByRole("button", { name: "Find", exact: true }).click();
    await page.getByRole("button", { name: "Send request" }).click();
    await other.goto("/account/contacts?lang=en");
    await other.getByRole("button", { name: "Accept", exact: true }).click();
    await unlock(page); await unlock(other);
    await page.getByRole("button", { name: bob, exact: true }).click();
    await other.getByRole("button", { name: alice, exact: true }).click();
    const direct = "Synthetic private message: Grüße 🦊 <b>plain text</b>";
    await send(page, direct);
    await expect(other.getByRole("log").getByText(direct, { exact: true })).toBeVisible({ timeout: 15_000 });
    expect(await other.getByRole("log").locator("b").count()).toBe(0);
    for (const [tab, handle] of [[page, bob], [other, alice]] as const) {
      await tab.locator(".chat-safety summary").click();
      await tab.locator(".chat-safety").getByRole("button", { name: handle, exact: true }).click();
    }
    expect(await page.locator(".chat-number").textContent()).toBe(await other.locator(".chat-number").textContent());
    await page.getByRole("dialog").getByRole("button", { name: "Confirm these keys" }).click();
    await other.getByRole("dialog").getByRole("button", { name: "Confirm these keys" }).click();
    await unlock(other);
    await other.getByRole("button", { name: alice, exact: true }).click();
    await expect(other.getByRole("log").getByText(direct, { exact: true })).toBeVisible({ timeout: 15_000 });
    await page.getByLabel("Group name").fill("Synthetic group");
    await page.getByRole("button", { name: "Create group" }).click();
    await page.getByText(/Manage group/).click();
    await page.getByLabel("Invite friend").selectOption({ label: bob });
    await page.getByRole("button", { name: "Invite friend", exact: true }).click();
    await other.getByRole("button", { name: "Accept", exact: true }).click();
    const groupText = "A message for both accepted group members";
    await send(page, groupText);
    await expect(other.getByRole("log").getByText(groupText, { exact: true })).toBeVisible({ timeout: 15_000 });
    if (process.env["REAL_OUT"]) await other.screenshot({ path: join(process.env["REAL_OUT"], `chat-${info.project.name}.png`), fullPage: true });
    expect(await other.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    await other.getByText(/Manage group/).click();
    await other.getByRole("button", { name: "Leave group", exact: true }).click();
    await other.getByRole("dialog").getByRole("button", { name: "Leave group", exact: true }).click();
    await expect(other.getByText("You are no longer a member of this group. Previously delivered history remains readable.")).toBeVisible();
    await expect(other.getByRole("log").getByText(groupText, { exact: true })).toBeVisible();
    await expect(other.getByLabel("Write a message")).toHaveCount(0);
    const stored = await other.evaluate(() => JSON.stringify(window.localStorage));
    expect(stored).not.toContain(direct); expect(stored).not.toContain(password);
    expect(stored).toContain("baylee-chat-trust:v1:");
    await other.getByRole("button", { name: "Lock chat" }).click();
    await expect(other.getByLabel("Account password")).toBeVisible();
    await expect(other.getByRole("log")).toHaveCount(0);
    expect(failures).toEqual([]);
  } finally { await peer.close(); }
});
