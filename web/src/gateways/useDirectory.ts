// The directory as far as it has been read: the pages so far, whether more
// may follow, and what went wrong with the last request.

import { useCallback, useEffect, useState } from "react";

import { DirectoryError, PAGE_SIZE, type DirectoryClient, type DirectoryFailure, type GatewayListing } from "../central/directory";

interface Listing {
  gateways: GatewayListing[];
  /** More may follow: the last page was full. */
  more: boolean;
  loading: boolean;
  failure: DirectoryFailure | null;
}

export function useDirectory(client: DirectoryClient) {
  const [state, setState] = useState<Listing>({ gateways: [], more: false, loading: true, failure: null });

  /** Asks for the page after `after`; the caller has marked the state as loading. */
  const fetchPage = useCallback(
    (after: string | null) =>
      client.page(after).then(
        (page) =>
          setState((s) => ({
            gateways: after === null ? page : [...s.gateways, ...page],
            more: page.length === PAGE_SIZE,
            loading: false,
            failure: null,
          })),
        (e: unknown) =>
          setState((s) => ({ ...s, loading: false, failure: e instanceof DirectoryError ? e.kind : "network" })),
      ),
    [client],
  );

  useEffect(() => {
    void fetchPage(null);
  }, [fetchPage]);

  const last = state.gateways.at(-1)?.id ?? null;
  const again = () => {
    setState((s) => ({ ...s, loading: true, failure: null }));
    void fetchPage(last);
  };
  return { ...state, next: again, retry: again };
}
