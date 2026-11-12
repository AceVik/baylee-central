import { expect, type Page, test } from "@playwright/test";

import {
  ID, OWN, OTHER, PASSWORD, accountOf, asked, button, changes, hex, id, nav, open, operations, sound, tabs, world,
} from "./admin-central.ts";

// The accounts and one account (crates/central-web/src/islands/admin/users.rs,
// user.rs) in a real browser against a stand-in Central (admin-central.ts):
// the list, its search as Central reads it, and an account's changes, each
// sent once and only after its confirmation (and the password, where the
// sign-in is older than 30 minutes); Central's refusals said in words, and
// an answer that did not arrive never called one.

test.use({ serviceWorkers: "block" });

const rows = (page: Page) => page.locator(".admin-table tbody tr");
const search = (page: Page) => page.locator("#admin-users-search");
const userPath = (account: number[]) => `/en/account/admin/users/${hex(account)}`;
const done = (page: Page) => page.locator("#admin-done");
const note = (page: Page) => page.locator("#admin-note");
const confirm = (page: Page) => page.locator(".admin-confirm");

test("the accounts, page by page, as a table that names and links each", async ({ page }) => {
  const w = world();
  const { seen, problems } = await open(page, w, "/en/account/admin/users");
  await expect(rows(page)).toHaveCount(3);
  expect(asked(seen)).toEqual([{ action: "accounts", query: null, before: null }]);
  await expect(page.getByText("Accounts shown: 3")).toBeVisible();
  await expect(page.locator(".admin-table thead th")).toHaveText(["Handle", "Sign-in name", "Account", "Created", "Last sign-in"]);
  const first = rows(page).first();
  await expect(first.locator("th a")).toHaveText("Erika#1a2b");
  await expect(first.locator("th a")).toHaveAttribute("href", userPath(ID.erika));
  await expect(first.locator("td")).toHaveText(["erika", "Registered on this site", /2026, \d\d:\d\d UTC$/, /2026/]);
  await expect(rows(page).nth(1).locator("td").nth(1)).toHaveText("Moved from the official gateway");
  await expect(rows(page).nth(2).locator(".pill")).toHaveText("Administrator");
  await sound(page);
  await tabs(page, search(page), [button(page, "Search"), page.locator(".admin-table"), first.locator("a")]);

  // the next page follows the last account shown, and the keyboard goes on at its first
  await button(page, "Show more").click();
  await expect(rows(page)).toHaveCount(6);
  await expect(page.locator(`#admin-user-${hex(ID.neu)}`)).toBeFocused();
  expect(asked(seen).at(-1)).toEqual({ action: "accounts", query: null, before: ID.ada });
  await expect(rows(page).nth(3).locator("th a")).toHaveText("neu");
  await expect(rows(page).nth(3).locator("td").last()).toHaveText("–");
  await button(page, "Show more").click();
  await expect(rows(page)).toHaveCount(7);
  await expect(button(page, "Show more")).toHaveCount(0);
  expect(problems).toEqual([]);
});

test("the list follows the search as it is typed, Escape empties it, and Ctrl+K reaches it", async ({ page }) => {
  const w = world();
  const { seen } = await open(page, w, "/en/account/admin/users");
  await expect(rows(page)).toHaveCount(3);
  // typing: one request once it pauses, for what stands then
  let from = seen.length;
  await search(page).pressSequentially("ERI", { delay: 40 });
  await expect(rows(page)).toHaveCount(1);
  expect(asked(seen, from)).toEqual([{ action: "accounts", query: "ERI", before: null }]);
  await expect(rows(page).locator("th a")).toHaveText("Erika#1a2b");
  // a tag
  from = seen.length;
  await search(page).fill("#4d5e");
  await expect(rows(page).locator("th a")).toHaveText(["Jonas#4d5e"]);
  expect(asked(seen, from)).toEqual([{ action: "accounts", query: "#4d5e", before: null }]);
  // nothing matches
  await search(page).fill("nobody");
  await expect(page.locator(".admin-results")).toHaveText("No account matches this search.");
  // what Central would refuse is said, and not sent
  from = seen.length;
  for (const bad of ["jürgen", "#xyz", "#"]) {
    await search(page).fill(bad);
    await expect(page.locator(".admin-search [role=alert]")).toHaveText(
      "A search is 1 to 64 plain characters without accents, or # and a tag such as #1a2b.",
    );
    await search(page).press("Enter");
    await expect(search(page)).toBeFocused();
  }
  await page.waitForTimeout(500);
  expect(seen.slice(from)).toEqual([]);
  // Escape empties the field, and every account is listed again
  await search(page).press("Escape");
  await expect(search(page)).toHaveValue("");
  await expect(rows(page)).toHaveCount(3);
  await expect(page.locator(".admin-search [role=alert]")).toBeHidden();
  expect(asked(seen).at(-1)).toEqual({ action: "accounts", query: null, before: null });
  // the same search again asks nothing
  from = seen.length;
  await search(page).press("Enter");
  await expect(rows(page)).toHaveCount(3);
  // Ctrl+K from anywhere on the page
  await rows(page).first().locator("a").focus();
  await page.keyboard.press("Control+k");
  await expect(search(page)).toBeFocused();
  await expect(search(page)).toHaveAttribute("aria-keyshortcuts", "Control+K Meta+K");
  expect(asked(seen, from)).toEqual([{ action: "accounts", query: null, before: null }]);
});

