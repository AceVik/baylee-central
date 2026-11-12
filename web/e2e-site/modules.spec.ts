import { expect, test, type Page } from "@playwright/test";

import { NAV, nav, open, world } from "./admin-central.ts";
import { open as openFeedback, world as feedbackWorld } from "./feedback-stand-in.ts";

// The islands come as two WebAssembly modules (docs/proposals/frontend-rewrite.md
// §27): the public one, which every page with islands loads, and the
// administration's, which only /{lang}/account/admin… loads. A page loads
// one of them and never the other, and its islands work with it: a public
// page never fetches the administration, and the administration's pages
// never fetch the public module.

// a route does not see what a service worker fetches: the canned Worker
test.use({ serviceWorkers: "block" });

const PUBLIC = /^\/assets\/islands(?:-glue)?\.[0-9a-f]{12}\.(?:js|wasm)$/;
const ADMIN = /^\/assets\/admin-islands(?:-glue)?\.[0-9a-f]{12}\.(?:js|wasm)$/;

/** The islands' files the page fetches from now on: bootstraps, glue and modules. */
function fetched(page: Page): string[] {
  const paths: string[] = [];
  page.on("request", (request) => {
    const path = new URL(request.url()).pathname;
    if (PUBLIC.test(path) || ADMIN.test(path)) paths.push(path);
  });
  return paths;
}

/**
 * What of the module `which` was fetched: its bootstrap, its glue, the
 * module itself. Each once at most, so a page fetched again from the
 * browser's cache counts the same.
 */
const of = (paths: string[], which: RegExp) => [
  ...new Set(
    paths
      .filter((path) => which.test(path))
      .map((path) => (path.endsWith(".wasm") ? "module" : path.includes("-glue.") ? "glue" : "bootstrap")),
  ),
].sort();
const ALL = ["bootstrap", "glue", "module"];

test("a public page with islands loads the public module and nothing of the administration", async ({ page }) => {
  const paths = fetched(page);
  await page.goto("/en/contact");
  await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
  // the form is the island's: its count follows the typing
  await page.locator("#contact-message").fill("Hello");
  await expect(page.locator("#contact-count")).toContainText("5");
  expect(of(paths, PUBLIC)).toEqual(ALL);
  expect(of(paths, ADMIN)).toEqual([]);
});

test("the sign-in dialog over a page takes its island from the public module", async ({ page }) => {
  const paths = fetched(page);
  await page.goto("/en/faq");
  const menu = page.locator(".menu-btn");
  if (await menu.isVisible()) await menu.click();
  await page.locator("a.nav-account").click();
  const dialog = page.locator("#auth-signin");
  await expect(dialog).toBeVisible();
  await expect(dialog.locator("leptos-island")).toHaveAttribute("data-hydrated", "", { timeout: 15_000 });
  expect(of(paths, PUBLIC)).toEqual(ALL);
  expect(of(paths, ADMIN)).toEqual([]);
});

test("the account area and the messenger load the public module, an administrator's too", async ({ page }) => {
  const paths = fetched(page);
  const { problems } = await open(page, world(), "/en/account");
  // the area works, and leads the administrator on
  await expect(page.locator(".account-who")).toBeVisible();
  await expect(page.getByRole("link", { name: "Administration", exact: true })).toHaveAttribute("href", "/en/account/admin/overview");
  await page.goto("/en/account/chat");
  await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
  await expect(page.locator("leptos-island")).toHaveAttribute("data-hydrated", "");
  expect(of(paths, PUBLIC)).toEqual(ALL);
  expect(of(paths, ADMIN)).toEqual([]);
  expect(problems).toEqual([]);
});

test("the administration's pages load the administration's module and not the public one", async ({ page }) => {
  const paths = fetched(page);
  const { problems } = await open(page, world(), "/en/account/admin/overview");
  await expect(nav(page).getByRole("link")).toHaveText(NAV);
  await expect(page.locator(".admin-stats")).toBeVisible();
  await page.goto("/en/account/admin");
  await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
  await expect(nav(page).getByRole("link")).toHaveText(NAV);
  expect(of(paths, ADMIN)).toEqual(ALL);
  expect(of(paths, PUBLIC)).toEqual([]);
  expect(problems).toEqual([]);
});

test("the feedback's pages load the administration's module and not the public one", async ({ page }) => {
  const paths = fetched(page);
  const { problems } = await openFeedback(page, feedbackWorld(), "/en/account/admin/feedback");
  await expect(page.locator(".fb-results .fb-table tbody tr").first()).toBeVisible();
  expect(of(paths, ADMIN)).toEqual(ALL);
  expect(of(paths, PUBLIC)).toEqual([]);
  expect(problems).toEqual([]);
});

test("without the role the administration's module says so, and nothing more", async ({ page }) => {
  const paths = fetched(page);
  const { seen, problems } = await open(page, world({ admin: false }), "/en/account/admin/users");
  await expect(page.getByText("This page is only for administrators.")).toBeVisible();
  expect(seen.filter((body) => body.operation === "admin")).toEqual([]);
  expect(of(paths, ADMIN)).toEqual(ALL);
  expect(of(paths, PUBLIC)).toEqual([]);
  expect(problems).toEqual([]);
});
