// The account area and the administration against a real Central in its
// deployed mode (see README.md): an owner registers, is appointed by the
// operator's own command, registers a gateway, mints a token, changes a tag
// and approves the gateway; the token is kept in REAL_OUT for a QA gateway's
// console, and every page is photographed. Synthetic data only.
//
// REAL_KEYS: a file of beta keys; REAL_OUT: a directory for the run's name,
// token and pictures; REAL_SERVICE: the central-service binary; REAL_MIGRATE:
// the migrator's document (`--migrate-stdin`'s), from which the appointment
// takes its database login. Run with `--project desktop`.

import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { expect, test, type Page } from "@playwright/test";

const KEY = readFileSync(process.env["REAL_KEYS"] ?? "", "utf8").split("\n")[0]?.trim() ?? "";
const OUT = process.env["REAL_OUT"] ?? "";
const PASSWORD = "violet lantern harbour quietly";
const NAME = `live_${String(Date.now() % 1_000_000)}`;
/** The QA gateway's registered origin; its console is reached on loopback. */
const QA_ORIGIN = "https://qa-gateway.invalid";

async function shoot(page: Page, file: string, mask: string[] = []) {
  await page.waitForTimeout(400);
  await page.screenshot({ path: join(OUT, `${file}.png`), fullPage: true, mask: mask.map((m) => page.locator(m)) });
}

/** The operator appoints `name` (`--appoint-admin-stdin`, docs/operations.md §"Administration"). */
function appoint(name: string) {
  const migrate = JSON.parse(readFileSync(process.env["REAL_MIGRATE"] ?? "", "utf8")) as { database: unknown };
  execFileSync(process.env["REAL_SERVICE"] ?? "", ["--appoint-admin-stdin"], {
    input: JSON.stringify({ database: migrate.database, username: name }),
    stdio: ["pipe", "ignore", "inherit"],
  });
}

test("an owner registers a gateway, mints a token, and approves it as an administrator", async ({ page }) => {
  expect(KEY).toMatch(/^CENTRAL-/);
  await page.goto("/register?lang=en");
  await page.getByLabel("Beta key").fill(KEY);
  await page.getByLabel("Username").fill(NAME);
  await page.getByLabel("Display name").fill("Live_Owner");
  await page.getByLabel("Password", { exact: true }).fill(PASSWORD);
  await page.getByLabel("Password again").fill(PASSWORD);
  await page.getByLabel("I am at least 16 years old.").check();
  await page.getByRole("checkbox", { name: /I accept the terms of use/ }).check();
  await page.getByRole("button", { name: "Create account" }).click();
  const step = page.getByRole("region", { name: "Your recovery code" });
  await expect(step).toBeVisible({ timeout: 60_000 });
  await step.getByLabel("I have kept my recovery code in a safe place.").check();
  await step.getByRole("button", { name: "Continue" }).click();
  await expect(page).toHaveURL(/\/account$/, { timeout: 60_000 });
  writeFileSync(join(OUT, "owner"), NAME);

  appoint(NAME);
  await page.reload();
  await expect(page.locator(".app-nav").getByRole("link", { name: "Accounts" })).toBeVisible({ timeout: 30_000 });
  await shoot(page, "01-account-profile");

  await page.locator(".app-nav").getByRole("link", { name: "Security and sessions" }).click();
  await expect(page.getByText("This website")).toBeVisible();
  await shoot(page, "02-account-security");

  await page.locator(".app-nav").getByRole("link", { name: "Your gateways" }).click();
  await page.getByLabel("Address").fill(QA_ORIGIN);
  await page.getByLabel("Name in the list").fill("QA table");
  await page.getByRole("button", { name: "Register" }).click();
  await expect(page.getByRole("heading", { name: "QA table" })).toBeVisible();
  await expect(page.getByText("Awaiting approval").first()).toBeVisible();

  await page.getByRole("button", { name: "Make a token" }).click();
  const dialog = page.getByRole("dialog");
  await dialog.getByLabel("Name").fill("QA machine");
  await dialog.getByRole("button", { name: "Make the token" }).click();
  const secret = dialog.locator(".secret code");
  await expect(secret).toHaveText(/^bgt_[0-9a-f]{64}$/, { timeout: 30_000 });
  writeFileSync(join(OUT, "token"), (await secret.textContent()) ?? "", { mode: 0o600 });
  await shoot(page, "03-token-shown-once", [".secret code"]);
  await dialog.getByRole("button", { name: "I kept it" }).click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await shoot(page, "04-account-gateways");

  // the administration: the owner's own account, another tag
  await page.locator(".app-nav").getByRole("link", { name: "Accounts" }).click();
  await expect(page.getByRole("link", { name: /^Live_Owner#/ })).toBeVisible();
  await shoot(page, "05-admin-accounts");
  await page.getByRole("link", { name: /^Live_Owner#/ }).click();
  const tag = (Date.now() % 0xefff) + 0x1000;
  await page.getByLabel("Tag", { exact: true }).fill(`#${tag.toString(16)}`);
  await page.getByRole("button", { name: "Set the tag" }).click();
  await page.getByRole("dialog").getByRole("button", { name: "Set the tag" }).click();
  await expect(page.getByText("The tag is set.")).toBeVisible();
  await expect(page.getByRole("heading", { name: `Live_Owner#${tag.toString(16).padStart(4, "0")}` })).toBeVisible();
  await shoot(page, "06-admin-account");

  await page.locator(".app-nav").getByRole("link", { name: "Gateways", exact: true }).click();
  await expect(page.getByRole("heading", { name: "QA table" })).toBeVisible();
  await shoot(page, "07-admin-gateways-pending");
  await page.getByRole("button", { name: "Approve" }).click();
  await page.getByRole("dialog").getByRole("button", { name: "Approve" }).click();
  await expect(page.getByText("The gateway is approved.")).toBeVisible();

  await page.locator(".app-nav").getByRole("link", { name: "Audit log" }).click();
  await expect(page.getByText("Gateway approved")).toBeVisible();
  await expect(page.getByText("Tag changed")).toBeVisible();
  await shoot(page, "08-admin-audit");
});
