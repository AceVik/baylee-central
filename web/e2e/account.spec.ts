// The account pages against the build, its CSP and a strict stand-in for
// Central's browser surface (e2e/central.ts): registration to the recovery
// code and the terms, the refusals, sign-in and sign-out, choosing a display
// name and the handle in the header, at a phone's and a desktop's width. Two tests use the real WebAssembly in its Worker, under
// the production CSP: it starts both exchanges with the suite's message
// sizes, and refuses what a forged Central answers. A third proves the CSP's
// `'wasm-unsafe-eval'` is what lets it run.

import { readFile } from "node:fs/promises";

import { expect, test, type Page } from "@playwright/test";

import { csp } from "../scripts/csp.ts";
import { CODE, KEY, NEW_CODE, PASSWORD, SEALED_DOCUMENT, mockCentral, stubCrypto } from "./central.ts";
import { expectNoSideScroll, mock, watchErrors } from "./mocks.ts";

const TERMS = {
  version: "2026-10-21",
  texts: [
    { lang: "de", updated: "2026-10-21", markdown: "# Bedingungen\n\nSynthetische Bedingungen für den Test." },
    { lang: "en", updated: "2026-10-21", markdown: "# Terms\n\nSynthetic terms for the test." },
  ],
};

async function openMenu(page: Page, phone: boolean) {
  if (phone) await page.getByRole("button", { name: "Menu" }).click();
}

async function fillRegistration(page: Page, name = "Synthetic_Name", key = KEY) {
  await page.getByLabel("Beta key").fill(key);
  await page.getByLabel("Username").fill(name);
  await page.getByLabel("Display name").fill("Synthetic_Name");
  await page.getByLabel("Password", { exact: true }).fill(PASSWORD);
  await page.getByLabel("Password again").fill(PASSWORD);
  await page.getByLabel("I am at least 16 years old.").check();
  await page.getByRole("checkbox", { name: /I accept the terms of use/ }).check();
}

