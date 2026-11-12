import { expect, type Locator, type Page, type Route, test } from "@playwright/test";

import { narrow, schemes, targets } from "./check.ts";

// An administrator makes a reset code on the new site
// (crates/central-web/src/islands/admin.rs, docs/account-reset.md) in a real
// browser, against a stand-in for Central's browser surface that refuses a
// code as Central's store does (central-postgres `issue_reset`): for an
// administrator's account, one's own, a pending one, an unfinished move,
// and a stale sign-in, all with the same 401. All of it is synthetic: the
// accounts, and a code that is none of Central's.
//
// The code needs a sign-in once more. The stand-in cannot answer OPAQUE
// truthfully, so that sign-in runs in a canned Worker here; the real one
// refusing the stand-in's challenge is the wrong password's path (the last
// test), and a code made by a real Central is e2e-real's.

const id = (n: number) => Array.from({ length: 16 }, (_, i) => ({ 6: 0x70, 8: 0x80, 15: n })[i] ?? 0x10 + i);
const hex = (bytes: number[]) => bytes.map((b) => b.toString(16).padStart(2, "0")).join("");

const OWN = id(1);
const OTHER = id(99);
const PASSWORD = "violet lantern harbour";
/** The code the stand-in makes: well formed, and nobody's. */
const CODE = "RESET-7K2M-9QPA-4XWZ-H3JD-8NBC-5TRV-6YEF-1GS0";
const THEN = 1_794_000_000;
const EXPIRES = THEN + 86_400;

type Source = "registered" | "legacy";
type State = "pending" | "enrolled" | "restored";
type Row = {
  id: number[];
  username: string;
  display_name: string | null;
  tag: number | null;
  source: Source;
  state: State;
  created_at: number;
  last_sign_in_at: number | null;
  admin: boolean;
  reset_until: number | null;
};
type Account = { row: Row; recovery: boolean; closed: boolean };

const accountOf = (n: number, username: string, more: Partial<Row> = {}, recovery = true, closed = false): Account => ({
  row: {
    id: id(n), username, display_name: null, tag: null, source: "registered", state: "restored",
    created_at: THEN - n * 3600, last_sign_in_at: THEN, admin: false, reset_until: null, ...more,
  },
  recovery,
  closed,
});

/** The accounts, newest first: one of each kind Central treats apart. */
const ACCOUNTS = () => [
  accountOf(10, "erika", { display_name: "Erika", tag: 0x1a2b }),
  // moved, in use, and its move not closed: only Central knows
  accountOf(11, "moritz", { display_name: "Moritz", tag: 0x2b3c, source: "legacy" }),
  accountOf(12, "ada", { display_name: "Ada", tag: 0x3c4d, admin: true }),
  accountOf(13, "neu", { state: "pending", last_sign_in_at: null }, false),
  // moved, not in use yet: its move cannot be closed
  accountOf(14, "umzug", { source: "legacy", state: "enrolled" }, false),
  accountOf(15, "jonas", { display_name: "Jonas", tag: 0x4d5e, state: "enrolled", reset_until: THEN + 3600 }, false),
  accountOf(1, "chef", { display_name: "Chef", tag: 0x0c4e, admin: true }),
];
const ID = { erika: id(10), moritz: id(11), ada: id(12), neu: id(13), umzug: id(14), jonas: id(15), chef: OWN };

type Asked = { action?: string; query?: string | null; before?: number[] | null; account?: number[] };
type Body = { operation?: string; request?: Asked; identity?: unknown } & Record<string, unknown>;

type World = {
  accounts: Account[];
  /** The browser's account holds the role. */
  admin: boolean;
  /** Central counts the sign-in as older than 30 minutes. */
  stale: boolean;
  /** The account this browser's session is. */
  as: number[];
  /** Central no longer knows the session. */
  gone: boolean;
  /** How the next `reset_login` ends instead: a status, an answer of another kind, or no answer after the code was made. */
  next: number | "lost" | "other" | null;
};

const world = (more: Partial<World> = {}): World => ({
  accounts: ACCOUNTS(), admin: true, stale: false, as: OWN, gone: false, next: null, ...more,
});

