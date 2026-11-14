import { expect, type Locator, type Page, type Route, test } from "@playwright/test";

import { narrow, schemes, targets } from "./check.ts";

// An account's optional e-mail address in the account area
// (crates/central-web/src/islands/email.rs) in a real browser, against a
// stand-in for Central's browser surface that keeps an address as
// central_api::email describes it. All of it is synthetic: reserved
// domains, a code that is none of Central's.
//
// Naming an address signs in once more. The stand-in cannot answer OPAQUE
// truthfully, so that sign-in runs in a canned Worker here; the real one
// refusing the stand-in's challenge is the wrong password's path (the last
// test), and an address named against a Central that holds the account is
// e2e-real's.

const ACCOUNT = [1, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 16];
const PASSWORD = "violet lantern harbour";
/** The code the stand-in "mailed". */
const CODE = "000G-40R4";
const THEN = 1_794_000_000;
const OLD = "old@example.org";
const NEW = "player@example.org";
/** As long as an address gets (254), and without a place to break. */
const LONG = `${"a".repeat(64)}@${"b".repeat(63)}.${"c".repeat(63)}.${"d".repeat(53)}.example`;

type Held = { mail: boolean; address: string | null; confirmed_at: number | null; pending: string | null; pending_until: number | null };
type Action = "status" | "set" | "resend" | "confirm" | "withdraw" | "remove";
type Asked = { action?: string; address?: string; lang?: string; code?: string };
type Body = { operation?: string; request?: Asked } & Record<string, unknown>;

type World = {
  held: Held;
  /** Wrong codes taken for the address that waits; the third gives it up. */
  wrong: number;
  /** Central no longer knows the session. */
  gone: boolean;
  /** Central knows the session and refuses every question about addresses. */
  deaf: boolean;
  /** How the next request of an action ends instead: a status, or no answer after it was carried out. */
  next: Partial<Record<Action, number | "lost">>;
};

const world = (held: Partial<Held> = {}): World => ({
  held: { mail: true, address: null, confirmed_at: null, pending: null, pending_until: null, ...held },
  wrong: 0,
  gone: false,
  deaf: false,
  next: {},
});
const confirmed = (address = OLD): Partial<Held> => ({ address, confirmed_at: THEN });
const waiting = (pending = NEW): Partial<Held> => ({ pending, pending_until: THEN + 900 });

/** What Central does with an address, as far as the page can tell. */
function email(w: World, asked: Asked): number {
  const held = w.held;
  switch (asked.action) {
    case "status":
      return 200;
    case "set":
      if (!held.mail) return 401;
      if (asked.address === held.address) return 409;
      Object.assign(held, waiting(asked.address));
      w.wrong = 0;
      return 200;
    case "resend":
      if (!held.mail || held.pending === null) return 401;
      held.pending_until = THEN + 1800;
      return 200;
    case "confirm":
      if (!held.mail || held.pending === null) return 401;
      if (asked.code !== CODE) {
        w.wrong += 1;
        if (w.wrong >= 3) Object.assign(held, { pending: null, pending_until: null });
        return 401;
      }
      Object.assign(held, { address: held.pending, confirmed_at: THEN + 60, pending: null, pending_until: null });
      return 200;
    case "withdraw":
      Object.assign(held, { pending: null, pending_until: null });
      return 200;
    case "remove":
      Object.assign(held, { address: null, confirmed_at: null, pending: null, pending_until: null });
      return 200;
    default:
      return 401;
  }
}

function central(seen: Body[], w: World) {
  return (route: Route) => {
    const request = route.request();
    expect(request.headers()["x-baylee-central"]).toBe("1");
    // nothing a person typed travels in an address
    expect(new URL(request.url()).search).toBe("");
    const body = JSON.parse(request.postData() ?? "{}") as Body;
    seen.push(body);
    const json = (value: unknown) => route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(value) });
    const refused = (status: number) => route.fulfill({ status, body: "account request rejected\n" });
    if (w.gone) return refused(401);
    switch (body.operation) {
      case "account_summary":
        return json({
          result: "account_summary", identity: { account: ACCOUNT, epoch: 1 }, username: "erika", display_name: "Erika", tag: 0x1a2b,
          source: "registered", recovery: true, admin: false, signed_in_at: THEN, expires_at: 4_102_444_800,
        });
      case "owner":
        if (body.request?.action !== "sessions") return refused(401);
        return json({
          result: "owner",
          answer: { answer: "sessions", sessions: [{ kind: "browser", current: true, signed_in_at: THEN, expires_at: 4_102_444_800, gateways: [] }] },
        });
      case "login_start":
        return json({ result: "login_challenge", attempt: Array(32).fill(7), message: Array.from({ length: 320 }, (_, i) => (i * 37) % 251) });
      case "login_finish":
        return json({
          result: "browser_session", identity: { account: ACCOUNT, epoch: 1 }, username: "erika",
          wrapped_master: Array(73).fill(9), expires_at: 4_102_444_800,
        });
      case "account_export":
        return json({ result: "account_export", export: EXPORT });
      case "challenges":
        if (body.request?.action === "tower") return json({ result: "challenges", response: { answer: "tower", response: { answer: "export", runs: [], after: null } } });
        return body.request?.action === "export"
          ? json({ result: "challenges", response: { answer: "export", entries: [], after: null } })
          : refused(401);
      case "email": {
        if (w.deaf) return refused(401);
        const asked = body.request ?? {};
        const action = asked.action as Action | undefined;
        const instead = action && w.next[action];
        if (action) delete w.next[action];
        if (typeof instead === "number") return refused(instead);
        const status = email(w, asked);
        // carried out, and the answer never arrives
        if (instead === "lost") return route.abort();
        return status === 200 ? json({ result: "email", state: w.held }) : refused(status);
      }
      default:
        return refused(401);
    }
  };
}

