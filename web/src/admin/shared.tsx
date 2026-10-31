// What every administration page shares: the frame for an administrator
// (and the refusal for anybody else, which Central would refuse too), the
// way a change is run (the password again when the sign-in is stale), and
// how an account is named.

import { useCallback, useEffect, useState, type ReactNode } from "react";

import type { AccountSummary } from "../account/api";
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
  const { state, replace } = useSummary();
  const admin = state.at === "ready" && state.summary.admin;
  // the counts (for the navigation's "awaiting approval", the filters and
  // the first view) are read once an administrator is known, never for anyone else
  const [overview, setOverview] = useState<Overview | null>(null);
  const [uncounted, setUncounted] = useState(false);
  const recount = useCallback(() => {
    api.overview().then(
      (counts) => {
        setOverview(counts);
        setUncounted(false);
      },
      () => setUncounted(true),
    );
  }, [api]);
  // the new reports not opened yet, for the navigation; a count that cannot
  // be read is simply not shown
  const [unread, setUnread] = useState<number | null>(null);
  const recountUnread = useCallback(() => {
    api.reportsUnread().then(setUnread, () => setUnread(null));
  }, [api]);
  useEffect(() => {
    if (!admin) return;
    recount();
    recountUnread();
  }, [admin, recount, recountUnread]);
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
