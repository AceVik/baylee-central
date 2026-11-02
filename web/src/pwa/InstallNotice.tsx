import { useEffect, useRef, useState } from "react";
import { T } from "../i18n";

type InstallPrompt = Event & { prompt(): Promise<unknown> };
function canPrompt(event: Event): event is InstallPrompt {
  return "prompt" in event && typeof event.prompt === "function";
}
function standalone(): boolean {
  return window.matchMedia?.("(display-mode: standalone)").matches === true || ("standalone" in navigator && navigator.standalone === true);
}

/** Offer installation only when the browser confirms this app is installable. */
export function InstallNotice() {
  const life = useRef<{ generation: number; pending: InstallPrompt | null }>({ generation: 0, pending: null });
  const [offered, setOffered] = useState(false);
  const [failed, setFailed] = useState(false);
  useEffect(() => {
    const scope = life.current;
    const clear = () => { ++scope.generation; scope.pending = null; setOffered(false); setFailed(false); };
    const ready = (event: Event) => {
      if (!canPrompt(event) || standalone()) return;
      event.preventDefault();
      ++scope.generation; scope.pending = event; setOffered(true); setFailed(false);
    };
    window.addEventListener("beforeinstallprompt", ready);
    window.addEventListener("appinstalled", clear);
    return () => {
      ++scope.generation; scope.pending = null;
      window.removeEventListener("beforeinstallprompt", ready);
      window.removeEventListener("appinstalled", clear);
    };
  }, []);
  const install = async () => {
    const scope = life.current;
    const prompt = scope.pending;
    if (prompt === null) return;
    const current = scope.generation;
    // Each browser event can be consumed once. Invoke it in the click itself,
    // before any await, preserving the browser's required user activation.
    scope.pending = null; setOffered(false);
    try { await prompt.prompt(); }
    catch { if (current === scope.generation) setFailed(true); }
  };
  if (!offered && !failed) return null;
  return <aside className="pwa-notice">
    {failed ? <p role="alert"><T id="pwa.installFailed" /></p> : <>
      <p><T id="pwa.installLead" /></p>
      <button type="button" className="button button-ghost" onClick={() => { void install(); }}><T id="pwa.install" /></button>
    </>}
    <button type="button" className="text-link" onClick={() => { life.current.pending = null; setOffered(false); setFailed(false); }}><T id="push.later" /></button>
  </aside>;
}