const EXPORT = {
  account: ACCOUNT, username: "erika", display_name: "Erika", display_name_at: THEN, tag: 0x1a2b, source: "registered", legacy_id: null,
  created_at: THEN, last_sign_in_at: THEN, vault_epoch: 1, document_revision: 1, state: "enrolled", admin: false, recovery: true,
  reset_until: null, objects: 0, object_bytes: 0, archives: 0, sessions: [], terms: [], gateways: [], administers: [], audit: [], complete: true,
};
const CHALLENGES = {
  participation: { entries: [], complete: true, after: null },
  attempts: { entries: [], complete: true, after: null },
};

/** The sign-in once more, canned: the stand-in's OPAQUE answers are none. */
const WORKER = `
self.addEventListener("message", (event) => {
  const m = JSON.parse(event.data);
  const ok = (value) => self.postMessage(JSON.stringify({ id: m.id, ok: value }));
  switch (m.op) {
    case "login_start": return ok({ message: Array(96).fill(1) });
    case "login_finish": return ok({ message: Array(64).fill(2) });
    default: return self.postMessage(JSON.stringify({ id: m.id, error: "failed" }));
  }
});`;

type Opened = { seen: Body[]; problems: string[] };

async function open(page: Page, w: World, options: { path?: string; canned?: boolean } = {}): Promise<Opened> {
  const seen: Body[] = [];
  const problems: string[] = [];
  page.on("pageerror", (error) => problems.push(error.message));
  await page.addInitScript(() => {
    // once: a later look must see what the page itself did to it
    if (!sessionStorage.getItem("seeded")) {
      sessionStorage.setItem("seeded", "1");
      localStorage.setItem("baylee-central-session", JSON.stringify({ expiresAt: 4_102_444_800 }));
    }
  });
  await page.route("**/api/v1", central(seen, w));
  if (options.canned ?? true) {
    await page.route("**/assets/crypto-worker.*.js", (route) => route.fulfill({ contentType: "text/javascript", body: WORKER }));
  }
  await page.goto(options.path ?? "/en/account/security");
  await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
  return { seen, problems };
}

// a route does not see what a service worker fetches: the canned Worker
test.use({ serviceWorkers: "block" });

const card = (page: Page) => page.locator("section[aria-labelledby=email-title]");
const address = (page: Page) => page.locator("#email-address");
const password = (page: Page) => page.locator("#email-password");
const code = (page: Page) => page.locator("#email-code");
const alert = (page: Page) => card(page).getByRole("alert");
const said = (page: Page) => page.locator("#email-said");
const button = (page: Page, name: string) => card(page).getByRole("button", { name, exact: true });
/** What was asked about the address since `from`. */
const actions = (seen: Body[], from = 0) => seen.slice(from).filter((body) => body.operation === "email").map((body) => body.request?.action);
const operations = (seen: Body[], from: number) => seen.slice(from).map((body) => body.operation);

/** The page as it stands passes the checks every page passes: axe in both schemes, the targets, 320 px. */
async function sound(page: Page) {
  expect(await schemes(page)).toEqual([]);
  expect(await targets(page)).toEqual([]);
  expect(await narrow(page)).toBe(0);
}

/** Fills the form that names an address and sends it. */
async function send(page: Page, typed: string, secret = PASSWORD) {
  await address(page).fill(typed);
  await password(page).fill(secret);
  await button(page, "Send confirmation code").click();
}

/** Tab leads through `stops` in this order, starting behind `from`. */
async function tabs(page: Page, from: Locator, stops: Locator[]) {
  await from.focus();
  for (const stop of stops) {
    await page.keyboard.press("Tab");
    await expect(stop).toBeFocused();
  }
}

