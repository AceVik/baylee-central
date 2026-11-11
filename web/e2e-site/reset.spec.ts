import { expect, type Page, type Route, test } from "@playwright/test";

import { axe, targets, watch } from "./check.ts";

// The reset page (docs/account-reset.md) in a real browser, against a
// stand-in for Central's browser surface. The crypto is real: the committed
// central-web-crypto module in its Worker. A stand-in holds no account's
// OPAQUE key and no recovery wrapper, so what needs them (a vault kept with
// the recovery code, the sign-in afterwards) is e2e-real's. A registration
// response only has to be two valid group elements, so a new vault runs
// here to its end and the result's page is checked as a person sees it.

const ACCOUNT = [1, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 16];
const CODE = "RESET-0123-4567-89AB-CDEF-GHJK-MNPQ-RSTV-WXYZ";
const RECOVERY = "ZYXW-VTSR-QPNM-KJHG-FEDC-BA98-7654-3210";
const PASSWORD = "violet lantern harbour";
// ristretto255's base point: a valid element for the evaluated blind and
// for the server's key alike, which is all a registration's finish asks.
const POINT = [
  0xe2, 0xf2, 0xae, 0x0a, 0x6a, 0xbc, 0x4e, 0x71, 0xa8, 0x84, 0xa9, 0x61, 0xc5, 0x00, 0x51, 0x5f, 0x58, 0xe3, 0x0b, 0x6a, 0xa5, 0x82,
  0xdd, 0x8d, 0xb6, 0xa6, 0x59, 0x45, 0xe0, 0x8d, 0x2d, 0x76,
];

const MISSING = "Enter your sign-in name, the reset code and a new password.";
const REFUSED =
  "This sign-in name and reset code do not go together, or the code has expired or was used already. Check both. If nothing helps, ask for a new code.";
const NOT_OPENED =
  "This recovery code does not open your account's data. Check it and try again. Nothing was changed, and your reset code still works.";
const KEEP = "I have my recovery code. My data stays.";
const FRESH = "I do not have it. Start with an empty vault.";

type Body = Record<string, unknown> & { operation?: string };
/** How the stand-in answers; whatever is left out goes well. */
type Answers = {
  discover?: "unknown" | "legacy";
  start?: "refused";
  finish?: "refused" | "lost";
  /** The account's recovery wrapper, as `ResetChallenge` carries it. */
  recovery?: number[] | null;
};

function central(seen: Body[], answers: Answers = {}) {
  return (route: Route) => {
    const request = route.request();
    expect(request.headers()["x-baylee-central"]).toBe("1");
    const body = JSON.parse(request.postData() ?? "{}") as Body;
    seen.push(body);
    const json = (value: unknown) => route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(value) });
    const rejected = () => route.fulfill({ status: 401, body: "account request rejected\n" });
    const identity = { account: ACCOUNT, epoch: 1 };
    switch (body.operation) {
      case "discover":
        if (answers.discover === "unknown") return rejected();
        if (answers.discover === "legacy") return json({ result: "discovery", discovery: { kind: "legacy" } });
        return json({ result: "discovery", discovery: { kind: "central", identity } });
      case "reset_start":
        if (answers.start === "refused") return rejected();
        return json({
          result: "reset_challenge",
          attempt: Array(32).fill(7),
          identity,
          registration: [...POINT, ...POINT],
          recovery: answers.recovery ?? null,
        });
      case "reset_finish":
        if (answers.finish === "refused") return rejected();
        if (answers.finish === "lost") return route.abort("connectionreset");
        return json({ result: "done" });
      default:
        return rejected();
    }
  };
}

/** Console and page errors, but for the refusals and the lost answer the stand-in gives on purpose. */
function errors(page: Page): string[] {
  const problems: string[] = [];
  page.on("pageerror", (error) => problems.push(error.message));
  page.on("console", (message) => {
    const text = message.text();
    if (message.type() === "error" && !text.includes("status of 401") && !text.includes("ERR_CONNECTION_RESET")) problems.push(text);
  });
  return problems;
}

async function open(page: Page, lang = "en") {
  await page.goto(`/${lang}/reset`);
  await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
}

/** Fills the form: with `recovery` the data is kept, without it a new vault is chosen and its warning confirmed. */
async function fill(page: Page, recovery?: string) {
  await page.getByLabel("Sign-in name").fill("Erika");
  await page.getByLabel("Reset code").fill(CODE);
  if (recovery === undefined) {
    await page.getByRole("radio", { name: FRESH }).check();
    await page.getByRole("checkbox", { name: "I understand that this data is lost for good." }).check();
  } else {
    await page.getByLabel("Recovery code", { exact: true }).fill(recovery);
  }
  await page.getByLabel("New password", { exact: true }).fill(PASSWORD);
  await page.getByLabel("Repeat new password").fill(PASSWORD);
}

