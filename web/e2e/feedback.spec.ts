// The administration's feedback section against the area mock (`area.ts`,
// `reports.ts`), at a phone's and a desktop's width: the list with its
// views and filters in the address, a report beside the list on a desktop
// and on its own page on a phone, a triage change with its revision, a note,
// and nothing refused, thrown or wider than the screen.

import { expect, test } from "@playwright/test";

import { signedIn } from "./signed-in.ts";
import { expectNoSideScroll, watchErrors } from "./mocks.ts";
import { hexOf, rid } from "./reports.ts";

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

  test("decides a report on its page: status, a note, the way back", async ({ page, baseURL }, info) => {
    const errors = watchErrors(page);
    const central = await signedIn(page, new URL(baseURL ?? "").origin);
    await page.goto("/admin/feedback?lang=en&status=new,triaged,in_progress");
    await page.getByRole("link", { name: /The client closed itself/ }).click();
    // beside the list on a desktop, its own page on a phone
    if (info.project.name === "desktop") await page.getByRole("link", { name: "Open its page" }).click();
    await expect(page).toHaveURL(/\/admin\/feedback\/[0-9a-f]{32}\?/);
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

test.describe("the feedback section's tools", () => {
  test("a selection, the trash with its undo, crash groups, the chart and the keys", async ({ page, baseURL }) => {
    const errors = watchErrors(page);
    const central = await signedIn(page, new URL(baseURL ?? "").origin);
    await page.goto("/admin/feedback?lang=en&status=new");
    await page.getByRole("checkbox", { name: /Select: The stack resolved twice/ }).check();
    await page.getByRole("checkbox", { name: /Select: The client closed itself/ }).check();
    const bar = page.getByRole("region", { name: "The selected reports" });
    await bar.getByLabel("Set the status").selectOption("triaged");
    await expect(page.getByText("2 reports set to Triaged.").first()).toBeVisible();
    await page.goto("/admin/feedback?lang=en&status=triaged");
    await page.getByRole("checkbox", { name: /Select: The stack resolved twice/ }).check();
    await page.getByRole("region", { name: "The selected reports" }).getByRole("button", { name: "Move to the trash" }).click();
    await expect(page.getByRole("link", { name: /The stack resolved twice/ })).toHaveCount(0);
    await page.getByRole("button", { name: "Undo" }).click();
    await expect(page.getByRole("link", { name: /The stack resolved twice/ })).toBeVisible();
    await page.goto("/admin/feedback?lang=en");
    await page.getByRole("button", { name: "Crash groups" }).click();
    await expect(page.getByText("index 7 out of range for slice of length 3")).toBeVisible();
    await page.getByRole("button", { name: "List" }).click();
    await page.getByText("The last 30 days").click();
    await expect(page.getByText(/Crashes: \d+/)).toBeVisible();
    await page.waitForLoadState("networkidle");
    await expectNoSideScroll(page);
    // the single keys act when no field has the keyboard
    await page.evaluate(() => (document.activeElement instanceof HTMLElement ? document.activeElement.blur() : undefined));
    await page.keyboard.press("?");
    await expect(page.getByRole("dialog", { name: "Keys" })).toBeVisible();
    await page.keyboard.press("Escape");
    expect(central.violations).toEqual([]);
    expect(errors).toEqual([]);
  });
});

test.describe("what a report carries", () => {
  test("the client's details under the policy, and the record as a download", async ({ page, baseURL }) => {
    const errors = watchErrors(page);
    const central = await signedIn(page, new URL(baseURL ?? "").origin);
    await page.goto(`/admin/feedback/${hexOf(rid(1))}?lang=en`);
    // a checked PNG through a blob: address, which the policy allows (and data: it does not)
    const shot = page.getByRole("img", { name: /The client's window when the report was written/ });
    await expect(shot).toBeVisible();
    await expect(shot).toHaveAttribute("src", /^blob:/);
    expect(await shot.evaluate((img: HTMLImageElement) => img.naturalWidth)).toBe(1);
    await expect(page.getByRole("link", { name: /^Lightning Bolt/ })).toHaveAttribute("href", "https://scryfall.com/card/77c6fa74-5543-42ac-9ead-0e890b188e99");
    await page.getByText("Everything it sent, as a tree").click();
    await page.getByText("screenshot:", { exact: true }).click();
    await expect(page.getByText(/characters of base64/)).toBeVisible();
    const download = page.waitForEvent("download");
    await page.getByRole("button", { name: "Download the record (.gz)" }).click();
    expect((await download).suggestedFilename()).toMatch(/^baylee-report-[0-9a-f]{8}-record\.jsonl\.gz$/);
    await page.waitForLoadState("networkidle");
    await expectNoSideScroll(page);
    expect(central.operations.filter((o) => o.startsWith("part:"))).toEqual(["part:client", "part:record"]);
    expect(central.violations).toEqual([]);
    expect(errors).toEqual([]);
  });
});
