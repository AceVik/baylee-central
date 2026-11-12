// An account's e-mail address and a reset code by mail (docs/email.md) on
// the new site against a real Central with a mailer stand-in
// (e2e-real/mailer.ts): the registration names the address typed beside the
// recovery code with its own fresh sign-in and confirms it on the step
// after; the account area names another after proving the password again,
// the code Central mails confirms it, the first is told, and the reset page
// asks for a code by mail, which sets a new password with the recovery
// code. What came of it is read from outside the page as well: the mails
// the stand-in took, Central's answer to the page's own session, and a
// client's sign-in over the native listener.
//
// REAL_MAILER: the stand-in (default 127.0.0.1:47925), which Central's
// `mail` block names; REAL_NATIVE: the native listener. One run registers
// one account. Synthetic accounts and reserved addresses (example.org) only.

import { request } from "node:http";

import { expect, test, type Page } from "@playwright/test";

import {
  ask,
  client,
  open,
  operations,
  problems,
  refused,
  register,
  sent,
  signedIn,
  summary,
} from "./site.ts";

const MAILER = process.env["REAL_MAILER"] ?? "127.0.0.1:47925";
const stamp = String(Date.now() % 1_000_000);
const FIRST = "violet lantern harbour quietly";
const SECOND = "amber kettle window softly";
const SENT =
  "If this account has a confirmed e-mail address, a reset code is on its way to it. It is valid for one hour.";
const KEEP = "I have my recovery code. My data stays.";

type Mail = {
  to: string;
  subject: string;
  text: string;
  html: string;
  ttl_secs?: number;
  limit_key?: string;
  inline?: string[];
};

/** The mails the stand-in took for `to`, oldest first. */
function mails(to: string): Promise<Mail[]> {
  const [host = "", port = ""] = MAILER.split(":");
  return new Promise((done, fail) => {
    const asked = request(
      { host, port: Number(port), path: `/mails?to=${encodeURIComponent(to)}` },
      (answer) => {
        const chunks: Buffer[] = [];
        answer.on("data", (chunk: Buffer) => chunks.push(chunk));
        answer.on("end", () =>
          done(JSON.parse(Buffer.concat(chunks).toString("utf8")) as Mail[]),
        );
      },
    );
    asked.on("error", fail);
    asked.end();
  });
}

/** The newest of `to`'s mails once there are `count`. */
async function newest(to: string, count: number): Promise<Mail> {
  await expect
    .poll(async () => (await mails(to)).length, { timeout: 30_000 })
    .toBe(count);
  const all = await mails(to);
  return all[all.length - 1] as Mail;
}

/**
 * What a mail may hold: no image but the logo the mailer attaches, and that
 * only where Central's `mail` block has `"logo": true` (then the request
 * names it in `inline` and the HTML once as `cid:baylee-logo`); no script,
 * nothing from elsewhere, links to the site Central names (its `mail`
 * block's `site`, here baylee.app as the browser sees it): the button's
 * page and the start page. A code only in the fragment of a code mail's
 * button (`#code=` and that mail's code), never before it.
 */
