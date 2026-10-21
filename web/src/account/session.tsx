// Who is signed in, for the header and the account page. The session itself
// is Central's `__Host-central` cookie, which the page cannot read; the page
// remembers only the name it signed in with and when the session ends
// (localStorage, `baylee-central-account`), so the header can say so after
// a reload, and asks Central (`SessionStatus`) once per load whether that
// session still lives. Signing out, an expired session or Central's "no"
// forget it. Nothing else about the account stays in the browser.

import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";

import { ApiError, api as defaultApi, type Api } from "./api";
import { workerCrypto, type CryptoFactory } from "./crypto";

export interface Account {
  /** The name as Central keeps it (lower case). */
  name: string;
  /** When the session ends, in Unix seconds. */
  expiresAt: number;
}

export const ACCOUNT_KEY = "baylee-central-account";

/** The remembered account, unless it is missing, malformed or expired. */
export function remembered(now = Date.now()): Account | null {
  let raw: string | null;
  try {
    raw = window.localStorage.getItem(ACCOUNT_KEY);
  } catch {
    return null;
  }
  if (raw === null) return null;
  try {
    const v: unknown = JSON.parse(raw);
    if (typeof v !== "object" || v === null) return null;
    const { name, expiresAt } = v as Record<string, unknown>;
    if (typeof name !== "string" || !/^[a-z0-9_.-]{3,24}$/.test(name)) return null;
    if (typeof expiresAt !== "number" || !Number.isSafeInteger(expiresAt) || expiresAt * 1000 <= now) return null;
    return { name, expiresAt };
  } catch {
    return null;
  }
}

function remember(account: Account | null): void {
  try {
    if (account === null) window.localStorage.removeItem(ACCOUNT_KEY);
    else window.localStorage.setItem(ACCOUNT_KEY, JSON.stringify({ name: account.name, expiresAt: account.expiresAt }));
  } catch {
    /* blocked storage: the header knows only for this page's life */
  }
}

interface AccountState {
  account: Account | null;
  api: Api;
  /** A new flow's crypto: one Worker each. */
  crypto: CryptoFactory;
  signedIn(account: Account): void;
  /**
   * Ends the session at Central and forgets it here, also when Central
   * cannot be reached: `false` then, since the cookie lives until its end.
   */
  signOut(): Promise<boolean>;
}

const AccountContext = createContext<AccountState | null>(null);

/** The longest a timer may wait (setTimeout's limit). */
const MAX_WAIT = 2 ** 31 - 1;

/** The one client of the page, so that a render never makes another. */
const CENTRAL = defaultApi();
const WORKER: CryptoFactory = () => workerCrypto();

export function AccountProvider({ api = CENTRAL, crypto = WORKER, children }: { api?: Api; crypto?: CryptoFactory; children: ReactNode }) {
  const [account, setAccount] = useState<Account | null>(() => remembered());

  const change = useCallback((next: Account | null) => {
    remember(next);
    setAccount(next);
  }, []);

  // a remembered session is Central's to confirm, once per load
  useEffect(() => {
    const known = remembered();
    if (known === null) return;
    let live = true;
    api.sessionStatus().then(
      (status) => {
        if (live) change({ name: known.name, expiresAt: status.expiresAt });
      },
      (error: unknown) => {
        // no live session: forget it; Central unreachable: keep it until it ends
        if (live && error instanceof ApiError && error.kind === "rejected") change(null);
      },
    );
    return () => {
      live = false;
    };
  }, [api, change]);

  // the session ends at its time without a word from Central
  useEffect(() => {
    if (account === null) return;
    const wait = Math.min(Math.max(0, account.expiresAt * 1000 - Date.now()), MAX_WAIT);
    const timer = window.setTimeout(() => change(null), wait);
    return () => window.clearTimeout(timer);
  }, [account, change]);

  // another tab signed in or out
  useEffect(() => {
    const other = (e: StorageEvent) => {
      if (e.key === ACCOUNT_KEY || e.key === null) setAccount(remembered());
    };
    window.addEventListener("storage", other);
    return () => window.removeEventListener("storage", other);
  }, []);

  const signOut = useCallback(async () => {
    let ended = true;
    try {
      await api.logout();
    } catch (error) {
      // a session Central no longer has is ended too; an unreachable one ends at its time
      ended = error instanceof ApiError && error.kind === "rejected";
    }
    change(null);
    return ended;
  }, [api, change]);

  const value = useMemo(() => ({ account, api, crypto, signedIn: change, signOut }), [account, api, crypto, change, signOut]);
  return <AccountContext value={value}>{children}</AccountContext>;
}

export function useAccount(): AccountState {
  const state = useContext(AccountContext);
  if (state === null) throw new Error("useAccount outside AccountProvider");
  return state;
}
