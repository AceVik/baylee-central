// What every administration page shares: the frame for an administrator
// (and the refusal for anybody else, which Central would refuse too), the
// way a change is run (the password again when the sign-in is stale), and
// how an account is named.

import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";

import type { AccountSummary } from "../account/api";
import { useAccount } from "../account/session";
import { handle } from "../account/names";
import { AppFrame, SignedOut } from "../app/AppFrame";
import { Reauth } from "../app/Reauth";
import { fresh, useSummary } from "../app/useSummary";
import { T, type Key } from "../i18n";
import type { Page } from "../router";
import { adminApi, type AdminApi, type Overview } from "./admin-api";

/** An account as the administration names it: its handle, or its sign-in name until it has one. */
export function who(a: { displayName: string | null; tag: number | null; username?: string }): string {
  if (a.displayName !== null && a.tag !== null) return handle(a.displayName, a.tag);
  return a.username ?? "—";
}

/** What an administration page is handed: the administrator, and the counts of the first view. */
export interface AdminContext {
  summary: AccountSummary;
  replace(s: AccountSummary): void;
  /** The counts, once read; null before, and when Central did not answer. */
  overview: Overview | null;
  /** Central did not answer the counts. */
  uncounted: boolean;
  /** Reads the counts again, after a change. */
  recount(): void;
  /** Reads the navigation's count of unread reports again, after one was opened or decided. */
  recountUnread(): void;
}

const cachedOverview = new WeakMap<AdminApi, { owner: number; overview: Overview | null; uncounted: boolean }>();
const cachedUnread = new WeakMap<AdminApi, { owner: number; unread: number | null }>();

/** Keep administration counts within the current authentication and newest request. */
function useOverview(api: AdminApi, admin: boolean, generation: number) {
  // the counts (for the navigation's "awaiting approval", the filters and
  // the first view) are read once an administrator is known, never for anyone else
  const [result, setResult] = useState<{ owner: number; overview: Overview | null; uncounted: boolean } | null>(() => {
    const hit = cachedOverview.get(api);
    return hit?.owner === generation ? hit : null;
  });
  const [unreadState, setUnreadState] = useState<{ owner: number; unread: number | null } | null>(() => {
    const hit = cachedUnread.get(api);
    return hit?.owner === generation ? hit : null;
  });
  const lifecycle = useRef({ live: false, owner: generation, request: 0, unreadRequest: 0 });
  const hitOverview = cachedOverview.get(api);
  const current = admin ? (result?.owner === generation ? result : hitOverview?.owner === generation ? hitOverview : null) : null;
  const overview = current?.overview ?? null;
  const uncounted = current?.uncounted ?? false;
  const hitUnread = cachedUnread.get(api);
  const currentUnread = admin ? (unreadState?.owner === generation ? unreadState : hitUnread?.owner === generation ? hitUnread : null) : null;
  const unread = currentUnread?.unread ?? null;
  const recount = useCallback(() => {
    const scope = lifecycle.current;
    if (!scope.live || scope.owner !== generation) return;
    const request = ++scope.request;
    api.overview().then(
      (counts) => {
        if (!scope.live || scope.owner !== generation || request !== scope.request) return;
        const next = { owner: generation, overview: counts, uncounted: false };
        cachedOverview.set(api, next);
        setResult(next);
      },
      () => {
        if (!scope.live || scope.owner !== generation || request !== scope.request) return;
        const next = { owner: generation, overview: null, uncounted: true };
        cachedOverview.set(api, next);
        setResult(next);
      },
    );
  }, [api, generation]);
  // the new reports not opened yet, for the navigation; a count that cannot
  // be read is simply not shown
  const recountUnread = useCallback(() => {
    const scope = lifecycle.current;
    if (!scope.live || scope.owner !== generation) return;
    const request = ++scope.unreadRequest;
    api.reportsUnread().then(
      (n) => {
        if (!scope.live || scope.owner !== generation || request !== scope.unreadRequest) return;
        const next = { owner: generation, unread: n };
        cachedUnread.set(api, next);
        setUnreadState(next);
      },
      () => {
        if (!scope.live || scope.owner !== generation || request !== scope.unreadRequest) return;
        const next = { owner: generation, unread: null };
        cachedUnread.set(api, next);
        setUnreadState(next);
      },
    );
  }, [api, generation]);
  useEffect(() => {
    const scope = lifecycle.current;
    scope.live = admin;
    scope.owner = generation;
    if (admin) {
      recount();
      recountUnread();
    }
    return () => {
      scope.live = false;
      scope.request += 1;
      scope.unreadRequest += 1;
    };
  }, [admin, generation, recount, recountUnread]);
  return { overview, uncounted, unread, recount, recountUnread };
}

export function AdminFrame({
  page,
  api,
  title,
  lead,
  children,
}: {
  page: Page;
  api: AdminApi;
  title: ReactNode;
  lead?: ReactNode;
  children: (ctx: AdminContext) => ReactNode;
}) {
  const { generation } = useAccount();
  const { state, replace } = useSummary();
  const admin = state.at === "ready" && state.summary.admin;
  const { overview, uncounted, unread, recount, recountUnread } = useOverview(api, admin, generation);
  const badges =
    overview === null && unread === null
      ? undefined
      : { ...(overview === null ? {} : { adminGateways: overview.pending }), ...(unread === null ? {} : { adminFeedback: unread }) };
  return (
    <AppFrame page={page} admin={admin} {...(badges === undefined ? {} : { badges })} eyebrow="app.admin" title={title} {...(lead === undefined ? {} : { lead })}>
      {state.at === "signedOut" && <SignedOut />}
      {state.at === "loading" && <div className="card skeleton app-skeleton" aria-hidden="true" />}
      {state.at === "failed" && (
        <p className="small" role="alert">
          <T id="acct.errUnreachable" />
        </p>
      )}
      {state.at === "ready" && !state.summary.admin && (
        <div className="card app-empty">
          <p>
            <T id="adm.noAccess" />
          </p>
        </div>
      )}
      {state.at === "ready" && state.summary.admin && children({ summary: state.summary, replace, overview, uncounted, recount, recountUnread })}
    </AppFrame>
  );
}

/** The administration's client, made once per page (or a test's stand-in). */
export function useAdminApi(client?: AdminApi): AdminApi {
  const [api] = useState(() => client ?? adminApi());
  return api;
}

/**
 * Runs a change: at once with a sign-in less than 30 minutes old, else after
 * the password again. `pending` is the dialog that asks for it.
 */
export function useChange(summary: AccountSummary, replace: (s: AccountSummary) => void): {
  run(change: () => void): void;
  pending: ReactNode;
} {
  const [waiting, setWaiting] = useState<(() => void) | null>(null);
  const run = (change: () => void) => {
    if (fresh(summary)) change();
    else setWaiting(() => change);
  };
  const pending =
    waiting === null ? null : (
      <Reauth
        summary={summary}
        onDone={(now) => {
          replace(now);
          const change = waiting;
          setWaiting(null);
          change();
        }}
        onCancel={() => setWaiting(null)}
      />
    );
  return { run, pending };
}

/** What a failed administration request says. */
export function failureKey(kind: string): Key {
  switch (kind) {
    case "conflict":
      return "adm.errConflict";
    case "rejected":
      return "adm.errRejected";
    case "busy":
      return "acct.errBusy";
    default:
      return "acct.errUnreachable";
  }
}