const submit = (page: Page) => page.getByRole("button", { name: "Set new password" });
const operations = (seen: Body[]) => seen.map((body) => body.operation);

test("every field is labelled, the data is kept unless chosen otherwise, and the warning stands before a new vault", async ({ page, baseURL }) => {
  const problems = watch(page, baseURL ?? "");
  await open(page);
  await expect(page.getByRole("heading", { level: 1, name: "Set a new password" })).toBeVisible();
  const recovery = page.getByLabel("Recovery code", { exact: true });
  const password = page.getByLabel("New password", { exact: true });
  for (const field of [page.getByLabel("Sign-in name"), page.getByLabel("Reset code"), recovery, password, page.getByLabel("Repeat new password")]) {
    await expect(field).toBeVisible();
    await expect(field).toBeEditable();
  }
  // nothing a person types has a name a browser could send it under
  for (const id of ["reset-name", "reset-code", "reset-recovery", "reset-password", "reset-repeat", "reset-confirm"]) {
    await expect(page.locator(`#${id}`)).not.toHaveAttribute("name", /.*/);
  }
  await expect(page.getByRole("group", { name: "Your encrypted data" })).toBeVisible();
  const keep = page.getByRole("radio", { name: KEEP });
  const fresh = page.getByRole("radio", { name: FRESH });
  await expect(keep).toBeChecked();
  await expect(fresh).not.toBeChecked();
  const warning = page.locator("#reset-warning");
  const confirm = page.getByRole("checkbox", { name: "I understand that this data is lost for good." });
  await expect(warning).toBeHidden();
  await expect(confirm).toBeHidden();
  // the choice that loses the data says so before it is taken
  await expect(fresh).toHaveAccessibleDescription(/your encrypted data is lost for good.*Nobody can bring it back/);
  expect(await axe(page)).toEqual([]);
  expect(await targets(page)).toEqual([]);

  await recovery.fill(RECOVERY);
  await fresh.check();
  await expect(warning).toBeVisible();
  await expect(warning).toContainText("the settings saved in your account, your deck backups and your chat history");
  await expect(warning).toContainText("Your account, your display name and your contacts stay.");
  await expect(confirm).toBeVisible();
  await expect(confirm).not.toBeChecked();
  await expect(recovery).toBeHidden();
  expect(await axe(page)).toEqual([]);
  expect(await targets(page)).toEqual([]);
  // back again: the recovery code that was typed is still there
  await keep.check();
  await expect(warning).toBeHidden();
  await expect(recovery).toHaveValue(RECOVERY);

  // each password has an eye of its own; the codes are text from the start
  const eyes = page.getByRole("button", { name: "Show the password" });
  await expect(eyes).toHaveCount(2);
  await password.fill(PASSWORD);
  await expect(password).toHaveAttribute("type", "password");
  await eyes.first().click();
  await expect(password).toHaveAttribute("type", "text");
  await expect(password).toHaveValue(PASSWORD);
  await expect(page.locator("#reset-repeat")).toHaveAttribute("type", "password");
  await expect(password).toHaveAttribute("autocomplete", "new-password");
  expect(problems).toEqual([]);
});

test("nothing is sent before the form is complete, and no new vault before the warning is confirmed", async ({ page }) => {
  const seen: Body[] = [];
  await page.route("**/api/v1", central(seen));
  await open(page);
  const alert = page.getByRole("alert");
  await submit(page).click();
  await expect(alert).toHaveText(MISSING);
  await page.getByLabel("Sign-in name").fill("erika");
  await page.getByLabel("New password", { exact: true }).fill(PASSWORD);
  await page.getByLabel("Repeat new password").fill(PASSWORD);
  // a recovery code is no reset code
  await page.getByLabel("Reset code").fill(RECOVERY);
  await submit(page).click();
  await expect(alert).toHaveText("That is not a reset code. It starts with RESET, followed by eight groups of four characters.");
  await page.getByLabel("Reset code").fill(CODE);
  await submit(page).click();
  await expect(alert).toHaveText("Enter your recovery code, or choose the empty vault.");
  await page.getByRole("radio", { name: FRESH }).check();
  await submit(page).click();
  await expect(alert).toHaveText("Confirm first that you know what is lost.");
  await page.getByRole("checkbox", { name: "I understand that this data is lost for good." }).check();
  await page.getByLabel("Repeat new password").fill("another harbour");
  await submit(page).click();
  await expect(alert).toHaveText("The two passwords are not the same.");
  await page.getByLabel("New password", { exact: true }).fill("short");
  await page.getByLabel("Repeat new password").fill("short");
  await submit(page).click();
  await expect(alert).toHaveText("The password needs at least 8 characters.");
  expect(seen).toEqual([]);
});

