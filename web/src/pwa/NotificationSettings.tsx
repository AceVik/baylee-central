import { useEffect, useRef, useState } from "react";
import { T, useLang, type Key } from "../i18n";
import { BITS, notifications, type Category, type PushState } from "./api";
import { disablePush, installation, keyBytes, register, rememberInstallation, supported, workerState } from "./client";
import "../styles/pwa.css";

const LABELS: Record<Category, Key> = { chat: "push.chat", contacts: "push.contacts", updates: "push.updates", feedback: "push.feedback", gateways: "push.gateways" };
export function NotificationSettings({ admin }: { admin: boolean }) {
  const lang = useLang();
  const [state, setState] = useState<PushState | null>(null);
  const [active, setActive] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<Key | null>(null);
  const life = useRef({ generation: 0 });
  const available = supported();
  const categories: Category[] = admin ? ["chat", "contacts", "updates", "feedback", "gateways"] : ["chat", "contacts", "updates"];
  useEffect(() => {
    const scope = life.current;
    const generation = ++scope.generation;
    async function load() {
      const subscription = available ? await register().then((r) => r.pushManager.getSubscription()).catch(() => null) : null;
      const next = await notifications({ command: "read", endpoint: subscription?.endpoint ?? null });
      if (life.current.generation !== generation) return;
      const enabled = available && Notification.permission === "granted" && next.subscribed && installation() !== null;
      if (!enabled && installation() !== null) rememberInstallation(null);
      setState(next); setActive(enabled);
      if (available) await workerState(enabled ? installation() : null, enabled ? next.enabled : 0);
    }
    void load().catch(() => { if (life.current.generation === generation) setError("push.failed"); });
    return () => { ++scope.generation; };
  }, [available]);
  async function toggle(category: Category, enabled: boolean) {
    if (busy || state === null) return;
    const generation = life.current.generation;
    setBusy(true); setError(null);
    try {
      // Opt-out is applied locally first, even if the network is unavailable.
      if (!enabled && active) {
        try { await workerState(installation(), state.enabled & ~BITS[category]); }
        catch { /* Still withdraw consent at the server; worker state is retried below. */ }
      }
      const next = await notifications({ command: "set", category, enabled });
      if (life.current.generation !== generation) return;
      setState(next);
      if (active) await workerState(installation(), next.enabled);
    } catch { if (life.current.generation === generation) setError("push.failed"); }
    finally { if (life.current.generation === generation) setBusy(false); }
  }
  function enable() {
    if (!available || busy || state?.publicKey == null) return;
    // Permission is requested directly in the user's click, before network waits.
    const permission = Notification.requestPermission();
    const generation = life.current.generation;
    const publicKey = state.publicKey;
    setBusy(true); setError(null);
    void (async () => {
      if (await permission !== "granted") { if (life.current.generation === generation) setError("push.denied"); return; }
      if (life.current.generation !== generation) return;
      const r = await register();
      if (life.current.generation !== generation) return;
      // A fresh browser capability prevents a previous account's queued push
      // being assigned to this account on a shared device.
      const old = await r.pushManager.getSubscription();
      if (old) {
        await notifications({ command: "unsubscribe", endpoint: old.endpoint });
        if (!await old.unsubscribe()) throw new Error("previous subscription still active");
      }
      if (life.current.generation !== generation) return;
      const sub = await r.pushManager.subscribe({ userVisibleOnly: true, applicationServerKey: keyBytes(publicKey) });
      if (life.current.generation !== generation) { await sub.unsubscribe(); return; }
      const data = sub.toJSON();
      const nonce = [...crypto.getRandomValues(new Uint8Array(16))].map((b) => b.toString(16).padStart(2, "0")).join("");
      try {
        if (!data.keys?.["p256dh"] || !data.keys["auth"]) throw new Error("missing browser keys");
        const next = await notifications({ command: "subscribe", subscription: { endpoint: sub.endpoint, p256dh: data.keys["p256dh"], auth: data.keys["auth"], nonce, lang } });
        if (life.current.generation !== generation) { await sub.unsubscribe(); return; }
        rememberInstallation(nonce); await workerState(nonce, next.enabled);
        if (life.current.generation === generation) { setState(next); setActive(true); }
      } catch (cause) { await sub.unsubscribe(); throw cause; }
    })().catch(() => { if (life.current.generation === generation) setError("push.failed"); }).finally(() => { if (life.current.generation === generation) setBusy(false); });
  }
  function disable() {
    if (busy) return;
    const generation = life.current.generation;
    setBusy(true); setError(null);
    void disablePush().then(() => { if (life.current.generation === generation) setActive(false); }, () => { if (life.current.generation === generation) { setActive(false); setError("push.failed"); } }).finally(() => { if (life.current.generation === generation) setBusy(false); });
  }
  return <section className="notification-settings" id="notifications" aria-labelledby="notification-title">
    <h2 id="notification-title"><T id="push.title" /></h2>
    <p><T id="push.lead" /></p>
    {!available && <p><T id="push.unsupported" /></p>}
    {state !== null && state.publicKey === null && <p><T id="push.unconfigured" /></p>}
    {error !== null && <p role="alert"><T id={error} /></p>}
    <button type="button" className="button button-ghost" role="switch" aria-checked={active} disabled={busy || !available || state?.publicKey == null} onClick={active ? disable : enable}><T id="push.device" /></button>
    <p className="small"><T id={active ? "push.on" : "push.off"} /></p>
    <fieldset disabled={busy || state === null}>
      <legend><T id="push.categories" /></legend>
      {categories.map((category) => <label key={category} className="notification-toggle"><span><T id={LABELS[category]} /></span><input type="checkbox" role="switch" aria-checked={state !== null && Boolean(state.enabled & BITS[category])} checked={state !== null && Boolean(state.enabled & BITS[category])} onChange={(e) => { void toggle(category, e.target.checked); }} /></label>)}
    </fieldset>
    <p className="small"><T id="push.scope" /></p>
  </section>;
}