test("without an address the section explains itself, and the form sends nothing before it is complete", async ({ page }) => {
  const { seen, problems } = await open(page, world());
  const section = card(page);
  await expect(section.getByRole("heading", { name: "E-mail address" })).toBeVisible();
  // below the password's change
  await expect(page.locator("section[aria-labelledby=password-title] + section")).toHaveAttribute("aria-labelledby", "email-title");
  await expect(address(page)).toBeVisible();
  await expect(section).toContainText("An e-mail address is optional.");
  await expect(section.locator(".notice")).toHaveText(
    "Whoever can read this mailbox can reset your sign-in. Your encrypted data survives a reset only with your recovery code.",
  );
  await expect(section.locator("label[for=email-address]")).toHaveText("E-mail address");
  await expect(address(page)).toHaveAttribute("type", "email");
  await expect(address(page)).toHaveAttribute("autocomplete", "email");
  await expect(section.locator("label[for=email-password]")).toHaveText("Your password, to confirm");
  await expect(password(page)).toHaveAttribute("type", "password");
  await expect(password(page)).toHaveAttribute("autocomplete", "current-password");
  // no field has a name: nothing here is a form a browser could send
  await expect(section.locator("input[name]")).toHaveCount(0);
  await expect(section.locator("input")).toHaveCount(3);
  // a password manager learns whose password it is, and no person sees it
  const whose = section.locator("input[autocomplete=username]");
  await expect(whose).toHaveValue("erika");
  await expect(whose).toBeHidden();
  await sound(page);
  await tabs(page, address(page), [password(page), section.getByRole("button", { name: "Show the password" }), button(page, "Send confirmation code")]);

  const asked = seen.length;
  await button(page, "Send confirmation code").click();
  await expect(alert(page)).toHaveText("Enter an e-mail address and your password.");
  await address(page).fill(NEW);
  await button(page, "Send confirmation code").click();
  await expect(alert(page)).toHaveText("Enter an e-mail address and your password.");
  const unwritable = "We cannot write to this address. Enter one plain address such as name@example.org; accented letters and umlauts do not work.";
  // (an umlaut in the domain is not among them: Chrome hands the page its
  // punycode, which is an address like any other)
  for (const bad of ["erika", "erika@localhost", "jürgen@example.org", "Erika <erika@example.org>"]) {
    await send(page, bad);
    await expect(alert(page)).toHaveText(unwritable);
    // the password is still there to send with the corrected address
    await expect(password(page)).toHaveValue(PASSWORD);
  }
  expect(seen.slice(asked)).toEqual([]);
  await sound(page);
  expect(problems).toEqual([]);
});

test("naming an address signs in once more, and the mailed code confirms it", async ({ page }) => {
  const w = world();
  const { seen, problems } = await open(page, w);
  await expect(address(page)).toBeVisible();
  const asked = seen.length;
  await send(page, NEW);
  await expect(card(page)).toContainText(`We sent a code to ${NEW}. It is valid for 15 minutes.`);
  // the sign-in first, with the session's own account; the address after it
  expect(operations(seen, asked).filter((operation) => operation !== "owner")).toEqual(["account_summary", "login_start", "login_finish", "email"]);
  const start = seen[asked + 1] as { identity: unknown; message: number[] };
  expect(start.identity).toEqual({ account: ACCOUNT, epoch: 1 });
  expect(seen.findLast((body) => body.operation === "email")).toEqual({ operation: "email", request: { action: "set", address: NEW, lang: "en" } });
  // the session is a newer one: the list beside the section is read again
  await expect.poll(() => seen.findLastIndex((body) => body.operation === "owner")).toBeGreaterThan(asked + 2);
  // the keyboard goes on at the code
  await expect(code(page)).toBeFocused();
  await expect(code(page)).toHaveAttribute("autocomplete", "one-time-code");
  await expect(code(page)).toHaveAttribute("inputmode", "text");
  await expect(code(page)).toHaveAttribute("type", "text");
  await expect(code(page)).toHaveAccessibleDescription("Eight characters, as in XXXX-XXXX. Upper or lower case, with or without the hyphen.");
  await expect(card(page).locator("input[name]")).toHaveCount(0);
  await expect(card(page).locator("code")).toHaveAttribute("translate", "no");
  await sound(page);
  await tabs(page, code(page), [button(page, "Confirm"), button(page, "Send again"), button(page, "Use another address")]);

  // typed as a person types it; shown in capitals, sent as it was mailed
  await code(page).fill(" 000g 40r4");
  expect(await code(page).evaluate((field) => getComputedStyle(field).textTransform)).toBe("uppercase");
  await code(page).press("Enter");
  await expect(said(page)).toHaveText("Your e-mail address is confirmed.");
  await expect(said(page)).toBeFocused();
  expect(seen.at(-1)).toEqual({ operation: "email", request: { action: "confirm", code: CODE } });
  const facts = card(page).locator("dl.facts");
  await expect(facts.locator("dt")).toHaveText(["Your address", "Confirmed"]);
  await expect(facts.locator("dd").first()).toHaveText(NEW);
  await expect(facts.locator("dd").first()).toHaveAttribute("translate", "no");
  await expect(facts.locator("dd").last()).toContainText("2026");
  await expect(code(page)).toHaveCount(0);
  await expect(address(page)).toHaveCount(0);
  await sound(page);
  await tabs(page, button(page, "Change"), [button(page, "Remove")]);
  // the password went to the Worker and nowhere else
  expect(JSON.stringify(seen)).not.toContain("lantern");
  expect(problems).toEqual([]);
});