test("an account, its facts, its gateways and its audit events; copying its sign-in name and id", async ({ page, context }) => {
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  const w = world();
  const { seen, problems } = await open(page, w, "/en/account/admin/users");
  await rows(page).first().locator("a").click();
  await expect(page).toHaveURL(new RegExp(`${userPath(ID.erika)}$`));
  await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
  await expect(page.locator("h1")).toHaveText("An account");
  await expect(page.locator("#admin-user-title")).toHaveText("Erika#1a2b");
  expect(asked(seen).at(-1)).toEqual({ action: "account", account: ID.erika });
  const facts = page.locator("section[aria-labelledby=admin-user-title] dl.facts");
  await expect(facts.locator("dt")).toHaveText([
    "Sign-in name", "Id", "Tag", "Account", "State", "Created", "Last sign-in", "Recovery code", "Role",
  ]);
  await expect(facts.locator("dd code")).toHaveText(["erika", hex(ID.erika), "#1a2b"]);
  await expect(facts.locator("dd").nth(8)).toHaveText("Member");
  // the gateways it registered, and its events
  // (Central says of each only whether it is approved, as the old page showed it)
  await expect(page.locator(".admin-owned li")).toHaveText([
    "https://garden-table.example · Awaiting approval", "https://night-owls.example · Awaiting approval",
    "https://sun-court.example · Approved", "https://closed-door.example · Awaiting approval",
  ]);
  const events = page.locator(".admin-table[aria-labelledby=admin-user-audit] tbody tr");
  await expect(events.locator("th")).toHaveText(["Tag changed"]);
  await expect(events.locator("td a")).toHaveText(["…1c1d1e01", "Erika#1a2b"]);
  // the reset code's own page for this account
  await expect(page.locator("#admin-reset-link")).toHaveAttribute("href", `/en/account/admin#account=${hex(ID.erika)}`);
  await sound(page);
  await tabs(page, page.getByRole("button", { name: "Copy Sign-in name" }), [
    page.getByRole("button", { name: "Copy Id" }), page.locator("#admin-reset-link"), page.locator("#admin-tag-field"),
    button(page, "Set the tag"), button(page, "Make administrator"),
  ]);
  await page.getByRole("button", { name: "Copy Id" }).click();
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(hex(ID.erika));
  await page.getByRole("button", { name: "Copy Sign-in name" }).click();
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe("erika");
  // reading is all that was asked
  expect(changes(seen)).toEqual([]);
  expect(problems).toEqual([]);
});

test("a reserved account and one's own offer no change, and an unknown account is said", async ({ page }) => {
  const w = world();
  await open(page, w, userPath(ID.neu));
  await expect(page.locator("#admin-user-title")).toHaveText("neu");
  await expect(page.locator("#admin-tag-field, #admin-role, #admin-reset-link")).toHaveCount(0);
  await sound(page);
  await page.goto(userPath(OWN));
  await expect(page.locator("#admin-user-title")).toContainText("Chef#0c4e");
  await expect(page.locator("#admin-tag-field")).toBeVisible();
  await expect(page.locator("#admin-role, #admin-reset-link")).toHaveCount(0);
  // an administrator's account: the role can go, no reset code
  await page.goto(userPath(ID.ada));
  await expect(button(page, "Remove administrator")).toBeVisible();
  await expect(page.locator("#admin-reset-link")).toHaveCount(0);
  await page.goto(userPath(OTHER));
  await expect(page.getByRole("alert").first()).toHaveText("This account no longer exists.");
  await expect(nav(page)).toBeVisible();
});