test("a name or a code that Central refuses reads the same, whichever it was", async ({ page }) => {
  const problems = errors(page);
  const cases: [Answers, string[]][] = [
    [{ discover: "unknown" }, ["discover"]],
    [{ discover: "legacy" }, ["discover"]],
    [{ start: "refused" }, ["discover", "reset_start"]],
  ];
  for (const [answers, asked] of cases) {
    const seen: Body[] = [];
    await page.unrouteAll();
    await page.route("**/api/v1", central(seen, answers));
    await open(page);
    await fill(page);
    await submit(page).click();
    await expect(page.getByRole("alert")).toHaveText(REFUSED, { timeout: 30_000 });
    expect(operations(seen)).toEqual(asked);
    expect(seen[0]).toEqual({ operation: "discover", username: "erika" });
    // the form is back with what it held, but for the passwords
    await expect(page.getByLabel("Sign-in name")).toBeFocused();
    await expect(page.getByLabel("Sign-in name")).toHaveValue("Erika");
    await expect(page.getByLabel("Reset code")).toHaveValue(CODE);
    await expect(page.getByRole("radio", { name: FRESH })).toBeChecked();
    await expect(page.getByLabel("New password", { exact: true })).toHaveValue("");
    await expect(page.getByLabel("Repeat new password")).toHaveValue("");
    // the password never left the Worker
    expect(JSON.stringify(seen)).not.toContain("lantern");
    if (asked.length === 2) {
      const start = seen[1] as { username: string; code: string; registration: number[] };
      expect(Object.keys(start).sort()).toEqual(["code", "operation", "registration", "username"]);
      expect(start.username).toBe("erika");
      expect(start.code).toBe(CODE);
      expect(start.registration).toHaveLength(32);
    }
  }
  expect(problems).toEqual([]);
});

test("a recovery code that is none, or that opens nothing, stops the reset before the code is spent", async ({ page }) => {
  const problems = errors(page);
  const cases: [string, Answers, string[]][] = [
    // not a recovery code at all: the reset code goes nowhere
    ["certainly not a code", {}, ["discover"]],
    // well formed, and the wrapper is not its own
    [RECOVERY, { recovery: Array(73).fill(0) }, ["discover", "reset_start"]],
    // an account that has no recovery wrapper
    [RECOVERY, { recovery: null }, ["discover", "reset_start"]],
  ];
  for (const [recovery, answers, asked] of cases) {
    const seen: Body[] = [];
    await page.unrouteAll();
    await page.route("**/api/v1", central(seen, answers));
    await open(page);
    await fill(page, recovery);
    await submit(page).click();
    await expect(page.getByRole("alert")).toHaveText(NOT_OPENED, { timeout: 30_000 });
    // no finish: the reset code is not spent
    expect(operations(seen)).toEqual(asked);
    await expect(page.getByLabel("Recovery code", { exact: true })).toHaveValue(recovery);
    await expect(page.getByLabel("New password", { exact: true })).toHaveValue("");
    // neither the password nor the recovery code left the Worker
    expect(JSON.stringify(seen)).not.toContain("lantern");
    expect(JSON.stringify(seen)).not.toContain(recovery);
    expect(JSON.stringify(seen)).not.toContain(recovery.replaceAll("-", ""));
  }
  expect(problems).toEqual([]);
});

