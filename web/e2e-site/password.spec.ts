import { expect, type Page, type Route, test } from "@playwright/test";

import { axe, narrow, targets } from "./check.ts";

// The password's change in the account area (docs/native-login.md §"Password
// change") in a real browser, against a stand-in for Central's browser
// surface. The crypto is real, and the stand-in cannot answer OPAQUE
// truthfully: the Worker refuses its KE2, which is the wrong current
// password's path. A change that goes through needs a Central that holds
// the account, and is e2e-real's.

const ACCOUNT = [1, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 16];
const PASSWORD = "violet lantern harbour";

type Body = { operation?: string; request?: { action?: string } } & Record<string, unknown>;

const session = (kind: string, current: boolean, gateways: string[]) => ({
  kind, current, signed_in_at: 1_794_000_000, expires_at: 4_102_444_800, gateways,
});

/** A signed-in account, until `state.gone`. */
function central(seen: Body[], state: { gone: boolean }) {
  return (route: Route) => {
    const request = route.request();
    expect(request.headers()["x-baylee-central"]).toBe("1");
    const body = JSON.parse(request.postData() ?? "{}") as Body;
    seen.push(body);
    const json = (value: unknown) => route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(value) });
    const rejected = () => route.fulfill({ status: 401, body: "account request rejected\n" });
    if (state.gone) return rejected();
    switch (body.operation) {
      case "account_summary":
        return json({
          result: "account_summary", identity: { account: ACCOUNT, epoch: 1 }, username: "erika", display_name: "Erika", tag: 0x1a2b,
          source: "registered", recovery: true, admin: false, signed_in_at: 1_794_000_000, expires_at: 4_102_444_800,
        });
      case "owner":
        if (body.request?.action !== "sessions") return rejected();
        return json({
          result: "owner",
          answer: { answer: "sessions", sessions: [session("browser", true, []), session("native", false, ["https://one.example"])] },
        });
      case "login_start":
        return json({ result: "login_challenge", attempt: Array(32).fill(7), message: Array.from({ length: 320 }, (_, i) => (i * 37) % 251) });
      default:
        return rejected();
    }
  };
}

async function open(page: Page, seen: Body[], state = { gone: false }) {
  await page.addInitScript(() => {
    // once: a later reload must see what the page itself did to it
    if (!sessionStorage.getItem("seeded")) {
      sessionStorage.setItem("seeded", "1");
      localStorage.setItem("baylee-central-session", JSON.stringify({ expiresAt: 4_102_444_800 }));
    }
  });
  await page.route("**/api/v1", central(seen, state));
  await page.goto("/en/account/security");
  await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
  await expect(page.locator("#password-current")).toBeVisible();
  return state;
}

const card = (page: Page) => page.locator("section[aria-labelledby=password-title]");
const submit = (page: Page) => page.getByRole("button", { name: "Change password" });

test("the form is labelled, each password has its eye, and nothing is sent before it is complete", async ({ page }) => {
  const seen: Body[] = [];
  await open(page, seen);
  await expect(card(page).getByRole("heading", { name: "Password" })).toBeVisible();
  await expect(card(page)).toContainText(
    "Your encrypted data and your recovery code stay as they are. Changing the password signs you out everywhere except here.",
  );
  await expect(page.getByText("will follow")).toHaveCount(0);
  const current = page.getByLabel("Current password");
  const fresh = page.getByLabel("New password", { exact: true });
  const repeat = page.getByLabel("Repeat new password");
  for (const field of [current, fresh, repeat]) {
    await expect(field).toBeEditable();
    await expect(field).toHaveAttribute("type", "password");
    await expect(field).not.toHaveAttribute("name", /.*/);
  }
  await expect(current).toHaveAttribute("autocomplete", "current-password");
  await expect(fresh).toHaveAttribute("autocomplete", "new-password");
  // a password manager learns whose password it is, and no person sees it
  const whose = card(page).locator("input[autocomplete=username]");
  await expect(whose).toHaveValue("erika");
  await expect(whose).toBeHidden();
  await expect(whose).not.toHaveAttribute("name", /.*/);
  const eyes = card(page).getByRole("button", { name: "Show the password" });
  await expect(eyes).toHaveCount(3);
  await fresh.fill(PASSWORD);
  await eyes.nth(1).click();
  await expect(fresh).toHaveAttribute("type", "text");
  await expect(current).toHaveAttribute("type", "password");
  expect(await axe(page)).toEqual([]);
  expect(await targets(page)).toEqual([]);
  // the three fields with their glyphs and eyes hold on the narrowest phone
  expect(await narrow(page)).toBe(0);

  const asked = seen.length;
  const alert = card(page).getByRole("alert");
  await fresh.fill("");
  await submit(page).click();
  await expect(alert).toHaveText("Enter your current password and a new one.");
  await current.fill("the old words");
  await fresh.fill("short");
  await repeat.fill("short");
  await submit(page).click();
  await expect(alert).toHaveText("The password needs at least 8 characters.");
  await fresh.fill(PASSWORD);
  await repeat.fill("another harbour");
  await submit(page).click();
  await expect(alert).toHaveText("The two passwords are not the same.");
  expect(seen.slice(asked)).toEqual([]);
});

test("a wrong current password is refused in the Worker, and nothing more is sent", async ({ page }) => {
  const problems: string[] = [];
  page.on("pageerror", (error) => problems.push(error.message));
  const seen: Body[] = [];
  await open(page, seen);
  const asked = seen.length;
  await page.getByLabel("Current password").fill("the wrong old words");
  await page.getByLabel("New password", { exact: true }).fill(PASSWORD);
  await page.getByLabel("Repeat new password").fill(PASSWORD);
  await submit(page).click();
  await expect(card(page).getByRole("alert")).toHaveText("The current password is not right. Check it and try again.", { timeout: 30_000 });
  // the stand-in's KE2 does not verify: no proof, no change was asked for
  expect(seen.slice(asked).map((body) => body.operation)).toEqual(["account_summary", "login_start"]);
  const start = seen[asked + 1] as { identity: unknown; message: number[] };
  expect(start.identity).toEqual({ account: ACCOUNT, epoch: 1 });
  expect(start.message).toHaveLength(96);
  expect(JSON.stringify(seen)).not.toContain("lantern");
  expect(JSON.stringify(seen)).not.toContain("old words");
  // the form is drawn anew, empty, with the keyboard at its start
  await expect(page.getByLabel("Current password")).toBeFocused();
  for (const id of ["password-current", "password-new", "password-repeat"]) await expect(page.locator(`#${id}`)).toHaveValue("");
  expect(await axe(page)).toEqual([]);
  expect(problems).toEqual([]);
});

test("a session that is gone is said, not papered over", async ({ page }) => {
  const seen: Body[] = [];
  const state = await open(page, seen);
  state.gone = true;
  await page.getByLabel("Current password").fill("the old words");
  await page.getByLabel("New password", { exact: true }).fill(PASSWORD);
  await page.getByLabel("Repeat new password").fill(PASSWORD);
  await submit(page).click();
  await expect(page.getByText("You are not signed in.")).toBeVisible({ timeout: 30_000 });
  await expect(page.locator("#password-current")).toHaveCount(0);
  expect(await page.evaluate(() => localStorage.getItem("baylee-central-session"))).toBeNull();
  expect(JSON.stringify(seen)).not.toContain("lantern");
});
