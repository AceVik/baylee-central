// Stand-ins for the account pages' two collaborators: Central's browser
// surface as an in-memory `Api` (the shapes of docs/web-accounts.md, with
// its refusals, an account moved from the official gateway without a
// recovery code, and display names with the tags drawn for them) and the
// crypto Worker as a `CryptoFactory` that knows the right password. Synthetic data only. The real crypto is tested in Rust
// (crates/central-web-crypto) and test/central-wasm.test.ts; the real HTTP
// shapes in test/account/api.test.ts and e2e/account.spec.ts.

import { reservedDisplayName, validDisplayName } from "../../src/account/names";
import { ApiError, type AccountSource, type AccountSummary, type Api, type BrowserSession, type Identity, type TermsDocument, type TermsVersion } from "../../src/account/api";
import { CryptoError, type Crypto, type CryptoFactory } from "../../src/account/crypto";

export const fill = (n: number, byte: number) => new Uint8Array(n).fill(byte);

export const ACCOUNT = Uint8Array.from([0x01, 0x92, 0xd0, 0xc5, 0x7b, 0x3e, 0x7a, 0x1f, 0x8c, 0x2d, 0x3e, 0x4f, 0x5a, 0x6b, 0x7c, 0x8d]);
export const KEY = "CENTRAL-TEST-0000-0000-0001";
export const CODE = "Y06Z-NKPA-ZTZE-Y093-8NKR-KAYD-XZZD-SEMR";
export const WRAPPED = fill(73, 2);
/** The code the stand-in crypto makes after a sign-in, and its wrapper. */
export const NEW_CODE = "7Q3K-AXWD-0P4N-9RTB-5MHE-ZC2G-8VFJ-1YS6";
export const NEW_WRAPPER = fill(73, 11);
export const SIGNED_IN_AT = 3_999_998_200;
/** The tag the stand-in draws for a registration: `#3fa2`. */
export const TAG = 0x3fa2;

export interface FakeAccount {
  identity: Identity;
  wrapped: Uint8Array;
  /** The recovery wrapper; none for an account moved from the official gateway. */
  recovery: Uint8Array | null;
  source?: AccountSource;
  displayName?: string | null;
  tag?: number | null;
}

export interface FakeCentral extends Api {
  calls: string[];
  accounts: Map<string, FakeAccount>;
  session: { name: string; expiresAt: number } | null;
  current: TermsDocument | null;
  accepted: TermsVersion | null;
  /** Every wrapper `setRecovery` was sent, in order. */
  recoveries: Uint8Array[];
  /** Every name `setDisplayName` was sent, in order. */
  names: string[];
  /** Refuse `setRecovery` for an account without a wrapper, as for a sign-in 10 minutes old. */
  late: boolean;
  /** Make the next call to `operation` fail with `failure`. */
  fail(operation: string, failure: ApiError["kind"]): void;
  /** Let the next call to `operation` take effect, then lose its answer. */
  lose(operation: string): void;
  /** An account moved from the official gateway: enrolled, no recovery code; its name and tag as carried, if they were. */
  moved(name: string, profile?: { displayName: string | null; tag: number | null }): void;
}