const same = (a: number[] | null | undefined, b: number[]) => a !== null && a !== undefined && a.length === b.length && a.every((v, i) => v === b[i]);
const PAGE = 3;

/** What Central answers an administrator, as far as the page can tell. */
function admin(w: World, asked: Asked, json: (value: unknown) => Promise<void>, refused: (status: number) => Promise<void>, abort: () => Promise<void>) {
  if (!w.admin) return refused(401);
  const answer = (value: unknown) => json({ result: "admin", answer: value });
  switch (asked.action) {
    case "accounts": {
      const query = asked.query?.trim().toLowerCase() ?? null;
      let found = w.accounts.filter(({ row }) => {
        if (query === null) return true;
        if (query.startsWith("#")) return row.tag === Number.parseInt(query.slice(1), 16);
        return row.username.includes(query) || (row.display_name ?? "").toLowerCase().includes(query);
      });
      if (asked.before) found = found.slice(found.findIndex(({ row }) => same(asked.before, row.id)) + 1);
      return answer({ answer: "accounts", accounts: found.slice(0, PAGE).map(({ row }) => row), more: found.length > PAGE });
    }
    case "account": {
      const one = w.accounts.find(({ row }) => same(asked.account, row.id));
      if (!one) return refused(401);
      return answer({ answer: "account", account: one.row, recovery: one.recovery, gateways: [], audit: [] });
    }
    case "reset_login": {
      const one = w.accounts.find(({ row }) => same(asked.account, row.id));
      const next = w.next;
      w.next = null;
      if (typeof next === "number") return refused(next);
      if (next === "other") return json({ result: "done" });
      // Central's refusals, all alike
      if (!one || w.stale || same(asked.account, w.as) || one.row.admin || one.row.state === "pending") return refused(401);
      if (one.row.source === "legacy" && !one.closed) return refused(401);
      one.row.reset_until = EXPIRES;
      if (next === "lost") return abort();
      return answer({ answer: "reset_code", code: CODE, expires_at: EXPIRES });
    }
    default:
      return refused(401);
  }
}

function central(seen: Body[], w: World) {
  return (route: Route) => {
    const request = route.request();
    expect(request.headers()["x-baylee-central"]).toBe("1");
    // nothing typed or shown travels in an address
    expect(new URL(request.url()).search).toBe("");
    const body = JSON.parse(request.postData() ?? "{}") as Body;
    seen.push(body);
    const json = (value: unknown) => route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(value) });
    const refused = (status: number) => route.fulfill({ status, body: "account request rejected\n" });
    if (w.gone) return refused(401);
    const now = Math.floor(Date.now() / 1000);
    switch (body.operation) {
      case "account_summary":
        return json({
          result: "account_summary", identity: { account: w.as, epoch: 1 }, username: "chef", display_name: "Chef", tag: 0x0c4e,
          source: "registered", recovery: true, admin: w.admin, signed_in_at: w.stale ? now - 3600 : now - 60, expires_at: 4_102_444_800,
        });
      case "admin":
        if (body.request?.action === "registration") return json({ result: "admin", answer: { answer: "registration", open: true, changed_at: null } });
        return admin(w, body.request ?? {}, json, refused, () => route.abort());
      case "login_start":
        return json({ result: "login_challenge", attempt: Array(32).fill(7), message: Array.from({ length: 320 }, (_, i) => (i * 37) % 251) });
      case "login_finish":
        return json({
          result: "browser_session", identity: { account: w.as, epoch: 1 }, username: "chef",
          wrapped_master: Array(73).fill(9), expires_at: 4_102_444_800,
        });
      default:
        return refused(401);
    }
  };
}

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
  await page.goto(options.path ?? "/en/account/admin");
  await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
  return { seen, problems };
}

// a route does not see what a service worker fetches: the canned Worker
test.use({ serviceWorkers: "block" });

