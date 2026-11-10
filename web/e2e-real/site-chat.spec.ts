// The server-rendered site's messenger (crates/central-web) against a real
// Central: real OPAQUE and Argon2 in the page's Worker, the real cookie, the
// committed central-web-crypto module, PostgreSQL. Two synthetic accounts
// register, become contacts, write both ways, reload without a password,
// compare safety numbers, share a group and leave it. Set up as README.md
// §"The new site" describes; never real accounts.
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { expect, test, type Page } from "@playwright/test";

const password = "violet lantern harbour quietly";
const key = readFileSync(process.env["REAL_KEYS"] ?? "", "utf8").split("\n")[0]?.trim() ?? "";
const stamp = String(Date.now() % 1_000_000);

async function register(page: Page, username: string, display: string): Promise<string> {
  await page.goto("/en/register");
  await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
  await page.getByLabel("Beta key").fill(key);
  await page.getByLabel("Sign-in name").fill(username);
  await page.getByLabel("Display name").fill(display);
  await page.getByLabel("Password", { exact: true }).fill(password);
  await page.getByLabel("Password again").fill(password);
  await page.getByLabel("I am 16 or older.").check();
  await page.getByRole("checkbox", { name: /I accept the terms of use/ }).check();
  await page.getByRole("button", { name: "Create account" }).click();
  await expect(page.getByRole("heading", { name: "Your recovery code" })).toBeVisible({ timeout: 90_000 });
  await page.getByLabel("I have saved my recovery code").check();
  await page.getByRole("button", { name: "Continue" }).click();
  // where Central's terms are in force, they come after the code
  const terms = page.getByRole("heading", { name: "New terms of use" });
  await expect(terms.or(page.locator(".account-who"))).toBeVisible({ timeout: 30_000 });
  if (await terms.isVisible()) await page.getByRole("button", { name: "Accept", exact: true }).click();
  await expect(page).toHaveURL(/\/en\/account$/, { timeout: 30_000 });
  const handle = page.locator(".account-who .handle");
  await expect(handle).toHaveText(new RegExp(`^${display}#[a-f0-9]{4,8}$`), { timeout: 30_000 });
  return (await handle.textContent()) ?? "";
}

async function openChat(page: Page, name: string) {
  await page.goto("/en/account/chat");
  await expect(page.getByRole("status").filter({ hasText: "Opening your chats" })).toHaveCount(0, { timeout: 60_000 });
  await page.locator(".conv-row", { hasText: name }).click();
  await expect(page.getByRole("heading", { level: 2, name })).toBeVisible();
}

async function send(page: Page, text: string) {
  await page.getByLabel("Write a message").fill(text);
  await page.getByRole("button", { name: "Send", exact: true }).click();
  await expect(page.locator(".conv-msgs").getByText(text, { exact: true })).toBeVisible({ timeout: 30_000 });
  await expect(page.locator(".msg.pending")).toHaveCount(0, { timeout: 30_000 });
}

