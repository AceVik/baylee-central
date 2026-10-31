// Registration and sign-in, step by step (docs/web-accounts.md §"Flows"):
// the page carries each message between the crypto Worker and Central. A
// flow's Worker is closed when the flow ends, however it ends. A sign-in to
// an account without a recovery code (one moved from the official gateway)
// makes one in its Worker; the page shows it, and stores it only once the
// person says they kept it (§"A recovery code after a sign-in"). An account
// without a display name chooses one (§"Display name and tag").

import { ApiError, sameIdentity, type AccountSummary, type Api, type BrowserSession } from "./api";
import { CryptoError, type CryptoFactory } from "./crypto";
import { handle } from "./names";

/** What the page says when a flow stops. */
export type FlowFailure =
  /** Registration: the beta key is unknown, used up or expired. */
  | "badKey"
  /** Registration: the name is taken (only said for a valid key). */
  | "taken"
  /** Sign-in: an unknown name or a wrong password; the page cannot tell which. */
  | "wrongLogin"
  /** Too many attempts. */
  | "busy"
  /** Central did not answer, or not usefully. */
  | "unreachable"
  /** The browser crypto did not run (no Worker, no WebAssembly, no randomness). */
  | "crypto"
  /** Central answered what it must not: a foreign identity, a wrapper that does not open. */
  | "mismatch"
  /** A new recovery code: Central did not take it (the sign-in is too old, or the session over). */
  | "codeRefused"
  /** A new recovery code: the account has another one already. */
  | "codeTaken"
  /** A display name: Central did not take it (not allowed, or the session is over). */
  | "nameRefused"
  /** A display name: the account has another one by now. */
  | "nameTaken"
  /** Central's terms in force must be accepted first. */
  | "terms";

export class FlowError extends Error {
  readonly kind: FlowFailure;
  constructor(kind: FlowFailure) {
    super(`account: ${kind}`);
    this.kind = kind;
  }
}

/** Where a flow is, for the page to say while it waits. */
export type Progress = "key" | "securing" | "saving" | "signingIn";

export interface Deps {
  api: Api;
  crypto: CryptoFactory;
}

function fromApi(error: unknown, rejected: FlowFailure): FlowError {
  if (error instanceof FlowError) return error;
  if (error instanceof CryptoError) return new FlowError(error.kind === "rejected" ? "mismatch" : "crypto");
  if (!(error instanceof ApiError)) return new FlowError("unreachable");
  switch (error.kind) {
    case "rejected":
      return new FlowError(rejected);
    case "busy":
      return new FlowError("busy");
    default:
      return new FlowError("unreachable");
  }
}

/**
 * Registers `username` with `betaKey`, `displayName` (null: chosen later)
 * and `password`; the recovery code to show once. The key is checked before
 * the names, so `taken` comes only with a valid key.
 */
export async function register(
  input: { username: string; betaKey: string; displayName?: string | null; password: string },
  { api, crypto }: Deps,
  progress: (p: Progress) => void = () => {},
): Promise<string> {
  progress("key");
  let enrollment;
  try {
    enrollment = await api.register(input.username, input.betaKey, input.displayName ?? null);
  } catch (error) {
    if (error instanceof ApiError && error.kind === "conflict") throw new FlowError("taken");
    throw fromApi(error, "badKey");
  }
  const flow = crypto();
  try {
    progress("securing");
    const request = await flow.enrollmentStart(input.password, enrollment.identity);
    const challenge = await api.enrollmentStart(enrollment.ticket, request);
    const enrolled = await flow.enrollmentFinish(challenge);
    progress("saving");
    await api.enrollmentFinish({
      ticket: enrollment.ticket,
      record: enrolled.record,
      wrappedMaster: enrolled.wrappedMaster,
      document: enrolled.document,
      recovery: enrolled.recovery,
    });
    return enrolled.recoveryCode;
  } catch (error) {
    // the ticket was this registration's alone; an expired one gives the key's use back
    throw fromApi(error, "unreachable");
  } finally {
    flow.close();
  }
}

/** A recovery code made after a sign-in, held by the page until the person decides. */
export interface NewCode {
  /** Shown once, never kept. */
  code: string;
  /** The master key under it, 73 bytes: what `SetRecovery` sends, again on a retry. */
  wrapper: Uint8Array;
}

