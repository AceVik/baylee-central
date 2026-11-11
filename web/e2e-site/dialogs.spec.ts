import { expect, test, type Page } from "@playwright/test";

import { axe, targets, watch } from "./check.ts";

/** Presses the bar's sign-in link; on a phone it lies in the menu. */
async function ask(page: Page): Promise<void> {
  const menu = page.locator(".menu-btn");
  if (await menu.isVisible()) await menu.click();
  await page.locator("a.nav-account").click();
}

test("signing in opens over the page that asked, and fetches its WebAssembly only then", async ({ page, baseURL }) => {
  const problems = watch(page, baseURL ?? "");
  const wasm: string[] = [];
  page.on("request", (request) => {
    if (request.url().endsWith(".wasm")) wasm.push(request.url());
  });
  await page.goto("/en/faq");
  await page.waitForLoadState("load");
  await page.waitForTimeout(1200);
  expect(wasm).toEqual([]);
  const dialog = page.locator("#auth-signin");
  await expect(dialog).toBeHidden();

  await ask(page);
  await expect(dialog).toBeVisible();
  await expect(page).toHaveURL(/\/en\/faq$/);
  // the keyboard is in the first field, and what is typed before the
  // island runs stays
  const name = dialog.getByLabel("Sign-in name");
  await expect(name).toBeFocused();
  await name.fill("early_erika");
  await expect(dialog.locator("leptos-island")).toHaveAttribute("data-hydrated", "", { timeout: 15_000 });
  expect(wasm).toHaveLength(1);
  await expect(name).toHaveValue("early_erika");
  // the password never has a name a browser could send it under
  const password = dialog.locator("#signin-password");
  await expect(password).not.toHaveAttribute("name", /.*/);
  await expect(name).not.toHaveAttribute("name", /.*/);

  // the eye inside the field shows what was typed, and hides it again
  const eye = dialog.getByRole("button", { name: "Show the password" });
  await password.fill("violet lantern harbour");
  await expect(password).toHaveAttribute("type", "password");
  await eye.click();
  await expect(eye).toHaveAttribute("aria-pressed", "true");
  await expect(password).toHaveAttribute("type", "text");
  await expect(password).toHaveValue("violet lantern harbour");
  await eye.click();
  await expect(password).toHaveAttribute("type", "password");

  expect(await axe(page)).toEqual([]);
  expect(await targets(page)).toEqual([]);

  // an empty form is refused in place
  await name.fill("");
  await dialog.getByRole("button", { name: "Sign in", exact: true }).click();
  await expect(dialog.getByRole("alert")).toHaveText("Enter your sign-in name and password.");
  expect(problems).toEqual([]);
});

test("the two dialogs lead to each other, and close by Escape, their button and the veil", async ({ page }) => {
  await page.goto("/en/");
  await ask(page);
  const signin = page.locator("#auth-signin");
  const register = page.locator("#auth-register");
  await expect(signin).toBeVisible();

  await signin.getByRole("link", { name: "Create one" }).click();
  await expect(register).toBeVisible();
  await expect(signin).toBeHidden();
  await expect(page).toHaveURL(/\/en\/$/);
  await expect(register.getByLabel("Sign-in name")).toBeFocused();
  await expect(register.locator("leptos-island")).toHaveAttribute("data-hydrated", "", { timeout: 15_000 });
  // each password has an eye of its own
  const eyes = register.getByRole("button", { name: "Show the password" });
  await expect(eyes).toHaveCount(2);
  await eyes.first().click();
  await expect(register.locator("#register-password")).toHaveAttribute("type", "text");
  await expect(register.locator("#register-repeat")).toHaveAttribute("type", "password");
  expect(await axe(page)).toEqual([]);
  expect(await targets(page)).toEqual([]);

  await register.getByRole("link", { name: "Sign in" }).click();
  await expect(signin).toBeVisible();
  await expect(register).toBeHidden();

  await page.keyboard.press("Escape");
  await expect(signin).toBeHidden();

  await ask(page);
  await expect(signin).toBeVisible();
  await signin.getByRole("button", { name: "Close" }).click();
  await expect(signin).toBeHidden();

  await ask(page);
  await expect(signin).toBeVisible();
  await page.mouse.click(4, 4);
  await expect(signin).toBeHidden();
});

test("a link to the registration in a page's text opens the dialog too; a modifier key keeps the link", async ({ page, context }) => {
  await page.goto("/en/");
  const link = page.locator('main a[href$="/register"]').first();
  await link.scrollIntoViewIfNeeded();
  await link.click();
  await expect(page.locator("#auth-register")).toBeVisible();
  await page.keyboard.press("Escape");
  const opened = context.waitForEvent("page");
  await link.click({ modifiers: ["ControlOrMeta"] });
  const tab = await opened;
  await tab.waitForLoadState("domcontentloaded");
  expect(new URL(tab.url()).pathname).toBe("/en/register");
  await expect(page.locator("#auth-register")).toBeHidden();
});

test("the pages for signing in and registering stay pages, without a dialog of their own", async ({ page }) => {
  for (const slug of ["signin", "register"]) {
    await page.goto(`/en/${slug}`);
    await expect(page.locator("dialog.auth-dialog")).toHaveCount(0);
    await expect(page.locator("html")).toHaveAttribute("data-islands", "ready", { timeout: 15_000 });
    const eye = page.getByRole("button", { name: "Show the password" }).first();
    await eye.click();
    await expect(eye).toHaveAttribute("aria-pressed", "true");
  }
});

test("without script the link leads to the page", async ({ browser }) => {
  const context = await browser.newContext({ javaScriptEnabled: false });
  const page = await context.newPage();
  await page.goto("/en/faq");
  const menu = page.locator(".menu-btn");
  if (await menu.isVisible()) await menu.click();
  await page.locator("a.nav-account").click();
  await expect(page).toHaveURL(/\/en\/signin$/);
  await context.close();
});
