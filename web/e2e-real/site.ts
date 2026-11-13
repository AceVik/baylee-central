// Steps the new site's specs share against a real Central (README.md §"The
// new site"): registering and signing in on the page, asking Central with
// the page's own session, the operator's appointment of an administrator,
// and a sign-in over the native listener as Baylee's client makes it, with
// the committed WebAssembly. Synthetic accounts only. A passing run prints
// no password, code or token and writes none. A failing one may: its
// assertion's message and its trace (`../test-results-real`, kept on
// failure) hold what was typed and sent. Delete that folder after reading.

import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { request } from "node:http";
import { resolve } from "node:path";

import { expect, type Page } from "@playwright/test";

import { chat_unlock, initSync, login_finish, login_start } from "../../crates/central-web/crypto/central_web_crypto.js";

/** A recovery code as the page shows it. */
export const RECOVERY = /^[0-9A-HJKMNP-TV-Z]{4}(-[0-9A-HJKMNP-TV-Z]{4}){7}$/;
/** A reset code as an administrator is shown it. */
export const RESET = /^RESET(-[0-9A-HJKMNP-TV-Z]{4}){8}$/;
export const WRONG_SIGN_IN = "That sign-in name and password do not match. Check both and try again.";

const NATIVE = process.env["REAL_NATIVE"] ?? "127.0.0.1:47920";

type Json = Record<string, unknown>;
export type Identity = { account: number[]; epoch: number };

const hex = (bytes: unknown): string => Buffer.from(bytes as number[]).toString("hex");
const bytes = (v: unknown): Uint8Array => Uint8Array.from(v as number[]);
const array = (b: Uint8Array): number[] => Array.from(b);

/** The page loaded and its islands running. */
export async function open(page: Page, path: string) {
  await page.goto(path);
  await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
}

/**
 * Registers on the site's page and answers the recovery code it showed once.
 * With `email`, the address is typed beside the code; the page names it after
 * the code (and Central's terms), and this returns on the step that asks for
 * the mailed code, which is the caller's to confirm.
 */
export async function register(page: Page, username: string, display: string, password: string, email?: string): Promise<string> {
  await open(page, "/en/register");
  await page.getByLabel("Sign-in name").fill(username);
  await page.getByLabel("Display name").fill(display);
  await page.getByLabel("Password", { exact: true }).fill(password);
  await page.getByLabel("Repeat password").fill(password);
  await page.getByLabel("I am 16 or older.").check();
  await page.getByRole("checkbox", { name: /I accept the terms of use/ }).check();
  // a person takes longer than three seconds to fill a form; the page refuses faster ones
  await page.waitForTimeout(3_000);
  await page.getByRole("button", { name: "Create account" }).click();
  await expect(page.getByRole("heading", { name: "Your recovery code" })).toBeVisible({ timeout: 90_000 });
  const code = (await page.locator("#register-steps .secret code").textContent()) ?? "";
  expect(code).toMatch(RECOVERY);
  // the code can be copied as it stands there (the clipboard is this test browser's own)
  await page.context().grantPermissions(["clipboard-read", "clipboard-write"]);
  await page.getByRole("button", { name: "Copy", exact: true }).click();
  await expect(page.locator("#register-steps").getByRole("status").filter({ hasText: "Copied." })).toHaveText("Copied.");
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(code);
  // the other ways to keep it are there; none of them is used here
  for (const way of ["Save as a text file", "Print", "Mail it to myself"]) {
    await expect(page.getByRole("button", { name: way, exact: true })).toBeVisible();
  }
  if (email !== undefined) await page.locator("#code-email").fill(email);
  await page.getByLabel("I have saved my recovery code").check();
  await page.getByRole("button", { name: "Continue", exact: true }).click();
  // where Central's terms are in force, they come after the code
  const terms = page.getByRole("heading", { name: "New terms of use" });
  const next =
    email === undefined ? page.locator(".account-who") : page.getByRole("heading", { name: "Confirm your e-mail address" });
  await expect(terms.or(next)).toBeVisible({ timeout: 30_000 });
  if (await terms.isVisible()) await page.getByRole("button", { name: "Accept", exact: true }).click();
  if (email !== undefined) {
    await expect(next).toBeVisible({ timeout: 30_000 });
    return code;
  }
  await expect(page).toHaveURL(/\/en\/account$/, { timeout: 30_000 });
  await expect(page.locator(".account-who .handle")).toHaveText(new RegExp(`^${display}#[a-f0-9]{4,8}$`), { timeout: 30_000 });
  return code;
}

/** Fills the sign-in page and sends it; what follows is the caller's to expect. */
export async function signIn(page: Page, username: string, password: string) {
  await open(page, "/en/signin");
  const form = page.locator("#signin-steps");
  await form.getByLabel("Sign-in name").fill(username);
  await form.getByLabel("Password", { exact: true }).fill(password);
  await form.getByRole("button", { name: "Sign in", exact: true }).click();
}

