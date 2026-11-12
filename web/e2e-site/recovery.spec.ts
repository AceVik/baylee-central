import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";

import { expect, type Locator, type Page, type Route, test } from "@playwright/test";

import { narrow, schemes, targets } from "./check.ts";

// The registration's recovery-code step (crates/central-web/src/islands/auth.rs
// `code_view`, keep.rs, and email.rs for the address) in a real browser,
// against a stand-in for Central's browser surface. The stand-in cannot
// answer OPAQUE or the proof truthfully, so the crypto Worker is a canned one
// that speaks its protocol; the page's own code runs everywhere else. All of
// it is synthetic: reserved domains, a recovery code and a mail code that are
// nobody's.

const ACCOUNT = [1, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 16];
const CODE = "0123-4567-89AB-CDEF-GHJK-MNPQ-RSTV-WXYZ";
const MAILED = "000G-40R4";
const ADDRESS = "erika@example.org";
const PASSWORD = "violet lantern harbour";
const THEN = 1_794_000_000;
const FILE = [
  "Baylee recovery code",
  "Sign-in name: erika",
  "",
  CODE,
  "",
  "With this code your encrypted Baylee data survives a forgotten password: you enter it on the reset page, together with a reset code.",
  "Keep it where only you can reach it. Nobody at Baylee will ever ask you for it.",
  "",
].join("\n");
const WARNING =
  "Careful: the code then lies in your mailbox. Whoever can read your mail can also have a reset code sent there, and with both reset your sign-in and open your encrypted data.";

/** The crypto Worker, canned: the proof, the enrolment and the sign-ins. */
const WORKER = `
self.addEventListener("message", (event) => {
  const m = JSON.parse(event.data);
  const ok = (value) => self.postMessage(JSON.stringify({ id: m.id, ok: value }));
  const bytes = (n, v) => Array(n).fill(v);
  switch (m.op) {
    case "work_start": case "work_end": case "chat_unlock": case "keep": case "login_confirm": return ok({});
    case "work_search": return ok({ counter: m.part + 1 });
    case "enrollment_start": return ok({ message: bytes(32, 3) });
    case "enrollment_finish":
      return ok({ recovery_code: "${CODE}", recovery: bytes(73, 4), record: bytes(64, 5), wrapped_master: bytes(73, 6), document: bytes(16, 7) });
    case "login_start": return ok({ message: bytes(96, 1) });
    case "login_finish": return ok({ message: bytes(64, 2) });
    default: return self.postMessage(JSON.stringify({ id: m.id, error: "failed" }));
  }
});`;

type Held = { mail: boolean; address: string | null; confirmed_at: number | null; pending: string | null; pending_until: number | null };
type Body = { operation?: string; request?: { action?: string; address?: string; lang?: string; code?: string } } & Record<string, unknown>;
type World = {
  held: Held;
  /** Central's terms are in force and not accepted yet. */
  terms: boolean;
  /** The next `set` finds the sign-in too old. */
  stale: boolean;
};

const world = (held: Partial<Held> = {}, more: Partial<World> = {}): World => ({
  held: { mail: true, address: null, confirmed_at: null, pending: null, pending_until: null, ...held },
  terms: false,
  stale: false,
  ...more,
});

const TEXTS = [{ lang: "en", updated: "2026-11-12", markdown: "# Terms\n\nPlay fair." }];

/** `central_api::terms::digest`: SHA-256 over a label and each text's fields, each after its length. */
function digest(texts: typeof TEXTS): number[] {
  const hash = createHash("sha256");
  hash.update("baylee-terms-v1\0");
  for (const text of texts) {
    for (const field of [text.lang, text.updated, text.markdown]) {
      const bytes = Buffer.from(field, "utf8");
      const length = Buffer.alloc(8);
      length.writeBigUInt64BE(BigInt(bytes.length));
      hash.update(length);
      hash.update(bytes);
    }
  }
  return [...hash.digest()];
}
const VERSION = { version: "2026-11-12", digest: digest(TEXTS) };

