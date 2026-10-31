// The signed-in account's summary for the account area and the
// administration: who it is, whether it is an administrator, and when its
// password sign-in was, which a change needs to be under 30 minutes old.

import { useCallback, useEffect, useState } from "react";

import { ApiError, type AccountSummary } from "../account/api";
import { useAccount } from "../account/session";

/** central-identity's freshness for a change, less a minute's margin for the request on its way. */
export const FRESH_SECONDS = 29 * 60;

/** Whether `summary`'s sign-in is recent enough for a change now. */
export function fresh(summary: AccountSummary | null, now = Date.now()): boolean {
  return summary !== null && now / 1000 - summary.signedInAt < FRESH_SECONDS;
}

export type SummaryState =
  | { at: "signedOut" }
  | { at: "loading" }
  | { at: "failed" }
  | { at: "ready"; summary: AccountSummary };

export function useSummary(): { state: SummaryState; reload(): void; replace(summary: AccountSummary): void } {
  const { account, api } = useAccount();
  const [state, setState] = useState<SummaryState>({ at: "loading" });
  const present = account !== null;
  const read = useCallback(() => {
    let live = true;
    api.accountSummary().then(
      (summary) => {
        if (live) setState({ at: "ready", summary });
      },
      (error: unknown) => {
        if (!live) return;
        setState(error instanceof ApiError && error.kind === "rejected" ? { at: "signedOut" } : { at: "failed" });
      },
    );
    return () => {
      live = false;
    };
  }, [api]);
  useEffect(() => (present ? read() : undefined), [present, read]);
  const reload = useCallback(() => {
    read();
  }, [read]);
  const replace = useCallback((summary: AccountSummary) => setState({ at: "ready", summary }), []);
  return { state: present ? state : { at: "signedOut" }, reload, replace };
}