test("a wrong code is said and the state read again; after several the address is named anew", async ({ page }) => {
  const w = world(waiting());
  const { seen } = await open(page, w);
  await expect(code(page)).toBeVisible();
  const asked = seen.length;
  // what is no code at all is not sent, and is no wrong code either
  for (const none of ["", "000G-40R", "RESET-000G-40R4", "000G-40RU"]) {
    await code(page).fill(none);
    await button(page, "Confirm").click();
    await expect(alert(page)).toHaveText("That is not a confirmation code. It has eight characters, as in XXXX-XXXX.");
  }
  expect(seen.slice(asked)).toEqual([]);

  const wrong = "That code is not right. Check it and try again. After several wrong codes you have to enter the address again.";
  await code(page).fill("zzzz-zzzz");
  await button(page, "Confirm").click();
  await expect(alert(page)).toHaveText(wrong);
  expect(actions(seen, asked)).toEqual(["confirm", "status"]);
  // to type it again: the field has the keyboard and still what was typed
  await expect(code(page)).toBeFocused();
  await expect(code(page)).toHaveValue("zzzz-zzzz");
  await sound(page);

  await code(page).press("Enter");
  await expect.poll(() => actions(seen, asked)).toEqual(["confirm", "status", "confirm", "status"]);
  // the button is the page's again once the state was read
  await expect(button(page, "Confirm")).not.toHaveAttribute("aria-disabled", "true");
  await expect(code(page)).toBeVisible();
  // the third: Central gave the address up, the page shows what it holds
  // and says that the address has to be entered again, not "try again"
  await code(page).press("Enter");
  await expect(address(page)).toBeFocused();
  await expect(alert(page)).toHaveText(
    "That code was not right either, so the address no longer waits. Enter it again to get a new code.",
  );
  await expect(code(page)).toHaveCount(0);
  expect(actions(seen, asked)).toEqual(["confirm", "status", "confirm", "status", "confirm", "status"]);
  expect(await page.evaluate(() => localStorage.getItem("baylee-central-session"))).not.toBeNull();
  await sound(page);
});

test("a code is sent again, too many mails are said, and another address takes the waiting one's place", async ({ page }) => {
  const w = world(waiting());
  const { seen } = await open(page, w);
  await expect(code(page)).toBeVisible();
  let asked = seen.length;
  await code(page).fill("000g");
  await button(page, "Send again").click();
  await expect(said(page)).toHaveText("A new code is on its way. The earlier one no longer works.");
  expect(seen.slice(asked)).toEqual([{ operation: "email", request: { action: "resend", lang: "en" } }]);
  // the code that was begun is the earlier one's
  await expect(code(page)).toBeFocused();
  await expect(code(page)).toHaveValue("");

  asked = seen.length;
  w.next.resend = 429;
  await button(page, "Send again").click();
  await expect(alert(page)).toHaveText("Too many mails for now. Try again later.");
  // what was said before is over
  await expect(said(page)).toHaveText("");
  // nothing was sent and nothing new waits: there is nothing to read again
  expect(actions(seen, asked)).toEqual(["resend"]);
  await expect(button(page, "Send again")).toBeFocused();
  await sound(page);

  asked = seen.length;
  await button(page, "Use another address").click();
  await expect(said(page)).toHaveText("The address is withdrawn. No code is waited for any more.");
  expect(actions(seen, asked)).toEqual(["withdraw"]);
  await expect(address(page)).toBeFocused();
  await expect(alert(page)).toBeHidden();
  await expect(code(page)).toHaveCount(0);
  // nothing is confirmed, so there is nothing to keep
  await expect(button(page, "Keep the current address")).toHaveCount(0);
  await sound(page);
});