test("a new vault: its recovery code is shown once, copied or saved, and kept before the way on", async ({ page, context }) => {
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  const problems = errors(page);
  const seen: Body[] = [];
  // this browser remembers a session the reset ended
  await page.addInitScript(() => localStorage.setItem("baylee-central-session", JSON.stringify({ expiresAt: 4_102_444_800 })));
  await page.route("**/api/v1", central(seen));
  await open(page);
  await fill(page);
  await submit(page).click();
  await expect(page.locator("#reset-steps [role=status]")).toHaveText("Securing your password … this takes a moment and happens only in your browser.");
  await expect(page.getByRole("heading", { name: "Your new password is set" })).toBeFocused({ timeout: 60_000 });
  await expect(page.getByText("You start with an empty vault and are signed out everywhere.")).toBeVisible();

  expect(operations(seen)).toEqual(["discover", "reset_start", "reset_finish", "account_summary"]);
  const finish = seen[2] as { attempt: number[]; record: number[]; wrapped_master: number[]; vault: { kind: string; document: number[]; recovery: number[] } };
  expect(Object.keys(finish).sort()).toEqual(["attempt", "operation", "record", "vault", "wrapped_master"]);
  expect(finish.attempt).toEqual(Array(32).fill(7));
  expect(finish.record).toHaveLength(192);
  expect(finish.wrapped_master).toHaveLength(73);
  expect(Object.keys(finish.vault).sort()).toEqual(["document", "kind", "recovery"]);
  expect(finish.vault.kind).toBe("new");
  expect(finish.vault.recovery).toHaveLength(73);
  expect(finish.vault.document.length).toBeGreaterThan(16);

  // the code is the Worker's own and never went to Central; nor did the password
  const shown = (await page.locator("#reset-steps .secret code").textContent()) ?? "";
  expect(shown).toMatch(/^([0-9A-HJKMNP-TV-Z]{4}-){7}[0-9A-HJKMNP-TV-Z]{4}$/);
  expect(JSON.stringify(seen)).not.toContain(shown);
  expect(JSON.stringify(seen)).not.toContain("lantern");
  // the session Central no longer knows is forgotten
  expect(await page.evaluate(() => localStorage.getItem("baylee-central-session"))).toBeNull();
  // the form's fields are gone, and with them what they held
  await expect(page.locator("#reset-steps input[type=password], #reset-steps input[type=text]")).toHaveCount(0);

  await page.getByRole("button", { name: "Copy" }).click();
  await expect(page.locator("#reset-steps [role=status]")).toHaveText("Copied.");
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(shown);
  await expect(page.getByRole("link", { name: "Save as a text file" })).toHaveAttribute(
    "href",
    `data:text/plain;charset=utf-8,Baylee%20recovery%20code%3A%20${shown}%0A`,
  );
  expect(await axe(page)).toEqual([]);
  expect(await targets(page)).toEqual([]);

  // no way on before the person says the code is saved
  const on = page.locator("#reset-steps").getByRole("button", { name: "Sign in" });
  await on.click();
  await expect(page.locator("#reset-steps [role=alert]")).toHaveText("Save the code first, then confirm it.");
  await expect(page).toHaveURL(/\/en\/reset$/);
  await page.getByLabel("I have saved my recovery code").check();
  await on.click();
  await expect(page).toHaveURL(/\/en\/signin$/);
  expect(problems).toEqual([]);
});

test("a finish whose answer is lost is not called done, and the code that may count is still shown", async ({ page }) => {
  const problems = errors(page);
  const seen: Body[] = [];
  await page.route("**/api/v1", central(seen, { finish: "lost" }));
  await open(page);
  await fill(page);
  await submit(page).click();
  await expect(page.getByRole("heading", { name: "No answer came back" })).toBeFocused({ timeout: 60_000 });
  await expect(page.getByRole("heading", { name: "Your new password is set" })).toHaveCount(0);
  await expect(page.getByText("we cannot tell whether your new password was set")).toBeVisible();
  await expect(page.getByText("If your new password does not work, this code is worth nothing.")).toBeVisible();
  await expect(page.locator("#reset-steps .secret code")).toHaveText(/^([0-9A-Z]{4}-){7}[0-9A-Z]{4}$/);
  expect(operations(seen)).toEqual(["discover", "reset_start", "reset_finish"]);
  expect(await axe(page)).toEqual([]);
  expect(await targets(page)).toEqual([]);
  expect(problems).toEqual([]);
});

test("a finish that Central refuses changes nothing and shows no code", async ({ page }) => {
  const problems = errors(page);
  const seen: Body[] = [];
  await page.route("**/api/v1", central(seen, { finish: "refused" }));
  await open(page);
  await fill(page);
  await submit(page).click();
  await expect(page.getByRole("alert")).toHaveText("Something went wrong. Try again; if it keeps happening, tell us.", { timeout: 60_000 });
  await expect(page.locator(".secret")).toHaveCount(0);
  await expect(page.getByLabel("Reset code")).toHaveValue(CODE);
  expect(operations(seen)).toEqual(["discover", "reset_start", "reset_finish"]);
  expect(problems).toEqual([]);
});

