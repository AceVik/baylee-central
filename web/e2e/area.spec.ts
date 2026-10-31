// The account area and the administration, signed in as an administrator
// against the area mock (`area.ts`), at a phone's and a desktop's width:
// every page draws its title, nothing scrolls sideways (a wide table scrolls
// in its own box), and nothing is refused or thrown.

import { expect, test, type Page } from "@playwright/test";

import { uid } from "./area.ts";
import { mockCentral, type CentralMock } from "./central.ts";
import { expectNoSideScroll, mock, watchErrors } from "./mocks.ts";

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
];

export async function signedIn(page: Page, origin: string, admin = true): Promise<CentralMock> {
  await mock(page, { gateways: 4 });
  const central = await mockCentral(page, origin, { named: { lantern: "Lantern" }, signedIn: "lantern", area: { admin } });
  // the page asks Central whose session it is when it remembers one (src/account/session.tsx)
  await page.addInitScript(() => localStorage.setItem("baylee-central-session", JSON.stringify({ expiresAt: Math.floor(Date.now() / 1000) + 1800 })));
  return central;
}

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
