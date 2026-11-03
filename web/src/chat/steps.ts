// The unlocked chat's steps inside a Worker, shared by the chat's own Worker
// (worker.ts, after a password typed on the chat page) and the sign-in's
// (account/crypto-worker.ts), which keeps its Worker and hands it to the
// chat instead of only checking the master key. Secrets and plaintext work
// buffers stay in that Worker's WASM instance.
import type { Chat } from "../central-wasm/central_web_crypto.js";
import type { Message, Registration, Review, Sealed } from "./api";
import type { Request, Result } from "./crypto";

const encode = (v: unknown) => new TextEncoder().encode(JSON.stringify(v));
const decode = <T>(v: Uint8Array): T => JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(v)) as T;

/** The steps of an unlocked chat; signing in is the Worker's own. */
export type ChatStep = Exclude<Request, { op: "start" | "finish" | "unlock" }>;

/** What an unlock answers: the device and its signed public registration. */
export function unlocked(chat: Chat): { device: string; registration: Registration } {
  return { device: chat.device, registration: decode<Registration>(chat.registration()) };
}

export function chatStep(chat: Chat, r: ChatStep): Result {
  switch (r.op) {
    case "restore": chat.restore_trust(r.trust); return { op: r.op };
    case "observe":
      return { op: r.op, reviews: r.keys.filter((k) => k.devices.length > 0).map((k) => decode<Review>(chat.observe(encode(k)))), trust: chat.save_trust() };
    case "confirm": chat.confirm(encode(r.snapshot)); return { op: r.op, trust: chat.save_trust() };
    case "seal": return { op: r.op, sealed: decode<Sealed>(chat.seal(r.conv, r.text, r.emote, Uint32Array.from(r.members), BigInt(r.now))) };
    case "open":
      return { op: r.op, messages: r.messages.map((m) => {
        try { return { seq: m.seq, message: decode<Message>(chat.open(encode(m))) }; }
        catch { return { seq: m.seq, message: null }; }
      }) };
  }
}

/**
 * Answers the chat's messages (`{ id, request }`, chat/crypto.ts) one at a
 * time; any other message is another protocol's. Errors leave as the few
 * words the page knows.
 */
export function serveChat(step: (request: Request) => Result, ready: () => Promise<unknown>): void {
  let queue: Promise<void> = Promise.resolve();
  self.addEventListener("message", (event: MessageEvent<unknown>) => {
    const data = event.data;
    if (typeof data !== "object" || data === null || !("id" in data) || !("request" in data)) return;
    const { id, request } = data as { id: number; request: Request };
    queue = queue.then(async () => {
      try {
        await ready();
        self.postMessage({ id, result: step(request) }, { transfer: [] });
      } catch (error) {
        const message = error instanceof Error ? error.message : "failed";
        const safe = ["rejected", "conflict", "busy", "unavailable"].includes(message) ? message : "failed";
        self.postMessage({ id, error: safe }, { transfer: [] });
      }
    });
  });
}
