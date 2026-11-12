// The new site's reset page (crates/central-web, /{lang}/reset) against a
// real Central: real OPAQUE and Argon2 in the page's Worker, the committed
// central-web-crypto module, PostgreSQL. A synthetic administrator, appointed
// by the operator's own command, makes reset codes for a synthetic account,
// which redeems them both ways (docs/account-reset.md): with its recovery
// code the vault stays, without it a new one begins. What became of the
// vault is read from outside the page: a client's sign-in over the native
// listener tells the master key's fingerprint and the stored document.
//
// REAL_SERVICE: the central-service binary; REAL_MIGRATE: the migrator's
// document (the appointment takes its database login from it); REAL_NATIVE:
// the native listener. One run registers two accounts and starts six
// resets: Central allows five registrations and ten reset starts an hour
// from one address, so run one project against a Central started afresh.

import { expect, test, type Page } from "@playwright/test";

import {
  alive,
  appoint,
  client,
  issue,
  open,
  operations,
  problems,
  RECOVERY,
  refused,
  register,
  sent,
  signedIn,
  summary,
} from "./site.ts";

const stamp = String(Date.now() % 1_000_000);
const FIRST = "violet lantern harbour quietly";
const SECOND = "amber kettle window softly";
const THIRD = "copper meadow bicycle early";
const FOURTH = "silver orchard compass gently";
/** Well formed, and nobody's. */
const NOBODYS = "ZYXW-VTSR-QPNM-KJHG-FEDC-BA98-7654-3210";

const KEEP = "I have my recovery code. My data stays.";
const FRESH = "I do not have it. Start with an empty vault.";
const REFUSED =
  "This sign-in name and reset code do not go together, or the code has expired or was used already. Check both. If nothing helps, ask for a new code.";
const NOT_OPENED =
  "This recovery code does not open your account's data. Check it and try again. Nothing was changed, and your reset code still works.";
const KEPT =
  "Your data and your recovery code stay as they were. You are signed out everywhere; sign in with your new password.";
const EMPTY =
  "You start with an empty vault and are signed out everywhere. Below is your new recovery code; the old one no longer works.";

type Choice = { recovery: string } | "new";

/** Fills the reset page's form and sends it. */
async function redeem(
  page: Page,
  username: string,
  code: string,
  choice: Choice,
  password: string,
) {
  await open(page, "/en/reset");
  // the form that redeems a code; the block before it asks for one by mail
  const form = page.locator("#reset-steps");
  await form.getByLabel("Sign-in name").fill(username);
  await form.getByLabel("Reset code").fill(code);
  if (choice === "new") {
    await page.getByRole("radio", { name: FRESH }).check();
    // the warning stands before anything is sent
    await expect(page.locator("#reset-warning")).toContainText(
      "your encrypted data is lost for good",
    );
    await page
      .getByRole("checkbox", {
        name: "I understand that this data is lost for good.",
      })
      .check();
  } else {
    await expect(page.getByRole("radio", { name: KEEP })).toBeChecked();
    await page
      .getByLabel("Recovery code", { exact: true })
      .fill(choice.recovery);
  }
  await page.getByLabel("New password", { exact: true }).fill(password);
  await page.getByLabel("Repeat new password").fill(password);
  await page.getByRole("button", { name: "Set new password" }).click();
}

const done = (page: Page) =>
  page.getByRole("heading", { name: "Your new password is set" });
const session = (page: Page) =>
  page.evaluate(() => localStorage.getItem("baylee-central-session"));