const rows = (page: Page) => page.locator(".admin-accounts > li");
const opener = (page: Page, account: number[]) => page.locator(`#admin-open-${hex(account)}`);
const steps = (page: Page) => page.locator("#admin-steps");
const alert = (page: Page) => page.locator("#admin-error");
const button = (page: Page, name: string) => page.getByRole("button", { name, exact: true });
const password = (page: Page) => page.locator("#admin-password");
/** The administration's requests since `from`. */
// The page's section of reported pictures asks for them on its own
// (picture.spec.ts covers it); the accounts' requests are what is checked here.
const asked = (seen: Body[], from = 0) =>
  seen
    .slice(from)
    .filter((body) => body.operation === "admin" && body.request?.action !== "picture")
    .map((body) => body.request);
const operations = (seen: Body[], from: number) => seen.slice(from).map((body) => body.operation);

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

/** From the list to the password's step for `account`. */
async function toPassword(page: Page, account: number[]) {
  await opener(page, account).click();
  await expect(page.locator("#admin-account-title")).toBeFocused();
  await button(page, "Reset login").click();
  await expect(page.locator("#admin-warning-title")).toBeFocused();
  await button(page, "I have checked who is asking").click();
  await expect(password(page)).toBeFocused();
}

/**
 * From the list, through the warning and the password, to what came of it;
 * the number of requests in `seen` before the password was sent.
 */
async function ask(page: Page, account: number[], seen: Body[] = []): Promise<number> {
  await toPassword(page, account);
  const from = seen.length;
  await password(page).fill(PASSWORD);
  await password(page).press("Enter");
  return from;
}

/** Back to the list from an account's step. */
async function back(page: Page, account: number[]) {
  await steps(page).getByRole("button", { name: "Back" }).click();
  await expect(opener(page, account)).toBeFocused();
}

test("without the role there is no page and no link, and nothing is asked", async ({ page }) => {
  const w = world({ admin: false });
  const { seen, problems } = await open(page, w, { path: "/en/account" });
  await expect(page.locator(".account-who")).toBeVisible();
  await expect(page.locator("a[href$='/account/admin']")).toHaveCount(0);
  await expect(page.getByRole("link", { name: "Reset a login" })).toHaveCount(0);

  await page.goto("/en/account/admin");
  await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
  await expect(page.getByText("This page is only for administrators.")).toBeVisible();
  await expect(page.getByRole("link", { name: "Overview" }).last()).toHaveAttribute("href", "/en/account");
  await expect(page.locator("#admin-search, #admin-steps, .admin-accounts")).toHaveCount(0);
  // the navigation is everyone's, and has no way there
  await expect(page.locator(".frame-nav a[href$='/account/admin']")).toHaveCount(0);
  expect(seen.filter((body) => body.operation === "admin")).toEqual([]);
  await sound(page);
  expect(problems).toEqual([]);
});

