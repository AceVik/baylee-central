import { useEffect, useState } from "react";
import { Confirm } from "../app/Confirm";
import { useAccount } from "../account/session";
import { T } from "../i18n";
import { Link } from "../router";
import { register, supported } from "./client";
import "../styles/pwa.css";
const DISMISSED = "baylee-push-prompt-v1";
export function PwaNotice() {
  const { account } = useAccount();
  const [confirm, setConfirm] = useState(false);
  const [waiting, setWaiting] = useState<ServiceWorker | null>(null);
  const [prompt, setPrompt] = useState(() => { try { return localStorage.getItem(DISMISSED) !== "later"; } catch { return false; } });
  useEffect(() => {
    if (!import.meta.env.PROD || !("serviceWorker" in navigator)) return;
    let live = true;
    let r: ServiceWorkerRegistration | undefined;
    const found = () => {
      const worker = r?.installing;
      if (worker) worker.addEventListener("statechange", () => { if (live && worker.state === "installed" && navigator.serviceWorker.controller) setWaiting(r?.waiting ?? null); });
    };
    void register().then((value) => { if (!live) return; r = value; setWaiting(r.waiting); r.addEventListener("updatefound", found); }).catch(() => {});
    const update = () => { if (document.visibilityState === "visible") void r?.update().catch(() => {}); };
    const timer = window.setInterval(update, 60 * 60_000);
    document.addEventListener("visibilitychange", update);
    return () => { live = false; r?.removeEventListener("updatefound", found); clearInterval(timer); document.removeEventListener("visibilitychange", update); };
  }, []);
  const later = () => { try { localStorage.setItem(DISMISSED, "later"); } catch { /* per-page dismissal still works */ } setPrompt(false); };
  const refresh = () => {
    if (!waiting) return;
    navigator.serviceWorker.addEventListener("controllerchange", () => window.location.reload(), { once: true });
    waiting.postMessage({ type: "ACTIVATE_UPDATE" }, { transfer: [] });
  };
  return <>
    {confirm && <Confirm title={<T id="pwa.reload" />} action="pwa.reload" onCancel={() => setConfirm(false)} onConfirm={refresh}><p><T id="pwa.confirm" /></p></Confirm>}
    {waiting && <aside className="pwa-notice" aria-live="polite"><p><T id="pwa.update" /></p><button className="button button-ghost" onClick={() => setConfirm(true)}><T id="pwa.reload" /></button></aside>}
    {prompt && account?.known && supported() && Notification.permission === "default" && <aside className="pwa-notice"><p><T id="push.prompt" /></p><Link to="/account#notifications" className="button button-ghost" onClick={later}><T id="push.choose" /></Link><button className="text-link" onClick={later}><T id="push.later" /></button></aside>}
  </>;
}