test("a reset code sets a new password: with the recovery code the vault stays, without it a new one begins", async ({
  browser,
  page,
  context,
}, info) => {
  test.setTimeout(600_000);
  const phone = info.project.name === "phone";
  const office = await browser.newContext({
    baseURL: "https://baylee.app",
    ignoreHTTPSErrors: true,
    viewport: phone
      ? { width: 412, height: 915 }
      : { width: 1366, height: 900 },
  });
  const admin = await office.newPage();
  const errors = problems(page);
  for (const c of [context, office]) {
    await c.route("**/*", (route) =>
      new URL(route.request().url()).hostname === "baylee.app"
        ? route.continue()
        : route.fulfill({
            status: 503,
            body: "synthetic test: external service disabled",
          }),
    );
  }
  try {
    const owner = `ra_${info.project.name}_${stamp}`;
    const name = `ru_${info.project.name}_${stamp}`;
    await register(admin, owner, "Reset_Admin", FIRST);
    appoint(owner);
    const recovery = await register(page, name, "Reset_User", FIRST);
    const bodies = sent(page);
    const before = await summary(page);
    expect(before.recovery).toBe(true);
    // the vault as a client finds it, and that client's session
    const first = await client(name, FIRST, before.tag);
    expect(first).not.toBeNull();
    if (first === null) return;
    expect(first.identity).toEqual(before.identity);
    expect(await alive(first)).toBe(true);

    // 1. With a recovery code that is not the account's, nothing is spent.
    const code = await issue(admin, name);
    await redeem(page, name, code, { recovery: NOBODYS }, SECOND);
    await expect(page.locator("#reset-steps [role=alert]")).toHaveText(
      NOT_OPENED,
      { timeout: 60_000 },
    );
    expect(
      operations(bodies).filter((operation) => operation.startsWith("reset")),
    ).toEqual(["reset_start"]);
    expect(await alive(first)).toBe(true);
    expect(await session(page)).not.toBeNull();

    // 2. With the account's own, the same code sets the password and the vault stays.
    await redeem(page, name, code, { recovery }, SECOND);
    await expect(done(page)).toBeFocused({ timeout: 60_000 });
    await expect(page.getByText(KEPT)).toBeVisible();
    await expect(page.locator("#reset-steps .secret")).toHaveCount(0);
    // signed out everywhere: the client's session, and this browser's, which the page forgets
    expect(await alive(first)).toBe(false);
    expect(await session(page)).toBeNull();
    await page
      .locator("#reset-steps")
      .getByRole("link", { name: "Sign in" })
      .click();
    await expect(page).toHaveURL(/\/en\/signin$/);
    await expect(page.locator("dialog[open]")).toHaveCount(0);
    await refused(page, name, FIRST);
    await signedIn(page, name, SECOND);
    const kept = await summary(page);
    expect(kept.identity).toEqual(before.identity);
    expect(kept.recovery).toBe(true);
    expect(await client(name, FIRST, before.tag)).toBeNull();
    const second = await client(name, SECOND, before.tag);
    // the same master key under the new password, and the document untouched
    expect(second?.device).toBe(first.device);
    expect(second?.document).toBe(first.document);
    expect(second?.revision).toBe(first.revision);

    // 3. A code that was used reads as any wrong one, and changes nothing.
    await redeem(page, name, code, { recovery }, THIRD);
    await expect(page.locator("#reset-steps [role=alert]")).toHaveText(
      REFUSED,
      { timeout: 60_000 },
    );
    expect(second && (await alive(second))).toBe(true);

    // 4. Without the recovery code: a new vault at the next epoch, and its new code shown once.
    const again = await issue(admin, name);
    expect(again).not.toBe(code);
    await redeem(page, name, again, "new", THIRD);
    await expect(done(page)).toBeFocused({ timeout: 60_000 });
    await expect(page.getByText(EMPTY)).toBeVisible();
    const fresh =
      (await page.locator("#reset-steps .secret code").textContent()) ?? "";
    expect(fresh).toMatch(RECOVERY);
    expect(fresh).not.toBe(recovery);
    expect(second && (await alive(second))).toBe(false);
    expect(await session(page)).toBeNull();
    const on = page
      .locator("#reset-steps")
      .getByRole("button", { name: "Sign in" });
    await on.click();
    await expect(page.locator("#reset-steps [role=alert]")).toHaveText(
      "Save the code first, then confirm it.",
    );
    await page.getByLabel("I have saved my recovery code").check();
    await on.click();
    await expect(page).toHaveURL(/\/en\/signin$/);
    await refused(page, name, SECOND);
    await signedIn(page, name, THIRD);
    const emptied = await summary(page);
    expect(emptied.identity).toEqual({
      account: before.identity.account,
      epoch: before.identity.epoch + 1,
    });
    expect(emptied.recovery).toBe(true);
    // the account's name and tag stay
    expect(emptied.tag).toBe(before.tag);
    const third = await client(name, THIRD, before.tag);
    // another master key, and a first document of its own
    expect(third?.identity).toEqual(emptied.identity);
    expect(third?.device).not.toBe(first.device);
    expect(third?.document).not.toBe(first.document);
    expect(third?.revision).toBe(1);

    // 5. The old recovery code opens nothing any more; the new one is the vault's.
    const last = await issue(admin, name);
    await redeem(page, name, last, { recovery }, FOURTH);
    await expect(page.locator("#reset-steps [role=alert]")).toHaveText(
      NOT_OPENED,
      { timeout: 60_000 },
    );
    await redeem(page, name, last, { recovery: fresh }, FOURTH);
    await expect(done(page)).toBeFocused({ timeout: 60_000 });
    await expect(page.getByText(KEPT)).toBeVisible();
    await signedIn(page, name, FOURTH);
    expect((await summary(page)).identity).toEqual(emptied.identity);
    const fourth = await client(name, FOURTH, before.tag);
    expect(fourth?.device).toBe(third?.device);
    expect(fourth?.document).toBe(third?.document);

    // Neither a password nor a recovery code went to Central, nor stays in the browser.
    const all = bodies.join("\n");
    for (const secret of [
      FIRST,
      SECOND,
      THIRD,
      FOURTH,
      recovery,
      fresh,
      NOBODYS,
    ]) {
      expect(all).not.toContain(secret);
      expect(all).not.toContain(secret.replaceAll("-", ""));
    }
    const stored = await page.evaluate(() =>
      JSON.stringify({ ...localStorage, ...sessionStorage }),
    );
    for (const secret of [
      FIRST,
      SECOND,
      THIRD,
      FOURTH,
      recovery,
      fresh,
      code,
      again,
      last,
    ])
      expect(stored).not.toContain(secret);
    // the refusals this run asked for are answers, not faults of the page
    expect(errors).toEqual([]);
  } finally {
    await office.close();
  }
});
