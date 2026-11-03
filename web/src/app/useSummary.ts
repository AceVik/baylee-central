// The signed-in account's summary for the account area and the
// administration: who it is, whether it is an administrator, and when its
// password sign-in was, which a change needs to be under 30 minutes old.

import { useCallback, useEffect, useRef, useState } from "react";

import { ApiError, type AccountSummary, type Api } from "../account/api";
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

const cached = new WeakMap<Api, { owner: number; value: SummaryState }>();

export function useSummary(): { state: SummaryState; reload(): void; replace(summary: AccountSummary): void } {
  const { account, api, generation } = useAccount();
  const [result, setResult] = useState<{ owner: number; value: SummaryState } | null>(() => {
    const hit = cached.get(api);
    return hit?.owner === generation ? hit : null;
  });
  const lifecycle = useRef({ live: false, owner: generation, request: 0 });
  const present = account !== null;
  const read = useCallback(() => {
    const scope = lifecycle.current;
    if (!scope.live || scope.owner !== generation) return;
    const request = ++scope.request;
    api.accountSummary().then(
      (summary) => {
        if (!scope.live || scope.owner !== generation || request !== scope.request) return;
        const next: { owner: number; value: SummaryState } = { owner: generation, value: { at: "ready", summary } };
        cached.set(api, next);
        setResult(next);
      },
      (error: unknown) => {
        if (!scope.live || scope.owner !== generation || request !== scope.request) return;
        const next: { owner: number; value: SummaryState } = {
          owner: generation,
          value: error instanceof ApiError && error.kind === "rejected" ? { at: "signedOut" } : { at: "failed" },
        };
        cached.set(api, next);
        setResult(next);
      },
    );
  }, [api, generation]);
  useEffect(() => {
    const scope = lifecycle.current;
    scope.owner = generation;
    scope.live = present;
    if (!present) cached.delete(api);
    if (present) read();
    return () => { scope.live = false; scope.request += 1; };
  }, [api, present, read, generation]);
  const replace = useCallback((summary: AccountSummary) => {
    const scope = lifecycle.current;
    if (!scope.live || scope.owner !== generation) return;
    scope.request += 1;
    const next: { owner: number; value: SummaryState } = { owner: generation, value: { at: "ready", summary } };
    cached.set(api, next);
    setResult(next);
  }, [api, generation]);
  const hit = cached.get(api);
  const active = result?.owner === generation ? result : hit?.owner === generation ? hit : null;
  const state: SummaryState = !present ? { at: "signedOut" } : active !== null ? active.value : { at: "loading" };
  return { state, reload: read, replace };
}
