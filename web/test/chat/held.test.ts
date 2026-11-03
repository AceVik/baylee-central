// The page's one unlocked chat: a sign-in's is kept until the chat page
// takes it, one account's at a time; locking closes the Worker and the
// session, and another account's chat is never handed out.
import { afterEach, describe, expect, it, vi } from "vitest";

import type { AccountSummary, Api } from "../../src/account/api";
import type { Unlocked } from "../../src/chat/crypto";
import { heldChat, holdChat, keepChat, lockChat, takeKept } from "../../src/chat/held";
import type { ChatSession } from "../../src/chat/session";

const A = Uint8Array.from({ length: 16 }, (_, i) => i + 1);
const B = Uint8Array.from({ length: 16 }, (_, i) => 32 - i);
const summary = (account: Uint8Array) => ({ identity: { account, epoch: 1 }, tag: 7 }) as unknown as AccountSummary;
const api = {} as Api;
function unlocked(): Unlocked & { close: ReturnType<typeof vi.fn>; asked: unknown[] } {
  const close = vi.fn<() => void>();
  const asked: unknown[] = [];
  const ask = vi.fn<(r: unknown) => Promise<never>>((r) => { asked.push(r); return Promise.reject(new Error("closed")); });
  return { crypto: { ask: ask as unknown as Unlocked["crypto"]["ask"], close }, device: "a".repeat(16), registration: { ik: "b", dk: "c", dsig: "d", label: "Browser" }, close, asked };
}
function session(): ChatSession & { close: ReturnType<typeof vi.fn> } {
  let closed = false;
  const close = vi.fn<() => void>(() => { closed = true; });
  return { close, isClosed: () => closed } as unknown as ChatSession & { close: ReturnType<typeof vi.fn> };
}
afterEach(() => lockChat());

describe("the kept chat", () => {
  it("hands a sign-in's chat to its own account's page once, never to another account's", () => {
    const first = unlocked();
    keepChat(summary(A), first, api);
    // the browser keeps the identity at once, so that a reload needs no password
    expect(first.asked).toEqual([{ op: "keep", owner: { account: A, epoch: 1, tag: 7 } }]);
    expect(heldChat(B)).toBeNull();
    expect(takeKept(B)).toBeNull();
    expect(heldChat(A)).toEqual({ session: null, kept: true });
    expect(takeKept(A)?.unlocked).toBe(first);
    expect(takeKept(A)).toBeNull();
    expect(first.close).not.toHaveBeenCalled();
  });

  it("closes the previous account's chat when another signs in, and the new Worker on a same-account reauthentication", () => {
    const first = unlocked();
    const again = unlocked();
    const other = unlocked();
    keepChat(summary(A), first, api);
    keepChat(summary(A), again, api);
    expect(again.close).toHaveBeenCalledTimes(1);
    expect(first.close).not.toHaveBeenCalled();
    keepChat(summary(B), other, api);
    expect(first.close).toHaveBeenCalledTimes(1);
    expect(heldChat(A)).toBeNull();
    expect(heldChat(B)?.kept).toBe(true);
  });

  it("keeps a session until it is locked, replaced or closed itself", () => {
    const one = session();
    const two = session();
    holdChat(A, one);
    expect(heldChat(A)?.session).toBe(one);
    holdChat(A, two);
    expect(one.close).toHaveBeenCalledTimes(1);
    expect(heldChat(A)?.session).toBe(two);
    two.close();
    expect(heldChat(A)).toBeNull();
    const three = session();
    holdChat(B, three);
    lockChat();
    expect(three.close).toHaveBeenCalledTimes(1);
    expect(heldChat(B)).toBeNull();
  });
});
