// What every administration page shares: the frame for an administrator
// (and the refusal for anybody else, which Central would refuse too), the
// way a change is run (the password again when the sign-in is stale), and
// how an account is named.

import { useState, type ReactNode } from "react";

import type { AccountSummary } from "../account/api";
import { handle } from "../account/names";
import { AppFrame, SignedOut } from "../app/AppFrame";
import { Reauth } from "../app/Reauth";
import { fresh, useSummary } from "../app/useSummary";
import { T, type Key } from "../i18n";
import type { Page } from "../router";
import "../styles/tables.css";
import { adminApi, type AdminApi } from "./admin-api";

/** An account as the administration names it: its handle, or its sign-in name until it has one. */
export function who(a: { displayName: string | null; tag: number | null; username?: string }): string {
  if (a.displayName !== null && a.tag !== null) return handle(a.displayName, a.tag);
  return a.username ?? "—";
}

export function AdminFrame({
  page,
  title,
  lead,
  children,
}: {
  page: Page;
  title: ReactNode;
  lead?: ReactNode;
  children: (ctx: { summary: AccountSummary; replace(s: AccountSummary): void }) => ReactNode;
}) {
  const { state, replace } = useSummary();
  const admin = state.at === "ready" && state.summary.admin;
  return (
    <AppFrame page={page} admin={admin} eyebrow="app.admin" title={title} {...(lead === undefined ? {} : { lead })}>
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
      {state.at === "ready" && state.summary.admin && children({ summary: state.summary, replace })}
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
