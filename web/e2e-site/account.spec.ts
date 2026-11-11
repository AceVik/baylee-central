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
  // The form that held the focus is gone: the way back takes it, and the
  // form's first field after that.
  await expect(page.getByRole("button", { name: "Try again" })).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.getByLabel("Sign-in name")).toBeFocused();
  expect(problems).toEqual([]);
});

test("registration says early what Central would refuse", async ({ page }) => {
  await page.goto("/de/register");
  await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
  // the open beta asks for no key
  await expect(page.getByLabel("Beta-Key")).toHaveCount(0);
  await page.getByRole("button", { name: "Account anlegen" }).click();
  await expect(page.getByRole("alert")).toHaveText("Der Anmeldename braucht 3 bis 24 Kleinbuchstaben, Ziffern, Punkte, Binde- oder Unterstriche.");
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

test("registration refuses a robot's haste and its hidden field, and says when it is paused", async ({ page }) => {
  const asked: string[] = [];
  await page.route("**/api/v1", (route) => {
    asked.push(String(JSON.parse(route.request().postData() ?? "{}").operation));
    return route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify({ result: "registration_closed" }) });
  });
  await page.goto("/en/register");
  await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
  await page.getByLabel("Sign-in name").fill("erika");
  await page.getByLabel("Display name").fill("Erika");
  const fill = async () => {
    await page.getByLabel("Password", { exact: true }).fill("violet lantern harbour");
    await page.getByLabel("Password again").fill("violet lantern harbour");
  };
  await fill();
  await page.getByLabel("I am 16 or older.").check();
  await page.getByRole("checkbox", { name: /I accept the terms of use/ }).check();
  const refused = "The registration was not accepted. Please try once more.";
  // faster than a person: not sent
  await page.getByRole("button", { name: "Create account" }).click();
  await expect(page.getByRole("alert")).toHaveText(refused);
  // the field people never see, filled as form robots do: not sent
  await page.waitForTimeout(3_000);
  await fill();
  await page.locator("#register-website").evaluate((field: HTMLInputElement) => {
    field.value = "https://spam.invalid";
    field.dispatchEvent(new Event("input", { bubbles: true }));
  });
  await page.getByRole("button", { name: "Create account" }).click();
  await expect(page.getByRole("alert")).toHaveText(refused);
  expect(asked).toEqual([]);
  // a person, while an administrator closed registration
  await page.locator("#register-website").evaluate((field: HTMLInputElement) => {
    field.value = "";
    field.dispatchEvent(new Event("input", { bubbles: true }));
  });
  await fill();
  await page.getByRole("button", { name: "Create account" }).click();
  await expect(page.getByRole("alert")).toHaveText("Registration is paused right now. Please try again later.", { timeout: 15_000 });
  expect(asked).toEqual(["register_challenge"]);
});

// UUIDv7-shaped ids, the last byte telling them apart
const id = (n: number) => Array.from({ length: 16 }, (_, i) => ({ 6: 0x70, 8: 0x80, 15: n })[i] ?? 0x10 + i);
// two origins of the same length: their cards' fields must still differ
const gateway = (n: number, origin: string) => ({
  id: id(n), origin, metadata: { name: `Gateway ${n}`, version: "1.0.0", website: origin },
  state: "approved", online: false, last_seen: null, created_at: 1_794_000_000, tokens: [],
});

