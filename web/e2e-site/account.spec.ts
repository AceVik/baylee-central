import { expect, type Route, test } from "@playwright/test";

// The sign-in's browser crypto, for real: the committed central-web-crypto
// module in its Worker (started through the Trusted Types policy, compiled
// under 'wasm-unsafe-eval'), against a stand-in for Central's browser
// surface. The stand-in cannot answer OPAQUE truthfully, so the Worker must
// refuse its challenge: that proves the module ran and the refusal reaches
// the page as "wrong sign-in", with nothing sent that should not be.

const ACCOUNT = [1, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 16];

async function central(route: Route, seen: unknown[]) {
  const request = route.request();
  const headers = request.headers();
  expect(headers["x-baylee-central"]).toBe("1");
  expect(headers.authorization).toBeUndefined();
  const body = JSON.parse(request.postData() ?? "{}");
  seen.push(body);
  const json = (value: unknown) => route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(value) });
  switch (body.operation) {
    case "discover":
      return json({ result: "discovery", discovery: { kind: "central", identity: { account: ACCOUNT, epoch: 1 } } });
    case "login_start":
      return json({ result: "login_challenge", attempt: Array(32).fill(7), message: Array.from({ length: 320 }, (_, i) => (i * 37) % 251) });
    default:
      return route.fulfill({ status: 401, body: "account request rejected\n" });
  }
}

test("a sign-in runs the real crypto in its Worker and refuses a false challenge", async ({ page }) => {
  const problems: string[] = [];
  page.on("pageerror", (error) => problems.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error" && !message.text().includes("status of 401")) problems.push(message.text());
  });
  const seen: Record<string, unknown>[] = [];
  await page.route("**/api/v1", (route) => central(route, seen));
  await page.goto("/en/signin");
  await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
  await page.getByLabel("Sign-in name").fill("Erika");
  await page.getByLabel("Password", { exact: true }).fill("correct horse battery staple");
  await page.getByRole("button", { name: "Sign in", exact: true }).click();
  await expect(page.getByRole("alert")).toHaveText("That sign-in name and password do not match. Check both and try again.", { timeout: 30_000 });
  expect(seen.map((b) => b.operation)).toEqual(["discover", "login_start"]);
  expect(seen[0]).toEqual({ operation: "discover", username: "erika" });
  const start = seen[1] as { identity: unknown; message: number[] };
  expect(start.identity).toEqual({ account: ACCOUNT, epoch: 1 });
  expect(start.message).toHaveLength(96);
  // Nothing of the password left the Worker: no request carried it.
  expect(JSON.stringify(seen)).not.toContain("battery");
  expect(problems).toEqual([]);
});

test("registration says early what Central would refuse", async ({ page }) => {
  await page.goto("/de/register");
  await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
  await page.getByRole("button", { name: "Account anlegen" }).click();
  await expect(page.getByRole("alert")).toHaveText("Gib deinen Beta-Key ein.");
  await page.getByLabel("Beta-Key").fill("CENTRAL-TEST");
  await page.getByLabel("Anmeldename").fill("erika");
  await page.getByLabel("Anzeigename").fill("Admin");
  await page.getByRole("button", { name: "Account anlegen" }).click();
  await expect(page.getByRole("alert")).toHaveText("Dieser Anzeigename ist reserviert. Wähle einen anderen.");
});

test("the account area says when nobody is signed in", async ({ page }) => {
  await page.goto("/en/account/security");
  await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
  await expect(page.getByText("You are not signed in.")).toBeVisible();
  await expect(page.getByRole("link", { name: "Sign in" }).last()).toHaveAttribute("href", "/en/signin");
});
