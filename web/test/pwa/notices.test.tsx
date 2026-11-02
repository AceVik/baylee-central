import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, expect, it, vi } from "vitest";
import { LangProvider } from "../../src/i18n";
import { InstallNotice } from "../../src/pwa/InstallNotice";
import { PushNotice } from "../../src/pwa/PushNotice";
import { rememberInstallation } from "../../src/pwa/client";

const f = vi.hoisted(() => ({ known: true, generation: 1 }));
vi.mock("../../src/account/session", () => ({ useAccount: () => ({ account: { known: f.known }, generation: f.generation }) }));
const permission = vi.fn<() => Promise<NotificationPermission>>(async () => "granted");
beforeEach(() => {
  f.known = true; f.generation = 1; permission.mockClear();
  vi.stubGlobal("Notification", { permission: "granted", requestPermission: permission });
  vi.stubGlobal("PushManager", {}); vi.stubGlobal("isSecureContext", true);
  vi.stubGlobal("navigator", { serviceWorker: {} });
});
function showPush() { return render(<LangProvider initial="en"><PushNotice /></LangProvider>); }
function showInstall() { return render(<LangProvider initial="en"><InstallNotice /></LangProvider>); }
const installPrompt = (prompt: () => Promise<unknown>) => Object.assign(new Event("beforeinstallprompt", { cancelable: true }), { prompt });

it("offers activation after logout even if browser permission and an old dismissal persist", () => {
  localStorage.setItem("baylee-push-prompt-v1", "later");
  showPush();
  expect(screen.getByRole("link", { name: "Choose notifications" }).getAttribute("href")).toBe("/account#notifications");
  expect(permission).not.toHaveBeenCalled();
});
it("keeps Later within this sign-in and offers it again after reauthentication", async () => {
  const view = showPush();
  await userEvent.click(screen.getByRole("button", { name: "Later" }));
  view.rerender(<LangProvider initial="en"><PushNotice /></LangProvider>);
  expect(screen.queryByRole("link", { name: "Choose notifications" })).toBeNull();
  ++f.generation;
  view.rerender(<LangProvider initial="en"><PushNotice /></LangProvider>);
  expect(screen.getByRole("link", { name: "Choose notifications" })).toBeTruthy();
});
it("hides immediately when enabled and respects an explicit opt-out for this sign-in", () => {
  showPush();
  act(() => rememberInstallation("a".repeat(32)));
  expect(screen.queryByRole("link", { name: "Choose notifications" })).toBeNull();
  act(() => rememberInstallation(null));
  expect(screen.queryByRole("link", { name: "Choose notifications" })).toBeNull();
});
it("does not request permission or offer it to a signed-out user or a blocked browser", () => {
  f.known = false; const view = showPush();
  expect(screen.queryByRole("link")).toBeNull();
  f.known = true; vi.stubGlobal("Notification", { permission: "denied", requestPermission: permission });
  view.rerender(<LangProvider initial="en"><PushNotice /></LangProvider>);
  expect(screen.queryByRole("link")).toBeNull();
  expect(permission).not.toHaveBeenCalled();
});
it("offers installation only after browser readiness and consumes its event on a deliberate click once", async () => {
  showInstall(); expect(screen.queryByRole("button", { name: "Install Baylee" })).toBeNull();
  const prompt = vi.fn<() => Promise<unknown>>(async () => ({ outcome: "accepted" }));
  const event = installPrompt(prompt);
  act(() => { window.dispatchEvent(event); });
  expect(event.defaultPrevented).toBe(true); expect(prompt).not.toHaveBeenCalled();
  await userEvent.click(screen.getByRole("button", { name: "Install Baylee" }));
  expect(prompt).toHaveBeenCalledTimes(1);
  expect(screen.queryByRole("button", { name: "Install Baylee" })).toBeNull();
});
it("clears installation offers when installed and never intercepts an installed app's event", () => {
  const view = showInstall();
  const prompt = vi.fn<() => Promise<unknown>>(async () => ({}));
  act(() => { window.dispatchEvent(installPrompt(prompt)); window.dispatchEvent(new Event("appinstalled")); });
  expect(screen.queryByRole("button", { name: "Install Baylee" })).toBeNull();
  vi.stubGlobal("navigator", { standalone: true });
  const event = installPrompt(prompt); act(() => { window.dispatchEvent(event); });
  expect(event.defaultPrevented).toBe(false); expect(prompt).not.toHaveBeenCalled();
  view.unmount();
  vi.stubGlobal("navigator", {});
  const after = installPrompt(prompt); window.dispatchEvent(after); expect(after.defaultPrevented).toBe(false);
});
it("explains a failed install prompt without an unhandled rejection or automatic retry", async () => {
  showInstall();
  const prompt = vi.fn<() => Promise<unknown>>(async () => { throw new Error("browser refused"); });
  act(() => { window.dispatchEvent(installPrompt(prompt)); });
  await userEvent.click(screen.getByRole("button", { name: "Install Baylee" }));
  expect((await screen.findByRole("alert")).textContent).toContain("browser menu");
  expect(prompt).toHaveBeenCalledTimes(1);
});