test("signed in, each action puts the focus where its result is", async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem("baylee-central-session", JSON.stringify({ expiresAt: 4_102_444_800 })));
  await page.route("**/api/v1", (route) => {
    const body = JSON.parse(route.request().postData() ?? "{}");
    const json = (value: unknown) => route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(value) });
    const owner = (answer: unknown) => json({ result: "owner", answer });
    if (body.operation === "account_summary") {
      return json({
        result: "account_summary", identity: { account: ACCOUNT, epoch: 1 }, username: "erika", display_name: "Erika", tag: 0x1a2b,
        source: "registered", recovery: true, admin: false, signed_in_at: 1_794_000_000, expires_at: 4_102_444_800,
      });
    }
    if (body.operation === "owner" && body.request?.action === "gateways") {
      return owner({ answer: "gateways", gateways: [gateway(1, "https://one.example"), gateway(2, "https://two.example")] });
    }
    if (body.operation === "owner" && body.request?.action === "create_token") {
      return owner({ answer: "token_created", id: id(9), token: Array(32).fill(5), expires_at: 4_102_444_800 });
    }
    if (body.operation === "account_export") return route.fulfill({ status: 503, body: "unavailable\n" });
    return route.fulfill({ status: 401, body: "rejected\n" });
  });

  await page.goto("/en/account/gateways");
  await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
  await expect(page.getByRole("heading", { name: "Gateway 2" })).toBeVisible();
  const ids = await page.locator("[id]").evaluateAll((all) => all.map((element) => element.id));
  expect(ids.filter((value, index) => ids.indexOf(value) !== index)).toEqual([]);
  const second = page.locator("article").filter({ hasText: "Gateway 2" });
  await second.getByLabel("Token name").fill("qa");
  await second.getByRole("button", { name: "Make a token" }).click();
  // the new token takes the focus; kept, its gateway's card does
  await expect(page.getByRole("heading", { name: "Your new token" })).toBeFocused();
  await page.getByRole("button", { name: "I have kept it" }).focus();
  await page.keyboard.press("Enter");
  await expect(page.getByRole("heading", { name: "Gateway 2" })).toBeFocused();

  await page.goto("/en/account/privacy");
  await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
  await page.getByRole("button", { name: "Prepare the export" }).click();
  await expect(page.locator("#export-error")).toBeFocused();
  await page.getByRole("button", { name: "Delete my account" }).click();
  await expect(page.locator("#delete-password")).toBeFocused();
});

test.describe("before the islands run", () => {
  // the service worker would fetch the WebAssembly past the page's routes
  test.use({ serviceWorkers: "block" });

  test("a sign-in sends nothing: the password stays in the page", async ({ page }) => {
    const posted: string[] = [];
    page.on("request", (request) => {
      if (request.method() !== "GET" && request.method() !== "HEAD") posted.push(`${request.method()} ${request.url()} ${request.postData() ?? ""}`);
    });
    // the WebAssembly never arrives: the islands never take the form over
    await page.route("**/assets/islands.*.wasm", (route) => route.abort());
    await page.goto("/en/signin");
    await expect(page.locator("#signin-password")).not.toHaveAttribute("name", /.*/);
    await expect(page.locator("#signin-name")).not.toHaveAttribute("name", /.*/);
    await page.locator("#signin-name").fill("erika");
    await page.locator("#signin-password").fill("violet lantern harbour");
    await page.locator("#signin-password").press("Enter");
    // and a submit by script, as a second press would make it
    await page.locator("#signin-password").evaluate((field: HTMLInputElement) => field.form?.requestSubmit());
    await page.waitForTimeout(500);
    await expect(page).toHaveURL(/\/en\/signin$/);
    expect(posted).toEqual([]);
  });
});

test("an administrator closes registration, signs in again when asked, and keeps the focus", async ({ page }) => {
  let open = true;
  let fresh = false;
  await page.addInitScript(() => localStorage.setItem("baylee-central-session", JSON.stringify({ expiresAt: 4_102_444_800 })));
  await page.route("**/api/v1", (route) => {
    const body = JSON.parse(route.request().postData() ?? "{}");
    const json = (value: unknown) => route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(value) });
    if (body.operation === "account_summary") {
      return json({
        result: "account_summary", identity: { account: ACCOUNT, epoch: 1 }, username: "erika", display_name: "Erika", tag: 0x1a2b,
        source: "registered", recovery: true, admin: true, signed_in_at: 1_794_000_000, expires_at: 4_102_444_800,
      });
    }
    if (body.operation === "admin" && body.request?.action === "registration") {
      return json({ result: "admin", answer: { answer: "registration", open, changed_at: null } });
    }
    if (body.operation === "admin" && body.request?.action === "set_registration") {
      // the first change goes through; the second wants a fresh sign-in
      if (fresh) return route.fulfill({ status: 401, body: "rejected\n" });
      fresh = true;
      open = body.request.open;
      return json({ result: "done" });
    }
    return route.fulfill({ status: 401, body: "rejected\n" });
  });
  await page.goto("/en/account");
  await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
  const status = page.getByText("Open: anyone may create an account, with the proof of work.");
  await expect(status).toBeVisible();
  await page.getByRole("button", { name: "Close registration" }).click();
  await expect(page.getByText("Closed: nobody can create an account until you open it again.")).toBeVisible();
  // the same button, its label changed, the focus still on it
  await expect(page.getByRole("button", { name: "Open registration again" })).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.locator("#reauth-name")).toBeFocused();
});
