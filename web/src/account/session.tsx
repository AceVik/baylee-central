// Who is signed in, for the header and the account page. The session itself
// is Central's `__Host-central` cookie, which the page cannot read. The page
// keeps only when that session ends (localStorage, `baylee-central-session`),
// so that after a reload it knows to ask Central (`AccountSummary`) whose
// session it is; the handle shown (`Name#3fa2`, never the private sign-in
// name) is always Central's, held in memory only. Signing out, an expired
// session or Central's "no" forget it. Nothing else about the account stays
// in the browser. While a page is open and in view, it renews the session
// (`RenewSession`), which Central caps at 12 hours after the sign-in.

import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";

import { ApiError, api as defaultApi, type Api } from "./api";
import { workerCrypto, type CryptoFactory } from "./crypto";
import { handleOf } from "./flows";

export interface Account {
  /** Central has said whose session this is; not yet after a reload. */
  known: boolean;
  /** The handle other players see, as Central says it; null until then, or while the account has no display name. */
  handle: string | null;
  /** When the session ends, in Unix seconds. */
  expiresAt: number;
}

export const SESSION_KEY = "baylee-central-session";
/** Where the page kept the name too, until 21.10.2026; deleted on sight. */
export const OLD_ACCOUNT_KEY = "baylee-central-account";

/** The remembered session, unless it is missing, malformed or over; its name is unknown. */
export function remembered(now = Date.now()): Account | null {
  let raw: string | null;
  try {
    window.localStorage.removeItem(OLD_ACCOUNT_KEY);
    raw = window.localStorage.getItem(SESSION_KEY);
  } catch {
    return null;
  }
  if (raw === null) return null;
  try {
    const v: unknown = JSON.parse(raw);
    if (typeof v !== "object" || v === null || Array.isArray(v) || Object.keys(v).join() !== "expiresAt") return null;
    const { expiresAt } = v as Record<string, unknown>;
    if (typeof expiresAt !== "number" || !Number.isSafeInteger(expiresAt) || expiresAt * 1000 <= now) return null;
    return { known: false, handle: null, expiresAt };
  } catch {
    return null;
  }
}

function remember(account: Account | null): void {
  try {
    if (account === null) window.localStorage.removeItem(SESSION_KEY);
    else window.localStorage.setItem(SESSION_KEY, JSON.stringify({ expiresAt: account.expiresAt }));
  } catch {
    /* blocked storage: the header knows only for this page's life */
  }
}

interface AccountState {
  /** Changes on authentication/account switches, never ordinary renewal. */
  generation: number;
  account: Account | null;
  api: Api;
  /** A new flow's crypto: one Worker each. */
  crypto: CryptoFactory;
  /** A sign-in's answer, or a name just chosen: Central's handle for the account (or none yet) and the session's end. */
  signedIn(account: { handle: string | null; expiresAt: number }): void;
  /** Extend only the current verified account after a same-account reauthentication. */
  refreshed(expiresAt: number): void;
  /**
   * Ends the session at Central and forgets it here, also when Central
   * cannot be reached: `false` then, since the cookie lives until its end.
   */
  signOut(): Promise<boolean>;
}

const AccountContext = createContext<AccountState | null>(null);

/** The longest a timer may wait (setTimeout's limit). */
const MAX_WAIT = 2 ** 31 - 1;
/**
 * How often a page in view renews its session: well inside the 30 minutes one
 * renewal grants. A page shown again renews at once, unless it renewed (or
 * signed in) within a tenth of that.
 */
export const RENEW_EVERY = 10 * 60 * 1000;

/** The one client of the page, so that a render never makes another. */
const CENTRAL = defaultApi();
const WORKER: CryptoFactory = () => workerCrypto();

export function AccountProvider({
  api = CENTRAL,
  crypto = WORKER,
  renewEvery = RENEW_EVERY,
  children,
}: {
  api?: Api;
  crypto?: CryptoFactory;
  /** Milliseconds between renewals; shorter in tests. */
  renewEvery?: number;
  children: ReactNode;
}) {
  const [account, setAccount] = useState<Account | null>(() => remembered());
  const serial = useRef(0);
  const [generation, setGeneration] = useState(0);

  const change = useCallback((next: Account | null) => {
    serial.current += 1;
    setGeneration(serial.current);
    remember(next);
    setAccount(next);
  }, []);
  const refreshed = useCallback((expiresAt: number) => {
    setAccount((current) => {
      if (current === null) return null;
      const next = { ...current, expiresAt };
      remember(next);
      return next;
    });
  }, []);
  const signedIn = useCallback((next: { handle: string | null; expiresAt: number }) => change({ ...next, known: true }), [change]);

  // a session known only by its end is Central's to name, or to end
  const unnamed = account !== null && !account.known;
  useEffect(() => {
    if (!unnamed) return;
    let live = true;
    const owner = generation;
    api.accountSummary().then(
      (summary) => {
        if (live && owner === serial.current) change({ known: true, handle: handleOf(summary), expiresAt: summary.expiresAt });
      },
      (error: unknown) => {
        // no live session: forget it; Central unreachable: keep it, unnamed, until it ends
        if (live && owner === serial.current && error instanceof ApiError && error.kind === "rejected") change(null);
      },
    );
    return () => {
      live = false;
    };
  }, [api, change, generation, unnamed]);

  // a named session renews while the page is in view: on a timer, and when the
  // page is shown again; Central's "no" means it is over, anything else waits
  const named = account !== null && account.known;
  useEffect(() => {
    if (!named) return;
    let live = true;
    const owner = generation;
    let last = Date.now();
    const renew = () => {
      if (document.visibilityState !== "visible") return;
      last = Date.now();
      api.renewSession().then(
        (expiresAt) => {
          if (!live || owner !== serial.current) return;
          setAccount((now) => {
            if (now === null) return now;
            const next = { ...now, expiresAt };
            remember(next);
            return next;
          });
        },
        (error: unknown) => {
          if (live && owner === serial.current && error instanceof ApiError && error.kind === "rejected") change(null);
        },
      );
    };
    const shown = () => {
      if (Date.now() - last >= renewEvery / 10) renew();
    };
    const timer = window.setInterval(renew, renewEvery);
    document.addEventListener("visibilitychange", shown);
    return () => {
      live = false;
      window.clearInterval(timer);
      document.removeEventListener("visibilitychange", shown);
    };
  }, [api, change, generation, named, renewEvery]);

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
      if (e.key !== SESSION_KEY && e.key !== null) return;
      const next = remembered();
      // the same session keeps its name; another is Central's to name
      // Storage events may be another account in the same cookie jar. Invalidate
      // pending reads/crypto immediately and ask Central to identify it afresh.
      serial.current += 1;
      setGeneration(serial.current);
      setAccount(next);
    };
    window.addEventListener("storage", other);
    return () => window.removeEventListener("storage", other);
  }, []);

  const signOut = useCallback(async () => {
    change(null);
    let ended = true;
    try {
      await api.logout();
    } catch (error) {
      // a session Central no longer has is ended too; an unreachable one ends at its time
      ended = error instanceof ApiError && error.kind === "rejected";
    }
    return ended;
  }, [api, change]);

  const value = useMemo(() => ({ account, generation, api, crypto, signedIn, refreshed, signOut }), [account, generation, api, crypto, signedIn, refreshed, signOut]);
  return <AccountContext value={value}>{children}</AccountContext>;
}

export function useAccount(): AccountState {
  const state = useContext(AccountContext);
  if (state === null) throw new Error("useAccount outside AccountProvider");
  return state;
}