/** `serverName`: the name Central gives the account, if not its own. */
export function fakeCentral(
  opts: { terms?: TermsDocument | null; foreignIdentity?: boolean; foreignSummary?: boolean; wrapped?: Uint8Array; serverName?: string } = {},
): FakeCentral {
  const failures = new Map<string, ApiError["kind"]>();
  const lost = new Set<string>();
  const tickets = new Map<string, { name: string; identity: Identity; displayName: string | null }>();
  let attempt: string | null = null;
  const c: FakeCentral = {
    calls: [],
    accounts: new Map(),
    session: null,
    current: opts.terms ?? null,
    accepted: null,
    recoveries: [],
    names: [],
    late: false,
    fail: (op: string, kind: ApiError["kind"]) => {
      failures.set(op, kind);
    },
    lose: (op: string) => {
      lost.add(op);
    },
    moved: (name: string, profile = { displayName: null, tag: null }) => {
      c.accounts.set(name, { identity: { account: ACCOUNT, epoch: 1 }, wrapped: WRAPPED, recovery: null, source: "legacy", ...profile });
    },
  } as unknown as FakeCentral;
  const said = (name: string) => opts.serverName ?? name;
  const step = (op: string) => {
    c.calls.push(op);
    const f = failures.get(op);
    if (f !== undefined) {
      failures.delete(op);
      throw new ApiError(f);
    }
  };
  Object.assign(c, {
    async register(username: string, betaKey: string, displayName: string | null) {
      step("register");
      if (betaKey !== KEY) throw new ApiError("rejected");
      // after the key, as Central
      if (displayName !== null && (!validDisplayName(displayName) || reservedDisplayName(displayName))) throw new ApiError("rejected");
      if (c.accounts.has(username)) throw new ApiError("conflict");
      const ticket = fill(32, tickets.size + 1);
      const identity = { account: ACCOUNT, epoch: 1 };
      tickets.set(ticket.join(), { name: username, identity, displayName });
      return { identity, ticket };
    },
    async enrollmentStart(ticket: Uint8Array, message: Uint8Array) {
      step("enrollment_start");
      if (!tickets.has(ticket.join()) || message.length !== 32) throw new ApiError("rejected");
      return fill(64, 5);
    },
    async enrollmentFinish(b: { ticket: Uint8Array; wrappedMaster: Uint8Array; recovery: Uint8Array; record: Uint8Array; document: Uint8Array }) {
      step("enrollment_finish");
      const t = tickets.get(b.ticket.join());
      if (t === undefined || b.wrappedMaster.length !== 73 || b.recovery.length !== 73 || b.record.length !== 192) throw new ApiError("rejected");
      tickets.delete(b.ticket.join());
      c.accounts.set(t.name, { identity: t.identity, wrapped: b.wrappedMaster, recovery: b.recovery, displayName: t.displayName, tag: TAG });
    },
    async discover(username: string) {
      step("discover");
      return c.accounts.get(username)?.identity ?? null;
    },
    async loginStart(_identity: Identity, message: Uint8Array) {
      step("login_start");
      if (message.length !== 96) throw new ApiError("rejected");
      attempt = "pending";
      return { attempt: fill(32, 9), message: fill(320, 7) };
    },
    async loginFinish(_attempt: Uint8Array, message: Uint8Array): Promise<BrowserSession> {
      step("login_finish");
      if (attempt === null || message.length !== 64) throw new ApiError("rejected");
      attempt = null;
      const [name, account] = [...c.accounts.entries()][0] ?? [];
      if (name === undefined || account === undefined) throw new ApiError("rejected");
      c.session = { name, expiresAt: 4_000_000_000 };
      const identity = opts.foreignIdentity ? { account: fill(16, 0x77), epoch: 1 } : account.identity;
      return { identity, username: said(name), wrappedMaster: opts.wrapped ?? account.wrapped, expiresAt: c.session.expiresAt };
    },
    async sessionStatus() {
      step("session_status");
      const [name, account] = [...c.accounts.entries()][0] ?? [];
      if (c.session === null || account === undefined || name === undefined) throw new ApiError("rejected");
      return { identity: account.identity, username: said(name), expiresAt: c.session.expiresAt };
    },
    async renewSession() {
      step("renew_session");
      if (c.session === null) throw new ApiError("rejected");
      c.session = { ...c.session, expiresAt: c.session.expiresAt + 1_800 };
      return c.session.expiresAt;
    },
    async logout() {
      step("logout");
      if (c.session === null) throw new ApiError("rejected");
      c.session = null;
    },
    async terms() {
      step("terms");
      return c.current;
    },
    async termsStatus() {
      step("terms_status");
      if (c.session === null) throw new ApiError("rejected");
      const current = c.current?.version ?? null;
      const accepted = current === null || (c.accepted?.version === current.version && c.accepted.digest.join() === current.digest.join());
      return { current, accepted };
    },
    async accountSummary(): Promise<AccountSummary> {
      step("account_summary");
      const [name, account] = [...c.accounts.entries()][0] ?? [];
      if (c.session === null || account === undefined || name === undefined) throw new ApiError("rejected");
      return {
        identity: opts.foreignSummary ? { account: fill(16, 0x77), epoch: 1 } : account.identity,
        username: said(name),
        displayName: account.displayName ?? null,
        tag: account.tag ?? null,
        source: account.source ?? "registered",
        recovery: account.recovery !== null,
        admin: false,
        signedInAt: SIGNED_IN_AT,
        expiresAt: c.session.expiresAt,
      };
    },
    async setRecovery(recovery: Uint8Array) {
      step("set_recovery");
      c.recoveries.push(recovery.slice());
      const account = [...c.accounts.values()][0];
      if (c.session === null || account === undefined || recovery.length !== 73) throw new ApiError("rejected");
      if (account.recovery !== null) {
        if (account.recovery.join() !== recovery.join()) throw new ApiError("conflict");
      } else if (c.late) throw new ApiError("rejected");
      else account.recovery = recovery.slice();
      if (lost.delete("set_recovery")) throw new ApiError("network");
    },
    async setDisplayName(displayName: string) {
      step("set_display_name");
      c.names.push(displayName);
      const account = [...c.accounts.values()][0];
      if (c.session === null || account === undefined) throw new ApiError("rejected");
      const current = c.current?.version ?? null;
      if (current !== null && c.accepted?.version !== current.version) throw new ApiError("forbidden");
      if (!validDisplayName(displayName) || reservedDisplayName(displayName) || (account.tag ?? null) === null) throw new ApiError("rejected");
      const had = account.displayName ?? null;
      if (had !== null && had !== displayName) throw new ApiError("conflict");
      account.displayName = displayName;
      if (lost.delete("set_display_name")) throw new ApiError("network");
    },
    async acceptTerms(v: TermsVersion) {
      step("accept_terms");
      if (c.session === null) throw new ApiError("rejected");
      if (c.current === null || v.version !== c.current.version.version || v.digest.join() !== c.current.version.digest.join()) throw new ApiError("conflict");
      c.accepted = v;
    },
  });
  return c;
}

