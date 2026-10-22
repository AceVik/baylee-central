// Registration and sign-in, step by step, against the stand-in Central and
// crypto: the order of the steps, what each refusal means to the page, that
// a foreign identity or a wrapper that does not open ends the new session,
// that a sign-in to an account without a recovery code makes one in its
// Worker and sends nothing of it, how a kept code is stored (again with the
// same bytes after a lost answer), and that every flow's Worker is closed
// however the flow ends.

import { describe, expect, it } from "vitest";

import { FlowError, register, signIn, storeCode, type Progress } from "../../src/account/flows";
import { CODE, KEY, NEW_CODE, NEW_WRAPPER, fakeCentral, fakeCrypto, fill } from "../support/central";

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
    const { session, summary, code } = await signIn({ username: NAME, password: PASSWORD }, { api, crypto: crypto.factory });
    expect(session.expiresAt).toBe(4_000_000_000);
    expect(api.calls).toEqual(["discover", "login_start", "login_finish", "account_summary"]);
    expect(summary).toMatchObject({ source: "registered", recovery: true });
    // a registered account has its code: the sign-in only confirms the password
    expect(code).toBeNull();
    expect([crypto.confirmed, crypto.created]).toEqual([1, 0]);
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
    expect([crypto.confirmed, crypto.created]).toEqual([0, 0]);
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

/** An account moved from the official gateway, without a recovery code. */
function moved(opts: Parameters<typeof fakeCentral>[0] = {}) {
  const api = fakeCentral(opts);
  api.moved(NAME);
  return { api, crypto: fakeCrypto(PASSWORD) };
}

/** A moved account's sign-in, and the code it made. */
async function made() {
  const { api, crypto } = moved();
  const { code } = await signIn({ username: NAME, password: PASSWORD }, { api, crypto: crypto.factory });
  if (code === null) throw new Error("a code");
  return { api, code };
}

describe("a sign-in to an account moved here without a recovery code", () => {
  it("makes the code in its Worker in place of the confirmation, and sends nothing of it", async () => {
    const { api, crypto } = moved();
    const { summary, code } = await signIn({ username: NAME, password: PASSWORD }, { api, crypto: crypto.factory });
    expect(summary).toMatchObject({ source: "legacy", recovery: false });
    expect(code).toEqual({ code: NEW_CODE, wrapper: NEW_WRAPPER });
    expect([crypto.confirmed, crypto.created]).toEqual([0, 1]);
    expect(api.calls).toEqual(["discover", "login_start", "login_finish", "account_summary"]);
    expect(api.recoveries).toEqual([]);
    expect(api.accounts.get(NAME)?.recovery).toBeNull();
    expect(crypto.opened).toBe(crypto.closed);
  });

  it("only confirms the password when Central does not say whether the account has a code", async () => {
    const { api, crypto } = moved();
    api.fail("account_summary", "network");
    const { summary, code } = await signIn({ username: NAME, password: PASSWORD }, { api, crypto: crypto.factory });
    expect([summary, code]).toEqual([null, null]);
    expect([crypto.confirmed, crypto.created]).toEqual([1, 0]);
    expect(api.session).not.toBeNull();
  });

  it("ends the new session when the wrapper does not open, or the summary names another account", async () => {
    for (const opts of [{ wrapped: fill(73, 9) }, { foreignSummary: true }]) {
      const { api, crypto } = moved(opts);
      expect(await kind(signIn({ username: NAME, password: PASSWORD }, { api, crypto: crypto.factory }))).toBe("mismatch");
      expect(api.calls.at(-1)).toBe("logout");
      expect(api.session).toBeNull();
      expect(crypto.opened).toBe(crypto.closed);
    }
  });
});

describe("storing a kept code", () => {
  it("sends its 73 bytes once, and the same bytes again after a lost answer", async () => {
    const { api, code } = await made();
    api.lose("set_recovery");
    expect(await kind(storeCode(code, { api }))).toBe("unreachable");
    // the answer was lost, the code stored: the retry is the same bytes, and done
    await storeCode(code, { api });
    expect(api.recoveries).toEqual([NEW_WRAPPER, NEW_WRAPPER]);
    expect(api.accounts.get(NAME)?.recovery).toEqual(NEW_WRAPPER);
  });

  it("says when Central refuses the code, and when the account has another", async () => {
    const { api, code } = await made();
    api.late = true;
    expect(await kind(storeCode(code, { api }))).toBe("codeRefused");
    api.late = false;
    const account = api.accounts.get(NAME);
    if (account === undefined) throw new Error("the account");
    account.recovery = fill(73, 12);
    expect(await kind(storeCode(code, { api }))).toBe("codeTaken");
    api.fail("set_recovery", "busy");
    expect(await kind(storeCode(code, { api }))).toBe("busy");
  });
});
