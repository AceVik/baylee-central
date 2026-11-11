import { createHash } from "node:crypto";

import { expect, type Page, type Route, test } from "@playwright/test";

import { axe, targets } from "./check.ts";

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
    await page.getByLabel("Repeat password").fill("violet lantern harbour");
  };
  await fill();
  await page.getByLabel("I am 16 or older.").check();
  await page.getByRole("checkbox", { name: /I accept the terms of use/ }).check();
  const refused = "The registration was not accepted. Try again.";
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
  await expect(page.getByRole("alert")).toHaveText("Registration is paused right now. Try again later.", { timeout: 15_000 });
  expect(asked).toEqual(["register_challenge_for"]);
});

// The register form, filled in as a person would, after the three seconds
// a person takes at the least.
async function fillRegistration(page: Page) {
  await page.goto("/en/register");
  await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
  await page.getByLabel("Sign-in name").fill("erika");
  await page.getByLabel("Display name").fill("Erika");
  await page.getByLabel("Password", { exact: true }).fill("violet lantern harbour");
  await page.getByLabel("Repeat password").fill("violet lantern harbour");
  await page.getByLabel("I am 16 or older.").check();
  await page.getByRole("checkbox", { name: /I accept the terms of use/ }).check();
  await page.waitForTimeout(3_000);
}

const REFUSED = "The registration was not accepted. Try again.";
const rejected = (route: Route) => route.fulfill({ status: 401, body: "account request rejected\n" });
const reply = (route: Route, value: unknown) => route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(value) });
const soon = () => Math.floor(Date.now() / 1000) + 1200;

// The pinned vector of central_crypto::proof (crates/central-crypto/src/proof.rs):
// this nonce at this price has these smallest counters, in Rust on the host
// and in the committed module in a browser.
const PINNED = {
  nonce: "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
  price: { parts: 4, difficulty: 3, memory: 64, passes: 1 },
  counters: [7, 25, 2, 7],
};

test("the page's Worker finds the counters Central checks, and sends them again when Central is busy", async ({ page }) => {
  const problems: string[] = [];
  page.on("pageerror", (error) => problems.push(error.message));
  const asked: Record<string, unknown>[] = [];
  const scripts: string[] = [];
  page.on("request", (request) => {
    if (/\/assets\/(pow|crypto)-worker\./.test(request.url())) scripts.push(new URL(request.url()).pathname.replace(/\.[0-9a-z]+\.js$/, ".js"));
  });
  await page.route("**/api/v1", (route) => {
    const body = JSON.parse(route.request().postData() ?? "{}");
    asked.push(body);
    if (body.operation === "register_challenge_for") {
      return reply(route, { result: "register_work", nonce: PINNED.nonce, ...PINNED.price, expires_at: soon() });
    }
    // no worker free at first: Central keeps the challenge and says so
    if (asked.filter((b) => b.operation === "register").length === 1) return route.fulfill({ status: 429, body: "account request limit reached\n" });
    return rejected(route);
  });
  await fillRegistration(page);
  await page.getByRole("button", { name: "Create account" }).click();
  await expect(page.getByRole("alert")).toHaveText(REFUSED, { timeout: 30_000 });
  const register = { operation: "register", username: "erika", display_name: "Erika", nonce: PINNED.nonce, counters: PINNED.counters };
  expect(asked).toEqual([{ operation: "register_challenge_for", signals: 1 }, register, register]);
  // The module's Worker did the work, and a fresh one stood ready for the
  // enrolment: a Worker's memory does not shrink, so the proof's is ended
  // and never holds the password's too. The older proof's script was not
  // even fetched.
  expect(scripts).toEqual(["/assets/crypto-worker.js", "/assets/crypto-worker.js"]);
  expect(problems).toEqual([]);
});

test("a price above what the page and Central agreed on is not worked on", async ({ page }) => {
  const asked: string[] = [];
  // What Central asks at its most, the page takes (the test below); one
  // bit more is twice that, and 64 MiB and a kibibyte is more memory than
  // any price may ask. central-api's tests have every other ceiling.
  const above = [
    { parts: 8, difficulty: 11, memory: 16_384, passes: 1 },
    { parts: 1, difficulty: 0, memory: 65_537, passes: 1 },
  ];
  let price = above[0];
  await page.route("**/api/v1", (route) => {
    const body = JSON.parse(route.request().postData() ?? "{}");
    asked.push(String(body.operation));
    if (body.operation !== "register_challenge_for") return rejected(route);
    return reply(route, { result: "register_work", nonce: "ab".repeat(32), ...price, expires_at: soon() });
  });
  for (price of above) {
    asked.length = 0;
    await fillRegistration(page);
    await page.getByRole("button", { name: "Create account" }).click();
    await expect(page.getByText("Something went wrong. Try again; if it keeps happening, tell us.")).toBeVisible({ timeout: 15_000 });
    // refused before any work, and nothing was handed in
    expect(asked, JSON.stringify(price)).toEqual(["register_challenge_for"]);
  }
});

const zeroBits = (hash: Buffer) => {
  let bits = 0;
  for (const byte of hash) {
    if (byte !== 0) return bits + Math.clz32(byte) - 24;
    bits += 8;
  }
  return bits;
};

