// One bounded, serial worker for an unlocked chat. Closing rejects pending work.
import type { Identity } from "../account/api";
import type { Delivery, Keys, Message, Registration, Review, Sealed } from "./api";
import type { Owner } from "./kept";

/** A chat a sign-in unlocked: its Worker, which now speaks only the chat's steps, and its device. */
export interface Unlocked { crypto: ChatCrypto; device: string; registration: Registration }

export type Request =
  | { op: "start"; password: string; identity: Identity }
  | { op: "finish"; response: Uint8Array }
  | { op: "unlock"; wrapped: Uint8Array; tag: number }
  | { op: "restore"; trust: Uint8Array }
  | { op: "keep"; owner: Owner }
  | { op: "resume"; owner: Owner }
  | { op: "observe"; keys: Keys[] }
  | { op: "confirm"; snapshot: Keys }
  | { op: "seal"; conv: string; text: string; emote: boolean; members: number[]; now: number }
  | { op: "open"; messages: Delivery[] };
export type Result =
  | { op: "start" | "finish"; message: Uint8Array }
  | { op: "unlock"; device: string; registration: Registration }
  | { op: "resume"; device: string; registration: Registration }
  | { op: "restore" }
  | { op: "keep" }
  | { op: "observe"; reviews: Review[]; trust: Uint8Array }
  | { op: "confirm"; trust: Uint8Array }
  | { op: "seal"; sealed: Sealed }
  | { op: "open"; messages: { seq: number; message: Message | null }[] };
export interface ChatCrypto { ask<T extends Request["op"]>(request: Extract<Request, { op: T }>): Promise<Extract<Result, { op: T }> | (T extends "start" | "finish" ? { op: "start" | "finish"; message: Uint8Array } : never)>; close(): void }

export function chatCrypto(make: () => Worker = () => new Worker(new URL("./worker.ts", import.meta.url), { type: "module" })): ChatCrypto {
  let worker: Worker | null = make();
  let pending: { id: number; resolve(v: Result): void; reject(e: Error): void; timer: ReturnType<typeof setTimeout> } | null = null;
  let serial = 0;
  let tail: Promise<unknown> = Promise.resolve();
  function close() {
    worker?.terminate(); worker = null;
    if (pending !== null) { clearTimeout(pending.timer); pending.reject(new Error("closed")); pending = null; }
  }
  worker.addEventListener("message", (event: MessageEvent<{ id: number; result?: Result; error?: string }>) => {
    if (pending === null || pending.id !== event.data.id) return;
    const p = pending; pending = null; clearTimeout(p.timer);
    if (event.data.error !== undefined || event.data.result === undefined) p.reject(new Error(event.data.error ?? "failed"));
    else p.resolve(event.data.result);
  });
  worker.addEventListener("error", close);
  function ask(request: Request): Promise<Result> {
    const call = tail.then(() => new Promise<Result>((resolve, reject) => {
      if (worker === null) { reject(new Error("closed")); return; }
      const id = ++serial;
      pending = { id, resolve, reject, timer: setTimeout(close, 45_000) };
      // oxlint-disable-next-line unicorn/require-post-message-target-origin -- Worker API has no target origin
      worker.postMessage({ id, request });
    })).then((result) => { if (result.op !== request.op) { close(); throw new Error("failed"); } return result; });
    tail = call.catch(() => {});
    return call;
  }
  return { ask: ask as ChatCrypto["ask"], close };
}