test("an administrator is led there and finds accounts, page by page; a search Central would refuse is not sent", async ({ page }) => {
  const w = world();
  const { seen, problems } = await open(page, w, { path: "/en/account" });
  const link = page.getByRole("link", { name: "Reset a login" });
  await expect(link).toHaveAttribute("href", "/en/account/admin");
  // the rest of the administration is still the old build's
  await expect(page.getByRole("link", { name: "Administration", exact: true })).toHaveAttribute("href", "/admin");
  const start = seen.length;
  await link.click();
  await expect(page).toHaveURL(/\/en\/account\/admin$/);
  await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
  await expect(page.locator("h1")).toHaveText("Reset a login");
  await expect(rows(page)).toHaveCount(3);
  await expect(page.getByText("Accounts shown: 3")).toBeVisible();
  expect(asked(seen, start)).toEqual([{ action: "accounts", query: null, before: null }]);

  const first = rows(page).first();
  await expect(first.locator("h3")).toHaveText("Erika#1a2b");
  await expect(first.locator("dt")).toHaveText(["Sign-in name", "State", "Account", "Created", "Last sign-in"]);
  await expect(first.locator("dd")).toHaveText(["erika", "In use", "Registered on this site", /^.+2026, \d\d:\d\d UTC$/, /2026/]);
  await expect(first.locator("code")).toHaveAttribute("translate", "no");
  await expect(rows(page).nth(1).locator("dd").nth(2)).toHaveText("Moved from the official gateway");
  await expect(rows(page).nth(2).locator(".pill")).toHaveText("Administrator");
  await expect(opener(page, ID.erika)).toHaveAccessibleName("Open Erika#1a2b");
  await expect(page.locator("input[name]")).toHaveCount(0);
  await sound(page);
  await tabs(page, page.locator("#admin-search"), [button(page, "Search"), opener(page, ID.erika), opener(page, ID.moritz)]);

  // the next page follows the last account shown, and the keyboard goes on at its first
  await button(page, "Show more").click();
  await expect(rows(page)).toHaveCount(6);
  await expect(opener(page, ID.neu)).toBeFocused();
  expect(asked(seen).at(-1)).toEqual({ action: "accounts", query: null, before: ID.ada });
  // an account with no handle is named by its sign-in name; one never signed in says so
  await expect(rows(page).nth(3).locator("h3")).toHaveText("neu");
  await expect(rows(page).nth(3).locator("dd").nth(1)).toHaveText("Reserved, never set up");
  await expect(rows(page).nth(3).locator("dd").last()).toHaveText("–");
  // an outstanding code's expiry
  await expect(rows(page).nth(5).locator("dt").last()).toHaveText("Reset code");
  await expect(rows(page).nth(5).locator("dd").last()).toHaveText(/^Until .+ UTC$/);
  await button(page, "Show more").click();
  await expect(rows(page)).toHaveCount(7);
  await expect(button(page, "Show more")).toHaveCount(0);
  await expect(page.getByText("Accounts shown: 7")).toBeVisible();

  // a search, as typed: Central reads it without case
  const search = page.locator("#admin-search");
  await search.fill("  ERIKA ");
  await search.press("Enter");
  await expect(rows(page)).toHaveCount(1);
  expect(asked(seen).at(-1)).toEqual({ action: "accounts", query: "ERIKA", before: null });
  await search.fill("#4d5e");
  await button(page, "Search").click();
  await expect(rows(page).locator("h3")).toHaveText(["Jonas#4d5e"]);
  await search.fill("nobody");
  await search.press("Enter");
  await expect(page.locator(".admin-results")).toHaveText("No account matches this search.");
  await sound(page);

  // what Central would refuse is said, and not sent
  const before = seen.length;
  for (const bad of ["jürgen", "#xyz", "#", "#123456789"]) {
    await search.fill(bad);
    await search.press("Enter");
    await expect(page.locator(".admin-search [role=alert]")).toHaveText(
      "A search is 1 to 64 plain characters without accents, or # and a tag such as #1a2b.",
    );
    await expect(search).toBeFocused();
  }
  expect(seen.slice(before)).toEqual([]);
  // nothing typed lists every account again
  await search.fill("   ");
  await search.press("Enter");
  await expect(rows(page)).toHaveCount(3);
  await expect(page.locator(".admin-search [role=alert]")).toBeHidden();
  expect(asked(seen).at(-1)).toEqual({ action: "accounts", query: null, before: null });
  expect(problems).toEqual([]);
});

