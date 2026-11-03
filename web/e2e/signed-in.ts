// Signed in as an administrator against the area mock (`area.ts`), for the
// specs of the account area and the administration.

import type { Page } from "@playwright/test";

import { mockCentral, type CentralMock } from "./central.ts";
import { mock } from "./mocks.ts";

export async function signedIn(page: Page, origin: string, admin = true): Promise<CentralMock> {
  await mock(page, { gateways: 4 });
  const central = await mockCentral(page, origin, { named: { lantern: "Lantern" }, signedIn: "lantern", area: { admin } });
  // the page asks Central whose session it is when it remembers one (src/account/session.tsx)
  await page.addInitScript(() => localStorage.setItem("baylee-central-session", JSON.stringify({ expiresAt: Math.floor(Date.now() / 1000) + 1800 })));
  return central;
}