test("the sign-in form leads to it by a plain link, on its page and in its dialog", async ({ page, baseURL }) => {
  const problems = watch(page, baseURL ?? "");
  await page.goto("/en/signin");
  await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
  const link = page.getByRole("link", { name: "Set a new one with a reset code" });
  await expect(link).toHaveAttribute("href", "/en/reset");
  await expect(page.getByText("Forgot your password?")).toBeVisible();
  expect(await axe(page)).toEqual([]);
  expect(await targets(page)).toEqual([]);
  await link.click();
  await expect(page).toHaveURL(/\/en\/reset$/);
  await expect(page.getByRole("heading", { level: 1, name: "Set a new password" })).toBeVisible();
  // a form page of its own: no dialog lies over it, and the bar's link
  // leads to the sign-in's page
  await expect(page.locator("dialog.auth-dialog")).toHaveCount(0);

  await page.goto("/en/faq");
  const answer = page.locator("section.qa").filter({ hasText: "I forgot my password" });
  await expect(answer.getByRole("link", { name: "on this page" })).toHaveAttribute("href", "/en/reset");
  await expect(answer).not.toContainText("cannot be redeemed");
  const menu = page.locator(".menu-btn");
  if (await menu.isVisible()) await menu.click();
  await page.locator("a.nav-account").click();
  const dialog = page.locator("#auth-signin");
  await expect(dialog).toBeVisible();
  await expect(dialog.locator("leptos-island")).toHaveAttribute("data-hydrated", "", { timeout: 15_000 });
  expect(await axe(page)).toEqual([]);
  expect(await targets(page)).toEqual([]);
  await dialog.getByRole("link", { name: "Set a new one with a reset code" }).click();
  await expect(page).toHaveURL(/\/en\/reset$/);
  await expect(page.locator("dialog[open]")).toHaveCount(0);
  await expect(page.getByLabel("Reset code")).toBeVisible();
  expect(problems).toEqual([]);
});

test("the page speaks German by hand", async ({ page }) => {
  await open(page, "de");
  await expect(page.getByRole("heading", { level: 1, name: "Neues Passwort setzen" })).toBeVisible();
  await page.getByRole("radio", { name: "Ich habe ihn nicht. Mit einem leeren Tresor neu anfangen." }).check();
  await expect(page.locator("#reset-warning")).toContainText("Niemand kann sie zurückholen, auch wir nicht.");
  expect(await axe(page)).toEqual([]);
  expect(await targets(page)).toEqual([]);
});

test.describe("without script", () => {
  test.use({ javaScriptEnabled: false });

  // Playwright's text matching passes over what a <noscript> holds, so the
  // notice is read from the page itself: the browser made an element of
  // it and drew it, in the form's own box.
  test("the page says that it needs script, and why", async ({ page }) => {
    await page.goto("/en/reset");
    const notice = await page.locator(".auth-form noscript > p.notice").evaluateAll((all) =>
      all.map((element) => ({ text: element.textContent, height: element.getBoundingClientRect().height })),
    );
    expect(notice).toHaveLength(1);
    expect(notice[0].text).toBe(
      "Setting a new password needs JavaScript: your password and your recovery code are processed in your browser and never sent to us.",
    );
    expect(notice[0].height).toBeGreaterThan(20);
    // the page is whole all the same: its fields, their labels, the choice
    await expect(page.locator("leptos-island")).not.toHaveAttribute("data-hydrated", /.*/);
    await expect(page.getByLabel("Reset code")).toBeVisible();
    await expect(page.getByRole("radio", { name: KEEP })).toBeChecked();
  });
});

test.describe("before the islands run", () => {
  // the service worker would fetch the WebAssembly past the page's routes
  test.use({ serviceWorkers: "block" });

  test("a submit sends nothing: the codes and the password stay in the page", async ({ page }) => {
    const posted: string[] = [];
    page.on("request", (request) => {
      if (request.method() !== "GET" && request.method() !== "HEAD") posted.push(`${request.method()} ${request.url()} ${request.postData() ?? ""}`);
    });
    // the WebAssembly never arrives: the island never takes the form over
    await page.route("**/assets/islands.*.wasm", (route) => route.abort());
    await page.goto("/en/reset");
    await page.locator("#reset-name").fill("erika");
    await page.locator("#reset-code").fill(CODE);
    await page.locator("#reset-recovery").fill(RECOVERY);
    await page.locator("#reset-password").fill(PASSWORD);
    await page.locator("#reset-repeat").fill(PASSWORD);
    await page.locator("#reset-repeat").press("Enter");
    await page.locator("#reset-repeat").evaluate((field: HTMLInputElement) => field.form?.requestSubmit());
    await page.waitForTimeout(500);
    await expect(page).toHaveURL(/\/en\/reset$/);
    expect(posted).toEqual([]);
  });
});