test("an account opens with its recovery code's state, and the warning and the password come before anything is sent", async ({ page }) => {
  const w = world();
  const { seen } = await open(page, w);
  await expect(rows(page)).toHaveCount(3);
  await opener(page, ID.erika).click();
  await expect(page.locator("#admin-account-title")).toBeFocused();
  await expect(page.locator("#admin-account-title")).toHaveText("Erika#1a2b");
  expect(asked(seen).at(-1)).toEqual({ action: "account", account: ID.erika });
  // the list waits, hidden, for the way back
  await expect(page.locator(".admin-list")).toBeHidden();
  const facts = steps(page).locator("dl.facts");
  await expect(facts.locator("dt")).toHaveText(["Sign-in name", "State", "Account", "Created", "Last sign-in", "Recovery code"]);
  await expect(facts.locator("dd").last()).toHaveText("Yes");
  await sound(page);
  await tabs(page, page.locator("#admin-account-title"), [button(page, "Reset login"), button(page, "Back")]);

  const from = seen.length;
  await button(page, "Reset login").click();
  const warning = steps(page).locator("section");
  await expect(page.locator("#admin-warning-title")).toBeFocused();
  await expect(page.locator("#admin-warning-title")).toHaveText("Before you make a code");
  await expect(warning.locator(".notice")).toHaveText(
    "Check outside this site that the person asking really holds this account, for example by a call or on a channel you already know them by. Whoever has the code can set a new password.",
  );
  await expect(warning.locator("li")).toHaveText([
    "Making a code ends nothing: the old password keeps working until the code is used, and every device stays signed in.",
    "The person's encrypted data survives the reset only with their recovery code or on a device that is still signed in. Without either, they start with an empty vault.",
    "A new code replaces any earlier one, one sent by e-mail included. It is valid for 24 hours and works once.",
    "A reset code answers a forgotten password, not a taken-over account: an intruder's sessions and the old password stay until the code is used, and for good if it never is.",
  ]);
  await sound(page);
  await tabs(page, page.locator("#admin-warning-title"), [button(page, "I have checked who is asking"), button(page, "Back")]);
  await button(page, "Back").click();
  await expect(page.locator("#admin-account-title")).toBeFocused();
  await button(page, "Reset login").click();
  await button(page, "I have checked who is asking").click();
  await expect(password(page)).toBeFocused();
  await expect(page.locator("label[for=admin-password]")).toHaveText("Your password, to confirm");
  await expect(password(page)).toHaveAttribute("type", "password");
  await expect(password(page)).toHaveAttribute("autocomplete", "current-password");
  await expect(password(page)).toHaveAccessibleDescription(
    "Making a code needs a sign-in less than 30 minutes old, so you enter your password once more. It stays in this browser.",
  );
  // a password manager learns whose password it is, and no person sees it
  const whose = steps(page).locator("input[autocomplete=username]");
  await expect(whose).toHaveValue("chef");
  await expect(whose).toBeHidden();
  await expect(page.locator("input[name]")).toHaveCount(0);
  await button(page, "Make the reset code").click();
  await expect(alert(page)).toHaveText("Enter your password.");
  await expect(password(page)).toBeFocused();
  await sound(page);
  await tabs(page, password(page), [button(page, "Show the password"), button(page, "Make the reset code"), button(page, "Back")]);
  // the warning, its way back and the password: nothing was sent
  expect(seen.slice(from)).toEqual([]);
  await password(page).fill(PASSWORD);
  await button(page, "Back").click();
  await expect(page.locator("#admin-account-title")).toBeFocused();
  await back(page, ID.erika);
  expect(seen.slice(from)).toEqual([]);
  expect(JSON.stringify(seen)).not.toContain("lantern");
});