test("against a Central from before, the page asks the older way and solves the older proof", async ({ page }) => {
  const problems: string[] = [];
  page.on("pageerror", (error) => problems.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error" && !message.text().includes("status of 401")) problems.push(message.text());
  });
  const asked: Record<string, unknown>[] = [];
  const nonce = "5c".repeat(32);
  await page.route("**/api/v1", (route) => {
    const body = JSON.parse(route.request().postData() ?? "{}");
    asked.push(body);
    // it knows neither the newer request nor the newer answer
    if (body.operation === "register_challenge") return reply(route, { result: "register_challenge", nonce, parts: 8, difficulty: 10, expires_at: soon() });
    return rejected(route);
  });
  await fillRegistration(page);
  await page.getByRole("button", { name: "Create account" }).click();
  await expect(page.getByRole("alert")).toHaveText(REFUSED, { timeout: 30_000 });
  expect(asked.map((b) => b.operation)).toEqual(["register_challenge_for", "register_challenge", "register"]);
  expect(asked[1]).toEqual({ operation: "register_challenge" });
  const sent = asked[2] as { nonce: string; counters: number[] };
  expect(sent.nonce).toBe(nonce);
  expect(sent.counters).toHaveLength(8);
  // SHA-256 over nonce ":" part ":" counter, as that Central checks it
  sent.counters.forEach((counter, part) => {
    expect(zeroBits(createHash("sha256").update(`${nonce}:${part}:${counter}`).digest())).toBeGreaterThanOrEqual(10);
  });
  expect(problems).toEqual([]);
});

// A stand-in for the crypto Worker's proof of work: each part takes a
// second and "finds" its own number, which the stand-in for Central does
// not check. It speaks the Worker's protocol, so the page's own code runs.
const SLOW_WORKER = `self.onmessage = (event) => {
  const m = JSON.parse(event.data);
  const answer = (ok) => self.postMessage(JSON.stringify({ id: m.id, ok }));
  if (m.op === "work_search") setTimeout(() => answer({ counter: m.part + 1 }), 1000);
  else if (m.op === "work_start" || m.op === "work_end") answer({});
  else self.postMessage(JSON.stringify({ id: m.id, error: "failed" }));
};`;

test("a proof that takes long says so politely, and the page reports its browser", async ({ page }) => {
  const problems: string[] = [];
  page.on("pageerror", (error) => problems.push(error.message));
  await page.route("**/assets/crypto-worker.*.js", (route) => route.fulfill({ status: 200, contentType: "text/javascript", body: SLOW_WORKER }));
  const asked: Record<string, unknown>[] = [];
  await page.route("**/api/v1", (route) => {
    const body = JSON.parse(route.request().postData() ?? "{}");
    asked.push(body);
    if (body.operation !== "register_challenge_for") return rejected(route);
    // the hardest challenge Central asks for, twenty minutes to hand it in
    return reply(route, { result: "register_work", nonce: "ab".repeat(32), parts: 8, difficulty: 10, memory: 16_384, passes: 1, expires_at: soon() });
  });
  await fillRegistration(page);
  // quiet until there is something to say, and polite when there is
  const solving = page.locator(".solving");
  await expect(solving).toHaveAttribute("aria-live", "polite");
  await expect(solving).toBeEmpty();
  await expect(solving).toBeHidden();
  await page.getByRole("button", { name: "Create account" }).click();
  // nothing for the first three seconds, then the line and the bar
  await expect(solving).toBeEmpty();
  await expect(solving.locator("p")).toContainText("Your browser is solving a puzzle that keeps robots out.", { timeout: 10_000 });
  await expect(solving.locator("p")).toContainText("%");
  await expect(solving.getByRole("progressbar")).toHaveAttribute("max", "100");
  await expect(solving.getByRole("progressbar")).toHaveAttribute("value", /^(37|50|62|75|87)$/);
  // the bar is named by the line, and the page keeps its rules while it works
  await expect(solving.getByRole("progressbar")).toHaveAccessibleName(/^Your browser is solving a puzzle/);
  expect(await axe(page)).toEqual([]);
  expect(await targets(page)).toEqual([]);
  // handed in, the line goes; this stand-in refuses the registration
  await expect(page.getByRole("alert")).toHaveText(REFUSED, { timeout: 20_000 });
  await expect(solving).toBeEmpty();
  // a browser a program drives says so (bit 1), and has languages (bit 2 unset)
  expect(asked[0]).toEqual({ operation: "register_challenge_for", signals: 1 });
  expect(asked[1]).toMatchObject({ operation: "register", username: "erika", display_name: "Erika", nonce: "ab".repeat(32), counters: [1, 2, 3, 4, 5, 6, 7, 8] });
  expect(asked).toHaveLength(2);
  expect(problems).toEqual([]);
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
  await second.getByRole("button", { name: "Create token" }).click();
  // the new token takes the focus; kept, its gateway's card does
  await expect(page.getByRole("heading", { name: "Your new token" })).toBeFocused();
  await page.getByRole("button", { name: "I have saved it" }).focus();
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
  const status = page.getByText("Open: anyone may create an account after solving the proof of work.");
  await expect(status).toBeVisible();
  await page.getByRole("button", { name: "Close registration" }).click();
  await expect(page.getByText("Closed: nobody can create an account until you open it again.")).toBeVisible();
  // the same button, its label changed, the focus still on it
  await expect(page.getByRole("button", { name: "Open registration again" })).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.locator("#reauth-name")).toBeFocused();
});