function central(seen: Body[], w: World) {
  return (route: Route) => {
    const request = route.request();
    expect(request.headers()["x-baylee-central"]).toBe("1");
    expect(new URL(request.url()).search).toBe("");
    const body = JSON.parse(request.postData() ?? "{}") as Body;
    seen.push(body);
    const json = (value: unknown) => route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(value) });
    const refused = (status: number) => route.fulfill({ status, body: "account request rejected\n" });
    const identity = { account: ACCOUNT, epoch: 1 };
    switch (body.operation) {
      case "register_challenge_for":
        return json({ result: "register_work", nonce: "ab".repeat(32), parts: 8, difficulty: 10, memory: 16_384, passes: 1, expires_at: Math.floor(Date.now() / 1000) + 1200 });
      case "register":
        return json({ result: "enrollment", identity, ticket: Array(32).fill(5) });
      case "enrollment_start":
        return json({ result: "enrollment_challenge", message: Array(64).fill(6) });
      case "enrollment_finish":
      case "accept_terms":
        if (body.operation === "accept_terms") w.terms = false;
        return json({ result: "done" });
      case "discover":
        return json({ result: "discovery", discovery: { kind: "central", identity } });
      case "login_start":
        return json({ result: "login_challenge", attempt: Array(32).fill(7), message: Array.from({ length: 320 }, (_, i) => (i * 37) % 251) });
      case "login_finish":
        return json({ result: "browser_session", identity, username: "erika", wrapped_master: Array(73).fill(9), expires_at: 4_102_444_800 });
      case "account_summary":
        return json({
          result: "account_summary", identity, username: "erika", display_name: "Erika", tag: 0x1a2b,
          source: "registered", recovery: true, admin: false, signed_in_at: THEN, expires_at: 4_102_444_800,
        });
      case "terms_status":
        return json({ result: "terms_status", current: w.terms ? VERSION : null, accepted: !w.terms });
      case "terms":
        return json({ result: "terms", terms: w.terms ? { version: VERSION, texts: TEXTS } : null });
      case "email": {
        const asked = body.request ?? {};
        // what Central refuses before the terms are accepted
        if (w.terms && asked.action !== "status") return refused(403);
        const held = w.held;
        if (asked.action === "set") {
          if (w.stale) {
            w.stale = false;
            return refused(401);
          }
          Object.assign(held, { pending: asked.address, pending_until: THEN + 900 });
        } else if (asked.action === "confirm") {
          if (asked.code !== MAILED) return refused(401);
          Object.assign(held, { address: held.pending, confirmed_at: THEN + 60, pending: null, pending_until: null });
        } else if (asked.action !== "status") {
          return refused(401);
        }
        return json({ result: "email", state: held });
      }
      default:
        return refused(401);
    }
  };
}

type Opened = { seen: Body[]; problems: string[]; consoled: string[] };

