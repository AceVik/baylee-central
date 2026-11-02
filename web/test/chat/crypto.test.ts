import { afterEach, describe, expect, it, vi } from "vitest";
import { chatCrypto } from "../../src/chat/crypto";

class FakeWorker extends EventTarget {
  requests: { id: number }[] = [];
  terminated = false;
  postMessage(value: { id: number }) { this.requests.push(value); }
  terminate() { this.terminated = true; }
  reply(id: number, result: unknown) { this.dispatchEvent(new MessageEvent("message", { data: { id, result } })); }
}
afterEach(() => vi.useRealTimers());

describe("chat worker ownership", () => {
  it("rejects both pending and queued work when closed, without posting another request", async () => {
    const worker = new FakeWorker(); const crypto = chatCrypto(() => worker as unknown as Worker);
    const first = crypto.ask({ op: "open", messages: [] });
    const second = crypto.ask({ op: "observe", keys: [] });
    const settled = Promise.allSettled([first, second]);
    await vi.waitFor(() => expect(worker.requests).toHaveLength(1));
    crypto.close();
    expect((await settled).map((result) => result.status)).toEqual(["rejected", "rejected"]);
    expect(worker.requests).toHaveLength(1); expect(worker.terminated).toBe(true);
  });
  it("ignores unrelated replies and terminates a worker with a mismatched operation", async () => {
    const worker = new FakeWorker(); const crypto = chatCrypto(() => worker as unknown as Worker);
    const result = crypto.ask({ op: "open", messages: [] });
    const settled = Promise.allSettled([result]);
    await vi.waitFor(() => expect(worker.requests).toHaveLength(1));
    worker.reply(9, { op: "open", messages: [] });
    expect(worker.terminated).toBe(false);
    worker.reply(1, { op: "confirm", trust: new Uint8Array() });
    expect((await settled)[0]?.status).toBe("rejected"); expect(worker.terminated).toBe(true);
  });
  it("bounds a stalled cryptographic operation and makes later calls fail closed", async () => {
    vi.useFakeTimers();
    const worker = new FakeWorker(); const crypto = chatCrypto(() => worker as unknown as Worker);
    const pending = crypto.ask({ op: "open", messages: [] });
    const settled = Promise.allSettled([pending]);
    await vi.advanceTimersByTimeAsync(45_001);
    expect((await settled)[0]?.status).toBe("rejected"); expect(worker.terminated).toBe(true);
    await expect(crypto.ask({ op: "open", messages: [] })).rejects.toThrow("closed");
  });
});