test("a tag is checked, confirmed, sent once and read again; a taken one is refused in words", async ({ page }) => {
  const w = world();
  const { seen, problems } = await open(page, w, userPath(ID.erika));
  const field = page.locator("#admin-tag-field");
  await expect(field).toBeVisible();
  let from = seen.length;
  // a tag Central would refuse: said, nothing asked
  for (const bad of ["", "#", "#0", "#80000000", "#xyz"]) {
    await field.fill(bad);
    await button(page, "Set the tag").click();
    await expect(page.locator("section[aria-labelledby=admin-tag-title] [role=alert]")).toHaveText(
      "A tag is one to eight hex digits, from #1 to #7fffffff.",
    );
    await expect(field).toBeFocused();
  }
  // a good one opens its confirmation, which says what follows; nothing is sent
  await field.fill("#ABC");
  await field.press("Enter");
  await expect(page.locator("#admin-confirm-title")).toBeFocused();
  await expect(page.locator("#admin-confirm-title")).toHaveText("Give Erika#1a2b the tag #0abc?");
  await expect(confirm(page).locator("p").first()).toContainText("earlier gateway-bound whisper history no longer appears");
  await sound(page);
  await tabs(page, page.locator("#admin-confirm-title"), [button(confirm(page), "Set the tag"), button(confirm(page), "Back")]);
  await button(confirm(page), "Back").click();
  await expect(field).toBeFocused();
  await expect(confirm(page)).toHaveCount(0);
  expect(seen.slice(from)).toEqual([]);

  // confirmed: the session's standing, the one change, the account read again
  await button(page, "Set the tag").click();
  from = seen.length;
  await page.locator("#admin-confirm").dblclick();
  await expect(done(page)).toHaveText("The tag is set.");
  await expect(done(page)).toBeFocused();
  expect(operations(seen, from)).toEqual(["account_summary", "admin", "admin"]);
  expect(asked(seen, from)).toEqual([{ action: "set_tag", account: ID.erika, tag: 0xabc }, { action: "account", account: ID.erika }]);
  await expect(page.locator("#admin-user-title")).toHaveText("Erika#0abc");
  await expect(field).toHaveValue("");
  await sound(page);

  // taken by another account: refused in words, and the account read again
  await field.fill("#4d5e");
  await button(page, "Set the tag").click();
  from = seen.length;
  await page.locator("#admin-confirm").click();
  await expect(note(page)).toHaveText(
    "Baylee Central refused this tag: another account holds it, or this account uses Central chat, which keeps its tag.",
  );
  await expect(note(page)).toBeFocused();
  expect(changes(seen, from)).toEqual([{ action: "set_tag", account: ID.erika, tag: 0x4d5e }]);
  await expect(page.locator("#admin-user-title")).toHaveText("Erika#0abc");
  await sound(page);
  expect(problems).toEqual([]);
});

test("the role is granted and removed after its confirmation, once each", async ({ page }) => {
  const w = world();
  const { seen } = await open(page, w, userPath(ID.erika));
  await button(page, "Make administrator").click();
  await expect(page.locator("#admin-confirm-title")).toHaveText("Make Erika#1a2b an administrator?");
  await expect(confirm(page)).toContainText("Administrators see every account and gateway and can change them.");
  await expect(button(confirm(page), "Make administrator")).toHaveClass(/primary/);
  let from = seen.length;
  await button(confirm(page), "Make administrator").click();
  await expect(done(page)).toHaveText("The account is an administrator now.");
  expect(changes(seen, from)).toEqual([{ action: "set_admin", account: ID.erika, admin: true }]);
  await expect(page.locator("#admin-user-title .pill")).toHaveText("Administrator");
  await expect(page.locator("dl.facts dd").nth(8)).toHaveText("Administrator");
  // an administrator's account has no reset code
  await expect(page.locator("#admin-reset-link")).toHaveCount(0);
  await button(page, "Remove administrator").click();
  await expect(page.locator("#admin-confirm-title")).toHaveText("Remove Erika#1a2b's administrator role?");
  await expect(button(confirm(page), "Remove administrator")).toHaveClass(/danger/);
  await sound(page);
  from = seen.length;
  await button(confirm(page), "Remove administrator").click();
  await expect(done(page)).toHaveText("The administrator role is removed.");
  expect(changes(seen, from)).toEqual([{ action: "set_admin", account: ID.erika, admin: false }]);
  await expect(page.locator("#admin-user-title .pill")).toHaveCount(0);
});

