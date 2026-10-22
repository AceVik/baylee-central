// The page's side of the crypto Worker (crypto-worker.ts): one Worker per
// registration or sign-in, one request at a time, terminated when the flow
// ends however it ends. The page never holds a state, a key or the module;
// it hands the Worker the password and bytes, and gets bytes back (and a
// recovery code to show once).

import type { Identity } from "./api";

export type WorkerRequest =
  | { op: "enrollment_start"; password: string; account: Uint8Array; epoch: number }
  | { op: "enrollment_finish"; response: Uint8Array }
  | { op: "login_start"; password: string; account: Uint8Array; epoch: number }
  | { op: "login_finish"; response: Uint8Array }
  | { op: "login_confirm"; wrappedMaster: Uint8Array }
  | { op: "recovery_create"; wrappedMaster: Uint8Array };

export interface Enrolled {
  record: Uint8Array;
  wrappedMaster: Uint8Array;
  recovery: Uint8Array;
  document: Uint8Array;
  /** Shown once, never kept. */
  recoveryCode: string;
}

/** A recovery code made after a sign-in, in place of `login_confirm`. */
export interface Recovered {
  /** The master key under the code (73 bytes), for `SetRecovery`. */
  recovery: Uint8Array;
  /** Shown once, never kept. */
  recoveryCode: string;
}

export type WorkerReply =
  | { op: "enrollment_start" | "login_start" | "login_finish"; message: Uint8Array }
  | ({ op: "enrollment_finish" } & Enrolled)
  | { op: "login_confirm" }
  | ({ op: "recovery_create" } & Recovered)
  | { op: "error"; error: CryptoFailure };

/**
 * `rejected`: the module refused (a wrong password at `loginFinish`, a
 * wrapper that does not open, a malformed message); `unavailable`: no
 * randomness; `failed`: the module or the Worker did not run.
 */
export type CryptoFailure = "rejected" | "unavailable" | "failed";

export class CryptoError extends Error {
  readonly kind: CryptoFailure;
  constructor(kind: CryptoFailure) {
    super(`crypto: ${kind}`);
    this.kind = kind;
  }
}

/** One flow's crypto. `close` ends it; nothing works after. */
export interface Crypto {
  enrollmentStart(password: string, identity: Identity): Promise<Uint8Array>;
  enrollmentFinish(response: Uint8Array): Promise<Enrolled>;
  loginStart(password: string, identity: Identity): Promise<Uint8Array>;
  loginFinish(response: Uint8Array): Promise<Uint8Array>;
  loginConfirm(wrappedMaster: Uint8Array): Promise<void>;
  /**
   * In place of `loginConfirm`, for an account without a recovery code:
   * opens the master key the same way, and wraps it under a new code.
   */
  recoveryCreate(wrappedMaster: Uint8Array): Promise<Recovered>;
  close(): void;
}

export type CryptoFactory = () => Crypto;

function message(reply: WorkerReply): Uint8Array {
  if (!("message" in reply)) throw new CryptoError("failed");
  return reply.message;
}

/** A Worker-backed flow; `make` is the Worker (a test gives another). */
export function workerCrypto(make: () => Worker = () => new Worker(new URL("./crypto-worker.ts", import.meta.url), { type: "module" })): Crypto {
  let worker: Worker | null = make();
  let busy = false;

  function ask(request: WorkerRequest, expected: WorkerReply["op"]): Promise<WorkerReply> {
    return new Promise((resolve, reject) => {
      const w = worker;
      if (w === null || busy) {
        reject(new CryptoError("failed"));
        return;
      }
      busy = true;
      const done = () => {
        busy = false;
        w.removeEventListener("message", onMessage);
        w.removeEventListener("error", onError);
      };
      const onMessage = (e: MessageEvent<WorkerReply>) => {
        done();
        const reply = e.data;
        if (reply.op === "error") reject(new CryptoError(reply.error));
        else if (reply.op !== expected) reject(new CryptoError("failed"));
        else resolve(reply);
      };
      const onError = () => {
        done();
        reject(new CryptoError("failed"));
      };
      w.addEventListener("message", onMessage);
      w.addEventListener("error", onError);
      // oxlint-disable-next-line unicorn/require-post-message-target-origin -- a Worker's postMessage takes no target origin
      w.postMessage(request);
    });
  }

  return {
    enrollmentStart: async (password, id) =>
      message(await ask({ op: "enrollment_start", password, account: id.account, epoch: id.epoch }, "enrollment_start")),
    enrollmentFinish: async (response) => {
      const reply = await ask({ op: "enrollment_finish", response }, "enrollment_finish");
      if (reply.op !== "enrollment_finish") throw new CryptoError("failed");
      const { record, wrappedMaster, recovery, document, recoveryCode } = reply;
      return { record, wrappedMaster, recovery, document, recoveryCode };
    },
    loginStart: async (password, id) => message(await ask({ op: "login_start", password, account: id.account, epoch: id.epoch }, "login_start")),
    loginFinish: async (response) => message(await ask({ op: "login_finish", response }, "login_finish")),
    loginConfirm: async (wrappedMaster) => {
      await ask({ op: "login_confirm", wrappedMaster }, "login_confirm");
    },
    recoveryCreate: async (wrappedMaster) => {
      const reply = await ask({ op: "recovery_create", wrappedMaster }, "recovery_create");
      if (reply.op !== "recovery_create") throw new CryptoError("failed");
      return { recovery: reply.recovery, recoveryCode: reply.recoveryCode };
    },
    close: () => {
      worker?.terminate();
      worker = null;
    },
  };
}
