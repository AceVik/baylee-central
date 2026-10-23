// The browser crypto in a module Worker (docs/web-accounts.md §"Browser
// crypto"): central-web-crypto's WebAssembly module, with its Argon2
// (64 MiB, three passes) off the page's thread. A file of its own, for the
// CSP (`worker-src 'self'`).
//
// One flow per Worker: the page starts one for a registration or a sign-in
// and terminates it at the end, which discards the module's memory with
// whatever a JavaScript string or a freed buffer left there. The states
// between two messages stay here; the page sends and receives bytes only.
// A sign-in ends in `login_confirm` or, for an account without a recovery
// code, in `recovery_create`, which opens the master key the same way and
// wraps it under a new code: the master key never leaves this Worker.

import init, {
  enrollment_finish,
  enrollment_start,
  login_confirm,
  login_finish,
  login_start,
  recovery_create,
  type Enrollment,
  type Login,
  type LoginKey,
} from "../central-wasm/central_web_crypto.js";
import wasm from "../central-wasm/central_web_crypto_bg.wasm?url";
import type { WorkerReply, WorkerRequest } from "./crypto";

interface Scope {
  postMessage(message: WorkerReply): void;
  addEventListener(type: "message", listener: (e: MessageEvent<WorkerRequest>) => void): void;
}

const scope = self as unknown as Scope;
let ready: Promise<unknown> | null = null;
let enrollment: Enrollment | null = null;
let login: Login | null = null;
let key: LoginKey | null = null;

/** Takes the state a step consumes; a step out of order is `failed`. */
function take<T>(state: T | null): T {
  if (state === null) throw new Error("failed");
  return state;
}

function step(m: WorkerRequest): WorkerReply {
  switch (m.op) {
    case "enrollment_start": {
      enrollment = enrollment_start(m.password, m.account, BigInt(m.epoch));
      return { op: m.op, message: enrollment.message };
    }
    case "enrollment_finish": {
      const state = take(enrollment);
      enrollment = null;
      const done = enrollment_finish(state, m.response);
      try {
        return {
          op: m.op,
          record: done.record,
          wrappedMaster: done.wrapped_master,
          recovery: done.recovery,
          document: done.document,
          recoveryCode: done.recovery_code,
        };
      } finally {
        // wipes the code's buffer in the module
        done.free();
      }
    }
    case "login_start": {
      login = login_start(m.password, m.account, BigInt(m.epoch));
      return { op: m.op, message: login.message };
    }
    case "login_finish": {
      const state = take(login);
      login = null;
      key = login_finish(state, m.response);
      return { op: m.op, message: key.message };
    }
    case "login_confirm": {
      const state = take(key);
      key = null;
      login_confirm(state, m.wrappedMaster);
      return { op: m.op };
    }
    case "recovery_create": {
      const state = take(key);
      key = null;
      const made = recovery_create(state, m.wrappedMaster);
      try {
        return { op: m.op, recovery: made.recovery, recoveryCode: made.recovery_code };
      } finally {
        // wipes the code's buffer in the module
        made.free();
      }
    }
  }
}

/* oxlint-disable unicorn/require-post-message-target-origin -- a worker's postMessage takes no target origin */
scope.addEventListener("message", (e) => {
  ready ??= init({ module_or_path: wasm });
  ready.then(
    () => {
      try {
        scope.postMessage(step(e.data));
      } catch (error) {
        const said = error instanceof Error ? error.message : "";
        scope.postMessage({ op: "error", error: said === "rejected" || said === "unavailable" ? said : "failed" });
      }
    },
    () => scope.postMessage({ op: "error", error: "failed" }),
  );
});