test("a sign-in older than 30 minutes asks for the password before the change, once", async ({ page }) => {
  const w = world({ stale: true });
  const { seen, problems } = await open(page, w, userPath(ID.erika));
  await button(page, "Make administrator").click();
  let from = seen.length;
  await button(confirm(page), "Make administrator").click();
  const password = page.locator("#admin-password");
  await expect(password).toBeFocused();
  // the standing was asked, and nothing that changes
  expect(operations(seen, from)).toEqual(["account_summary"]);
  await expect(password).toHaveAccessibleDescription(
    "This change needs a sign-in less than 30 minutes old, so you enter your password once more. It stays in this browser.",
  );
  await expect(confirm(page).locator("input[autocomplete=username]")).toHaveValue("chef");
  await expect(page.locator("input[name]")).toHaveCount(0);
  await button(confirm(page), "Make administrator").click();
  await expect(confirm(page).getByRole("alert")).toHaveText("Enter your password.");
  await sound(page);
  await tabs(page, password, [button(page, "Show the password"), button(confirm(page), "Make administrator"), button(confirm(page), "Back")]);
  expect(operations(seen, from)).toEqual(["account_summary"]);
  from = seen.length;
  await password.fill(PASSWORD);
  await password.press("Enter");
  await expect(done(page)).toHaveText("The account is an administrator now.");
  // the password proven for the session's own account, then the one change
  expect(operations(seen, from)).toEqual(["account_summary", "login_start", "login_finish", "account_summary", "admin", "admin"]);
  expect(changes(seen, from)).toEqual([{ action: "set_admin", account: ID.erika, admin: true }]);
  expect(JSON.stringify(seen)).not.toContain("lantern");

  // Central counting the sign-in as old where the browser did not: refused,
  // explained, and the password asked for instead of a second try
  w.stale = true;
  await button(page, "Remove administrator").click();
  from = seen.length;
  await page.evaluate(() => {
    // the browser's clock behind Central's
    const real = Date.now;
    Date.now = () => real() - 2 * 3600 * 1000;
  });
  await button(confirm(page), "Remove administrator").click();
  await expect(password).toBeFocused();
  expect(changes(seen, from)).toEqual([{ action: "set_admin", account: ID.erika, admin: false }]);
  expect(asked(seen, from).map((r) => r?.action)).toEqual(["set_admin", "account"]);
  expect(problems).toEqual([]);
});

test("Central's refusals are explained by what it holds afterwards", async ({ page }) => {
  const w = world();
  const { seen } = await open(page, w, userPath(ID.erika));
  // the account deleted meanwhile
  await button(page, "Make administrator").click();
  w.accounts = w.accounts.filter(({ row }) => row.username !== "erika");
  await button(confirm(page), "Make administrator").click();
  await expect(page.getByRole("alert").first()).toHaveText("This account no longer exists.");
  w.accounts.unshift(accountOf(10, "erika", { display_name: "Erika", tag: 0x1a2b }));
  await page.reload();
  // the role taken away meanwhile
  await button(page, "Make administrator").click();
  w.admin = false;
  const from = seen.length;
  await button(confirm(page), "Make administrator").click();
  await expect(page.getByText("Your account no longer holds the administrator role.").first()).toBeVisible();
  // nothing was sent but the standing
  expect(changes(seen, from)).toEqual([]);
  w.admin = true;
  await page.reload();
  // too many requests: said as the forms say it
  await button(page, "Make administrator").click();
  w.next = 429;
  await button(confirm(page), "Make administrator").click();
  await expect(note(page)).toHaveText("Too many attempts. Try again later; after several wrong passwords this can take an hour, at most a day.");
  await sound(page);
  // another browser tab signed in as someone else: nothing is sent
  await button(page, "Make administrator").click();
  w.as = OTHER;
  const before = seen.length;
  await button(confirm(page), "Make administrator").click();
  await expect(note(page)).toHaveText("This browser is signed in as another account now, so nothing was changed. Load the page again.");
  expect(changes(seen, before)).toEqual([]);
});

