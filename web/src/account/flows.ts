// Registration and sign-in, step by step (docs/web-accounts.md §"Flows"):
// the page carries each message between the crypto Worker and Central. A
// flow's Worker is closed when the flow ends, however it ends.

import { ApiError, sameIdentity, type Api, type BrowserSession } from "./api";
import { CryptoError, type CryptoFactory } from "./crypto";

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
  | "mismatch";

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
 * Registers `username` with `betaKey` and `password`; the recovery code to
 * show once. The key is checked before the name, so `taken` comes only
 * with a valid key.
 */
export async function register(
  input: { username: string; betaKey: string; password: string },
  { api, crypto }: Deps,
  progress: (p: Progress) => void = () => {},
): Promise<string> {
  progress("key");
  let enrollment;
  try {
    enrollment = await api.register(input.username, input.betaKey);
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

/**
 * Signs in: discovery, OPAQUE, then a check that the master key Central
 * hands back opens under this password. On a mismatch the new session is
 * ended at once. The cookie is Central's to set; the page keeps nothing.
 */
export async function signIn(
  input: { username: string; password: string },
  { api, crypto }: Deps,
  progress: (p: Progress) => void = () => {},
): Promise<BrowserSession> {
  progress("signingIn");
  let identity;
  try {
    identity = await api.discover(input.username);
  } catch (error) {
    throw fromApi(error, "wrongLogin");
  }
  // an unknown name, or one still at the legacy gateway: the page cannot say which
  if (identity === null) throw new FlowError("wrongLogin");
  const flow = crypto();
  try {
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
      await flow.loginConfirm(session.wrappedMaster);
    } catch (error) {
      await api.logout().catch(() => {});
      throw error;
    }
    return session;
  } catch (error) {
    throw fromApi(error, "wrongLogin");
  } finally {
    flow.close();
  }
}