/** What a sign-in leaves the page. */
export interface SignedIn {
  session: BrowserSession;
  /** What Central says of the account; null when it did not answer. */
  summary: AccountSummary | null;
  /** For an account without a recovery code, a new one; null otherwise. */
  code: NewCode | null;
}

/**
 * Signs in: discovery, OPAQUE, then, while the Worker still holds the
 * sign-in, `AccountSummary`. For an account without a recovery code the
 * Worker opens the master key Central hands back and wraps it under a new
 * code (`recoveryCreate`); for any other, or when Central does not say, it
 * only checks that the key opens (`loginConfirm`). Either proves the
 * password; nothing reaches Central of the code yet. On a mismatch the new
 * session is ended at once. The cookie is Central's to set. The Worker
 * starts (and loads its WebAssembly) while Central looks the name up; it
 * sees the password only after that, and is closed on every way out.
 */
export async function signIn(
  input: { username: string; password: string },
  { api, crypto }: Deps,
  progress: (p: Progress) => void = () => {},
): Promise<SignedIn> {
  progress("signingIn");
  const flow = crypto();
  try {
    const identity = await api.discover(input.username);
    // an unknown name, or one still at the legacy gateway: the page cannot say which
    if (identity === null) throw new FlowError("wrongLogin");
    const ke1 = await flow.loginStart(input.password, identity);
    const challenge = await api.loginStart(identity, ke1);
    let ke3;
    try {
      ke3 = await flow.loginFinish(challenge.message);
    } catch (error) {
      // OPAQUE finds a wrong password here, before anything is sent
      if (error instanceof CryptoError && error.kind === "rejected") throw new FlowError("wrongLogin");
      throw error;
    }
    const session = await api.loginFinish(challenge.attempt, ke3);
    try {
      if (!sameIdentity(session.identity, identity)) throw new FlowError("mismatch");
      let summary: AccountSummary | null = null;
      try {
        summary = await api.accountSummary();
      } catch {
        // Central did not say: the sign-in still proves the password, without a code
      }
      if (summary !== null && !sameIdentity(summary.identity, identity)) throw new FlowError("mismatch");
      if (summary !== null && !summary.recovery) {
        const made = await flow.recoveryCreate(session.wrappedMaster);
        return { session, summary, code: { code: made.recoveryCode, wrapper: made.recovery } };
      }
      await flow.loginConfirm(session.wrappedMaster);
      return { session, summary, code: null };
    } catch (error) {
      await api.logout().catch(() => {});
      throw error;
    }
  } catch (error) {
    throw fromApi(error, "wrongLogin");
  } finally {
    flow.close();
  }
}

/**
 * Stores a code the person kept (`SetRecovery`). Its bytes are kept until
 * Central answers, so a lost answer is retried with the same bytes, which
 * Central takes again. `codeRefused`: the sign-in is 10 minutes old or the
 * session is over; `codeTaken`: the account has another code; anything else
 * may be tried again.
 */
export async function storeCode(code: NewCode, { api }: Pick<Deps, "api">): Promise<void> {
  try {
    await api.setRecovery(code.wrapper);
  } catch (error) {
    if (error instanceof ApiError && error.kind === "conflict") throw new FlowError("codeTaken");
    throw fromApi(error, "codeRefused");
  }
}

/** The handle other players see, `Name#3fa2`; null while the account has no display name or tag. */
export function handleOf(summary: AccountSummary | null): string | null {
  if (summary === null || summary.displayName === null || summary.tag === null) return null;
  return handle(summary.displayName, summary.tag);
}

/**
 * Chooses the display name of the cookie's account (`SetDisplayName`). The
 * same name again is Central's `Done`, so a retry after a lost answer takes.
 * `nameTaken`: the account has another name by now; `nameRefused`: Central
 * does not take the name, or the session is over; `terms`: the terms in
 * force are not accepted.
 */
export async function chooseName(name: string, { api }: Pick<Deps, "api">): Promise<void> {
  try {
    await api.setDisplayName(name);
  } catch (error) {
    if (error instanceof ApiError && error.kind === "conflict") throw new FlowError("nameTaken");
    if (error instanceof ApiError && error.kind === "forbidden") throw new FlowError("terms");
    throw fromApi(error, "nameRefused");
  }
}
