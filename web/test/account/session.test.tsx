// Who is signed in: the page keeps only when the session ends, asks Central
// once whose session a remembered one is, forgets it on Central's "no", at
// its end, and on signing out, also when Central cannot be reached. The
// name is Central's alone; the old stored name is deleted on sight.

import { act, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { OLD_ACCOUNT_KEY, SESSION_KEY, AccountProvider, remembered, useAccount } from "../../src/account/session";
import { KEY, fakeCentral, fakeCrypto } from "../support/central";

const LATER = Math.floor(Date.now() / 1000) + 600;
const store = (v: unknown) => window.localStorage.setItem(SESSION_KEY, JSON.stringify(v));

function Probe() {
  const { account, signOut } = useAccount();
  return (
    <button type="button" onClick={() => void signOut()}>
      {account === null ? "nobody" : `${account.name ?? "(unnamed)"} until ${account.expiresAt}`}
    </button>
  );
}

async function signedInCentral(serverName?: string) {
  const api = fakeCentral(serverName === undefined ? {} : { serverName });
  const crypto = fakeCrypto("synthetic passphrase");
  const { register, signIn } = await import("../../src/account/flows");
  await register({ username: "synthetic_name", betaKey: KEY, password: "synthetic passphrase" }, { api, crypto: crypto.factory });
  await signIn({ username: "synthetic_name", password: "synthetic passphrase" }, { api, crypto: crypto.factory });
  api.calls.length = 0;
  return { api, crypto };
}

describe("the remembered session", () => {
  it("is only its end, read only when well-formed and not over", () => {
    expect(remembered()).toBeNull();
    store({ expiresAt: LATER });
    expect(remembered()).toEqual({ name: null, expiresAt: LATER });
    for (const bad of [{ expiresAt: 1 }, { expiresAt: LATER, name: "synthetic_name" }, { expiresAt: String(LATER) }, [LATER], "x", null]) {
      store(bad);
      expect(remembered()).toBeNull();
    }
    window.localStorage.setItem(SESSION_KEY, "{");
    expect(remembered()).toBeNull();
  });

  it("deletes the name an earlier version kept", () => {
    window.localStorage.setItem(OLD_ACCOUNT_KEY, JSON.stringify({ name: "synthetic_name", expiresAt: LATER }));
    expect(remembered()).toBeNull();
    expect(window.localStorage.getItem(OLD_ACCOUNT_KEY)).toBeNull();
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

  it("is named by Central, or forgotten when Central has no such session", async () => {
    const { api, crypto } = await signedInCentral("central_name");
    store({ expiresAt: LATER });
    const first = render(
      <AccountProvider api={api} crypto={crypto.factory}>
        <Probe />
      </AccountProvider>,
    );
    expect(screen.getByRole("button").textContent).toBe(`(unnamed) until ${LATER}`);
    // Central's name and end win
    await waitFor(() => expect(screen.getByRole("button").textContent).toBe("central_name until 4000000000"));
    expect(api.calls).toEqual(["session_status"]);
    expect(JSON.parse(window.localStorage.getItem(SESSION_KEY) ?? "{}")).toEqual({ expiresAt: 4_000_000_000 });
    first.unmount();

    api.session = null;
    render(
      <AccountProvider api={api} crypto={crypto.factory}>
        <Probe />
      </AccountProvider>,
    );
    await waitFor(() => expect(screen.getByRole("button").textContent).toBe("nobody"));
    expect(window.localStorage.getItem(SESSION_KEY)).toBeNull();
  });

  it("stays unnamed while Central cannot be reached, and is forgotten on signing out even then", async () => {
    const api = fakeCentral();
    api.fail("session_status", "network");
    store({ expiresAt: LATER });
    render(
      <AccountProvider api={api} crypto={fakeCrypto("x").factory}>
        <Probe />
      </AccountProvider>,
    );
    await waitFor(() => expect(api.calls).toEqual(["session_status"]));
    expect(screen.getByRole("button").textContent).toBe(`(unnamed) until ${LATER}`);
    api.fail("logout", "network");
    await act(async () => screen.getByRole("button").click());
    expect(screen.getByRole("button").textContent).toBe("nobody");
    expect(window.localStorage.getItem(SESSION_KEY)).toBeNull();
  });
});