/** The page with the stand-ins in place: Central, the Worker, and, where asked, a password manager. */
async function open(page: Page, w: World, options: { manager?: boolean; path?: string } = {}): Promise<Opened> {
  const seen: Body[] = [];
  const problems: string[] = [];
  const consoled: string[] = [];
  page.on("pageerror", (error) => problems.push(error.message));
  page.on("console", (message) => consoled.push(message.text()));
  await page.route("**/api/v1", central(seen, w));
  await page.route("**/assets/crypto-worker.*.js", (route) => route.fulfill({ contentType: "text/javascript", body: WORKER }));
  if (options.manager ?? true) {
    // Chromium's PasswordCredential and credentials.store, recorded: the
    // real ones ask the person, which a test cannot answer
    await page.addInitScript(() => {
      const stored: { id: string; password: string }[] = [];
      class Offered {
        readonly type = "password";
        readonly id: string;
        readonly password: string;
        constructor(data: { id: string; password: string }) {
          if (!data.id || !data.password) throw new TypeError("an id and a password");
          this.id = data.id;
          this.password = data.password;
        }
      }
      Object.defineProperty(window, "PasswordCredential", { value: Offered, configurable: true, writable: true });
      Object.defineProperty(navigator.credentials, "store", {
        configurable: true,
        value: (credential: Offered) => {
          stored.push({ id: credential.id, password: credential.password });
          return Promise.resolve(credential);
        },
      });
      Object.defineProperty(window, "storedCredentials", { value: stored });
    });
  } else {
    // as in Firefox and Safari: no PasswordCredential at all
    await page.addInitScript(() => {
      Reflect.deleteProperty(window, "PasswordCredential");
    });
  }
  // the print dialog, counted instead of shown
  await page.addInitScript(() => {
    let printed = 0;
    Object.defineProperty(window, "print", { configurable: true, value: () => (printed += 1) });
    Object.defineProperty(window, "printed", { get: () => printed });
  });
  await page.goto(options.path ?? "/en/register");
  // a page without islands of its own loads them when a dialog opens
  if ((options.path ?? "/register").endsWith("/register")) await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
  return { seen, problems, consoled };
}

// a route does not see what a service worker fetches: the canned Worker
test.use({ serviceWorkers: "block" });

// Each test registers first, which takes six seconds a person and Central
// ask for, and several measure both schemes: the whole suite at once is
// slower than 30 seconds a test allow.
test.beforeEach(() => {
  test.slow();
});

/** Fills the registration in `scope` as a person would, after the three seconds a person takes, and sends it. */
async function register(page: Page, scope: Locator | Page = page) {
  await scope.getByLabel("Sign-in name").fill("erika");
  await scope.getByLabel("Display name").fill("Erika");
  await scope.getByLabel("Password", { exact: true }).fill(PASSWORD);
  await scope.getByLabel("Repeat password").fill(PASSWORD);
  await scope.getByLabel("I am 16 or older.").check();
  await scope.getByRole("checkbox", { name: /I accept the terms of use/ }).check();
  await page.waitForTimeout(3_000);
  await scope.getByRole("button", { name: "Create account" }).click();
  await expect(scope.getByRole("heading", { name: "Your recovery code" })).toBeVisible({ timeout: 30_000 });
}

const step = (page: Page) => page.locator("section[aria-labelledby=code-title]");
const button = (page: Page, name: string) => step(page).getByRole("button", { name, exact: true });
const field = (page: Page) => page.locator("#code-email");
const emails = (seen: Body[]) => seen.filter((body) => body.operation === "email").map((body) => body.request?.action);

/** The page as it stands passes the checks every page passes: axe in both schemes, the targets, 320 px. */
async function sound(page: Page) {
  expect(await schemes(page)).toEqual([]);
  expect(await targets(page)).toEqual([]);
  expect(await narrow(page)).toBe(0);
}

/** Tab leads through `stops` in this order, starting behind `from`. */
async function tabs(page: Page, from: Locator, stops: Locator[]) {
  await from.focus();
  for (const stop of stops) {
    await page.keyboard.press("Tab");
    await expect(stop).toBeFocused();
  }
}