test.describe("accounts", () => {
  test("registration shows the recovery code once, then the terms, the account and signing out", async ({ page, context, baseURL }, info) => {
    const phone = info.project.name === "phone";
    await context.grantPermissions(["clipboard-read", "clipboard-write"]);
    const errors = watchErrors(page);
    const m = await mock(page);
    const central = await mockCentral(page, new URL(baseURL ?? "").origin, { terms: TERMS });
    await stubCrypto(page);
    await page.goto("/?lang=en");
    await openMenu(page, phone);
    await page.getByRole("navigation", { name: "Sections" }).getByRole("link", { name: "Sign in" }).click();
    await page.getByRole("link", { name: "Create one" }).click();
    await expect(page.getByRole("heading", { level: 1, name: "Create your account" })).toBeVisible();
    await expect(page.getByText(/3 to 24 characters/)).toBeVisible();
    await expectNoSideScroll(page);

    await fillRegistration(page);
    await expect(page.getByText("Strong", { exact: true })).toBeVisible();
    await page.getByRole("button", { name: "Create account" }).click();

    const step = page.getByRole("region", { name: "Your recovery code" });
    await expect(step.getByLabel("Recovery code", { exact: true })).toHaveText(CODE);
    await expect(step.getByRole("heading", { name: "Your recovery code" })).toBeFocused();
    // the whole handle, once the sign-in started at once has read the tag drawn
    await expect(step.getByText(/Other players see you as Synthetic_Name#3fa2\./)).toBeVisible();
    await expectNoSideScroll(page);
    await step.getByRole("button", { name: "Copy" }).click();
    await expect(step.getByText("Copied.")).toBeVisible();
    expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(CODE);
    const download = page.waitForEvent("download");
    await step.getByRole("button", { name: "Download as a text file" }).click();
    const file = await download;
    expect(file.suggestedFilename()).toBe("baylee-recovery-code.txt");
    const text = await readFile(await file.path(), "utf8");
    expect(text).toContain(CODE);
    expect(text).toContain("synthetic_name");

    const go = step.getByRole("button", { name: "Continue" });
    await expect(go).toBeDisabled();
    await step.getByLabel("I have kept my recovery code in a safe place.").check();
    await go.click();

    await expect(page.getByRole("heading", { name: "Accept the terms of use" })).toBeVisible();
    await expect(page.getByText("Synthetic terms for the test.")).toBeVisible();
    await page.getByRole("button", { name: "I accept" }).click();
    await expect(page).toHaveURL(/\/account$/);
    await expect(page.locator(".account-name bdi")).toHaveText("Synthetic_Name#3fa2");
    await expect(page.getByText("Sign-in name: synthetic_name")).toBeVisible();
    expect(central.accepted?.version).toBe("2026-10-21");
    await expectNoSideScroll(page);

    // the header names the account by its handle, also after a reload (Central confirms the session)
    await page.reload();
    await openMenu(page, phone);
    await expect(page.getByRole("link", { name: "Your account, Synthetic_Name#3fa2" })).toBeVisible();
    await openMenu(page, phone);
    await page.getByRole("button", { name: "Sign out" }).click();
    await expect(page.getByText(/You are not signed in/)).toBeVisible();
    await openMenu(page, phone);
    await expect(page.getByRole("navigation", { name: "Sections" }).getByRole("link", { name: "Sign in" })).toBeVisible();

    expect(central.operations.slice(0, 10)).toEqual([
      "register",
      "enrollment_start",
      "enrollment_finish",
      "discover",
      "login_start",
      "login_finish",
      "account_summary",
      "terms_status",
      "terms",
      "accept_terms",
    ]);
    // on the account page, its terms step and its summary each ask once, in either order
    expect(central.operations.slice(10, 12).sort()).toEqual(["account_summary", "terms_status"]);
    // after the reload, the provider, the terms step and the summary each ask once, in any order
    expect(central.operations.slice(12, 15).sort()).toEqual(["account_summary", "account_summary", "terms_status"]);
    expect(central.operations.slice(15)).toEqual(["logout"]);
    expect(central.bodies.get("register")).toEqual({ operation: "register", username: "synthetic_name", beta_key: KEY, display_name: "Synthetic_Name" });
    expect(central.bodies.get("enrollment_finish")?.["document"]).toHaveLength(SEALED_DOCUMENT);
    expect(central.violations).toEqual([]);
    expect(m.stray).toEqual([]);
    expect(errors).toEqual([]);
  });

  test("registration names a refused key and a taken name, and the form what is missing", async ({ page, baseURL }) => {
    const errors = watchErrors(page);
    const m = await mock(page);
    const central = await mockCentral(page, new URL(baseURL ?? "").origin, { accounts: ["taken_name"] });
    await stubCrypto(page);
    await page.goto("/register?lang=en");
    await page.getByRole("button", { name: "Create account" }).click();
    await expect(page.getByText("Please enter your beta key.")).toBeVisible();
    await expect(page.getByLabel("Beta key")).toBeFocused();
    await fillRegistration(page, "synthetic_name", "CENTRAL-NOPE-0000-0000-0000");
    await page.getByRole("button", { name: "Create account" }).click();
    await expect(page.getByText("That beta key is not valid, used up or expired.")).toBeVisible();
    await page.getByLabel("Beta key").fill(KEY);
    await page.getByLabel("Username").fill("taken_name");
    await page.getByRole("button", { name: "Create account" }).click();
    await expect(page.getByText("That username is taken. Please choose another.")).toBeVisible();
    expect(central.operations).toEqual(["register", "register"]);
    expect(central.violations).toEqual([]);
    expect(m.stray).toEqual([]);
    // the browser notes each refused request; nothing else may complain
    expect(errors.filter((e) => !/the server responded with a status of (401|409)/.test(e))).toEqual([]);
  });

  test("sign-in refuses a wrong password, then signs in and keeps the name", async ({ page, baseURL }) => {
    const errors = watchErrors(page);
    const m = await mock(page);
    const central = await mockCentral(page, new URL(baseURL ?? "").origin, { accounts: ["synthetic_name"] });
    await stubCrypto(page);
    await page.goto("/signin?lang=de");
    await page.getByLabel("Benutzername").fill("Synthetic_Name");
    await page.getByLabel("Passwort").fill("wrong passphrase");
    await page.getByRole("button", { name: "Anmelden" }).click();
    await expect(page.getByText("Benutzername oder Passwort stimmt nicht.")).toBeVisible();
    // a wrong password is found before a proof is sent
    expect(central.operations).toEqual(["discover", "login_start"]);
    await page.getByLabel("Passwort").fill(PASSWORD);
    await page.getByRole("button", { name: "Anmelden" }).click();
    await expect(page).toHaveURL(/\/account$/);
    // registered before display names: no handle yet, so the page asks for a name first
    await expect(page.locator(".account-name")).toHaveText("Angemeldet");
    await expect(page.getByText("Anmeldename: synthetic_name")).toBeVisible();
    await expect(page.getByRole("region", { name: "Wähl deinen Anzeigenamen" })).toBeVisible();
    await expectNoSideScroll(page);
    expect(central.violations).toEqual([]);
    expect(m.stray).toEqual([]);
    expect(errors).toEqual([]);
  });

  test("an account moved here without a recovery code makes one after a sign-in or on the account page", async ({ page, baseURL }) => {
    const errors = watchErrors(page);
    const m = await mock(page);
    const central = await mockCentral(page, new URL(baseURL ?? "").origin, { moved: ["moved_player"] });
    await stubCrypto(page);
    await page.goto("/signin?lang=en");
    await page.getByLabel("Username").fill("moved_player");
    await page.getByLabel("Password").fill(PASSWORD);
    await page.getByRole("button", { name: "Sign in" }).click();

    // right after the sign-in: the code, put off this time
    const step = page.getByRole("region", { name: "Your recovery code" });
    await expect(step.getByLabel("Recovery code", { exact: true })).toHaveText(NEW_CODE);
    await expect(step.getByText(/which has none yet/)).toBeVisible();
    await expectNoSideScroll(page);
    await step.getByRole("button", { name: "Not now" }).click();
    await expect(page).toHaveURL(/\/account$/);
    expect(central.operations).not.toContain("set_recovery");
    await expect(page.getByText(/Your account has no recovery code yet\. It moved here from the official gateway/)).toBeVisible();
    // moved before its name and tag were carried: they come with the next sign-in there, nothing to choose here
    await expect(page.getByText(/Your display name and tag come with you from the official gateway/)).toBeVisible();
    await expect(page.getByRole("region", { name: "Choose your display name" })).toHaveCount(0);

    // on the account page: the password again, then the code, stored once kept
    await page.getByRole("button", { name: "Make a recovery code" }).click();
    await expect(page.getByText("To make the code, sign in again")).toBeVisible();
    await page.getByLabel("Password").fill(PASSWORD);
    await page.getByRole("button", { name: "Sign in" }).click();
    await expect(step.getByLabel("Recovery code", { exact: true })).toHaveText(NEW_CODE);
    await expectNoSideScroll(page);
    await step.getByLabel("I have kept my recovery code in a safe place.").check();
    await step.getByRole("button", { name: "Continue" }).click();
    await expect(page.getByText(/^You have a recovery code\./)).toBeVisible();
    await expect(page.getByText("This account moved here from the official gateway.")).toBeVisible();
    expect(central.operations.filter((op) => op === "set_recovery")).toHaveLength(1);
    expect(central.bodies.get("set_recovery")).toEqual({ operation: "set_recovery", recovery: Array.from({ length: 73 }, () => 11) });
    expect(central.accounts.get("moved_player")?.recovery).toHaveLength(73);
    expect(central.violations).toEqual([]);
    expect(m.stray).toEqual([]);
    expect(errors).toEqual([]);
  });

  test("an account without a display name chooses one, and the header names it by its handle", async ({ page, baseURL }, info) => {
    const phone = info.project.name === "phone";
    const errors = watchErrors(page);
    const m = await mock(page);
    const central = await mockCentral(page, new URL(baseURL ?? "").origin, { accounts: ["synthetic_name"] });
    await stubCrypto(page);
    await page.goto("/signin?lang=en");
    await page.getByLabel("Username").fill("Synthetic_Name");
    await page.getByLabel("Password").fill(PASSWORD);
    await page.getByRole("button", { name: "Sign in" }).click();
    await expect(page).toHaveURL(/\/account$/);
    const form = page.getByRole("region", { name: "Choose your display name" });
    await expect(form.getByText(/beside your tag, #3fa2/)).toBeVisible();
    await expectNoSideScroll(page);
    await form.getByLabel("Display name").fill("Official");
    await form.getByRole("button", { name: "Save the name" }).click();
    await expect(form.getByText("That display name is kept for Baylee itself. Please choose another.")).toBeVisible();
    expect(central.operations).not.toContain("set_display_name");
    await form.getByLabel("Display name").fill("Chosen_Name");
    await form.getByRole("button", { name: "Save the name" }).click();
    await expect(page.locator(".account-name bdi")).toHaveText("Chosen_Name#3fa2");
    await expect(form).toHaveCount(0);
    expect(central.bodies.get("set_display_name")).toEqual({ operation: "set_display_name", display_name: "Chosen_Name" });
    await openMenu(page, phone);
    await expect(page.getByRole("link", { name: "Your account, Chosen_Name#3fa2" })).toBeVisible();
    await page.reload();
    await openMenu(page, phone);
    await expect(page.getByRole("link", { name: "Your account, Chosen_Name#3fa2" })).toBeVisible();
    expect(central.operations.filter((op) => op === "session_status")).toEqual([]);
    expect(central.violations).toEqual([]);
    expect(m.stray).toEqual([]);
    expect(errors).toEqual([]);
  });

  test("the real browser crypto starts both exchanges in its Worker and refuses forged answers", async ({ page, baseURL }) => {
    const errors = watchErrors(page);
    const m = await mock(page);
    const central = await mockCentral(page, new URL(baseURL ?? "").origin, { accounts: ["synthetic_name"], refuseCrypto: true });
    await page.goto("/register?lang=en");
    await fillRegistration(page, "new_name");
    await page.getByRole("button", { name: "Create account" }).click();
    // the module refuses a registration response it cannot use; nothing is enrolled
    await expect(page.getByText("Baylee Central's answer did not check out, so nothing was kept.", { exact: false })).toBeVisible({ timeout: 20_000 });
    const request = central.bodies.get("enrollment_start")?.["message"] as number[];
    expect(request).toHaveLength(32);
    expect(central.operations).toEqual(["register", "enrollment_start"]);

    await page.goto("/signin?lang=en");
    await page.getByLabel("Username").fill("synthetic_name");
    await page.getByLabel("Password").fill(PASSWORD);
    await page.getByRole("button", { name: "Sign in" }).click();
    await expect(page.getByText("The username or the password is wrong.")).toBeVisible({ timeout: 20_000 });
    expect(central.bodies.get("login_start")?.["message"]).toHaveLength(96);
    expect(central.operations.slice(2)).toEqual(["discover", "login_start"]);
    expect(central.violations).toEqual([]);
    expect(m.stray).toEqual([]);
    expect(errors).toEqual([]);
  });

  test("without 'wasm-unsafe-eval' the browser crypto cannot run, and the page says so", async ({ page, baseURL }) => {
    const errors = watchErrors(page);
    await mock(page);
    const central = await mockCentral(page, new URL(baseURL ?? "").origin);
    const strict = csp().replace(" 'wasm-unsafe-eval'", "");
    expect(strict).not.toContain("wasm-unsafe-eval");
    await page.route("**/assets/crypto-worker-*.js", async (route) => {
      const response = await route.fetch();
      await route.fulfill({ response, headers: { ...response.headers(), "content-security-policy": strict } });
    });
    await page.goto("/register?lang=en");
    await fillRegistration(page);
    await page.getByRole("button", { name: "Create account" }).click();
    await expect(page.getByText(/could not run the account's encryption/)).toBeVisible({ timeout: 20_000 });
    expect(central.operations).toEqual(["register"]);
    // the only complaints are the CSP's refusal of the module
    expect(errors.filter((e) => !/Content Security Policy|WebAssembly|wasm/i.test(e))).toEqual([]);
  });
});