/** A sign-in the page takes: the account area, signed in. */
export async function signedIn(page: Page, username: string, password: string) {
  await signIn(page, username, password);
  await expect(page).toHaveURL(/\/en\/account$/, { timeout: 60_000 });
  await expect(page.locator(".account-who .handle")).toBeVisible({ timeout: 30_000 });
}

/** A sign-in Central does not take: the form says so, and the page stays. */
export async function refused(page: Page, username: string, password: string) {
  await signIn(page, username, password);
  await expect(page.locator("#signin-steps [role=alert]")).toHaveText(WRONG_SIGN_IN, { timeout: 60_000 });
  await expect(page).toHaveURL(/\/en\/signin$/);
}

/** One request to Central as the page sends it, with the page's own session. */
export function ask(page: Page, body: Json): Promise<{ status: number; answer: Json }> {
  return page.evaluate(async (asked) => {
    const got = await fetch("/api/v1", {
      method: "POST",
      headers: { "Content-Type": "application/json", "X-Baylee-Central": "1" },
      body: JSON.stringify(asked),
    });
    return { status: got.status, answer: got.status === 200 ? ((await got.json()) as Record<string, unknown>) : {} };
  }, body);
}

/** What Central says of the page's session: the account, its epoch, its tag, whether it has a recovery wrapper. */
export async function summary(page: Page): Promise<{ identity: Identity; tag: number; recovery: boolean }> {
  const { status, answer } = await ask(page, { operation: "account_summary" });
  expect(status).toBe(200);
  return { identity: answer["identity"] as Identity, tag: answer["tag"] as number, recovery: answer["recovery"] as boolean };
}

/** The operator appoints `name` (`--appoint-admin-stdin`, docs/operations.md §"Administration"). */
export function appoint(name: string) {
  const migrate = JSON.parse(readFileSync(process.env["REAL_MIGRATE"] ?? "", "utf8")) as { database: unknown };
  execFileSync(process.env["REAL_SERVICE"] ?? "", ["--appoint-admin-stdin"], {
    input: JSON.stringify({ database: migrate.database, username: name }),
    stdio: ["pipe", "ignore", "inherit"],
  });
}

/**
 * The administrator signed in on `admin` makes a reset code for `username`
 * on the new site's page (`/en/account/admin`, docs/account-reset.md
 * §"Issuing a reset code"), as a person does: the search, the account, the
 * warning, the password once more (`password`, the administrator's), and
 * the code shown once, copied, and gone when its step is left.
 */
export async function issue(admin: Page, username: string, password: string): Promise<string> {
  await open(admin, "/en/account/admin");
  const search = admin.locator("#admin-search");
  await expect(admin.locator(".admin-accounts, .admin-results [role=status]").first()).toBeVisible({ timeout: 30_000 });
  await search.fill(username);
  await search.press("Enter");
  const rows = admin.locator(".admin-accounts > li");
  const row = rows.filter({ has: admin.locator("dd code", { hasText: new RegExp(`^${username}$`) }) });
  await expect(row).toHaveCount(1, { timeout: 30_000 });
  await row.getByRole("button", { name: /^Open/ }).click();
  await expect(admin.locator("#admin-account-title")).toBeFocused({ timeout: 30_000 });
  await admin.getByRole("button", { name: "Reset login", exact: true }).click();
  await expect(admin.locator("#admin-warning-title")).toBeFocused();
  await admin.getByRole("button", { name: "I have checked who is asking", exact: true }).click();
  await admin.locator("#admin-password").fill(password);
  await admin.locator("#admin-password").press("Enter");
  await expect(admin.locator("#admin-code-title")).toBeFocused({ timeout: 60_000 });
  const steps = admin.locator("#admin-steps");
  const code = (await steps.locator(".secret code").textContent()) ?? "";
  expect(code).toMatch(RESET);
  await expect(steps).toContainText("Valid until ");
  // the code can be copied as it stands there (the clipboard is this test browser's own)
  await admin.context().grantPermissions(["clipboard-read", "clipboard-write"]);
  await steps.getByRole("button", { name: "Copy", exact: true }).click();
  await expect(steps.getByRole("status")).toHaveText("Copied.");
  expect(await admin.evaluate(() => navigator.clipboard.readText())).toBe(code);
  await steps.getByRole("button", { name: "Done", exact: true }).click();
  await expect(steps.locator(".secret")).toHaveCount(0);
  expect(await admin.content()).not.toContain(code);
  // the list shows the account with its code; Central's own record says 24 hours
  await expect(row.locator("dt").last()).toHaveText("Reset code");
  const listed = await ask(admin, { operation: "admin", request: { action: "accounts", query: username, before: null } });
  expect(listed.status).toBe(200);
  const found = (listed.answer["answer"] as { accounts: { username: string; reset_until: number | null }[] }).accounts;
  const until = found.find((account) => account.username === username)?.reset_until ?? 0;
  expect(until - Date.now() / 1000).toBeGreaterThan(23 * 3600);
  return code;
}