test("the code step offers every way to keep the code, each made only when it is asked for", async ({ page }) => {
  const { seen, problems, consoled } = await open(page, world());
  await register(page);
  const section = step(page);
  await expect(section.locator(".secret code")).toHaveText(CODE);
  await expect(section).toContainText("Your handle: Erika#1a2b");
  await expect(section).toContainText("Keep it somewhere safe where you will find it even without this device.");
  await expect(field(page)).toBeVisible();

  // Before any of them is pressed, the code is in the page once, as text,
  // and no address, file or sheet holds it.
  const before = await page.content();
  expect(before.split(CODE)).toHaveLength(2);
  expect(before).not.toContain("mailto:");
  expect(before).not.toContain("data:text");
  await expect(page.locator("a[download], a[href^='mailto:'], .code-sheet")).toHaveCount(0);
  expect(page.url()).not.toContain(CODE);

  const ways = ["Copy", "Save as a text file", "Print", "Save in password manager", "Mail it to myself"];
  for (const name of ways) await expect(button(page, name)).toBeVisible();
  await expect(section.locator("#code-mail-warning")).toHaveText(WARNING);
  await expect(button(page, "Mail it to myself")).toHaveAccessibleDescription(WARNING);
  await expect(field(page)).toHaveAttribute("type", "email");
  await expect(field(page)).toHaveAttribute("autocomplete", "email");
  await expect(field(page)).not.toHaveAttribute("name", /.*/);
  await expect(section.locator("label[for=code-email]")).toHaveText("E-mail address (optional)");
  await expect(field(page)).toHaveAccessibleDescription(
    /^For a reset code if you forget your password\..*Whoever can read this mailbox can reset your sign-in\./,
  );
  await sound(page);
  // the keyboard goes through the ways, the address, the box and on
  await tabs(page, button(page, "Copy"), [
    ...ways.slice(1).map((name) => button(page, name)),
    field(page),
    page.locator("#code-kept"),
    button(page, "Continue"),
  ]);

  // The mail: built on the click, to the address typed, with the subject
  // and the body made here, and never put into the page.
  await field(page).fill(ADDRESS);
  const asked: string[] = [];
  const protocol = await page.context().newCDPSession(page);
  await protocol.send("Page.enable");
  protocol.on("Page.frameRequestedNavigation", (event) => asked.push(event.url));
  await button(page, "Mail it to myself").click();
  await expect.poll(() => asked.length).toBe(1);
  const mail = new URL(asked[0] ?? "");
  expect(mail.protocol).toBe("mailto:");
  expect(decodeURIComponent(mail.pathname)).toBe(ADDRESS);
  expect(mail.searchParams.get("subject")).toBe("My Baylee recovery code");
  expect(mail.searchParams.get("body")).toBe(FILE.replaceAll("\n", "\r\n"));
  expect(await page.content()).not.toContain("mailto:");
  await expect(page).toHaveURL(/\/en\/register$/);
  await expect(section).toBeVisible();

  // The file: made on the click, saved, and its link gone again.
  const [download] = await Promise.all([page.waitForEvent("download"), button(page, "Save as a text file").click()]);
  expect(download.suggestedFilename()).toBe("baylee-recovery-code.txt");
  expect(await readFile(await download.path(), "utf8")).toBe(FILE);
  await expect(page.locator("a[download]")).toHaveCount(0);

  // The password manager: offered under a name that says what it is.
  await button(page, "Save in password manager").click();
  await expect(section.getByRole("status").filter({ hasText: "Your browser was offered the code." })).toHaveText(
    "Your browser was offered the code. If it asked you, check that it saved it.",
  );
  expect(await page.evaluate(() => Reflect.get(window, "storedCredentials"))).toEqual([{ id: "Baylee recovery code (erika)", password: CODE }]);

  // Copying, as before.
  await page.context().grantPermissions(["clipboard-read", "clipboard-write"]);
  await button(page, "Copy").click();
  await expect(section.getByRole("status").filter({ hasText: "Copied." })).toBeVisible();
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(CODE);

  // The print: the sheet is drawn, and a print shows it and nothing else.
  await expect(page.locator(".code-sheet")).toHaveCount(0);
  await button(page, "Print").click();
  await expect.poll(() => page.evaluate(() => Reflect.get(window, "printed"))).toBe(1);
  await expect(page.locator(".code-sheet")).toBeHidden();
  await page.emulateMedia({ media: "print" });
  const sheet = page.locator(".code-sheet");
  await expect(sheet).toBeVisible();
  await expect(sheet.locator(".code-sheet-code")).toHaveText(CODE);
  const printed = await page.evaluate(() => document.body.innerText.trim());
  expect(printed.split("\n").filter((line) => line.trim() !== "")).toEqual([
    "Baylee",
    "Baylee recovery code",
    "Sign-in name: erika",
    CODE,
    "With this code your encrypted Baylee data survives a forgotten password: you enter it on the reset page, together with a reset code.",
    "Keep it where only you can reach it. Nobody at Baylee will ever ask you for it.",
  ]);
  await page.emulateMedia({ media: "screen" });
  await expect(sheet).toBeHidden();
  await expect(section.locator(".secret code")).toBeVisible();

  // Nothing of the code went to Central, and the page logged none of it.
  expect(JSON.stringify(seen)).not.toContain(CODE);
  expect(JSON.stringify(seen)).not.toContain(CODE.replaceAll("-", ""));
  expect(consoled.filter((line) => line.includes(CODE) || line.includes("mailto:"))).toEqual([]);
  expect(emails(seen)).toEqual(["status"]);
  expect(problems).toEqual([]);
});

