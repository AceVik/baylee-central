// Secrets and plaintext work buffers stay in this worker's WASM instance.
import init, { login_start, login_finish, chat_unlock, type Login, type LoginKey, type Chat } from "../central-wasm/central_web_crypto.js";
import wasm from "../central-wasm/central_web_crypto_bg.wasm?url";
import type { Request, Result } from "./crypto";
import type { Message, Registration, Review, Sealed } from "./api";

const ready = init({ module_or_path: wasm });
const encode = (v: unknown) => new TextEncoder().encode(JSON.stringify(v));
const decode = <T>(v: Uint8Array): T => JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(v)) as T;
let login: Login | null = null;
let key: LoginKey | null = null;
let chat: Chat | null = null;
function take<T>(v: T | null): T { if (v === null) throw new Error("failed"); return v; }
function step(r: Request): Result {
  switch (r.op) {
    case "start": {
      if (login !== null || key !== null || chat !== null) throw new Error("failed");
      login = login_start(r.password, r.identity.account, BigInt(r.identity.epoch));
      return { op: r.op, message: login.message };
    }
    case "finish": { const state = take(login); login = null; key = login_finish(state, r.response); return { op: r.op, message: key.message }; }
    case "unlock": {
      const state = take(key); key = null; chat = chat_unlock(state, r.wrapped, r.tag);
      if (r.trust !== null) chat.restore_trust(r.trust);
      return { op: r.op, device: chat.device, registration: decode<Registration>(chat.registration()) };
    }
    case "observe": {
      const c = take(chat);
      return { op: r.op, reviews: r.keys.filter((k) => k.devices.length > 0).map((k) => decode<Review>(c.observe(encode(k)))), trust: c.save_trust() };
    }
    case "confirm": { const c = take(chat); c.confirm(encode(r.snapshot)); return { op: r.op, trust: c.save_trust() }; }
    case "seal": return { op: r.op, sealed: decode<Sealed>(take(chat).seal(r.conv, r.text, r.emote, Uint32Array.from(r.members), BigInt(r.now))) };
    case "open": {
      const c = take(chat);
      return { op: r.op, messages: r.messages.map((m) => {
        try { return { seq: m.seq, message: decode<Message>(c.open(encode(m))) }; }
        catch { return { seq: m.seq, message: null }; }
      }) };
    }
  }
}
// The caller serializes requests; a worker also serializes initialization arrivals.
let queue: Promise<void> = Promise.resolve();
self.addEventListener("message", (event: MessageEvent<{ id: number; request: Request }>) => {
  queue = queue.then(async () => {
    const { id, request } = event.data;
    try {
      await ready;
      self.postMessage({ id, result: step(request) }, { transfer: [] });
    } catch (error) {
      const message = error instanceof Error ? error.message : "failed";
      const safe = ["rejected", "conflict", "busy", "unavailable"].includes(message) ? message : "failed";
      self.postMessage({ id, error: safe }, { transfer: [] });
    }
  });
});
