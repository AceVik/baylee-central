// The administration's feedback section against the area mock (`area.ts`,
// `reports.ts`), at a phone's and a desktop's width: the list with its
// views and filters in the address, a report beside the list on a desktop
// and on its own page on a phone, a triage change with its revision, a note,
// and nothing refused, thrown or wider than the screen.

import { expect, test } from "@playwright/test";

import { signedIn } from "./signed-in.ts";
import { expectNoSideScroll, watchErrors } from "./mocks.ts";

test.describe("the feedback section", () => {
  test("lists the reports, filters them in the address and opens one", async ({ page, baseURL }, info) => {
    const errors = watchErrors(page);
    const central = await signedIn(page, new URL(baseURL ?? "").origin);
    await page.goto("/admin/feedback?lang=en");
    await expect(page.getByRole("heading", { level: 1 })).toHaveText("Feedback");
    const report = page.getByRole("link", { name: /The stack resolved twice/ });
    await expect(report).toBeVisible();
    // the navigation counts the unread reports
    await expect(page.getByRole("link", { name: /^Feedback \d+ unread/ })).toBeVisible();
    await page.getByRole("button", { name: /^Inbox/ }).click();
    await expect(page).toHaveURL(/\?status=new$/);
    await expect(page.getByRole("link", { name: /Lovely table colours/ })).toHaveCount(0);
    await page.waitForLoadState("networkidle");
    await expectNoSideScroll(page);
    await report.click();
    if (info.project.name === "desktop") {
      const drawer = page.getByRole("complementary", { name: "The report beside the list" });
      await expect(drawer.getByRole("heading", { level: 2 })).toContainText("The stack resolved twice");
      await drawer.getByRole("button", { name: "Triaged" }).click();
      await expect(page.getByText("Status set to Triaged.").first()).toBeVisible();
      await expect(drawer.getByRole("button", { name: "Triaged" })).toHaveAttribute("aria-pressed", "true");
    } else {
      await expect(page).toHaveURL(/\/admin\/feedback\/[0-9a-f]{32}\?status=new$/);
      await expect(page.getByRole("heading", { name: /The stack resolved twice/ })).toBeVisible();
    }
    await page.waitForLoadState("networkidle");
    await expectNoSideScroll(page);
    expect(central.violations).toEqual([]);
    expect(errors).toEqual([]);
  });

  test("decides a report on its page: status, a note, the way back", async ({ page, baseURL }) => {
    const errors = watchErrors(page);
    const central = await signedIn(page, new URL(baseURL ?? "").origin);
    await page.goto("/admin/feedback?lang=en&status=new,triaged,in_progress");
    await page.getByRole("link", { name: /The client closed itself/ }).click({ modifiers: [] });
    if (!/\/admin\/feedback\/[0-9a-f]{32}/.test(page.url())) await page.getByRole("link", { name: "Open its page" }).click();
    await expect(page.getByRole("heading", { level: 1 })).toHaveText("A report");
    await expect(page.getByText(/^\d+ of \d+$/)).toBeVisible();
    await page.getByRole("button", { name: "High" }).click();
    await expect(page.getByText("Priority set to High.").first()).toBeVisible();
    await page.getByLabel("A note for the administrators").fill("Synthetic note from the e2e test.");
    await page.getByRole("button", { name: "Save note" }).click();
    await expect(page.getByText("Synthetic note from the e2e test.")).toBeVisible();
    await page.waitForLoadState("networkidle");
    await expectNoSideScroll(page);
    await page.getByRole("link", { name: "Back to the list" }).click();
    await expect(page).toHaveURL(/\/admin\/feedback\?status=new%2Ctriaged%2Cin_progress$/);
    expect(central.violations).toEqual([]);
    expect(errors).toEqual([]);
  });
});
