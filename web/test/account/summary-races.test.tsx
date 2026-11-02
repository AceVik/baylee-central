import { act, renderHook, waitFor } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { AccountProvider, useAccount } from "../../src/account/session";
import type { AccountSummary } from "../../src/account/api";
import { useSummary } from "../../src/app/useSummary";
import { fakeCentral } from "../support/central";
import type { ReactNode } from "react";

function deferred<T>() { let resolve!: (value: T) => void; const promise = new Promise<T>((done) => { resolve = done; }); return { promise, resolve }; }
async function fixture() {
  const api = fakeCentral(); api.moved("old", { displayName: "Old_User", tag: 7 }); api.session = { name: "old", expiresAt: 4_000_000_000 };
  const old = await api.accountSummary();
  const next = { ...old, displayName: "New_User", tag: 8 };
  const wrapper = ({ children }: { children: ReactNode }) => <AccountProvider api={api}>{children}</AccountProvider>;
  const hook = renderHook(() => ({ account: useAccount(), summary: useSummary() }), { wrapper });
  return { api, old, next, ...hook };
}

it("a delayed old-account summary cannot replace the new account", async () => {
  const f = await fixture(); const pending = deferred<AccountSummary>();
  f.api.accountSummary = vi.fn<typeof f.api.accountSummary>().mockImplementationOnce(() => pending.promise).mockResolvedValue(f.next);
  act(() => f.result.current.account.signedIn({ handle: "Old_User#0007", expiresAt: 4_000_000_000 }));
  await waitFor(() => expect(f.api.accountSummary).toHaveBeenCalledTimes(1));
  act(() => f.result.current.account.signedIn({ handle: "New_User#0008", expiresAt: 4_000_000_000 }));
  await waitFor(() => expect(f.result.current.summary.state).toEqual({ at: "ready", summary: f.next }));
  await act(async () => { pending.resolve(f.old); });
  expect(f.result.current.summary.state).toEqual({ at: "ready", summary: f.next });
});

it("an explicit replacement wins over an earlier reload", async () => {
  const f = await fixture();
  act(() => f.result.current.account.signedIn({ handle: "Old_User#0007", expiresAt: 4_000_000_000 }));
  await waitFor(() => expect(f.result.current.summary.state.at).toBe("ready"));
  const pending = deferred<AccountSummary>(); f.api.accountSummary = vi.fn<typeof f.api.accountSummary>(() => pending.promise);
  act(() => { f.result.current.summary.reload(); f.result.current.summary.replace(f.next); });
  await act(async () => { pending.resolve(f.old); });
  expect(f.result.current.summary.state).toEqual({ at: "ready", summary: f.next });
});

it("sign-out clears local state before the network replies and cannot erase a later sign-in", async () => {
  const f = await fixture(); const pending = deferred<void>(); f.api.logout = vi.fn<typeof f.api.logout>(() => pending.promise);
  act(() => f.result.current.account.signedIn({ handle: "Old_User#0007", expiresAt: 4_000_000_000 }));
  let ending!: Promise<boolean>;
  act(() => { ending = f.result.current.account.signOut(); });
  expect(f.result.current.account.account).toBeNull();
  act(() => f.result.current.account.signedIn({ handle: "New_User#0008", expiresAt: 4_000_000_000 }));
  await act(async () => { pending.resolve(); await ending; });
  expect(f.result.current.account.account?.handle).toBe("New_User#0008");
});