test("the code is made after the password, shown once and copied, and gone when its step is left", async ({ page, context }) => {
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  const w = world();
  const { seen, problems } = await open(page, w);
  await expect(rows(page)).toHaveCount(3);
  await toPassword(page, ID.erika);
  const from = seen.length;
  await password(page).fill(PASSWORD);
  await password(page).press("Enter");
  const title = page.locator("#admin-code-title");
  await expect(title).toBeFocused();
  await expect(title).toHaveText("The reset code");
  // the password once more, with the session's own account, and only then the code
  expect(operations(seen, from)).toEqual(["account_summary", "login_start", "login_finish", "admin"]);
  expect((seen[from + 1] as { identity: unknown }).identity).toEqual({ account: OWN, epoch: 1 });
  expect(asked(seen).at(-1)).toEqual({ action: "reset_login", account: ID.erika });
  const shown = steps(page).locator("section");
  await expect(shown.locator(".eyebrow")).toHaveText("Shown only now");
  await expect(shown.locator(".secret code")).toHaveText(CODE);
  await expect(shown.locator(".secret")).toHaveAttribute("translate", "no");
  await expect(shown).toContainText("Erika#1a2b");
  await expect(shown).toContainText("Valid until ");
  await expect(shown).toContainText(", and only once.");
  await expect(shown).toContainText("It is shown only now: neither Baylee Central nor this page can show it again.");
  await expect(steps(page).locator("input")).toHaveCount(0);
  await expect(page.locator("input[name]")).toHaveCount(0);
  await sound(page);
  await tabs(page, title, [button(page, "Copy"), button(page, "Done")]);

  await button(page, "Copy").click();
  await expect(shown.getByRole("status")).toHaveText("Copied.");
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(CODE);
  // the code is in no address, no storage and no request
  expect(page.url()).not.toContain("RESET");
  const stored = await page.evaluate(() => JSON.stringify({ ...localStorage, ...sessionStorage }));
  expect(stored).not.toContain(CODE);
  expect(JSON.stringify(seen)).not.toContain("RESET");
  expect(JSON.stringify(seen)).not.toContain("lantern");

  // left, the code is gone from the page; the list shows the account's code expiring
  await button(page, "Done").click();
  await expect(opener(page, ID.erika)).toBeFocused();
  expect(await page.content()).not.toContain(CODE);
  await expect(rows(page).first().locator("dt").last()).toHaveText("Reset code");
  await expect(rows(page).first().locator("dd").last()).toHaveText(/^Until .+ UTC$/);
  await opener(page, ID.erika).click();
  await expect(steps(page).locator("section")).toContainText(
    "Whether an administrator made it or it went by e-mail, Central does not say. A new code replaces it.",
  );
  await back(page, ID.erika);

  // a page put away (another page, the back-forward cache) takes no code with it
  await ask(page, ID.erika);
  await expect(title).toBeFocused();
  await expect(steps(page).locator(".secret code")).toHaveText(CODE);
  await page.evaluate(() => window.dispatchEvent(new PageTransitionEvent("pagehide", { persisted: true })));
  await expect(steps(page).locator(".secret")).toHaveCount(0);
  await expect(page.locator(".admin-list")).toBeVisible();
  expect(await page.content()).not.toContain(CODE);
  expect(problems).toEqual([]);
});

test("what Central would refuse is said before anything is asked", async ({ page }) => {
  test.setTimeout(60_000);
  const w = world();
  const { seen } = await open(page, w);
  await button(page, "Show more").click();
  await expect(rows(page)).toHaveCount(6);
  await button(page, "Show more").click();
  await expect(rows(page)).toHaveCount(7);
  const cases: [number[], string][] = [
    [ID.ada, "This account holds the administrator role, and Central makes no reset code for an administrator's account. Remove the role first."],
    [ID.chef, "This is your own account, and nobody makes a reset code for their own. Change your password under Security instead."],
    [ID.neu, "This account was reserved and never set up, so it has no sign-in to reset."],
    [ID.umzug, "This account's move from the official gateway is not finished. Until it is, its sign-in is reset there, not here."],
  ];
  for (const [account, said] of cases) {
    await opener(page, account).click();
    await expect(page.locator("#admin-account-title")).toBeFocused();
    await expect(page.locator("#admin-refusal")).toHaveText(said);
    await expect(button(page, "Reset login")).toHaveCount(0);
    if (account === ID.neu) await expect(steps(page).locator("dl.facts dd").last()).toHaveText("No");
    await sound(page);
    await back(page, account);
  }
  // reading each account is all that was asked
  expect(asked(seen).filter((request) => request?.action !== "accounts").map((request) => request?.action)).toEqual([
    "account", "account", "account", "account",
  ]);
});