/**
 * One request to the native listener, as Caddy passes a client's on: with
 * this Host, and with the address it came from, here the one the front
 * names for the page (`front.ts`), so that Central counts the sign-ins of
 * one run at one place, as it does a person's at home.
 */
export function native(body: Json, bearer?: string): Promise<{ status: number; answer: Json }> {
  const data = JSON.stringify(body);
  const [host = "", port = ""] = NATIVE.split(":");
  return new Promise((done, fail) => {
    const headers: Record<string, string | number> = {
      host: "central.baylee.app",
      // The protocol this build speaks; Central answers any other with 426.
      "baylee-central-protocol": "2",
      "content-type": "application/json",
      "content-length": Buffer.byteLength(data),
      "x-forwarded-for": "127.0.0.1",
    };
    if (bearer !== undefined) headers["authorization"] = bearer;
    const post = request({ host, port: Number(port), method: "POST", path: "/v1/native", headers }, (answer) => {
      const chunks: Buffer[] = [];
      answer.on("data", (chunk: Buffer) => chunks.push(chunk));
      answer.on("end", () => {
        const status = answer.statusCode ?? 0;
        done({ status, answer: status === 200 ? (JSON.parse(Buffer.concat(chunks).toString("utf8")) as Json) : {} });
      });
    });
    post.on("error", fail);
    post.end(data);
  });
}

/** A client's session: its bearer, and what its sign-in found. */
export type Client = {
  /** The `Authorization` header of the session. */
  bearer: string;
  identity: Identity;
  /**
   * The public fingerprint of the chat identity, which is derived from the
   * account's master key: the same after a change means the same key, and
   * so the same vault.
   */
  device: string;
  /** The encrypted account document as Central stores it, and its revision. */
  document: string;
  revision: number;
};

let loaded = false;

/**
 * Signs in over the native listener as Baylee's client does, with the
 * committed module for the crypto; `null` when `password` is not the
 * account's (the module refuses Central's answer, and nothing more is sent).
 */
export async function client(username: string, password: string, tag: number): Promise<Client | null> {
  if (!loaded) {
    initSync({ module: readFileSync(resolve("../crates/central-web/crypto/central_web_crypto_bg.wasm")) });
    loaded = true;
  }
  const found = await native({ operation: "discover", username });
  expect(found.status).toBe(200);
  const discovery = found.answer["discovery"] as { kind: string; identity: Identity };
  expect(discovery.kind).toBe("central");
  const identity = discovery.identity;
  const starting = login_start(password, bytes(identity.account), BigInt(identity.epoch));
  const kx = await native({ operation: "login_start", identity, message: array(starting.message) });
  expect(kx.status).toBe(200);
  let key;
  try {
    key = login_finish(starting, bytes(kx.answer["message"]));
  } catch {
    return null;
  }
  const session = await native({ operation: "login_finish", attempt: kx.answer["attempt"], message: array(key.message) });
  expect(session.status).toBe(200);
  // opens the wrapper Central stores with the key this password made, or throws
  const chat = chat_unlock(key, bytes(session.answer["wrapped_master"]), tag);
  const device = chat.device;
  chat.free();
  // a signed Central token, which a client presents as it got it
  const bearer = `Bearer ${session.answer["token"] as string}`;
  const read = await native({ operation: "read_document" }, bearer);
  expect(read.status).toBe(200);
  return { bearer, identity, device, document: hex(read.answer["document"]), revision: read.answer["revision"] as number };
}

/** Whether Central still knows the client's session. */
export async function alive(session: Client): Promise<boolean> {
  const { status } = await native({ operation: "session_status" }, session.bearer);
  expect([200, 401]).toContain(status);
  return status === 200;
}

/** Every body `page` sends to Central from now on, for what must never be in one. */
export function sent(page: Page): string[] {
  const bodies: string[] = [];
  page.on("request", (asked) => {
    if (new URL(asked.url()).pathname.startsWith("/api/")) bodies.push(asked.postData() ?? "");
  });
  return bodies;
}

/** The operations among `bodies`, in order. */
export const operations = (bodies: string[]): string[] => bodies.map((body) => String((JSON.parse(body || "{}") as Json)["operation"]));

/**
 * Page errors and console errors of `page`, for a run that must have none.
 * A refusal the run asked Central for is an answer: the browser's own line
 * about its status is left out.
 */
export function problems(page: Page): string[] {
  const seen: string[] = [];
  page.on("pageerror", (error) => seen.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error" && !message.text().startsWith("Failed to load resource")) seen.push(message.text());
  });
  return seen;
}
