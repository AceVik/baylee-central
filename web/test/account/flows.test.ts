// Registration and sign-in, step by step, against the stand-in Central and
// crypto: the order of the steps, what each refusal means to the page, that
// a foreign identity or a wrapper that does not open ends the new session,
// and that every flow's Worker is closed however the flow ends.

import { describe, expect, it } from "vitest";

import { FlowError, register, signIn, type Progress } from "../../src/account/flows";
import { CODE, KEY, fakeCentral, fakeCrypto, fill } from "../support/central";

const PASSWORD = "synthetic passphrase";
const NAME = "synthetic_name";

async function kind(p: Promise<unknown>): Promise<string> {
  try {
    await p;
  } catch (e) {
    return e instanceof FlowError ? e.kind : "other";
  }
  return "none";
}

describe("registration", () => {
  it("runs Register, EnrollmentStart and EnrollmentFinish and gives the code once", async () => {
    const api = fakeCentral();
    const crypto = fakeCrypto(PASSWORD);
    const said: Progress[] = [];
    const code = await register({ username: NAME, betaKey: KEY, password: PASSWORD }, { api, crypto: crypto.factory }, (p) => said.push(p));
    expect(code).toBe(CODE);
    expect(api.calls).toEqual(["register", "enrollment_start", "enrollment_finish"]);
    expect(said).toEqual(["key", "securing", "saving"]);
    expect(api.accounts.get(NAME)?.recovery).toEqual(fill(73, 4));
    expect([crypto.opened, crypto.closed]).toEqual([1, 1]);
  });

  it("says which refusal it met: the key first, then the name", async () => {
    const api = fakeCentral();
    const crypto = fakeCrypto(PASSWORD);
    const deps = { api, crypto: crypto.factory };
    expect(await kind(register({ username: NAME, betaKey: "CENTRAL-WRONG", password: PASSWORD }, deps))).toBe("badKey");
    await register({ username: NAME, betaKey: KEY, password: PASSWORD }, deps);
    expect(await kind(register({ username: NAME, betaKey: KEY, password: PASSWORD }, deps))).toBe("taken");
    api.fail("register", "busy");
    expect(await kind(register({ username: "other", betaKey: KEY, password: PASSWORD }, deps))).toBe("busy");
    api.fail("enrollment_finish", "unavailable");
    expect(await kind(register({ username: "other", betaKey: KEY, password: PASSWORD }, deps))).toBe("unreachable");
    // no Worker for a refused key; every started one closed
    expect(crypto.opened).toBe(crypto.closed);
  });

  it("calls a crypto that does not run what it is", async () => {
    const crypto = fakeCrypto(PASSWORD, { failStart: true });
    expect(await kind(register({ username: NAME, betaKey: KEY, password: PASSWORD }, { api: fakeCentral(), crypto: crypto.factory }))).toBe("crypto");
    expect(crypto.closed).toBe(1);
  });
});

async function registered(opts: Parameters<typeof fakeCentral>[0] = {}) {
  const api = fakeCentral(opts);
  const crypto = fakeCrypto(PASSWORD);
  await register({ username: NAME, betaKey: KEY, password: PASSWORD }, { api, crypto: crypto.factory });
  api.calls.length = 0;
  return { api, crypto };
}

describe("sign-in", () => {
  it("discovers, runs OPAQUE and checks the master key, keeping the session", async () => {
    const { api, crypto } = await registered();
    const session = await signIn({ username: NAME, password: PASSWORD }, { api, crypto: crypto.factory });
    expect(session.expiresAt).toBe(4_000_000_000);
    expect(api.calls).toEqual(["discover", "login_start", "login_finish"]);
    expect(api.session).not.toBeNull();
    expect(crypto.opened).toBe(crypto.closed);
  });

  it("cannot tell an unknown name from a wrong password, and sends nothing after a wrong one", async () => {
    const { api, crypto } = await registered();
    expect(await kind(signIn({ username: "nobody", password: PASSWORD }, { api, crypto: crypto.factory }))).toBe("wrongLogin");
    api.calls.length = 0;
    expect(await kind(signIn({ username: NAME, password: "wrong passphrase" }, { api, crypto: crypto.factory }))).toBe("wrongLogin");
    expect(api.calls).toEqual(["discover", "login_start"]);
    expect(api.session).toBeNull();
    expect(crypto.opened).toBe(crypto.closed);
  });

  it("ends the new session when Central answers another identity or a wrapper that does not open", async () => {
    for (const opts of [{ foreignIdentity: true }, { wrapped: fill(73, 9) }]) {
      const { api, crypto } = await registered(opts);
      expect(await kind(signIn({ username: NAME, password: PASSWORD }, { api, crypto: crypto.factory }))).toBe("mismatch");
      expect(api.calls.at(-1)).toBe("logout");
      expect(api.session).toBeNull();
      expect(crypto.opened).toBe(crypto.closed);
    }
  });

  it("passes on busy and unreachable", async () => {
    const { api, crypto } = await registered();
    api.fail("discover", "busy");
    expect(await kind(signIn({ username: NAME, password: PASSWORD }, { api, crypto: crypto.factory }))).toBe("busy");
    api.fail("login_finish", "unavailable");
    expect(await kind(signIn({ username: NAME, password: PASSWORD }, { api, crypto: crypto.factory }))).toBe("unreachable");
    api.fail("login_start", "network");
    expect(await kind(signIn({ username: NAME, password: PASSWORD }, { api, crypto: crypto.factory }))).toBe("unreachable");
  });
});
