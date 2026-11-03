// The page's one unlocked chat, kept between pages for as long as the
// sign-in that unlocked it: a sign-in (account/flows.ts) leaves its unlocked
// Worker here, which the chat page turns into a session on its first visit;
// a password typed on the chat page after a reload keeps its session here.
// Signing out, the session's end, Central's refusal and another account's
// sign-in close it. Nothing of it is stored: a reload starts locked. Types
// only from the chat, so the page's first bundle stays without it.
import { useSyncExternalStore } from "react";

import type { AccountSummary, Api } from "../account/api";
import type { Unlocked } from "./crypto";
import type { ChatSession } from "./session";

/** A sign-in's unlocked chat, before the chat page makes its session. */
export interface Kept { summary: AccountSummary; unlocked: Unlocked; accountApi: Api }
interface Held { account: string; kept: Kept | null; session: ChatSession | null }
let held: Held | null = null;
const listeners = new Set<() => void>();
const key = (account: Uint8Array) => Array.from(account, (b) => b.toString(16).padStart(2, "0")).join("");
function emit() { for (const listener of listeners) listener(); }
function subscribe(listener: () => void) { listeners.add(listener); return () => { listeners.delete(listener); }; }
function close(was: Held | null) {
  was?.kept?.unlocked.crypto.close();
  was?.session?.close();
}
const open = (h: Held | null): h is Held => h !== null && (h.kept !== null || (h.session !== null && !h.session.isClosed()));

/** A sign-in's chat. The same account's open chat stays (a reauthentication), and the new Worker is closed. */
export function keepChat(summary: AccountSummary, unlocked: Unlocked, accountApi: Api): void {
  const account = key(summary.identity.account);
  if (open(held) && held.account === account) { unlocked.crypto.close(); return; }
  const was = held;
  held = { account, kept: { summary, unlocked, accountApi }, session: null };
  close(was);
  emit();
}

/** Keeps `session` for `account`, closing any other. */
export function holdChat(account: Uint8Array, session: ChatSession): void {
  if (held?.session === session) return;
  const was = held;
  held = { account: key(account), kept: null, session };
  close(was);
  emit();
}

/** The sign-in's unlocked chat for `account`, once: its session is the caller's to make and hold. */
export function takeKept(account: Uint8Array): Kept | null {
  if (held === null || held.account !== key(account) || held.kept === null) return null;
  const { kept } = held;
  held.kept = null;
  return kept;
}

/** Closes the kept chat: its Worker, keys and messages go. */
export function lockChat(): void {
  const was = held;
  held = null;
  if (was !== null) { close(was); emit(); }
}

/** The kept chat if it is `account`'s and open: a session, or a sign-in's unlocked chat still to start. */
export function heldChat(account: Uint8Array): { session: ChatSession | null; kept: boolean } | null {
  if (!open(held) || held.account !== key(account)) return null;
  return { session: held.session !== null && !held.session.isClosed() ? held.session : null, kept: held.kept !== null };
}

/** Re-renders when the kept chat changes. */
export function useHeld(): void {
  useSyncExternalStore(subscribe, () => held);
}
