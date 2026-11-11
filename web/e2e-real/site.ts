// Steps the new site's specs share against a real Central (README.md §"The
// new site"): registering and signing in on the page, asking Central with
// the page's own session, the operator's appointment of an administrator,
// and a sign-in over the native listener as Baylee's client makes it, with
// the committed WebAssembly. Synthetic accounts only; no password, code or
// token is printed or written anywhere.

import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { request } from "node:http";
import { resolve } from "node:path";

import { expect, type Page } from "@playwright/test";

import { chat_unlock, initSync, login_finish, login_start } from "../src/central-wasm/central_web_crypto.js";

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

/** Registers on the site's page and answers the recovery code it showed once. */
export async function register(page: Page, username: string, display: string, password: string): Promise<string> {
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
  await page.getByLabel("I have saved my recovery code").check();
  await page.getByRole("button", { name: "Continue" }).click();
  // where Central's terms are in force, they come after the code
  const terms = page.getByRole("heading", { name: "New terms of use" });
  await expect(terms.or(page.locator(".account-who"))).toBeVisible({ timeout: 30_000 });
  if (await terms.isVisible()) await page.getByRole("button", { name: "Accept", exact: true }).click();
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
 * (`AdminRequest::ResetLogin`, docs/account-reset.md §"Issuing a reset
 * code"). The new site has no page for it yet, so the request goes out as
 * the page would send it, with the administrator's session.
 */
export async function issue(admin: Page, username: string): Promise<string> {
  const listed = await ask(admin, { operation: "admin", request: { action: "accounts", query: username, before: null } });
  expect(listed.status).toBe(200);
  const rows = (listed.answer["answer"] as { accounts: { id: number[]; username: string }[] }).accounts;
  const row = rows.find((account) => account.username === username);
  expect(row).toBeDefined();
  const made = await ask(admin, { operation: "admin", request: { action: "reset_login", account: row?.id } });
  expect(made.status).toBe(200);
  const answer = made.answer["answer"] as { answer: string; code: string; expires_at: number };
  expect(answer.answer).toBe("reset_code");
  expect(answer.code).toMatch(RESET);
  // valid for 24 hours
  expect(answer.expires_at - Date.now() / 1000).toBeGreaterThan(23 * 3600);
  return answer.code;
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
    initSync({ module: readFileSync(resolve("src/central-wasm/central_web_crypto_bg.wasm")) });
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
  const bearer = `Bearer ${hex(session.answer["token"])}`;
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
