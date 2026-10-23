// An account moved from the official gateway, against a real Central in its
// deployed mode (README.md): the bridge reserves it, and Baylee's client's
// steps enrol, restore and close its move over the native listener, with the
// committed WebAssembly for the crypto and no recovery wrapper, as the client
// sends none. Then the page signs in, accepts Central's terms where they are
// in force, shows the recovery code the sign-in made, stores it once kept, and
// the account page says the account has one.
// REAL_SECRETS: the `--generate-secrets` document itself, in the environment
// (never a file or an argument), read for its bridge credential only and
// never printed; REAL_NATIVE: the native listener
// (127.0.0.1:47920); REAL_OUT: where each code is kept for the checks after
// (the stored wrapper opens with it to the account's master key).

import { readFileSync, writeFileSync } from "node:fs";
import { request } from "node:http";
import { join, resolve } from "node:path";

import { expect, test } from "@playwright/test";

import { enrollment_finish, enrollment_start, initSync, login_confirm, login_finish, login_start } from "../src/central-wasm/central_web_crypto.js";

const SECRETS = process.env["REAL_SECRETS"] ?? "";
const NATIVE = process.env["REAL_NATIVE"] ?? "127.0.0.1:47920";
const OUT = process.env["REAL_OUT"] ?? "";
const PASSWORD = "copper meadow lantern slowly";
const STAMP = String(Date.now() % 1_000_000);
const CODE = /^[0-9A-HJKMNP-TV-Z]{4}(-[0-9A-HJKMNP-TV-Z]{4}){7}$/;

type Json = Record<string, unknown>;

const hex = (bytes: unknown): string => Buffer.from(bytes as number[]).toString("hex");
const bytes = (v: unknown): Uint8Array => Uint8Array.from(v as number[]);
const array = (b: Uint8Array): number[] => Array.from(b);

/** One request to the native listener, as the client sends it (Caddy passes it on with this Host). */
function native(body: Json, bearer?: string): Promise<{ status: number; answer: Json }> {
  const data = JSON.stringify(body);
  const [host = "", port = ""] = NATIVE.split(":");
  return new Promise((done, fail) => {
    const headers: Record<string, string | number> = { host: "central.baylee.app", "content-type": "application/json", "content-length": Buffer.byteLength(data) };
    if (bearer !== undefined) headers["authorization"] = bearer;
    const sent = request({ host, port: Number(port), method: "POST", path: "/v1/native", headers }, (answer) => {
      const chunks: Buffer[] = [];
      answer.on("data", (chunk: Buffer) => chunks.push(chunk));
      answer.on("end", () => {
        const status = answer.statusCode ?? 0;
        done({ status, answer: status === 200 ? (JSON.parse(Buffer.concat(chunks).toString("utf8")) as Json) : {} });
      });
    });
    sent.on("error", fail);
    sent.end(data);
  });
}

/** The move as Baylee's client and the official gateway's bridge make it. */
async function move(name: string, legacy: string): Promise<void> {
  const bridge = `Bearer ${hex((JSON.parse(SECRETS) as Json)["bridge"])}`;
  const reserved = await native({ operation: "legacy_reserve", legacy_id: legacy, username: name }, bridge);
  expect(reserved.status).toBe(200);
  const identity = reserved.answer["identity"] as { account: number[]; epoch: number };
  const ticket = reserved.answer["ticket"];
  const account = bytes(identity.account);
  const epoch = BigInt(identity.epoch);

  const enrolling = enrollment_start(PASSWORD, account, epoch);
  const challenge = await native({ operation: "enrollment_start", ticket, message: array(enrolling.message) });
  expect(challenge.status).toBe(200);
  const enrolled = enrollment_finish(enrolling, bytes(challenge.answer["message"]));
  // a legacy enrollment carries no recovery wrapper
  const finished = await native({
    operation: "enrollment_finish",
    ticket,
    record: array(enrolled.record),
    wrapped_master: array(enrolled.wrapped_master),
    document: array(enrolled.document),
  });
  enrolled.free();
  expect(finished.status).toBe(200);

  const signingIn = login_start(PASSWORD, account, epoch);
  const kx = await native({ operation: "login_start", identity, message: array(signingIn.message) });
  expect(kx.status).toBe(200);
  const key = login_finish(signingIn, bytes(kx.answer["message"]));
  const session = await native({ operation: "login_finish", attempt: kx.answer["attempt"], message: array(key.message) });
  expect(session.status).toBe(200);
  login_confirm(key, bytes(session.answer["wrapped_master"]));
  const token = `Bearer ${hex(session.answer["token"])}`;
  expect((await native({ operation: "confirm_restore", revision: 1 }, token)).status).toBe(200);
  expect((await native({ operation: "legacy_close", legacy_id: legacy, identity }, bridge)).status).toBe(200);
  expect((await native({ operation: "logout" }, token)).status).toBe(200);
}

test("an account moved from the official gateway makes its recovery code on the website", async ({ page }, info) => {
  const name = `mv_${info.project.name}_${STAMP}`;
  const errors: string[] = [];
  page.on("console", (m) => m.type() === "error" && errors.push(m.text()));
  initSync({ module: readFileSync(resolve("src/central-wasm/central_web_crypto_bg.wasm")) });
  await move(name, `moved-${info.project.name}-${STAMP}`);

  await page.goto("/signin?lang=en");
  await page.getByLabel("Username").fill(name);
  await page.getByLabel("Password").fill(PASSWORD);
  await page.getByRole("button", { name: "Sign in" }).click();
  // Central's terms where in force, then the code the sign-in made
  const accept = page.getByRole("button", { name: "I accept" });
  const step = page.getByRole("region", { name: "Your recovery code" });
  await expect(accept.or(step)).toBeVisible({ timeout: 60_000 });
  if (await accept.isVisible()) await accept.click();
  await expect(step.getByText(/which has none yet/)).toBeVisible({ timeout: 30_000 });
  const code = (await step.getByLabel("Recovery code", { exact: true }).textContent()) ?? "";
  expect(code).toMatch(CODE);
  writeFileSync(join(OUT, `${name}.code`), code);
  await step.getByLabel("I have kept my recovery code in a safe place.").check();
  await step.getByRole("button", { name: "Continue" }).click();
  await expect(page).toHaveURL(/\/account$/, { timeout: 30_000 });
  await expect(page.getByText(/^You have a recovery code\./)).toBeVisible();
  await expect(page.getByText("This account moved here from the official gateway.")).toBeVisible();
  // a reload asks Central again, which says the same
  await page.reload();
  await expect(page.getByText(/^You have a recovery code\./)).toBeVisible({ timeout: 30_000 });
  expect(errors).toEqual([]);
});