test("an address typed beside the code is named after it, without the password, and confirmed on the next step", async ({ page }) => {
  const { seen, problems } = await open(page, world());
  await register(page);
  await field(page).fill(ADDRESS);
  // the box first
  await button(page, "Continue").click();
  await expect(step(page).getByRole("alert")).toHaveText("Save the code first, then confirm it.");
  expect(emails(seen)).toEqual(["status"]);
  await page.locator("#code-kept").check();
  const signedIn = seen.length;
  await button(page, "Continue").click();

  const card = page.locator("section[aria-labelledby=email-title]");
  await expect(card.getByRole("heading", { name: "Confirm your e-mail address" })).toBeVisible();
  await expect(card).toContainText("Enter the code from the mail here, or later on your account's security page.");
  await expect(card).toContainText(`We sent a code to ${ADDRESS}. It is valid for 15 minutes.`);
  // the registration's sign-in is fresh: no password asked, no sign-in once more
  const operations = seen.slice(signedIn).map((body) => body.operation);
  expect(operations).not.toContain("login_start");
  expect(seen.find((body) => body.request?.action === "set")).toEqual({ operation: "email", request: { action: "set", address: ADDRESS, lang: "en" } });
  await expect(page.locator("#email-code")).toBeFocused();
  await sound(page);
  await tabs(page, page.locator("#email-code"), [
    card.getByRole("button", { name: "Confirm", exact: true }),
    card.getByRole("button", { name: "Send again", exact: true }),
    card.getByRole("button", { name: "Use another address", exact: true }),
    card.getByRole("button", { name: "Continue to your account", exact: true }),
  ]);

  await page.locator("#email-code").fill(MAILED.toLowerCase());
  await card.getByRole("button", { name: "Confirm", exact: true }).click();
  await expect(page.locator("#email-said")).toHaveText("Your e-mail address is confirmed.");
  await expect(card.locator("dl.facts dd").first()).toHaveText(ADDRESS);
  expect(emails(seen)).toEqual(["status", "set", "confirm"]);
  await sound(page);
  await card.getByRole("button", { name: "Continue to your account", exact: true }).click();
  await expect(page).toHaveURL(/\/en\/account$/);
  expect(JSON.stringify(seen)).not.toContain(PASSWORD);
  expect(problems).toEqual([]);
});

for (const mail of [true, false]) {
  test(`skipping the address is one click${mail ? "" : ", and where Central sends no mail there is no field"}`, async ({ page }) => {
    const { seen } = await open(page, world({ mail }));
    await register(page);
    await expect(field(page)).toHaveCount(mail ? 1 : 0);
    await page.locator("#code-kept").check();
    await button(page, "Continue").click();
    await expect(page).toHaveURL(/\/en\/account$/);
    // nothing about an address but the look whether one can be named
    expect(emails(seen)).toEqual(["status"]);
  });
}

