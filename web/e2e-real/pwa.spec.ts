// Real browser service worker, persistent consent, OS-notification API and offline
// shell. CDP injects a provider delivery; no external push provider or user is used.
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { expect, test, type BrowserContext } from "@playwright/test";

test("installable public shell and push consent survive page closure without caching account data", async ({ playwright }, info) => {
  // The headless-shell binary denies notification permission. Full Chromium
  // supports this API headlessly (Playwright issue #23954). Use an owned,
  // temporary profile, never the user's browser profile.
  const profile = mkdtempSync(join(tmpdir(), "central-pwa-"));
  let owned: BrowserContext | undefined;
  try {
  const context = await playwright.chromium.launchPersistentContext(profile, {
    channel: "chromium", headless: true, baseURL: "https://baylee.app", ignoreHTTPSErrors: true,
    viewport: info.project.use.viewport ?? { width: 1366, height: 900 },
    args: info.project.use.launchOptions?.args ?? [],
  });
  owned = context;
  const page = context.pages()[0] ?? await context.newPage();
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await context.grantPermissions(["notifications"]);
  await context.route("**/*", (route) => new URL(route.request().url()).hostname === "baylee.app" ? route.continue() : route.fulfill({ status: 503, body: "external services disabled for QA" }));
  const cdp = await context.newCDPSession(page);
  cdp.on("ServiceWorker.workerErrorReported", (event: { errorMessage: { errorMessage: string } }) => errors.push(event.errorMessage.errorMessage));
  let registrationId = "";
  cdp.on("ServiceWorker.workerRegistrationUpdated", (event: { registrations: { scopeURL: string; registrationId: string }[] }) => {
    registrationId = event.registrations.find((r) => r.scopeURL === "https://baylee.app/")?.registrationId ?? registrationId;
  });
  await cdp.send("ServiceWorker.enable");
  await page.goto("/account?lang=en");
  const registrationError = await page.evaluate(async () => { try { await navigator.serviceWorker.register("/sw.js", { scope: "/" }); return null; } catch (error) { return String(error); } });
  expect(registrationError).toBeNull();
  await expect.poll(() => page.evaluate(async () => Boolean((await navigator.serviceWorker.getRegistration("/"))?.active))).toBe(true);
  await expect.poll(() => registrationId).not.toBe("");
  const manifest = await cdp.send("Page.getAppManifest");
  expect(manifest.errors).toEqual([]);
  const app = JSON.parse(manifest.data ?? "{}") as { display: string; start_url: string; icons: unknown[] };
  expect(app.display).toBe("standalone"); expect(app.start_url).toBe("/account"); expect(app.icons.length).toBeGreaterThan(0);
  const install = await cdp.send("Page.getInstallabilityErrors"); expect(install.installabilityErrors).toEqual([]);
  const nonce = "0123456789abcdef0123456789abcdef";
  const setConsent = async (enabled: number) => page.evaluate(async ({ nonce: value, enabled: bits }) => {
    const r = await navigator.serviceWorker.ready;
    await new Promise<void>((resolve, reject) => {
      const channel = new MessageChannel(); const timer = setTimeout(() => reject(new Error("no worker acknowledgement")), 5000);
      channel.port1.addEventListener("message", (event: MessageEvent<{ ok: boolean }>) => { clearTimeout(timer); channel.port1.close(); if (event.data.ok) resolve(); else reject(new Error("storage unavailable")); }, { once: true });
      channel.port1.start(); r.active?.postMessage({ type: "PUSH_STATE", nonce: value, enabled: bits }, [channel.port2]);
    });
  }, { nonce, enabled });
  await setConsent(1);
  const sw = context.serviceWorkers()[0];
  if (!sw) throw new Error("missing service worker");
  expect(await page.evaluate(() => Notification.permission)).toBe("granted");
  expect(await sw.evaluate(() => Notification.permission)).toBe("granted");
  await sw.evaluate(() => {
    const probe = globalThis as unknown as { pushEvents: number };
    probe.pushEvents = 0;
    globalThis.addEventListener("push", () => { probe.pushEvents += 1; });
  });
  // Close the application document while keeping only the test's neutral target.
  await page.goto("about:blank");
  expect(await sw.evaluate(() => Notification.permission)).toBe("granted");
  await cdp.send("ServiceWorker.deliverPushMessage", { origin: "https://baylee.app", registrationId, data: JSON.stringify({ v: 1, kind: "chat", lang: "de", nonce }) });
  await page.goto("/account?lang=en");
  const notices = () => page.evaluate(async () => (await (await navigator.serviceWorker.ready).getNotifications()).map((n) => ({ body: n.body, tag: n.tag })));
  const pushes = await sw.evaluate(() => (globalThis as unknown as { pushEvents: number }).pushEvents);
  expect({ pushes, errors }).toEqual({ pushes: 1, errors: [] });
  await expect.poll(notices).toEqual([{ body: "Du hast eine neue Chat-Nachricht.", tag: "baylee-chat" }]);
  await setConsent(2); await expect.poll(notices).toEqual([]);
  await cdp.send("ServiceWorker.deliverPushMessage", { origin: "https://baylee.app", registrationId, data: JSON.stringify({ v: 1, kind: "chat", lang: "en", nonce }) });
  await cdp.send("ServiceWorker.deliverPushMessage", { origin: "https://baylee.app", registrationId, data: JSON.stringify({ v: 1, kind: "contacts", lang: "en", nonce }) });
  await expect.poll(notices).toEqual([{ body: "You have a new contact request.", tag: "baylee-contacts" }]);
  await setConsent(0); await expect.poll(notices).toEqual([]);
  const cached = await page.evaluate(async () => (await Promise.all((await caches.keys()).map(async (key) => (await (await caches.open(key)).keys()).map((r) => new URL(r.url).pathname)))).flat());
  expect(cached).toContain("/index.html");
  expect(cached.some((path) => path.startsWith("/api/") || path.startsWith("/account") || path.startsWith("/admin"))).toBe(false);
  await context.setOffline(true);
  await page.goto("/account/contacts?lang=en");
  await expect(page.getByRole("heading", { name: "Contacts", exact: true })).toBeVisible();
  await context.setOffline(false);
  expect(errors).toEqual([]);
  } finally { await owned?.close(); rmSync(profile, { recursive: true, force: true }); }
});
