// Central's per-address budget for Register (five an hour, counted in its
// memory): the sixth attempt from one address is refused with 429, and the
// page says so. Run on a freshly started Central, nothing else registering.

import { expect, test } from "@playwright/test";

test("the sixth registration attempt in an hour is refused, and the page says why", async ({ page }, info) => {
  test.skip(info.project.name !== "desktop", "one address, one budget");
  await page.goto("/register?lang=en");
  await page.getByLabel("Username").fill(`rr_budget_${Date.now() % 100_000}`);
  await page.getByLabel("Display name").fill("Real_Budget");
  await page.getByLabel("Password", { exact: true }).fill("violet lantern harbour quietly");
  await page.getByLabel("Password again").fill("violet lantern harbour quietly");
  await page.getByLabel("I am at least 16 years old.").check();
  await page.getByRole("checkbox", { name: /I accept the terms of use/ }).check();
  const statuses: number[] = [];
  page.on("response", (r) => {
    if (r.url().endsWith("/api/v1")) statuses.push(r.status());
  });
  for (let attempt = 1; attempt <= 6; attempt++) {
    await page.getByLabel("Beta key").fill(`CENTRAL-0000-0000-0000-000${attempt}`);
    await page.getByRole("button", { name: "Create account" }).click();
    const expected = attempt <= 5 ? "That beta key is not valid, used up or expired." : "Too many attempts from here. Please wait a while and try again.";
    await expect(page.getByText(expected)).toBeVisible();
    await expect.poll(() => statuses.length).toBe(attempt);
  }
  expect(statuses).toEqual([401, 401, 401, 401, 401, 429]);
});
