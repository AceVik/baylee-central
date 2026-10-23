// The account pages against a real Central in its deployed mode, through a
// local stand-in for the site's Caddy, at https://baylee.app (see
// playwright.config.ts). Real OPAQUE, Argon2 and the real __Host- cookie.
// REAL_KEYS: a file of beta keys (`--issue-beta-keys-stdin`); REAL_OUT: a
// directory for each new account's name and recovery code, read back by the
// checks that follow (the database rows, the native sign-in, the code).

import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { expect, test, type BrowserContext, type Page } from "@playwright/test";

const KEY = readFileSync(process.env["REAL_KEYS"] ?? "", "utf8").split("\n")[0]?.trim() ?? "";
const OUT = process.env["REAL_OUT"] ?? "";
const PASSWORD = "violet lantern harbour quietly";
/** The display name every run's account chooses at registration. */
const DISPLAY = "Real_Runner";
const STAMP = String(Date.now() % 1_000_000);
const CODE = /^[0-9A-HJKMNP-TV-Z]{4}(-[0-9A-HJKMNP-TV-Z]{4}){7}$/;

function watch(page: Page): string[] {
  const errors: string[] = [];
  page.on("console", (m) => {
    if (m.type() === "error") errors.push(m.text());
  });
  page.on("pageerror", (e) => errors.push(e.message));
  return errors;
}

async function session(context: BrowserContext) {
  return (await context.cookies("https://baylee.app")).find((c) => c.name === "__Host-central");
}

async function menu(page: Page, phone: boolean) {
  if (phone) await page.getByRole("button", { name: "Menu" }).click();
}

test("register, keep the code, sign out, sign in again", async ({ page, context }, info) => {
  const phone = info.project.name === "phone";
  const name = `rr_${info.project.name}_${STAMP}`;
  // what other players see: the display name chosen here, and the tag Central draws (four hex digits)
  const HANDLE = new RegExp(`^${DISPLAY}#[0-9a-f]{4}$`);
  const errors = watch(page);
  expect(KEY).toMatch(/^CENTRAL-/);

  // a name an earlier version of the page kept is deleted on sight
  await page.goto("/?lang=en");
  await page.evaluate(() => window.localStorage.setItem("baylee-central-account", JSON.stringify({ name: "stale_name", expiresAt: 4_000_000_000 })));
  await page.goto("/register?lang=en");
  expect(await page.evaluate(() => window.localStorage.getItem("baylee-central-account"))).toBeNull();
  await page.getByLabel("Beta key").fill(KEY.toLowerCase());
  await page.getByLabel("Username").fill(name.toUpperCase());
  await page.getByLabel("Display name").fill(DISPLAY);
  await page.getByLabel("Password", { exact: true }).fill(PASSWORD);
  await page.getByLabel("Password again").fill(PASSWORD);
  await page.getByLabel("I am at least 16 years old.").check();
  await page.getByRole("checkbox", { name: /I accept the terms of use/ }).check();
  const started = Date.now();
  await page.getByRole("button", { name: "Create account" }).click();

  const step = page.getByRole("region", { name: "Your recovery code" });
  await expect(step).toBeVisible({ timeout: 60_000 });
  // the whole handle, with the tag Central drew
  await expect(step.getByText(HANDLE)).toBeVisible({ timeout: 60_000 });
  const code = (await step.getByLabel("Recovery code", { exact: true }).textContent()) ?? "";
  expect(code).toMatch(CODE);
  writeFileSync(join(OUT, `${name}.code`), code);
  process.stdout.write(`${name}: registered in ${Date.now() - started} ms\n`);
  await step.getByLabel("I have kept my recovery code in a safe place.").check();
  await step.getByRole("button", { name: "Continue" }).click();
  await expect(page).toHaveURL(/\/account$/, { timeout: 60_000 });
  await expect(page.locator(".account-name bdi")).toHaveText(HANDLE);
  await expect(page.getByText(`Sign-in name: ${name}`)).toBeVisible();

  // Central's cookie: HttpOnly, Secure, SameSite=Strict, Path=/, 64 hex digits,
  // and invisible to the page
  const cookie = await session(context);
  expect(cookie).toMatchObject({ httpOnly: true, secure: true, sameSite: "Strict", path: "/" });
  expect(cookie?.value).toMatch(/^[0-9a-f]{64}$/);
  expect(await page.evaluate(() => document.cookie)).not.toContain("central");
  // the browser keeps only the session's end; the name is Central's
  const kept = await page.evaluate(() => ({ ...window.localStorage }));
  expect(Object.keys(kept).filter((k) => k.startsWith("baylee-central"))).toEqual(["baylee-central-session"]);
  expect(Object.keys(JSON.parse(kept["baylee-central-session"] ?? "{}") as object)).toEqual(["expiresAt"]);

  // a reload: the header names the account once Central confirms the session
  await page.reload();
  await menu(page, phone);
  await expect(page.getByRole("link", { name: new RegExp(`^Your account, ${DISPLAY}#[0-9a-f]{4}$`) })).toBeVisible();
  await menu(page, phone);
  await page.getByRole("button", { name: "Sign out" }).click();
  await expect(page.getByText(/You are not signed in/)).toBeVisible();
  expect(await session(context)).toBeUndefined();

  // sign-in: a wrong password, then the right one, the name in capitals
  await page.goto("/signin?lang=en");
  await page.getByLabel("Username").fill(name.toUpperCase());
  await page.getByLabel("Password").fill("wrong passphrase entirely");
  await page.getByRole("button", { name: "Sign in" }).click();
  await expect(page.getByText("The username or the password is wrong.")).toBeVisible({ timeout: 60_000 });
  expect(await session(context)).toBeUndefined();
  await page.getByLabel("Password").fill(PASSWORD);
  await page.getByRole("button", { name: "Sign in" }).click();
  await expect(page).toHaveURL(/\/account$/, { timeout: 60_000 });
  await expect(page.locator(".account-name bdi")).toHaveText(HANDLE);
  expect(await session(context)).toBeDefined();

  if (!phone) {
    // the refusals, once: a taken name with a valid key, then a bad key
    await page.goto("/register?lang=en");
    await page.getByLabel("Beta key").fill(KEY);
    await page.getByLabel("Username").fill(name);
    await page.getByLabel("Display name").fill(DISPLAY);
    await page.getByLabel("Password", { exact: true }).fill(PASSWORD);
    await page.getByLabel("Password again").fill(PASSWORD);
    await page.getByLabel("I am at least 16 years old.").check();
    await page.getByRole("checkbox", { name: /I accept the terms of use/ }).check();
    await page.getByRole("button", { name: "Create account" }).click();
    await expect(page.getByText("That username is taken. Please choose another.")).toBeVisible();
    await page.getByLabel("Beta key").fill("CENTRAL-0000-0000-0000-0000");
    await page.getByLabel("Username").fill(`${name}_x`);
    await page.getByRole("button", { name: "Create account" }).click();
    await expect(page.getByText("That beta key is not valid, used up or expired.")).toBeVisible();
  }

  // the browser notes each refused request (401, 409); nothing else may complain
  expect(errors.filter((e) => !/the server responded with a status of (401|409)/.test(e))).toEqual([]);
});
