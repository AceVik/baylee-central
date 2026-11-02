import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { AccountProvider, useAccount } from "../../src/account/session";
import { adminApi, type Overview } from "../../src/admin/admin-api";
import { AdminFrame } from "../../src/admin/shared";
import { LangProvider } from "../../src/i18n";
import { fakeCentral } from "../support/central";

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: Error) => void;
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}
const counts = (pending: number): Overview => ({ accounts: 12, accountsWeek: 3, moved: 4, admins: 2, pending, approved: 5, disabled: 1, online: 4, liveTokens: 7 });

function fixture() {
  const central = fakeCentral();
  central.moved("synthetic_admin", { displayName: "Synthetic_Admin", tag: 7 });
  central.session = { name: "synthetic_admin", expiresAt: 4_000_000_000 };
  const summary = central.accountSummary.bind(central);
  central.accountSummary = async () => ({ ...await summary(), admin: true });
  const overview = vi.fn<() => Promise<Overview>>();
  const api = { ...adminApi(), overview };
  function Page() {
    const account = useAccount();
    return <>
      <button onClick={() => account.signedIn({ handle: "Synthetic_Admin#0007", expiresAt: 4_000_000_000 })}>Switch</button>
      <AdminFrame api={api} page="admin" title="Admin">{(context) => <>
        <button onClick={context.recount}>Refresh</button>
        <output data-testid="counts">{context.uncounted ? "failed" : context.overview?.pending ?? "loading"}</output>
      </>}</AdminFrame>
    </>;
  }
  render(<LangProvider initial="en"><AccountProvider api={central}><Page /></AccountProvider></LangProvider>);
  return { overview, signIn: () => fireEvent.click(screen.getByRole("button", { name: "Switch" })), recount: () => fireEvent.click(screen.getByRole("button", { name: "Refresh" })) };
}

it("drops an old sign-in's pending admin counts when authentication changes", async () => {
  const f = fixture();
  const old = deferred<Overview>();
  const next = deferred<Overview>();
  f.overview.mockImplementationOnce(() => old.promise).mockImplementationOnce(() => next.promise);
  f.signIn();
  await waitFor(() => expect(f.overview).toHaveBeenCalledTimes(1));
  f.signIn();
  await waitFor(() => expect(f.overview).toHaveBeenCalledTimes(2));
  await act(async () => { old.resolve(counts(99)); });
  expect(screen.getByTestId("counts").textContent).toBe("loading");
  await act(async () => { next.resolve(counts(2)); });
  expect(screen.getByTestId("counts").textContent).toBe("2");
});

it("clears already loaded counts on a new sign-in", async () => {
  const f = fixture();
  const next = deferred<Overview>();
  f.overview.mockResolvedValueOnce(counts(99)).mockImplementationOnce(() => next.promise);
  f.signIn();
  await waitFor(() => expect(screen.getByTestId("counts").textContent).toBe("99"));
  f.signIn();
  await waitFor(() => expect(f.overview).toHaveBeenCalledTimes(2));
  expect(screen.getByTestId("counts").textContent).toBe("loading");
  await act(async () => { next.resolve(counts(2)); });
});

it.each(["success", "failure"])("ignores an earlier refresh's late %s", async (outcome) => {
  const f = fixture();
  const old = deferred<Overview>();
  f.overview.mockResolvedValueOnce(counts(1)).mockImplementationOnce(() => old.promise).mockResolvedValueOnce(counts(2));
  f.signIn();
  await waitFor(() => expect(screen.getByTestId("counts").textContent).toBe("1"));
  f.recount();
  f.recount();
  await waitFor(() => expect(screen.getByTestId("counts").textContent).toBe("2"));
  await act(async () => {
    if (outcome === "success") old.resolve(counts(99));
    else old.reject(new Error("synthetic network failure"));
  });
  expect(screen.getByTestId("counts").textContent).toBe("2");
});
