// `npm run screens`: pictures of the main pages at both widths and in both
// colour schemes, for a review or a pull request, into screens/ (gitignored),
// each page in slices of one screen and a half. Skipped in an ordinary run.
// With `npm run content` done, the build carries the real card status and
// screenshots and these show them; card pictures stay placeholders (no
// request reaches Scryfall), and so does everything else outside.

import { existsSync } from "node:fs";

import { test } from "@playwright/test";

import { uid } from "./area.ts";
import { mockCentral } from "./central.ts";
import { mock } from "./mocks.ts";

const PAGES = ["/", "/download", "/cards", "/gateways", "/contact", "/impressum", "/privacy", "/register", "/signin", "/account"];
/** The account area and the administration, signed in as an administrator (`e2e/area.ts`). */
const hex = (id: number[]) => id.map((b) => b.toString(16).padStart(2, "0")).join("");
const AREA = ["/account", "/account/security", "/account/gateways", "/admin", "/admin/users", `/admin/users/${hex(uid(0x10, 1))}`, "/admin/gateways", "/admin/audit", "/admin/legacy"];
const OWN = existsSync("dist/data/sets.json") && existsSync("dist/media");

test.skip(process.env["SCREENS"] !== "1", "pictures only on request");

for (const scheme of ["dark", "light"] as const) {
  for (const [path, signedIn] of [...PAGES.map((p) => [p, false] as const), ...AREA.map((p) => [p, true] as const)]) {
    test(`${scheme} ${path}${signedIn ? " signed in" : ""}`, async ({ page, baseURL }, info) => {
      await mock(page, { gateways: 4, ownContent: OWN });
      if (signedIn) {
        await mockCentral(page, new URL(baseURL ?? "").origin, { named: { lantern: "Lantern" }, signedIn: "lantern", area: { admin: true } });
        // the page asks Central whose session it is when it remembers one (src/account/session.tsx)
        await page.addInitScript(() => localStorage.setItem("baylee-central-session", JSON.stringify({ expiresAt: Math.floor(Date.now() / 1000) + 1800 })));
      }
      await page.emulateMedia({ colorScheme: scheme, reducedMotion: "reduce" });
      await page.goto(`${path}?lang=en`);
      await page.waitForLoadState("networkidle");
      if (path === "/cards") await page.getByText(OWN ? "Arabian Nights" : "The Dark", { exact: true }).first().click();
      if (path === "/impressum") await page.getByRole("button", { name: "Show the details" }).click();
      await page.waitForTimeout(500);
      const name = path === "/" ? "home" : `${path.slice(1).replaceAll("/", "-").slice(0, 40)}${signedIn ? "-in" : ""}`;
      const view = page.viewportSize() ?? { width: 1366, height: 900 };
      const height = await page.evaluate(() => document.documentElement.scrollHeight);
      const slice = Math.round(view.height * 1.5);
      for (let y = 0, n = 1; y < height; y += slice, n++) {
        await page.screenshot({
          path: `screens/${info.project.name}-${scheme}-${name}-${n}.png`,
          fullPage: true,
          clip: { x: 0, y, width: view.width, height: Math.min(slice, height - y) },
        });
      }
    });
  }
}
