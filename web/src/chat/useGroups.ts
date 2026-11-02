import { useCallback, useEffect, useRef, useState } from "react";
import { ApiError } from "../account/api";
import type { GroupApi, GroupChange, Groups } from "./groups";

const EMPTY: Groups = { groups: [], invites: [] };
/** A single account's group list. Unmount invalidates every pending operation. */
export function useGroups(api: GroupApi) {
  const [snapshot, setSnapshot] = useState<Groups>(EMPTY);
  const [error, setError] = useState(false);
  const [busy, setBusy] = useState(false);
  const life = useRef({ generation: 0, reading: false, mutating: false });
  const reload = useCallback(async () => {
    const state = life.current;
    if (state.reading || state.mutating) return;
    const generation = state.generation; state.reading = true;
    try {
      const result = await api.list();
      if (state.generation === generation) { setSnapshot(result); setError(false); }
    } catch (cause) {
      if (state.generation === generation) {
        setError(true);
        if (cause instanceof ApiError && cause.kind === "rejected") setSnapshot(EMPTY);
      }
    } finally { if (state.generation === generation) state.reading = false; }
  }, [api]);
  useEffect(() => {
    const state = life.current;
    const generation = state.generation;
    state.reading = false; state.mutating = false;
    void Promise.resolve().then(() => { if (state.generation === generation) void reload(); });
    const refresh = () => { if (document.visibilityState === "visible") void reload(); };
    const timer = window.setInterval(refresh, 5000);
    document.addEventListener("visibilitychange", refresh);
    return () => { ++state.generation; window.clearInterval(timer); document.removeEventListener("visibilitychange", refresh); };
  }, [reload]);
  const mutate = useCallback(async (operation: () => Promise<string | undefined>): Promise<string | undefined | false> => {
    const state = life.current;
    if (state.mutating) return false;
    const generation = ++state.generation;
    state.mutating = true; state.reading = false; setBusy(true);
    try {
      const value = await operation();
      if (state.generation !== generation) return false;
      const result = await api.list();
      if (state.generation !== generation) return false;
      setSnapshot(result); setError(false); return value;
    } catch (cause) {
      if (state.generation === generation) {
        setError(true);
        if (cause instanceof ApiError && cause.kind === "rejected") setSnapshot(EMPTY);
      }
      return false;
    } finally { if (state.generation === generation) { state.mutating = false; setBusy(false); } }
  }, [api]);
  const create = (name: string) => mutate(() => api.create(name));
  const change = (id: string, action: GroupChange) => mutate(async () => { await api.change(id, action); return undefined; });
  return { ...snapshot, error, busy, reload, create, change };
}
export type GroupState = ReturnType<typeof useGroups>;