test("two accounts write to each other, keep the chat across a reload and share a group", async ({ browser, page, context }, info) => {
  test.setTimeout(300_000);
  const phone = info.project.name === "phone";
  const peer = await browser.newContext({
    baseURL: "https://baylee.app",
    ignoreHTTPSErrors: true,
    viewport: phone ? { width: 412, height: 915 } : { width: 1366, height: 900 },
  });
  const other = await peer.newPage();
  const failures: string[] = [];
  for (const tab of [page, other]) {
    tab.on("pageerror", (error) => failures.push(error.message));
    tab.on("console", (message) => {
      if (message.type() === "error") failures.push(message.text());
    });
  }
  for (const c of [context, peer]) {
    await c.route("**/*", (route) =>
      new URL(route.request().url()).hostname === "baylee.app" ? route.continue() : route.fulfill({ status: 503, body: "synthetic test: external service disabled" }),
    );
  }
  try {
    const alice = await register(page, `sa_${info.project.name}_${stamp}`, "Site_Alice");
    const bob = await register(other, `sb_${info.project.name}_${stamp}`, "Site_Bob");
    const [aliceName, bobName] = [alice.split("#")[0] ?? "", bob.split("#")[0] ?? ""];

    // contacts: a request by handle, accepted on the other side
    await page.goto("/en/account/contacts");
    await page.getByLabel("Add a contact by their handle").fill(bob);
    await page.getByRole("button", { name: "Find", exact: true }).click();
    await page.getByRole("button", { name: "Send a contact request" }).click();
    await expect(page.getByText("Request sent.")).toBeVisible();
    await other.goto("/en/account/contacts");
    await other.getByRole("button", { name: "Accept", exact: true }).click();
    await expect(other.locator(".people", { hasText: "Contacts" }).getByText(alice)).toBeVisible({ timeout: 15_000 });

    // both ways, as text: markup in a message stays text
    await openChat(page, bobName);
    await openChat(other, aliceName);
    const direct = "Synthetic private message: Grüße 🦊 <b>plain text</b>";
    await send(page, direct);
    await expect(other.locator(".conv-msgs").getByText(direct, { exact: true })).toBeVisible({ timeout: 30_000 });
    expect(await other.locator(".conv-msgs b").count()).toBe(0);
    await send(other, "And back.");
    await expect(page.locator(".conv-msgs").getByText("And back.", { exact: true })).toBeVisible({ timeout: 30_000 });

    // a reload opens the chat again from what this browser kept: no password
    await other.reload();
    await expect(other.getByLabel("Password")).toHaveCount(0);
    await expect(other.locator(".conv-msgs").getByText(direct, { exact: true })).toBeVisible({ timeout: 60_000 });

    // the safety number is the same on both sides
    for (const tab of [page, other]) await tab.getByRole("button", { name: "Details" }).click();
    const numbers = await Promise.all([page, other].map((tab) => tab.locator(".safety-number").first().textContent()));
    expect(numbers[0]).toMatch(/^(\d{5} ){11}\d{5}$/);
    expect(numbers[0]).toBe(numbers[1]);
    for (const tab of [page, other]) await tab.getByRole("button", { name: "Close" }).click();

    // a group: created, a contact invited, the invitation taken, a message for both
    // on a phone the list and the conversation take turns
    if (phone) await page.getByRole("button", { name: "Back to the conversations" }).click();
    else await page.getByRole("link", { name: /^Chats/ }).click();
    await page.getByRole("button", { name: "New group" }).click();
    await page.getByLabel("Group name").fill("Synthetic group");
    await page.getByRole("button", { name: "Create", exact: true }).click();
    await expect(page.getByRole("heading", { level: 2, name: "Synthetic group" })).toBeVisible({ timeout: 15_000 });
    await page.getByRole("button", { name: "Details" }).click();
    await page.getByLabel("Invite a contact").selectOption({ label: bob });
    await page.getByRole("button", { name: "Invite", exact: true }).click();
    await other.goto("/en/account/chat");
    await other.getByRole("button", { name: "Accept", exact: true }).click({ timeout: 30_000 });
    await expect(other.getByRole("heading", { level: 2, name: "Synthetic group" })).toBeVisible({ timeout: 30_000 });
    await page.getByRole("button", { name: "Close" }).click();
    const groupText = "A message for both group members";
    await send(page, groupText);
    await expect(other.locator(".conv-msgs").getByText(groupText, { exact: true })).toBeVisible({ timeout: 30_000 });
    if (process.env["REAL_OUT"]) {
      for (const [tab, who] of [[page, "alice"], [other, "bob"]] as const) {
        await tab.screenshot({ path: join(process.env["REAL_OUT"], `site-chat-${who}-${info.project.name}.png`) });
      }
    }
    expect(await other.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);

    // leaving: the history stays, the composer goes
    await other.getByRole("button", { name: "Details" }).click();
    await other.getByRole("button", { name: "Leave group", exact: true }).click();
    await other.getByRole("group", { name: "Leave group" }).getByRole("button", { name: "Leave group" }).click();
    await other.goto("/en/account/chat");
    await other.locator(".conv-row", { hasText: "A former group" }).click({ timeout: 30_000 });
    await expect(other.locator(".conv-msgs").getByText(groupText, { exact: true })).toBeVisible({ timeout: 30_000 });
    await expect(other.getByLabel("Write a message")).toHaveCount(0);

    // nothing readable is kept: no message, no password
    const stored = await other.evaluate(() => JSON.stringify(window.localStorage));
    expect(stored).not.toContain(direct);
    expect(stored).not.toContain(password);
    expect(stored).toContain("baylee-chat-trust:v1:");
    expect(failures).toEqual([]);
  } finally {
    await peer.close();
  }
});