test("a confirmed address stays until another is confirmed, and the same address is no change", async ({ page }) => {
  const w = world(confirmed());
  const { seen } = await open(page, w);
  const facts = card(page).locator("dl.facts");
  await expect(facts.locator("dd").first()).toHaveText(OLD);
  await expect(address(page)).toHaveCount(0);
  await sound(page);

  await button(page, "Change").click();
  await expect(address(page)).toBeFocused();
  await expect(card(page)).toContainText("Your current address stays in use until you have confirmed the new one.");
  await expect(facts.locator("dd").first()).toHaveText(OLD);
  await sound(page);
  await button(page, "Keep the current address").click();
  await expect(button(page, "Change")).toBeFocused();
  await expect(address(page)).toHaveCount(0);

  await button(page, "Change").click();
  let asked = seen.length;
  await send(page, OLD);
  await expect(alert(page)).toHaveText("This is your address already.");
  expect(operations(seen, asked).filter((operation) => operation !== "owner")).toEqual(["account_summary", "login_start", "login_finish", "email"]);
  // the form is back as it was left, with the keyboard at the address
  await expect(address(page)).toBeFocused();
  await expect(address(page)).toHaveValue(OLD);
  await expect(password(page)).toHaveValue("");
  await expect(button(page, "Keep the current address")).toBeVisible();

  await send(page, NEW);
  await expect(code(page)).toBeFocused();
  await expect(alert(page)).toBeHidden();
  await expect(card(page)).toContainText(`We sent a code to ${NEW}. It is valid for 15 minutes.`);
  await expect(card(page)).toContainText(`Until you confirm it, ${OLD} stays your address.`);
  expect(w.held).toMatchObject({ address: OLD, pending: NEW });
  await sound(page);

  // withdrawn: the confirmed one is still there, and so is the way back to it
  asked = seen.length;
  await button(page, "Use another address").click();
  await expect(address(page)).toBeFocused();
  expect(actions(seen, asked)).toEqual(["withdraw"]);
  await expect(facts.locator("dd").first()).toHaveText(OLD);
  await button(page, "Keep the current address").click();
  await expect(button(page, "Change")).toBeFocused();
  expect(JSON.stringify(seen)).not.toContain("lantern");
});

test("removing asks first and says what it means", async ({ page }) => {
  const w = world(confirmed());
  const { seen } = await open(page, w);
  await expect(button(page, "Remove")).toBeVisible();
  const asked = seen.length;
  await button(page, "Remove").click();
  const question = page.locator("#email-remove-ask");
  await expect(question).toHaveText(
    "Remove this address? We tell the address that it was removed. After that you can no longer get a reset code by e-mail.",
  );
  // the question has the keyboard, not the button that answers it
  await expect(question).toBeFocused();
  await expect(card(page).locator("dl.facts dd").first()).toHaveText(OLD);
  await sound(page);
  await tabs(page, question, [button(page, "Remove address"), button(page, "Keep it")]);
  await button(page, "Keep it").click();
  await expect(button(page, "Remove")).toBeFocused();
  expect(seen.slice(asked)).toEqual([]);

  await button(page, "Remove").click();
  await button(page, "Remove address").click();
  await expect(said(page)).toHaveText("Your e-mail address is removed.");
  await expect(said(page)).toBeFocused();
  expect(seen.slice(asked)).toEqual([{ operation: "email", request: { action: "remove" } }]);
  // an address can be named again
  await expect(address(page)).toBeVisible();
  await expect(card(page).locator("dl.facts")).toHaveCount(0);
  await sound(page);
});

test("where Central sends no mail there is no form, and what is stored can still go", async ({ page }) => {
  const w = world({ mail: false, ...confirmed(), ...waiting() });
  const { seen } = await open(page, w);
  const section = card(page);
  await expect(section).toContainText("This server sends no mail, so an e-mail address cannot be set or confirmed here.");
  await expect(section.locator("dl.facts dt")).toHaveText(["Your address", "Confirmed", "Waits for its code"]);
  await expect(section.locator("dl.facts dd").last()).toHaveText(NEW);
  await expect(section.locator("input")).toHaveCount(0);
  await expect(button(page, "Change")).toHaveCount(0);
  await sound(page);

  const asked = seen.length;
  await button(page, "Withdraw").click();
  await expect(said(page)).toHaveText("The address is withdrawn. No code is waited for any more.");
  await expect(said(page)).toBeFocused();
  await expect(section.locator("dl.facts dt")).toHaveText(["Your address", "Confirmed"]);
  await expect(button(page, "Withdraw")).toHaveCount(0);

  await button(page, "Remove").click();
  // no mail goes out here, so the question does not say that one does
  await expect(page.locator("#email-remove-ask")).toHaveText("Remove this address? While this server sends no mail, you cannot set one again.");
  await expect(page.locator("#email-remove-ask")).toBeFocused();
  await sound(page);
  await button(page, "Remove address").click();
  await expect(said(page)).toHaveText("Your e-mail address is removed.");
  await expect(said(page)).toBeFocused();
  expect(actions(seen, asked)).toEqual(["withdraw", "remove"]);
  await expect(section.locator("input, button")).toHaveCount(0);
  await expect(section).toContainText("This server sends no mail");
  await sound(page);
});