test("Central's refusals after the password are explained by what it holds now", async ({ page }) => {
  test.setTimeout(60_000);
  const w = world();
  const { seen } = await open(page, w);
  await expect(rows(page)).toHaveCount(3);
  /** Asks for `account`'s code with the password, `meanwhile` changing what Central holds after the account was read. */
  const refused = async (account: number[], said: string, meanwhile: () => void = () => undefined) => {
    await toPassword(page, account);
    meanwhile();
    const from = seen.length;
    await password(page).fill(PASSWORD);
    await password(page).press("Enter");
    await expect(alert(page)).toHaveText(said);
    await expect(alert(page)).toBeFocused();
    await expect(steps(page).locator(".secret")).toHaveCount(0);
    return operations(seen, from);
  };

  // a move Central has not closed, which the row could not tell
  let ops = await refused(ID.moritz, "This account's move from the official gateway is not finished. Until it is, its sign-in is reset there, not here.");
  // the password once more, the refusal, then what stands now: the role and the sign-in, and the account
  expect(ops).toEqual(["account_summary", "login_start", "login_finish", "admin", "account_summary", "admin"]);
  expect(asked(seen).slice(-2)).toEqual([{ action: "reset_login", account: ID.moritz }, { action: "account", account: ID.moritz }]);
  // a new attempt stays open: Central decides
  await expect(button(page, "Reset login")).toBeVisible();
  await sound(page);
  await back(page, ID.moritz);

  // a sign-in Central counts as old
  await refused(ID.erika, "Baylee Central counts your sign-in as older than 30 minutes. Try again; you enter your password once more.", () => {
    w.stale = true;
  });
  w.stale = false;
  await back(page, ID.erika);

  // an account deleted meanwhile
  await refused(ID.moritz, "This account no longer exists.", () => {
    w.accounts = w.accounts.filter(({ row }) => row.username !== "moritz");
  });
  w.accounts = ACCOUNTS();
  await back(page, ID.moritz);

  // the role taken away meanwhile
  await refused(ID.erika, "Your account no longer holds the administrator role.", () => {
    w.admin = false;
  });
  w.admin = true;
  await back(page, ID.erika);

  // too many requests, and a refusal for the terms: said as the forms say them, with nothing more asked
  ops = await refused(ID.erika, "Too many attempts. Try again later; after several wrong passwords this can take an hour, at most a day.", () => {
    w.next = 429;
  });
  expect(ops).toEqual(["account_summary", "login_start", "login_finish", "admin"]);
  await back(page, ID.erika);
  ops = await refused(ID.erika, "Something went wrong. Try again; if it keeps happening, tell us.", () => {
    w.next = 403;
  });
  expect(ops).toEqual(["account_summary", "login_start", "login_finish", "admin"]);
  await back(page, ID.erika);

  // an account that became an administrator's meanwhile: the reason stands in the
  // button's place and takes the keyboard, said once
  await toPassword(page, ID.erika);
  const erika = w.accounts.find(({ row }) => row.username === "erika");
  if (erika) erika.row.admin = true;
  await password(page).fill(PASSWORD);
  await password(page).press("Enter");
  const refusal = page.locator("#admin-refusal");
  await expect(refusal).toHaveText(
    "This account holds the administrator role, and Central makes no reset code for an administrator's account. Remove the role first.",
  );
  await expect(refusal).toBeFocused();
  await expect(alert(page)).toBeHidden();
  await expect(button(page, "Reset login")).toHaveCount(0);
  await expect(page.locator("#admin-account-title .pill")).toHaveText("Administrator");
  await sound(page);
  await back(page, ID.erika);
  // the list shows it as Central answered it last
  await expect(rows(page).first().locator(".pill")).toHaveText("Administrator");
  expect(JSON.stringify(seen)).not.toContain("RESET");
});

test("an answer that does not arrive is no refusal: a code may have been made", async ({ page }) => {
  const w = world();
  const { seen } = await open(page, w);
  await expect(rows(page)).toHaveCount(3);
  const lost =
    "No answer came back, so we cannot tell whether a code was made. If one was, nobody can show it again, and it already replaced any earlier code. Make a new one: it replaces that one in turn.";

  // made, and its answer lost on the way: the account is read again and shows a code
  w.next = "lost";
  let from = await ask(page, ID.erika, seen);
  await expect(alert(page)).toHaveText(lost);
  await expect(alert(page)).toBeFocused();
  expect(asked(seen, from)).toEqual([{ action: "reset_login", account: ID.erika }, { action: "account", account: ID.erika }]);
  await expect(steps(page).locator("dl.facts dt").last()).toHaveText("Reset code");
  await expect(steps(page)).toContainText("An unused reset code is valid until");
  // a new one can be asked for at once
  await expect(button(page, "Reset login")).toBeVisible();
  await sound(page);
  await back(page, ID.erika);

  // Central unavailable, and an answer of another kind: the same
  for (const next of [503, "other"] as const) {
    w.next = next;
    from = await ask(page, ID.erika, seen);
    await expect(alert(page)).toHaveText(lost);
    expect(asked(seen, from).map((request) => request?.action)).toEqual(["reset_login", "account"]);
    await back(page, ID.erika);
  }
  expect(await page.content()).not.toContain("RESET-");
});