test("an answer that does not arrive is no refusal: the account is read again", async ({ page }) => {
  const w = world();
  const { seen } = await open(page, w, userPath(ID.erika));
  const lost = "No answer came back, so we cannot tell whether the change was made. The page has read it again: check what it shows before you try once more.";
  await button(page, "Make administrator").click();
  w.next = "lost";
  let from = seen.length;
  await button(confirm(page), "Make administrator").click();
  await expect(note(page)).toHaveText(lost);
  await expect(note(page)).toBeFocused();
  expect(asked(seen, from)).toEqual([{ action: "set_admin", account: ID.erika, admin: true }, { action: "account", account: ID.erika }]);
  // it was made: the page shows it
  await expect(page.locator("#admin-user-title .pill")).toHaveText("Administrator");
  await sound(page);
  // an answer of another kind (made), and Central unavailable (not made):
  // the same words, and the page shows what Central holds
  for (const [next, label, after] of [["other", "Remove administrator", 0], [503, "Make administrator", 0]] as const) {
    await button(page, label).click();
    w.next = next;
    from = seen.length;
    await button(confirm(page), label).click();
    await expect(note(page)).toHaveText(lost);
    expect(asked(seen, from).map((r) => r?.action)).toEqual(["set_admin", "account"]);
    await expect(page.locator("#admin-user-title .pill")).toHaveCount(after);
  }
  // the reading again lost too: then the page cannot say what stands
  w.next = "lost";
  w.failing = { account: 0 };
  await button(page, "Make administrator").click();
  await button(confirm(page), "Make administrator").click();
  await expect(note(page)).toHaveText(
    "No answer came back, so we cannot tell whether the change was made. Load the page again before you try once more.",
  );
  await expect(note(page)).toBeFocused();
});

test("the reset code's page opens at the account its link names", async ({ page }) => {
  const w = world();
  const { seen } = await open(page, w, userPath(ID.jonas));
  await page.locator("#admin-reset-link").click();
  await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
  await expect(page.locator("#admin-account-title")).toBeFocused();
  await expect(page.locator("#admin-account-title")).toHaveText("Jonas#4d5e");
  // the fragment is gone from the address
  await expect(page).toHaveURL(/\/en\/account\/admin$/);
  expect(asked(seen).filter((r) => r?.action === "account").at(-1)).toEqual({ action: "account", account: ID.jonas });
  // back to the list, which does not show it: the keyboard goes to the search
  await page.locator("#admin-steps").getByRole("button", { name: "Back" }).click();
  await expect(page.locator("#admin-search")).toBeFocused();
  // an account that is gone is said at the search (a link from another page)
  await page.goto("/en/account");
  await page.goto(`/en/account/admin#account=${hex(id(77))}`);
  await expect(page.locator(".admin-search [role=alert]")).toHaveText("This account no longer exists.");
  await expect(page.locator("#admin-search")).toBeFocused();
  // a fragment that is no account's asks nothing
  await page.goto("/en/account");
  const from = seen.length;
  await page.goto("/en/account/admin#account=nothing");
  await expect(page.locator(".admin-accounts > li")).toHaveCount(3);
  // (the account's overview asks for registration's state)
  expect(asked(seen, from).map((r) => r?.action).filter((action) => action !== "registration")).toEqual(["accounts"]);
});

test("German, and the longest names, hold at 320 px", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 812 });
  const w = world({ stale: true });
  w.accounts.unshift(accountOf(20, "a_very-long.sign-in_name", { display_name: "Wolkenkratzer_99", tag: 0x7fffffff }));
  await open(page, w, "/de/account/admin/users");
  await expect(page.locator("h1")).toHaveText("Accounts");
  await expect(rows(page)).toHaveCount(3);
  await sound(page);
  await rows(page).first().locator("a").click();
  await expect(page.locator("#admin-user-title")).toHaveText("Wolkenkratzer_99#7fffffff");
  await sound(page);
  await page.locator("#admin-tag-field").fill("#1");
  await button(page, "Tag setzen").click();
  await expect(page.locator("#admin-confirm-title")).toHaveText("Wolkenkratzer_99#7fffffff den Tag #0001 geben?");
  await sound(page);
  await page.locator("#admin-confirm").click();
  await expect(page.locator("#admin-password")).toBeFocused();
  await expect(page.locator("label[for=admin-password]")).toHaveText("Dein Passwort, zur Bestätigung");
  await sound(page);
  await button(page, "Zum Administrator machen").click();
  await expect(page.locator("#admin-confirm-title")).toHaveText("Wolkenkratzer_99#7fffffff zum Administrator machen?");
  await sound(page);
});
