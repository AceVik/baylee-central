// The chat's own Worker, for a page that reloaded: it resumes the identity
// this browser kept (kept.ts), or, where none is kept, unlocks it with a
// password typed on the chat page; a sign-in unlocks it in the sign-in's
// Worker. Secrets and plaintext work buffers stay in this Worker.
import init, { login_start, login_finish, chat_unlock, chat_resume, type Login, type LoginKey, type Chat } from "../central-wasm/central_web_crypto.js";
import wasm from "../central-wasm/central_web_crypto_bg.wasm?url";
import type { Request, Result } from "./crypto";
import { forgetKept, openIdentity, readKept } from "./kept";
import { chatStep, serveChat, unlocked } from "./steps";

const ready = init({ module_or_path: wasm });
let login: Login | null = null;
let key: LoginKey | null = null;
let chat: Chat | null = null;
function take<T>(v: T | null): T { if (v === null) throw new Error("failed"); return v; }
async function step(r: Request): Promise<Result> {
  switch (r.op) {
    case "start": {
      if (login !== null || key !== null || chat !== null) throw new Error("failed");
      login = login_start(r.password, r.identity.account, BigInt(r.identity.epoch));
      return { op: r.op, message: login.message };
    }
    case "finish": { const state = take(login); login = null; key = login_finish(state, r.response); return { op: r.op, message: key.message }; }
    case "unlock": { const state = take(key); key = null; chat = chat_unlock(state, r.wrapped, r.tag); return { op: r.op, ...unlocked(chat) }; }
    case "resume": {
      if (login !== null || key !== null || chat !== null) throw new Error("failed");
      const record = await readKept();
      const bytes = record === null ? null : await openIdentity(record, r.owner);
      if (bytes === null) {
        // another account's, altered or unreadable: of no use to anybody
        if (record !== null) await forgetKept();
        throw new Error("unavailable");
      }
      try { chat = chat_resume(bytes, r.owner.tag); } finally { bytes.fill(0); }
      return { op: r.op, ...unlocked(chat) };
    }
    default: return chatStep(take(chat), r);
  }
}
// The caller serializes requests; a worker also serializes initialization arrivals.
serveChat(step, () => ready);
