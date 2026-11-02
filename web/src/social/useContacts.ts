import { useCallback, useEffect, useRef, useState } from "react";
import { ApiError } from "../account/api";
import type { Change, Contact, SocialApi } from "./api";

/** One request at a time, paused in hidden tabs. A failed refresh keeps the last list. */
export function useContacts(api: SocialApi, every = 10_000) {
  const [items, setItems] = useState<Contact[] | null>(null);
  const [error, setError] = useState(false);
  const [busy, setBusy] = useState(false);
  const [denied, setDenied] = useState(false);
  const lifecycle = useRef({ generation: 0, running: false, mutating: false });

  const reload = useCallback(async () => {
    if (lifecycle.current.running) return;
    lifecycle.current.running = true;
    const run = lifecycle.current.generation;
    try {
      const contacts = await api.contacts();
      if (lifecycle.current.generation === run) { setItems(contacts); setError(false); setDenied(false); }
    } catch (e) {
      if (lifecycle.current.generation === run) {
        setError(true);
        if (e instanceof ApiError && e.kind === "rejected") { setDenied(true); setItems(null); }
      }
    } finally { if (lifecycle.current.generation === run) lifecycle.current.running = false; }
  }, [api]);

  useEffect(() => {
    const state = lifecycle.current;
    const run = state.generation;
    state.running = false;
    void Promise.resolve().then(() => { if (state.generation === run) void reload(); });
    const refresh = () => { if (document.visibilityState === "visible") void reload(); };
    const timer = window.setInterval(refresh, every);
    document.addEventListener("visibilitychange", refresh);
    return () => {
      ++state.generation;
      window.clearInterval(timer);
      document.removeEventListener("visibilitychange", refresh);
    };
  }, [every, reload]);

  const change = useCallback(async (tag: number, action: Change): Promise<boolean> => {
    if (lifecycle.current.mutating) return false;
    lifecycle.current.mutating = true;
    // Invalidates a pending read so it cannot restore a removed contact later.
    ++lifecycle.current.generation;
    const run = lifecycle.current.generation;
    lifecycle.current.running = true;
    setBusy(true);
    try {
      await api.change(tag, action);
      if (lifecycle.current.generation !== run) return false;
      const contacts = await api.contacts();
      if (lifecycle.current.generation !== run) return false;
      setItems(contacts); setError(false);
      return true;
    } catch {
      if (lifecycle.current.generation === run) setError(true);
      return false;
    } finally {
      if (lifecycle.current.generation === run) { lifecycle.current.running = false; lifecycle.current.mutating = false; setBusy(false); }
    }
  }, [api]);

  return { items, error, denied, busy, reload, change };
}
