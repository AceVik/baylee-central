// The account area and the administration, signed in as an administrator
// against the area mock (`area.ts`), at a phone's and a desktop's width:
// every page draws its title, nothing scrolls sideways (a wide table scrolls
// in its own box), and nothing is refused or thrown.

import { expect, test } from "@playwright/test";

import { uid } from "./area.ts";
import { hexOf, rid } from "./reports.ts";
import { expectNoSideScroll, watchErrors } from "./mocks.ts";
import { signedIn } from "./signed-in.ts";

const hex = (id: number[]) => id.map((b) => b.toString(16).padStart(2, "0")).join("");

const PAGES: [string, string][] = [
  ["/account", "Your account"],
  ["/account/security", "Security and sessions"],
  ["/account/gateways", "Your gateways"],
  ["/admin", "Overview"],
  ["/admin/users", "Accounts"],
  [`/admin/users/${hex(uid(0x10, 1))}`, "An account"],
  ["/admin/gateways", "Gateways"],
  ["/admin/audit", "Audit log"],
  ["/admin/legacy", "Legacy accounts"],
  ["/admin/feedback", "Feedback"],
  [`/admin/feedback/${hexOf(rid(1))}`, "A report"],
];


test.describe("the signed-in area", () => {
  for (const [path, title] of PAGES) {
    test(`${path} draws and fits`, async ({ page, baseURL }) => {
      const errors = watchErrors(page);
      const central = await signedIn(page, new URL(baseURL ?? "").origin);
      await page.goto(`${path}?lang=en`);
      await expect(page.getByRole("heading", { level: 1 })).toHaveText(title);
      await page.waitForLoadState("networkidle");
      await expectNoSideScroll(page);
      expect(central.violations).toEqual([]);
      expect(errors).toEqual([]);
    });
  }
});
