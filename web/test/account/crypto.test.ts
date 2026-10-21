// The page's side of the crypto Worker: one request at a time, each answer
// matched to its request, the module's two errors passed on, anything else
// `failed`, and `close` terminating the Worker.

import { describe, expect, it } from "vitest";

import { CryptoError, workerCrypto, type WorkerReply, type WorkerRequest } from "../../src/account/crypto";

class FakeWorker extends EventTarget {
  sent: WorkerRequest[] = [];
  terminated = false;
  constructor(private readonly answer: (r: WorkerRequest) => WorkerReply | "crash") {
    super();
  }
  postMessage(r: WorkerRequest) {
    this.sent.push(r);
    const reply = this.answer(r);
    queueMicrotask(() => this.dispatchEvent(reply === "crash" ? new Event("error") : new MessageEvent("message", { data: reply })));
  }
  terminate() {
    this.terminated = true;
  }
}

const ID = { account: new Uint8Array(16).fill(1), epoch: 3 };

async function kind(p: Promise<unknown>): Promise<string> {
  try {
    await p;
  } catch (e) {
    return e instanceof CryptoError ? e.kind : "other";
  }
  return "none";
}

describe("the crypto Worker's client", () => {
  it("sends the password and identity, and returns the module's bytes", async () => {
    const w = new FakeWorker((r) => (r.op === "login_start" ? { op: r.op, message: new Uint8Array(96) } : { op: "login_confirm" }));
    const c = workerCrypto(() => w as unknown as Worker);
    expect(await c.loginStart("synthetic", ID)).toHaveLength(96);
    expect(w.sent[0]).toEqual({ op: "login_start", password: "synthetic", account: ID.account, epoch: 3 });
    await c.loginConfirm(new Uint8Array(73));
    c.close();
    expect(w.terminated).toBe(true);
    expect(await kind(c.loginStart("synthetic", ID))).toBe("failed");
  });

  it("passes on rejected and unavailable, and calls anything else failed", async () => {
    for (const [reply, expected] of [
      [{ op: "error", error: "rejected" }, "rejected"],
      [{ op: "error", error: "unavailable" }, "unavailable"],
      [{ op: "login_start", message: new Uint8Array(96) }, "failed"],
      ["crash", "failed"],
    ] as const) {
      const c = workerCrypto(() => new FakeWorker(() => reply as WorkerReply | "crash") as unknown as Worker);
      expect({ reply, kind: await kind(c.loginFinish(new Uint8Array(320))) }).toEqual({ reply, kind: expected });
    }
  });

  it("refuses a second request while one is open", async () => {
    const c = workerCrypto(() => new FakeWorker(() => ({ op: "enrollment_start", message: new Uint8Array(32) })) as unknown as Worker);
    const first = c.enrollmentStart("synthetic", ID);
    expect(await kind(c.enrollmentStart("synthetic", ID))).toBe("failed");
    expect(await first).toHaveLength(32);
  });
});