test("where Central's terms are in force the address is named once they are accepted", async ({ page }) => {
  const { seen } = await open(page, world({}, { terms: true }));
  await register(page);
  await field(page).fill(ADDRESS);
  await page.locator("#code-kept").check();
  await button(page, "Continue").click();
  await expect(page.getByRole("heading", { name: "New terms of use" })).toBeVisible();
  expect(emails(seen)).toEqual(["status"]);
  await page.getByRole("button", { name: "Accept", exact: true }).click();
  await expect(page.locator("#email-code")).toBeFocused();
  const order = seen.map((body) => body.operation === "email" ? `email ${body.request?.action}` : body.operation);
  expect(order.indexOf("accept_terms")).toBeGreaterThan(0);
  expect(order.indexOf("email set")).toBeGreaterThan(order.indexOf("accept_terms"));
});

test("an address Central would not mail to is refused before anything is sent", async ({ page }) => {
  const { seen } = await open(page, world());
  await register(page);
  await page.locator("#code-kept").check();
  for (const bad of ["erika", "erika@localhost", "jürgen@example.org"]) {
    await field(page).fill(bad);
    await button(page, "Continue").click();
    await expect(step(page).getByRole("alert")).toHaveText(
      "We cannot write to this address. Enter one plain address such as name@example.org; accented letters and umlauts do not work.",
    );
    await expect(field(page)).toBeFocused();
  }
  expect(emails(seen)).toEqual(["status"]);
  expect(seen.map((body) => body.operation)).not.toContain("terms_status");
  // the mail to oneself then goes to nobody yet: the mail program asks
  const asked: string[] = [];
  const protocol = await page.context().newCDPSession(page);
  await protocol.send("Page.enable");
  protocol.on("Page.frameRequestedNavigation", (event) => asked.push(event.url));
  await button(page, "Mail it to myself").click();
  await expect.poll(() => asked.length).toBe(1);
  expect(asked[0]).toMatch(/^mailto:\?subject=My%20Baylee%20recovery%20code&body=Baylee%20recovery%20code%0D%0A/);
});

test("a sign-in older than Central takes: the step asks for the password, then names the address", async ({ page }) => {
  const { seen } = await open(page, world({}, { stale: true }));
  await register(page);
  await field(page).fill(ADDRESS);
  await page.locator("#code-kept").check();
  await button(page, "Continue").click();
  const card = page.locator("section[aria-labelledby=email-title]");
  await expect(page.locator("#email-said")).toHaveText("To send the code, enter your password once more.");
  await expect(page.locator("#email-password")).toBeFocused();
  await expect(page.locator("#email-address")).toHaveValue(ADDRESS);
  await expect(card.locator("#email-error")).toBeEmpty();
  await sound(page);
  const asked = seen.length;
  await page.locator("#email-password").fill(PASSWORD);
  await card.getByRole("button", { name: "Send confirmation code", exact: true }).click();
  // the sign-in once more runs in a fresh Worker
  await expect(card).toContainText(`We sent a code to ${ADDRESS}. It is valid for 15 minutes.`, { timeout: 15_000 });
  expect(seen.slice(asked).map((body) => body.operation)).toEqual(["account_summary", "login_start", "login_finish", "email"]);
  expect(JSON.stringify(seen)).not.toContain(PASSWORD);
});

test("without PasswordCredential, as in Firefox and Safari, there is no password manager's button", async ({ page }) => {
  await open(page, world(), { manager: false });
  await register(page);
  await expect(button(page, "Copy")).toBeVisible();
  await expect(button(page, "Print")).toBeVisible();
  await expect(step(page).getByRole("button", { name: "Save in password manager" })).toHaveCount(0);
  expect(await page.evaluate(() => "PasswordCredential" in window)).toBe(false);
});