test("an answer that does not arrive is no refusal: the page says so and reads what Central holds", async ({ page }) => {
  const w = world(waiting());
  const { seen } = await open(page, w);
  await expect(code(page)).toBeVisible();
  const unsure = "No answer came back, so we cannot tell whether that went through. Look at what is shown here, and try again if it did not.";

  // Central is unavailable: nothing is known, and what it holds is the same
  let asked = seen.length;
  w.next.resend = 503;
  await button(page, "Send again").click();
  await expect(alert(page)).toHaveText(unsure);
  expect(actions(seen, asked)).toEqual(["resend", "status"]);
  await expect(button(page, "Send again")).toBeFocused();
  await expect(code(page)).toBeVisible();

  // the confirmation went through and its answer was lost on the way
  asked = seen.length;
  w.next.confirm = "lost";
  await code(page).fill(CODE);
  await button(page, "Confirm").click();
  await expect(alert(page)).toHaveText(unsure);
  expect(actions(seen, asked)).toEqual(["confirm", "status"]);
  await expect(card(page).locator("dl.facts dd").first()).toHaveText(NEW);
  await expect(code(page)).toHaveCount(0);
  // what was said takes the keyboard, since the field it was in is gone
  await expect(page.locator("#email-error")).toBeFocused();
  await sound(page);
});

test("a session that is gone is said, and a Central that keeps no addresses signs nobody out", async ({ page }) => {
  const w = world(waiting());
  w.deaf = true;
  const { seen } = await open(page, w);
  const section = card(page);
  // refused, and the session is alive: the rest of the page is as it was
  await expect(section).toContainText("The e-mail address cannot be shown right now.");
  await expect(page.locator("#password-current")).toBeVisible();
  await expect(page.locator(".sessions li")).toHaveCount(1);
  expect(await page.evaluate(() => localStorage.getItem("baylee-central-session"))).not.toBeNull();
  const first = seen.findIndex((body) => body.operation === "email");
  expect(seen.slice(first + 1).map((body) => body.operation)).toContain("account_summary");
  await sound(page);

  w.deaf = false;
  await button(page, "Try again").click();
  await expect(code(page)).toBeVisible();

  // now the session ends: the refusal of a code is read as what it is
  w.gone = true;
  const asked = seen.length;
  await code(page).fill(CODE);
  await button(page, "Confirm").click();
  await expect(page.getByText("You are not signed in.")).toBeVisible();
  // the code's refusal, the state's, and only then the question that settles it
  expect(operations(seen, asked).slice(0, 3)).toEqual(["email", "email", "account_summary"]);
  await expect(section).toHaveCount(0);
  expect(await page.evaluate(() => localStorage.getItem("baylee-central-session"))).toBeNull();
});

test("Central not answering at all is said in the section, with a way to try again", async ({ page }) => {
  const w = world();
  w.next.status = 503;
  await open(page, w);
  await expect(alert(page)).toHaveText("Baylee Central is not answering right now. Try again in a moment.");
  await expect(address(page)).toHaveCount(0);
  await sound(page);
  await button(page, "Try again").click();
  await expect(address(page)).toBeVisible();
});

test("German and an address as long as one gets hold at 320 px", async ({ page }) => {
  const w = world({ ...confirmed(LONG), ...waiting(LONG.replace("a", "e")) });
  const { seen } = await open(page, w, { path: "/de/account/security" });
  const section = card(page);
  await expect(section.getByRole("heading", { name: "E-Mail-Adresse" })).toBeVisible();
  await expect(section).toContainText("geschickt. Er gilt 15 Minuten.");
  await expect(section.locator("code")).toHaveCount(2);
  await expect(page.locator("label[for=email-code]")).toHaveText("Bestätigungscode");
  await sound(page);

  // the mail is asked for in the page's language
  const asked = seen.length;
  await button(page, "Noch einmal schicken").click();
  await expect(said(page)).toHaveText("Ein neuer Code ist unterwegs. Der frühere gilt nicht mehr.");
  expect(seen.slice(asked)).toEqual([{ operation: "email", request: { action: "resend", lang: "de" } }]);

  await button(page, "Andere Adresse verwenden").click();
  await expect(address(page)).toBeFocused();
  await expect(section.locator(".notice")).toHaveText(
    "Wer dieses Postfach lesen kann, kann deine Anmeldung zurücksetzen. Deine verschlüsselten Daten überstehen einen Reset nur mit deinem Wiederherstellungscode.",
  );
  await expect(section.locator("label[for=email-password]")).toHaveText("Dein Passwort, zur Bestätigung");
  await sound(page);
  await button(page, "Bisherige Adresse behalten").click();
  await expect(section.locator("dl.facts dd").first()).toHaveText(LONG);
  await sound(page);
  await button(page, "Entfernen").click();
  await expect(page.locator("#email-remove-ask")).toBeFocused();
  await sound(page);
});

