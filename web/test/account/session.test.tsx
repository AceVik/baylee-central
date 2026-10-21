// Who is signed in: the page remembers only the name and the end, asks
// Central once whether that session lives, forgets it on Central's "no",
// at its end, and on signing out, also when Central cannot be reached.

import { act, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { ACCOUNT_KEY, AccountProvider, remembered, useAccount } from "../../src/account/session";
import { KEY, fakeCentral, fakeCrypto } from "../support/central";

const LATER = Math.floor(Date.now() / 1000) + 600;

const store = (v: unknown) => window.localStorage.setItem(ACCOUNT_KEY, JSON.stringify(v));

function Probe() {
  const { account, signOut } = useAccount();
  return (
    <button type="button" onClick={() => void signOut()}>
      {account === null ? "nobody" : `${account.name} until ${account.expiresAt}`}
    </button>
  );
}

describe("the remembered account", () => {
  it("is read only when well-formed and not expired", () => {
    expect(remembered()).toBeNull();
    store({ name: "synthetic_name", expiresAt: LATER });
    expect(remembered()).toEqual({ name: "synthetic_name", expiresAt: LATER });
    for (const bad of [{ name: "synthetic_name", expiresAt: 1 }, { name: "<b>x</b>", expiresAt: LATER }, { name: "ab", expiresAt: LATER }, "x", null]) {
      store(bad);
      expect(remembered()).toBeNull();
    }
    window.localStorage.setItem(ACCOUNT_KEY, "{");
    expect(remembered()).toBeNull();
  });

  it("asks Central nothing when there is none", () => {
    const api = fakeCentral();
    render(
      <AccountProvider api={api} crypto={fakeCrypto("x").factory}>
        <Probe />
      </AccountProvider>,
    );
    expect(screen.getByRole("button").textContent).toBe("nobody");
    expect(api.calls).toEqual([]);
  });

  it("is confirmed by Central, or forgotten when Central has no such session", async () => {
    const api = fakeCentral();
    const crypto = fakeCrypto("synthetic passphrase");
    const { register, signIn } = await import("../../src/account/flows");
    await register({ username: "synthetic_name", betaKey: KEY, password: "synthetic passphrase" }, { api, crypto: crypto.factory });
    await signIn({ username: "synthetic_name", password: "synthetic passphrase" }, { api, crypto: crypto.factory });
    window.localStorage.setItem(ACCOUNT_KEY, JSON.stringify({ name: "synthetic_name", expiresAt: LATER }));
    const first = render(
      <AccountProvider api={api} crypto={crypto.factory}>
        <Probe />
      </AccountProvider>,
    );
    // Central's end wins over the remembered one
    await waitFor(() => expect(screen.getByRole("button").textContent).toBe("synthetic_name until 4000000000"));
    first.unmount();

    api.session = null;
    render(
      <AccountProvider api={api} crypto={crypto.factory}>
        <Probe />
      </AccountProvider>,
    );
    await waitFor(() => expect(screen.getByRole("button").textContent).toBe("nobody"));
    expect(window.localStorage.getItem(ACCOUNT_KEY)).toBeNull();
  });

  it("is kept while Central cannot be reached, and forgotten on signing out even then", async () => {
    const api = fakeCentral();
    api.fail("session_status", "network");
    window.localStorage.setItem(ACCOUNT_KEY, JSON.stringify({ name: "synthetic_name", expiresAt: LATER }));
    render(
      <AccountProvider api={api} crypto={fakeCrypto("x").factory}>
        <Probe />
      </AccountProvider>,
    );
    await waitFor(() => expect(api.calls).toEqual(["session_status"]));
    expect(screen.getByRole("button").textContent).toBe(`synthetic_name until ${LATER}`);
    api.fail("logout", "network");
    await act(async () => screen.getByRole("button").click());
    expect(screen.getByRole("button").textContent).toBe("nobody");
    expect(window.localStorage.getItem(ACCOUNT_KEY)).toBeNull();
  });
});
