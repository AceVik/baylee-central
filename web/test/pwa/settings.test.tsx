import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { LangProvider } from "../../src/i18n";
import { NotificationSettings } from "../../src/pwa/NotificationSettings";
import type { PushState } from "../../src/pwa/api";
import type { Json } from "../../src/account/api";

const f = vi.hoisted(() => ({
  api: vi.fn<(r: Json) => Promise<PushState>>(),
  state: vi.fn<(nonce: string | null, enabled: number) => Promise<void>>(async () => {}),
  disable: vi.fn<() => Promise<void>>(async () => {}),
  installed: vi.fn<() => string | null>(() => null),
  remember: vi.fn<(nonce: string | null) => void>(),
  get: vi.fn<() => Promise<PushSubscription | null>>(async () => null),
  subscribe: vi.fn<(options: PushSubscriptionOptionsInit) => Promise<PushSubscription>>(),
}));
vi.mock("../../src/pwa/api", () => ({ BITS: { chat: 1, contacts: 2, updates: 4, feedback: 8, gateways: 16 }, notifications: f.api }));
vi.mock("../../src/pwa/client", () => ({ supported: () => true, register: async () => ({ pushManager: { getSubscription: f.get, subscribe: f.subscribe } }), workerState: f.state, disablePush: f.disable, installation: f.installed, rememberInstallation: f.remember, keyBytes: () => new Uint8Array(65) }));
const defaults: PushState = { enabled: 7, publicKey: `B${"a".repeat(86)}`, subscribed: false };
const permission = vi.fn<() => Promise<NotificationPermission>>(async () => "granted");
beforeEach(() => { vi.clearAllMocks(); f.api.mockResolvedValue(defaults); f.installed.mockReturnValue(null); f.get.mockResolvedValue(null); vi.stubGlobal("Notification", { permission: "default", requestPermission: permission }); });
afterEach(() => vi.unstubAllGlobals());
function show(admin = false) { return render(<LangProvider initial="en"><NotificationSettings admin={admin} /></LangProvider>); }

it("offers only ordinary categories to users and the two additional categories to administrators", async () => {
  const view = show();
  await screen.findByRole("switch", { name: "Chat messages" });
  expect(screen.queryByRole("switch", { name: /Feedback/ })).toBeNull();
  expect(screen.queryByRole("switch", { name: /Gateway applications/ })).toBeNull();
  view.rerender(<LangProvider initial="en"><NotificationSettings admin /></LangProvider>);
  expect(screen.getByRole("switch", { name: /Feedback/ })).toBeTruthy();
  expect(screen.getByRole("switch", { name: /Gateway/ })).toBeTruthy();
  expect(permission).not.toHaveBeenCalled();
});
it("changes a single category, preserving the others", async () => {
  show(); const chat = await screen.findByRole("switch", { name: "Chat messages" });
  await waitFor(() => expect(chat.getAttribute("aria-checked")).toBe("true"));
  f.api.mockResolvedValue({ ...defaults, enabled: 6 });
  await userEvent.click(chat);
  expect(f.api).toHaveBeenLastCalledWith({ command: "set", category: "chat", enabled: false });
  expect(chat.getAttribute("aria-checked")).toBe("false");
  expect(screen.getByRole("switch", { name: "Contact requests" }).getAttribute("aria-checked")).toBe("true");
});
it("asks for permission only on a deliberate click, binds the browser capability, then enables worker delivery", async () => {
  const sub = { endpoint: "https://fcm.googleapis.com/fcm/send/test", toJSON: () => ({ keys: { p256dh: "public-key", auth: "auth-key" } }), unsubscribe: vi.fn<() => Promise<boolean>>(async () => true) } as unknown as PushSubscription;
  f.subscribe.mockResolvedValue(sub);
  show();
  const device = await screen.findByRole("switch", { name: "Notifications on this device" });
  await waitFor(() => expect((device as HTMLButtonElement).disabled).toBe(false));
  expect(permission).not.toHaveBeenCalled();
  f.api.mockResolvedValue({ ...defaults, subscribed: true });
  await userEvent.click(device);
  await waitFor(() => expect(device.getAttribute("aria-checked")).toBe("true"));
  expect(permission).toHaveBeenCalledTimes(1);
  expect(f.subscribe).toHaveBeenCalledWith({ userVisibleOnly: true, applicationServerKey: new Uint8Array(65) });
  expect(f.api).toHaveBeenLastCalledWith({ command: "subscribe", subscription: { endpoint: sub.endpoint, p256dh: "public-key", auth: "auth-key", lang: "en", nonce: expect.stringMatching(/^[a-f0-9]{32}$/) } });
  expect(f.state).toHaveBeenLastCalledWith(expect.stringMatching(/^[a-f0-9]{32}$/), 7);
  await userEvent.click(device); expect(f.disable).toHaveBeenCalledTimes(1);
  expect(device.getAttribute("aria-checked")).toBe("false");
});
it("applies category opt-out locally even when the server rejects the network write", async () => {
  vi.stubGlobal("Notification", { permission: "granted", requestPermission: permission });
  f.api.mockResolvedValue({ ...defaults, subscribed: true }); f.installed.mockReturnValue("a".repeat(32));
  show(); const chat = await screen.findByRole("switch", { name: "Chat messages" });
  await waitFor(() => expect(f.state).toHaveBeenCalledWith("a".repeat(32), 7));
  f.api.mockRejectedValue(new Error("offline")); await userEvent.click(chat);
  expect(f.state).toHaveBeenLastCalledWith("a".repeat(32), 6);
  expect(await screen.findByRole("alert")).toBeTruthy();
});
it("does not adopt a subscription for an account after its settings have unmounted", async () => {
  let done!: (sub: PushSubscription | null) => void;
  f.get.mockImplementationOnce(() => new Promise((resolve) => { done = resolve; }));
  const view = show(); await waitFor(() => expect(f.get).toHaveBeenCalled()); view.unmount(); done(null);
  await waitFor(() => expect(f.api).toHaveBeenCalled());
  expect(f.state).not.toHaveBeenCalled();
});

it("withdraws server consent even when the local worker cannot store the opt-out", async () => {
  vi.stubGlobal("Notification", { permission: "granted", requestPermission: permission });
  f.api.mockResolvedValue({ ...defaults, subscribed: true }); f.installed.mockReturnValue("a".repeat(32));
  show(); const chat = await screen.findByRole("switch", { name: "Chat messages" });
  await waitFor(() => expect(f.state).toHaveBeenCalledWith("a".repeat(32), 7));
  f.state.mockRejectedValueOnce(new Error("worker storage unavailable"));
  f.api.mockResolvedValue({ ...defaults, enabled: 6, subscribed: true });
  await userEvent.click(chat);
  expect(f.api).toHaveBeenLastCalledWith({ command: "set", category: "chat", enabled: false });
  expect(chat.getAttribute("aria-checked")).toBe("false");
  expect(f.state).toHaveBeenLastCalledWith("a".repeat(32), 6);
});