test("the export's file holds the address beside the export, or the page says that it does not", async ({ page }) => {
  const w = world({ ...confirmed(), ...waiting() });
  const { seen } = await open(page, w, { path: "/en/account/privacy" });
  const file = async () => {
    const href = await page.locator("#export-save").getAttribute("href");
    expect(href).toMatch(/^data:application\/json;charset=utf-8,/);
    return JSON.parse(decodeURIComponent((href ?? "").replace(/^[^,]*,/, ""))) as Record<string, unknown>;
  };
  let asked = seen.length;
  await page.getByRole("button", { name: "Prepare the export" }).click();
  await expect(page.locator("#export-save")).toBeFocused();
  // the address is asked for once the export answered
  // the picture's facts and the reports filed join the file (picture.spec.ts)
  const after = seen.slice(asked);
  expect(after.filter((body) => body.operation !== "picture")).toEqual([
    { operation: "account_export" }, { operation: "email", request: { action: "status" } },
    { operation: "challenges", request: { action: "export", table: "participation", after: null } },
    { operation: "challenges", request: { action: "export", table: "attempts", after: null } },
    { operation: "challenges", request: { action: "tower", request: { action: "export", after: null } } },
  ]);
  expect(after.filter((body) => body.operation === "picture").map((body) => body.request?.action).sort()).toEqual(["mine", "reports"]);
  const whole = await file();
  // Central's export as it is, and the account's own data of the address:
  // whether this Central sends mail is not the account's
  expect(whole).toEqual({ ...EXPORT, challenges: CHALLENGES, tower: { runs: [], complete: true, after: null }, email: { address: OLD, confirmed_at: THEN, pending: NEW, pending_until: THEN + 900 } });
  await expect(page.locator("#export-no-email")).toHaveCount(0);
  await sound(page);

  // a Central that does not answer for addresses: the export is there all the same
  w.deaf = true;
  await page.reload();
  await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
  asked = seen.length;
  await page.getByRole("button", { name: "Prepare the export" }).click();
  await expect(page.locator("#export-save")).toBeFocused();
  await expect(page.locator("#export-no-email")).toHaveText("The e-mail address could not be read; it is not in this file.");
  expect(await file()).toEqual({ ...EXPORT, challenges: CHALLENGES, tower: { runs: [], complete: true, after: null } });
  expect(await page.evaluate(() => localStorage.getItem("baylee-central-session"))).not.toBeNull();
  await sound(page);
});

test("a wrong password is refused in the real Worker, and nothing about the address is sent", async ({ page }) => {
  const w = world();
  const { seen, problems } = await open(page, w, { canned: false });
  await expect(address(page)).toBeVisible();
  const asked = seen.length;
  await send(page, NEW, "the wrong old words");
  await expect(alert(page)).toHaveText("The current password is not right. Check it and try again.", { timeout: 30_000 });
  // the stand-in's KE2 does not verify: no proof, and no address was named
  expect(operations(seen, asked)).toEqual(["account_summary", "login_start"]);
  expect(w.held.pending).toBeNull();
  // the form is back with the address; the password is to be typed again
  await expect(password(page)).toBeFocused();
  await expect(password(page)).toHaveValue("");
  await expect(address(page)).toHaveValue(NEW);
  expect(JSON.stringify(seen)).not.toContain("old words");
  expect(JSON.stringify(seen)).not.toContain(NEW);
  await sound(page);
  expect(problems).toEqual([]);
});

/** The security page's address bar and its history's current entry hold no fragment. */
async function gone(page: Page) {
  await expect(page).toHaveURL(/\/en\/account\/security$/);
  expect(await page.evaluate(() => [location.hash, location.href.includes("#")])).toEqual(["", false]);
}