test("a browser signed in as another account meanwhile asks for no code, and a session that is gone is said", async ({ page }) => {
  const w = world();
  const { seen } = await open(page, w);
  await expect(rows(page)).toHaveCount(3);
  await toPassword(page, ID.erika);
  w.as = OTHER;
  let from = seen.length;
  await password(page).fill(PASSWORD);
  await password(page).press("Enter");
  await expect(alert(page)).toHaveText("This browser is signed in as another account now, so no code was asked for. Load the page again.");
  expect(operations(seen, from)).toEqual(["account_summary", "login_start", "login_finish"]);
  await back(page, ID.erika);

  w.as = OWN;
  await toPassword(page, ID.erika);
  w.gone = true;
  from = seen.length;
  await password(page).fill(PASSWORD);
  await password(page).press("Enter");
  await expect(page.getByText("You are not signed in.")).toBeVisible();
  expect(operations(seen, from)).not.toContain("admin");
  await expect(page.locator("#admin-steps, .admin-list")).toHaveCount(0);
  expect(await page.evaluate(() => localStorage.getItem("baylee-central-session"))).toBeNull();
});

test("German, and the longest names, hold at 320 px in every step", async ({ page, context }) => {
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  const w = world();
  // as long as a display name (16) and a sign-in name (24) get
  w.accounts.unshift(accountOf(20, "a_very-long.sign-in_name", { display_name: "Wolkenkratzer_99", tag: 0x7fffffff }));
  const long = id(20);
  await open(page, w, { path: "/de/account/admin" });
  await expect(page.locator("h1")).toHaveText("Anmeldung zurücksetzen");
  await expect(rows(page)).toHaveCount(3);
  await expect(page.locator("label[for=admin-search]")).toHaveText("Accounts suchen");
  await expect(page.getByText("Angezeigte Accounts: 3")).toBeVisible();
  await sound(page);
  await opener(page, long).click();
  await expect(page.locator("#admin-account-title")).toHaveText("Wolkenkratzer_99#7fffffff");
  await expect(steps(page).locator("dl.facts dt").last()).toHaveText("Wiederherstellungscode");
  await sound(page);
  await button(page, "Anmeldung zurücksetzen").click();
  await expect(page.locator("#admin-warning-title")).toHaveText("Bevor du einen Code ausstellst");
  await sound(page);
  await button(page, "Ich habe geprüft, wer fragt").click();
  await expect(page.locator("label[for=admin-password]")).toHaveText("Dein Passwort, zur Bestätigung");
  await sound(page);
  await password(page).fill(PASSWORD);
  await password(page).press("Enter");
  await expect(page.locator("#admin-code-title")).toHaveText("Der Reset-Code");
  await expect(steps(page).locator(".eyebrow")).toHaveText("Nur jetzt sichtbar");
  await sound(page);
  await button(page, "Kopieren").click();
  await expect(steps(page).getByRole("status")).toHaveText("Kopiert.");
  await button(page, "Fertig").click();
  await expect(opener(page, long)).toBeFocused();
  await sound(page);
});

test("a wrong password is refused in the real Worker, and no code is asked for", async ({ page }) => {
  const w = world();
  const { seen, problems } = await open(page, w, { canned: false });
  await expect(rows(page)).toHaveCount(3);
  await toPassword(page, ID.erika);
  const from = seen.length;
  await password(page).fill("the wrong old words");
  await password(page).press("Enter");
  await expect(alert(page)).toHaveText("The current password is not right. Check it and try again.", { timeout: 30_000 });
  // the stand-in's KE2 does not verify: no proof, and no code was asked for
  expect(operations(seen, from)).toEqual(["account_summary", "login_start"]);
  await expect(password(page)).toBeFocused();
  await expect(password(page)).toHaveValue("");
  expect(JSON.stringify(seen)).not.toContain("old words");
  await sound(page);
  expect(problems).toEqual([]);
});
