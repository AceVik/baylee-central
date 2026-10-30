// After area.spec.ts and a QA gateway that took the token in its console:
// the public directory lists it online with its deep link, the owner sees the
// token used, and the pages are photographed in both schemes and at a
// phone's width. REAL_OUT as for area.spec.ts.

import { readFileSync } from "node:fs";
import { join } from "node:path";

import { expect, test, type Page } from "@playwright/test";

// stills of whole pages: no section waits to come into view
test.use({ contextOptions: { reducedMotion: "reduce" } });

const OUT = process.env["REAL_OUT"] ?? "";
const PASSWORD = "violet lantern harbour quietly";
const QA_ORIGIN = readFileSync(join(OUT, "origin"), "utf8").trim();

async function shoot(page: Page, file: string) {
  // from the top: a sticky header photographed mid-scroll covers the page's title
  await page.evaluate(() => window.scrollTo(0, 0));
  await page.waitForTimeout(400);
  await page.screenshot({ path: join(OUT, `${file}.png`), fullPage: true });
}

test("the directory lists the linked gateway with its address to copy", async ({ page }, info) => {
  const suffix = info.project.name;
  for (const theme of ["dark", "light"] as const) {
    await page.goto(`/gateways?lang=de&theme=${theme}`);
    const card = page.locator(".gateways > li").filter({ hasText: "QA table" });
    await expect(card).toBeVisible({ timeout: 30_000 });
    await expect(card.getByText("online")).toBeVisible();
    // gateway links stay off until a client release reads them: the address is copied
    await expect(card.getByText(QA_ORIGIN)).toBeVisible();
    await expect(card.getByRole("button", { name: "Adresse kopieren" })).toBeVisible();
    await expect(card.getByRole("link", { name: "In Baylee öffnen" })).toHaveCount(0);
    await shoot(page, `10-gateways-${theme}-${suffix}`);
  }
  await page.goto("/?lang=de&theme=dark");
  await shoot(page, `11-home-dark-${suffix}`);
});

test("the owner sees the token used and the client's sessions", async ({ page }, info) => {
  const name = readFileSync(join(OUT, "owner"), "utf8").trim();
  await page.goto("/signin?lang=de");
  await page.getByLabel("Benutzername").fill(name);
  await page.getByLabel("Passwort").fill(PASSWORD);
  await page.getByRole("button", { name: "Anmelden" }).click();
  await expect(page).toHaveURL(/\/account$/, { timeout: 60_000 });
  await page.goto("/account/gateways?lang=de");
  await expect(page.getByText("Freigegeben").first()).toBeVisible();
  const row = page.getByRole("row", { name: /QA machine/ });
  await expect(row.getByText("Gültig")).toBeVisible();
  await expect(row.locator("td.num").nth(1)).not.toHaveText("—");
  await shoot(page, `12-account-gateways-de-${info.project.name}`);
  await page.goto("/admin/gateways?lang=de");
  await page.getByRole("button", { name: "Freigegeben" }).click();
  await expect(page.getByRole("heading", { name: "QA table" })).toBeVisible();
  await shoot(page, `13-admin-gateways-approved-de-${info.project.name}`);
});
