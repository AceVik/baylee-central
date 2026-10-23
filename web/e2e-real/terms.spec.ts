// Central's terms in force (a test build whose compiled-in texts carry a
// version instead of `draft`): signing in shows the texts Central serves, in
// the page's language, and the page accepts them only because they hash to
// the digest Central names (the page's digest against central-api's).
// Declining signs out. REAL_OUT holds the accounts account.spec.ts made.

import { readdirSync } from "node:fs";

import { expect, test, type Page } from "@playwright/test";

const OUT = process.env["REAL_OUT"] ?? "";
const PASSWORD = "violet lantern harbour quietly";

function account(project: string): string {
  const file = readdirSync(OUT).find((f) => f.startsWith(`rr_${project}_`) && f.endsWith(".code"));
  if (file === undefined) throw new Error(`no ${project} account in ${OUT}`);
  return file.slice(0, -".code".length);
}

async function signIn(page: Page, name: string, lang: "en" | "de") {
  await page.goto(`/signin?lang=${lang}`);
  await page.getByLabel(lang === "en" ? "Username" : "Benutzername").fill(name);
  await page.getByLabel(lang === "en" ? "Password" : "Passwort").fill(PASSWORD);
  await page.getByRole("button", { name: lang === "en" ? "Sign in" : "Anmelden" }).click();
}

test("the terms in force are shown, accepted once, and not asked again", async ({ page }, info) => {
  test.skip(info.project.name !== "desktop", "one account accepts");
  const errors: string[] = [];
  page.on("console", (m) => m.type() === "error" && errors.push(m.text()));
  await signIn(page, account("desktop"), "en");
  await expect(page.getByRole("heading", { name: "Accept the terms of use" })).toBeVisible({ timeout: 60_000 });
  await expect(page.getByText(/Version realrun-2026-10-21 of Baylee Central's terms of use/)).toBeVisible();
  await expect(page.getByRole("heading", { name: "Terms of use for Baylee accounts" })).toBeVisible();
  await page.getByRole("button", { name: "I accept" }).click();
  await expect(page).toHaveURL(/\/account$/, { timeout: 30_000 });
  await page.reload();
  // the handle account.spec.ts registered it with
  await expect(page.locator(".account-name bdi")).toHaveText(/^Real_Runner#[0-9a-f]{4}$/);
  await expect(page.getByRole("heading", { name: "Accept the terms of use" })).toHaveCount(0);
  expect(errors).toEqual([]);
});

test("in German the German text is shown, and declining signs out", async ({ page, context }, info) => {
  test.skip(info.project.name !== "phone", "one account declines");
  const errors: string[] = [];
  page.on("console", (m) => m.type() === "error" && errors.push(m.text()));
  await signIn(page, account("phone"), "de");
  await expect(page.getByRole("heading", { name: "Nutzungsbedingungen annehmen" })).toBeVisible({ timeout: 60_000 });
  await expect(page.getByRole("heading", { name: "Nutzungsbedingungen für Baylee-Konten" })).toBeVisible();
  await page.getByRole("button", { name: "Nicht jetzt (abmelden)" }).click();
  await expect(page.getByText(/Du bist abgemeldet\./)).toBeVisible();
  await expect(page.getByRole("button", { name: "Ich nehme sie an" })).toHaveCount(0);
  await expect.poll(async () => (await context.cookies("https://baylee.app")).some((c) => c.name === "__Host-central")).toBe(false);
  await page.goto("/account?lang=de");
  await expect(page.getByText(/Du bist nicht angemeldet/)).toBeVisible();
  expect(errors).toEqual([]);
});
