// The password's change in the new site's account area (crates/central-web,
// /{lang}/account/security) against a real Central: real OPAQUE and Argon2
// in the page's Worker, the committed central-web-crypto module, PostgreSQL.
// A synthetic account changes its password; the page stays signed in, the
// client's session ends, the old password is refused and the new one opens
// the same vault (docs/native-login.md §"Password change"). What became of
// the vault and of the client's session is read from outside the page, over
// the native listener (REAL_NATIVE).
//
// One run registers one account; with site-reset.spec.ts that is three of
// the five registrations an hour Central allows one address.

import { expect, test } from "@playwright/test";

import { alive, ask, client, open, operations, problems, refused, register, sent, signedIn, summary } from "./site.ts";

const stamp = String(Date.now() % 1_000_000);
const OLD = "violet lantern harbour quietly";
const NEW = "amber kettle window softly";
const CHANGED =
  "Your password is changed. You stay signed in here and are signed out everywhere else, the Baylee client included; sign in there with the new password.";

test("the account area changes the password: this browser stays, the client is signed out, and the vault is the same", async ({ page, context }, info) => {
  test.setTimeout(300_000);
  const errors = problems(page);
  await context.route("**/*", (route) =>
    new URL(route.request().url()).hostname === "baylee.app" ? route.continue() : route.fulfill({ status: 503, body: "synthetic test: external service disabled" }),
  );
  const name = `pw_${info.project.name}_${stamp}`;
  await register(page, name, "Password_User", OLD);
  const bodies = sent(page);
  const before = await summary(page);
  const first = await client(name, OLD, before.tag);
  expect(first).not.toBeNull();
  if (first === null) return;

  await open(page, "/en/account/security");
  const sessions = page.locator("ul.sessions > li");
  await expect(sessions).toHaveCount(2, { timeout: 30_000 });
  await expect(sessions.filter({ hasText: "The Baylee client" })).toHaveCount(1);
  const card = page.locator("section[aria-labelledby=password-title]");
  const fill = async (current: string) => {
    await card.getByLabel("Current password").fill(current);
    await card.getByLabel("New password", { exact: true }).fill(NEW);
    await card.getByLabel("Repeat new password").fill(NEW);
    await card.getByRole("button", { name: "Change password" }).click();
  };

  // A wrong current password changes nothing, and no change was asked for.
  await fill("not the password at all");
  await expect(card.getByRole("alert")).toHaveText("The current password is not right. Check it and try again.", { timeout: 60_000 });
  expect(operations(bodies).filter((operation) => operation.startsWith("change_password"))).toEqual([]);
  expect(await alive(first)).toBe(true);

  // The right one changes it: said where the form was, with the focus on it.
  await fill(OLD);
  const said = card.getByRole("status");
  await expect(said).toHaveText(CHANGED, { timeout: 60_000 });
  await expect(said).toBeFocused();
  await expect(card.locator("input")).toHaveCount(0);
  expect(operations(bodies).filter((operation) => operation.startsWith("change_password"))).toEqual(["change_password_start", "change_password_finish"]);
  // signed out everywhere else: the client's session is gone, and the list says so
  expect(await alive(first)).toBe(false);
  await expect(sessions).toHaveCount(1);
  await expect(sessions).toContainText("This website");
  // signed in here: Central still answers this browser, across a reload too
  const after = await summary(page);
  expect(after.identity).toEqual(before.identity);
  expect(after.recovery).toBe(true);
  await page.reload();
  await expect(page.getByRole("heading", { name: "Where you are signed in" })).toBeVisible({ timeout: 30_000 });
  await expect(page.locator("ul.sessions > li")).toHaveCount(1, { timeout: 30_000 });

  // The chat opens from what this browser kept, as before the change, and
  // across a reload: no password is asked for (owner, 03.11.2026). The
  // master key is the same, so the kept identity still is this account's.
  await open(page, "/en/account/chat");
  for (const reloaded of [false, true]) {
    if (reloaded) await page.reload();
    // the opened chat's own greeting (a phone shows the list in its place, so it is asked for in the page, not on the screen)
    await expect(page.locator(".msgr-main .msgr-welcome-icon")).toBeAttached({ timeout: 60_000 });
    await expect(page.locator(".msgr-side")).toBeVisible();
    await expect(page.locator("#chat-missing-title")).toHaveCount(0);
    await expect(page.locator(".msgr-main").getByLabel("Password")).toHaveCount(0);
    await expect(page.locator(".msgr-main [role=alert]")).toHaveCount(0);
  }
  await open(page, "/en/account/security");

  // The old password opens nothing any more; the new one opens the same vault.
  expect(await client(name, OLD, before.tag)).toBeNull();
  const second = await client(name, NEW, before.tag);
  expect(second?.identity).toEqual(before.identity);
  expect(second?.device).toBe(first.device);
  expect(second?.document).toBe(first.document);
  expect(second?.revision).toBe(first.revision);

  // And on the page: signed out here, the old password is refused, the new one signs in.
  await page.getByRole("button", { name: "Sign out here" }).click();
  await expect(page).toHaveURL(/\/en\/$/, { timeout: 30_000 });
  expect((await ask(page, { operation: "account_summary" })).status).toBe(401);
  await refused(page, name, OLD);
  await signedIn(page, name, NEW);

  // Neither password went to Central, nor stays in the browser.
  const all = bodies.join("\n");
  const stored = await page.evaluate(() => JSON.stringify({ ...localStorage, ...sessionStorage }));
  for (const secret of [OLD, NEW]) {
    expect(all).not.toContain(secret);
    expect(stored).not.toContain(secret);
  }
  expect(errors).toEqual([]);
});