// The confirmation mail's button links to `/{lang}/account/security#code=XXXX-XXXX`
// (docs/email.md, "The code in the button's link"): the area takes the
// fragment off the address bar at once, and the e-mail section fills the
// field of an address that waits; nothing is confirmed until the person
// presses the button.
test.describe("the code from the confirmation mail's button", () => {
  const LINKED = `/en/account/security#code=${CODE}`;

  test("fills the field of the address that waits, leaves the address bar at once, and confirms only when pressed", async ({ page }) => {
    const w = world(waiting());
    const sent: string[] = [];
    page.on("request", (request) => {
      // a navigation's address is reported with the fragment Playwright was given; the browser sends none
      if (!request.isNavigationRequest()) sent.push(`${request.url()} ${request.headers()["referer"] ?? ""}`);
    });
    const { seen, problems } = await open(page, w, { path: LINKED });
    await expect(code(page)).toHaveValue(CODE);
    await gone(page);
    // only the state was read: nothing was confirmed by itself
    await page.waitForTimeout(500);
    expect(actions(seen)).toEqual(["status"]);
    expect(w.held.address).toBeNull();
    expect(sent.filter((line) => line.includes(CODE) || line.includes("code="))).toEqual([]);
    await sound(page);
    // a reload reads the state again and finds no code
    await page.reload();
    await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
    await expect(code(page)).toBeVisible();
    await expect(code(page)).toHaveValue("");
    await gone(page);
    // The link opened in the tab that shows the page already: a move within
    // the page, which runs no island. The fragment goes and fills nothing;
    // the page is left as it was, what was typed included.
    await code(page).fill("zzzz");
    const reads = seen.length;
    await page.goto(LINKED);
    await gone(page);
    await expect(code(page)).toHaveValue("zzzz");
    expect(seen.length).toBe(reads);
    // opened as a page of its own, it is filled, and the person confirms it
    await page.goto("/en/account");
    await page.goto(LINKED);
    await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
    await expect(code(page)).toHaveValue(CODE);
    await gone(page);
    const asked = seen.length;
    await button(page, "Confirm").click();
    await expect(said(page)).toHaveText("Your e-mail address is confirmed.");
    expect(actions(seen, asked)).toEqual(["confirm"]);
    expect(seen.at(-1)).toEqual({ operation: "email", request: { action: "confirm", code: CODE } });
    expect(problems).toEqual([]);
  });

  test("lower case and without the hyphen, it is filled in as printed", async ({ page }) => {
    const { seen } = await open(page, world(waiting()), { path: `/en/account/security#code=${CODE.toLowerCase().replace("-", "")}` });
    await expect(code(page)).toHaveValue(CODE);
    await gone(page);
    expect(actions(seen)).toEqual(["status"]);
  });

  for (const [what, hash] of [
    ["a reset code", "#code=RESET-0123-4567-89AB-CDEF-GHJK-MNPQ-RSTV-WXYZ"],
    ["a code with a letter no code has", "#code=000G-40RU"],
    ["a code cut short", "#code=000G-40R"],
    ["an encoded character", "#code=000G%2D40R4"],
    ["nothing", "#code="],
  ] as const) {
    test(`${what} in the fragment fills nothing, and the fragment goes all the same`, async ({ page }) => {
      const { seen, problems } = await open(page, world(waiting()), { path: `/en/account/security${hash}` });
      await expect(code(page)).toBeVisible();
      await expect(code(page)).toHaveValue("");
      await gone(page);
      expect(actions(seen)).toEqual(["status"]);
      expect(problems).toEqual([]);
    });
  }

  test("with no address that waits the code is dropped: no field now, and an empty one later", async ({ page }) => {
    const w = world(confirmed());
    // naming the address is carried out and its answer lost: the page
    // reads the state anew and draws the field without clearing it, so
    // only a code that was dropped leaves it empty
    w.next.set = "lost";
    const { seen } = await open(page, w, { path: LINKED });
    await expect(button(page, "Change")).toBeVisible();
    await expect(code(page)).toHaveCount(0);
    await gone(page);
    await button(page, "Change").click();
    await send(page, NEW);
    await expect(code(page)).toBeVisible();
    await expect(alert(page)).toHaveText(/./);
    await expect(code(page)).toHaveValue("");
    expect(actions(seen)).toEqual(["status", "set", "status"]);
  });

  test("a first read that fails drops the code: after trying again the field is empty", async ({ page }) => {
    const w = world(waiting());
    w.next.status = 503;
    const { seen } = await open(page, w, { path: LINKED });
    await expect(alert(page)).toHaveText("Baylee Central is not answering right now. Try again in a moment.");
    await gone(page);
    await button(page, "Try again").click();
    await expect(code(page)).toBeVisible();
    await expect(code(page)).toHaveValue("");
    expect(actions(seen)).toEqual(["status", "status"]);
  });

  test("where Central sends no mail the code is dropped", async ({ page }) => {
    const { seen } = await open(page, world({ mail: false, ...waiting() }), { path: LINKED });
    await expect(card(page)).toContainText("This server sends no mail, so an e-mail address cannot be set or confirmed here.");
    await expect(code(page)).toHaveCount(0);
    await gone(page);
    expect(actions(seen)).toEqual(["status"]);
  });

  test("signed out, the code is dropped silently and the fragment goes", async ({ page }) => {
    // Central no longer knows the session this browser remembers
    const w = world(waiting());
    w.gone = true;
    const { seen } = await open(page, w, { path: LINKED });
    await expect(page.getByText("You are not signed in.")).toBeVisible();
    await expect(code(page)).toHaveCount(0);
    await gone(page);
    expect(actions(seen)).toEqual([]);
    expect(JSON.stringify(seen)).not.toContain(CODE);
  });

  test("with no session remembered, nothing is asked and the fragment goes", async ({ page }) => {
    const seen: Body[] = [];
    await page.route("**/api/v1", central(seen, world(waiting())));
    await page.goto(LINKED);
    await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
    await expect(page.getByText("You are not signed in.")).toBeVisible();
    await gone(page);
    expect(seen).toEqual([]);
  });

  test("on another page of the area the fragment goes too, and leaves no code for later", async ({ page }) => {
    const { seen } = await open(page, world(waiting()), { path: `/en/account#code=${CODE}` });
    await expect(page).toHaveURL(/\/en\/account$/);
    expect(await page.evaluate(() => location.hash)).toBe("");
    // the security page reached from there has the field empty
    await page.goto("/en/account/security");
    await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
    await expect(code(page)).toHaveValue("");
    expect(actions(seen)).toEqual(["status"]);
  });
});