function sound(mail: Mail, code: string) {
  const images = mail.html.match(/<img\b[^>]*>/gi) ?? [];
  if (mail.inline === undefined) {
    expect(images).toEqual([]);
  } else {
    expect(mail.inline).toEqual(["baylee-logo"]);
    expect(images).toHaveLength(1);
    expect(images[0]).toMatch(/^<img src="cid:baylee-logo" /);
  }
  const rest = mail.html.replace(/<img\b[^>]*>/gi, "");
  expect(rest).not.toMatch(/<img|<script|<iframe|<link|@import|url\(/i);
  const links = [...mail.html.matchAll(/href="([^"]*)"/g)].map(
    ([, href]) => href ?? "",
  );
  expect(links).toContain("https://baylee.app/en/");
  for (const href of links) {
    expect(href).toMatch(
      /^https:\/\/baylee\.app\/en\/((reset|account\/security)(#code=[0-9A-Z-]+)?)?$/,
    );
    const [before, fragment] = href.split("#");
    expect(before).not.toContain(code);
    if (fragment !== undefined) expect(fragment).toBe(`code=${code}`);
  }
  expect(mail.text).toContain(code);
  expect(mail.html).toContain(code);
}

const card = (page: Page) =>
  page.locator("section[aria-labelledby=email-title]");
const resetForm = (page: Page) => page.locator("#reset-steps");

test("an address is named, confirmed by the mailed code, and a mailed reset code sets a new password", async ({
  page,
  browser,
}, info) => {
  test.slow();
  const project = info.project.name.slice(0, 1);
  const username = `mail${project}${stamp}`;
  const address = `${username}@example.org`;
  const first = `${username}.first@example.org`;
  const seen = problems(page);
  const registering = sent(page);

  // An address typed beside the recovery code: the page names it after the
  // code and Central's terms, with the registration's own sign-in (no
  // password asked again), and its code is confirmed on the step after.
  const recovery = await register(
    page,
    username,
    `Mail${project}`,
    FIRST,
    first,
  );
  const step = card(page);
  await expect(step).toContainText(
    `We sent a code to ${first}. It is valid for 15 minutes.`,
    { timeout: 30_000 },
  );
  expect(
    operations(registering).filter((operation) => operation === "login_start"),
  ).toHaveLength(1);
  const welcome = await newest(first, 1);
  expect(welcome.ttl_secs).toBe(900);
  const firstCode =
    /([0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4})/.exec(welcome.text)?.[1] ??
    "";
  expect(firstCode).not.toBe("");
  sound(welcome, firstCode);
  await page
    .locator("#email-code")
    .fill(firstCode.startsWith("0") ? "1111-1111" : "0000-0000");
  await step.getByRole("button", { name: "Confirm", exact: true }).click();
  await expect(step.getByRole("alert")).toContainText(
    "That code is not right.",
  );
  await page.locator("#email-code").fill(firstCode);
  await step.getByRole("button", { name: "Confirm", exact: true }).click();
  await expect(step).toContainText("Your e-mail address is confirmed.", {
    timeout: 30_000,
  });
  await step
    .getByRole("button", { name: "Continue to your account", exact: true })
    .click();
  await expect(page).toHaveURL(/\/en\/account$/, { timeout: 30_000 });
  // Neither the password nor the recovery code was in a request.
  for (const body of registering) {
    expect(body).not.toContain(FIRST);
    expect(body).not.toContain(recovery);
  }

  // The account area shows it; another address is named there. A wrong
  // password names nothing and sends no mail.
  await open(page, "/en/account/security");
  await expect(
    card(page).getByRole("heading", { name: "E-mail address" }),
  ).toBeVisible();
  await expect(card(page)).toContainText(first);
  const bodies = sent(page);
  await card(page).getByRole("button", { name: "Change", exact: true }).click();
  await page.locator("#email-address").fill(address);
  await page.locator("#email-password").fill("not the password at all");
  await card(page)
    .getByRole("button", { name: "Send confirmation code", exact: true })
    .click();
  await expect(card(page).getByRole("alert")).toBeVisible({ timeout: 60_000 });
  expect(operations(bodies)).not.toContain("email");
  expect(await mails(address)).toEqual([]);

  // The password, proven again in the page's Worker, then the address.
  await page.locator("#email-password").fill(FIRST);
  await card(page)
    .getByRole("button", { name: "Send confirmation code", exact: true })
    .click();
  await expect(page.locator("#email-code")).toBeVisible({ timeout: 60_000 });
  await expect(card(page)).toContainText(
    `We sent a code to ${address}. It is valid for 15 minutes.`,
  );
  // No password, no address and no code leaves the page in a URL or a body
  // other than the one that names the address.
  for (const body of bodies) expect(body).not.toContain(FIRST);
  const confirm = await newest(address, 1);
  expect(confirm.ttl_secs).toBe(900);
  expect(confirm.limit_key).toMatch(/^account:[0-9a-f]{32}$/);
  const code =
    /([0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4})/.exec(confirm.text)?.[1] ??
    "";
  expect(code).not.toBe("");
  sound(confirm, code);

  // A wrong code is said, and the right one confirms the address.
  await page
    .locator("#email-code")
    .fill(code.startsWith("0") ? "1111-1111" : "0000-0000");
  await card(page)
    .getByRole("button", { name: "Confirm", exact: true })
    .click();
  await expect(card(page).getByRole("alert")).toContainText(
    "That code is not right.",
  );
  await page.locator("#email-code").fill(code.toLowerCase());
  await card(page)
    .getByRole("button", { name: "Confirm", exact: true })
    .click();
  await expect(card(page)).toContainText("Your e-mail address is confirmed.", {
    timeout: 30_000,
  });
  await expect(card(page)).toContainText(address);
  const status = await ask(page, {
    operation: "email",
    request: { action: "status" },
  });
  expect(status.status).toBe(200);
  const state = (status.answer["state"] ?? {}) as Record<string, unknown>;
  expect([state["mail"], state["address"], state["pending"]]).toEqual([
    true,
    address,
    null,
  ]);
  // The address it replaced is told so, with no code and no time to live.
  const replaced = await newest(first, 2);
  expect(replaced.subject).toBe("Your e-mail address on Baylee was replaced");
  expect(replaced.ttl_secs).toBeUndefined();
  expect(replaced.text).not.toContain(code);
  expect(replaced.text).not.toContain(firstCode);
  // the notice links to the security page and, in its footer, the site
  expect(
    [...replaced.html.matchAll(/href="([^"]*)"/g)].map(([, href]) => href),
  ).toEqual(["https://baylee.app/en/account/security", "https://baylee.app/en/"]);

  // Signed out, someone asks the reset page for a code by mail. A name that
  // is nobody's is answered alike and gets no mail.
  const other = await browser.newContext();
  const stranger = await other.newPage();
  const asked = sent(stranger);
  await open(stranger, "/en/reset");
  const block = stranger.locator("section[aria-labelledby=reset-mail-title]");
  for (const name of [`nobody${stamp}`, username]) {
    await stranger.locator("#reset-mail-name").fill(name);
    // a person takes longer than three seconds; the page refuses faster ones
    await stranger.waitForTimeout(3_000);
    await block.getByRole("button", { name: "Send a code by e-mail" }).click();
    await expect(block).toContainText(SENT, { timeout: 30_000 });
  }
  expect(
    operations(asked).filter((operation) => operation === "reset_mail"),
  ).toHaveLength(2);
  const reset = await newest(address, 2);
  expect(reset.ttl_secs).toBe(3_600);
  expect(reset.text).toContain(username);
  const resetCode =
    /(RESET(?:-[0-9A-HJKMNP-TV-Z]{4}){8})/.exec(reset.text)?.[1] ?? "";
  expect(resetCode).not.toBe("");
  sound(reset, resetCode);
  expect(await mails(`nobody${stamp}@example.org`)).toEqual([]);

  // The mailed code sets a new password with the recovery code: the data stays.
  await expect(resetForm(stranger).getByLabel("Sign-in name")).toHaveValue(
    username,
  );
  await resetForm(stranger).getByLabel("Reset code").fill(resetCode);
  await expect(stranger.getByRole("radio", { name: KEEP })).toBeChecked();
  await stranger.getByLabel("Recovery code", { exact: true }).fill(recovery);
  await stranger.getByLabel("New password", { exact: true }).fill(SECOND);
  await stranger.getByLabel("Repeat new password").fill(SECOND);
  await stranger.getByRole("button", { name: "Set new password" }).click();
  await expect(
    stranger.getByRole("heading", { name: "Your new password is set" }),
  ).toBeVisible({ timeout: 90_000 });
  await other.close();

  // The old password is refused, the new one signs in, and the address stays.
  const fresh = await browser.newContext();
  const later = await fresh.newPage();
  await refused(later, username, FIRST);
  await signedIn(later, username, SECOND);
  const after = await ask(later, {
    operation: "email",
    request: { action: "status" },
  });
  expect(
    ((after.answer["state"] ?? {}) as Record<string, unknown>)["address"],
  ).toBe(address);
  // A client signs in with the new password over the native listener.
  expect(
    await client(username, SECOND, (await summary(later)).tag),
  ).not.toBeNull();
  await fresh.close();
  expect(seen).toEqual([]);
});
