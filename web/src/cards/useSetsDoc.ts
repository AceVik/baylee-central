import { useEffect, useState } from "react";

import { loadSets } from "./data";
import type { SetsDoc } from "./model";

/** `undefined` while loading, `null` when it did not load. */
export function useSetsDoc(enabled = true): SetsDoc | null | undefined {
  const [doc, setDoc] = useState<SetsDoc | null | undefined>(undefined);
  useEffect(() => {
    if (!enabled) return;
    let live = true;
    loadSets().then(
      (d) => live && setDoc(d),
      () => live && setDoc(null),
    );
    return () => {
      live = false;
    };
  }, [enabled]);
  return doc;
}