test("in the registration's dialog the print is the sheet alone too", async ({ page }) => {
  await open(page, world(), { path: "/en/faq" });
  const menu = page.locator(".menu-btn");
  if (await menu.isVisible()) await menu.click();
  await page.locator("a.nav-account").click();
  await page.locator("#auth-signin a[href$='/register']").click();
  const dialog = page.locator("#auth-register");
  await expect(dialog).toBeVisible();
  await expect(dialog.locator("leptos-island")).toHaveAttribute("data-hydrated", "", { timeout: 15_000 });
  await register(page, dialog);
  await expect(page).toHaveURL(/\/en\/faq$/);
  await sound(page);
  await dialog.getByRole("button", { name: "Print", exact: true }).click();
  await expect.poll(() => page.evaluate(() => Reflect.get(window, "printed"))).toBe(1);
  await page.emulateMedia({ media: "print" });
  await expect(page.locator(".code-sheet")).toBeVisible();
  await expect(page.locator(".code-sheet .code-sheet-code")).toHaveText(CODE);
  for (const gone of ["main", ".top", "#code-title", ".keep", ".auth-mark"]) await expect(page.locator(gone).first()).toBeHidden();
  const printed = await page.evaluate(() => document.body.innerText);
  expect(printed).toContain("Sign-in name: erika");
  expect(printed).not.toContain("Save as a text file");
  await page.emulateMedia({ media: "screen" });
  await expect(page.locator("#code-title")).toBeVisible();
});

/** What in `card` reaches past its right edge, and how far it scrolls sideways. */
async function overflow(card: Locator): Promise<{ beyond: string[]; scroll: number }> {
  return card.evaluate((el) => {
    const edge = el.getBoundingClientRect().right + 0.5;
    const beyond = [...el.querySelectorAll("*")]
      .filter((node) => {
        const box = node.getBoundingClientRect();
        return box.width > 0 && box.right > edge;
      })
      .map((node) => `${node.tagName.toLowerCase()}.${node.className}`);
    return { beyond, scroll: el.scrollWidth - el.clientWidth };
  });
}

// German has the longest words, and the dialog hides what runs out of its
// card: at 320 px nothing of the two steps may reach past the card's edge,
// on the page or in the dialog.
for (const where of ["page", "dialog"] as const) {
  test(`at 320 px in German both steps stay inside their card: ${where}`, async ({ page }) => {
    await page.setViewportSize({ width: 320, height: 760 });
    await open(page, world(), { path: where === "page" ? "/de/register" : "/de/faq" });
    let card = page.locator(".auth-form");
    if (where === "dialog") {
      const menu = page.locator(".menu-btn");
      if (await menu.isVisible()) await menu.click();
      await page.locator("a.nav-account").click();
      await page.locator("#auth-signin a[href$='/register']").click();
      card = page.locator("#auth-register");
      await expect(card.locator("leptos-island")).toHaveAttribute("data-hydrated", "", { timeout: 15_000 });
    }
    await card.locator("#register-name").fill("erika");
    await card.locator("#register-display").fill("Erika");
    await card.locator("#register-password").fill(PASSWORD);
    await card.locator("#register-repeat").fill(PASSWORD);
    await card.locator("#register-age").check();
    await card.locator("#register-consent").check();
    await page.waitForTimeout(3_000);
    await card.getByRole("button", { name: "Account anlegen" }).click();
    await expect(card.locator("#code-title")).toBeVisible({ timeout: 30_000 });
    await expect(card.getByRole("button", { name: "Im Passwortmanager speichern" })).toBeVisible();
    await card.locator("#code-email").fill(ADDRESS);
    expect(await overflow(card)).toEqual({ beyond: [], scroll: 0 });
    expect(await narrow(page)).toBe(0);
    expect(await targets(page)).toEqual([]);
    await card.locator("#code-kept").check();
    await card.getByRole("button", { name: "Weiter", exact: true }).click();
    await expect(page.locator("#email-code")).toBeFocused();
    expect(await overflow(card)).toEqual({ beyond: [], scroll: 0 });
    expect(await narrow(page)).toBe(0);
  });
}