export interface FakeCrypto {
  factory: CryptoFactory;
  /** Flows started, and how many were closed. */
  opened: number;
  closed: number;
  passwords: string[];
  /** Sign-ins that ended in `loginConfirm`, and in `recoveryCreate`. */
  confirmed: number;
  created: number;
}

/** Crypto that accepts `password` only, and only `WRAPPED` as the master key's wrapper. */
export function fakeCrypto(password: string, opts: { failStart?: boolean } = {}): FakeCrypto {
  const f: FakeCrypto = { factory: () => flow(), opened: 0, closed: 0, passwords: [], confirmed: 0, created: 0 };
  function flow(): Crypto {
    f.opened++;
    let given = "";
    return {
      async enrollmentStart(p) {
        if (opts.failStart) throw new CryptoError("failed");
        f.passwords.push(p);
        return fill(32, 1);
      },
      async enrollmentFinish(response) {
        if (response.length !== 64) throw new CryptoError("rejected");
        return { record: fill(192, 3), wrappedMaster: WRAPPED, recovery: fill(73, 4), document: fill(173, 6), recoveryCode: CODE };
      },
      async loginStart(p) {
        if (opts.failStart) throw new CryptoError("failed");
        f.passwords.push(p);
        given = p;
        return fill(96, 8);
      },
      async loginFinish(response) {
        if (response.length !== 320 || given !== password) throw new CryptoError("rejected");
        return fill(64, 10);
      },
      async loginConfirm(wrapped) {
        f.confirmed++;
        if (wrapped.join() !== WRAPPED.join()) throw new CryptoError("rejected");
      },
      async recoveryCreate(wrapped) {
        f.created++;
        if (wrapped.join() !== WRAPPED.join()) throw new CryptoError("rejected");
        return { recovery: NEW_WRAPPER, recoveryCode: NEW_CODE };
      },
      close() {
        f.closed++;
      },
    };
  }
  return f;
}
