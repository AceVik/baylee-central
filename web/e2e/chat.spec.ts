// The messenger against the area mock with the stand-in crypto (`chat.ts`):
// a sign-in unlocks the chat, so it asks for no second password; it lists
// the conversations with their last lines, sends and receives, shows the
// contacts beside them, and after a reload asks once.

import { expect, test, type Page } from "@playwright/test";

import { PASSWORD, mockCentral, stubCrypto } from "./central.ts";
import { mock } from "./mocks.ts";

async function signIn(page: Page, origin: string) {
  const outside = await mock(page, { gateways: 4 });
  const central = await mockCentral(page, origin, { named: { lantern: "Lantern" }, area: { admin: false } });
  await stubCrypto(page);
  await page.goto("/signin?lang=en");
  await page.getByLabel("Username").fill("lantern");
  await page.getByLabel("Password", { exact: true }).fill(PASSWORD);
  await page.getByRole("button", { name: "Sign in" }).click();
  await expect(page).toHaveURL(/\/account$/, { timeout: 15_000 });
  return { outside, central };
}

test.describe("the messenger", () => {
  test("a sign-in unlocks it: conversations, a message and its answer, contacts, no second password", async ({ page, baseURL }, info) => {
    const { outside, central } = await signIn(page, new URL(baseURL ?? "").origin);
    const narrow = info.project.name === "phone";
    await page.getByRole("navigation", { name: /account/i }).getByRole("link", { name: "Chat" }).click();
    await expect(page).toHaveURL(/\/account\/chat$/);
    const side = page.getByRole("complementary", { name: "Conversations" });
    await expect(side.getByRole("button", { name: /Quiet-Heron/ })).toBeVisible();
    await expect(side.getByRole("button", { name: /Friday Night Table/ })).toContainText("See you all on Friday!");
    await expect(page.getByLabel("Account password")).toHaveCount(0);
    await side.getByRole("button", { name: /Quiet-Heron/ }).click();
    const log = page.getByRole("log", { name: "Messages" });
    await expect(log).toContainText("Starting at 8 pm, four players, Commander rules.");
    await expect(log.getByText("Yesterday")).toBeVisible();
    const box = page.getByRole("textbox", { name: "Write a message" });
    await box.fill("Count me in!");
    if (narrow) await page.getByRole("button", { name: "Send" }).click();
    else await box.press("Enter");
    await expect(log.locator(".chat-bubble[data-own=true]").last()).toContainText("Count me in!");
    await expect(box).toHaveValue("");
    await expect(log).toContainText("Sounds good — see you at the table!", { timeout: 10_000 });
    if (narrow) await page.getByRole("button", { name: "Back to the conversations" }).click();
    await page.getByRole("tab", { name: /Contacts/ }).click();
    await expect(side.getByText("Moss_Keeper")).toBeVisible();
    await expect(side.getByRole("button", { name: "Accept" }).first()).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    expect(central.violations).toEqual([]);
    expect(outside.stray).toEqual([]);
  });

  test("after a reload it asks once, then the chat is back", async ({ page, baseURL }) => {
    await signIn(page, new URL(baseURL ?? "").origin);
    await page.goto("/account/chat?lang=en");
    const password = page.getByLabel("Account password");
    await password.first().waitFor({ state: "attached" });
    // the narrow layout opens the lock from the list
    if (!(await password.isVisible())) await page.getByRole("complementary").getByRole("button", { name: "Unlock" }).click();
    await password.fill("not the password");
    await page.getByRole("button", { name: "Unlock", exact: true }).last().click();
    await expect(page.getByRole("alert")).toContainText("Chat could not be unlocked");
    await password.fill(PASSWORD);
    await page.getByRole("button", { name: "Unlock", exact: true }).last().click();
    await expect(password).toHaveCount(0);
    await page.getByRole("complementary").getByRole("button", { name: /Tidepool/ }).click();
    await expect(page.getByRole("log")).toContainText("Rematch on the weekend?");
  });
});
