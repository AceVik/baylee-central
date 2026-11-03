import { useEffect } from "react";
import { register } from "./client";
import { PushNotice } from "./PushNotice";
import "../styles/pwa.css";

export function PwaNotice() {
  useEffect(() => {
    if (!import.meta.env.PROD || !("serviceWorker" in navigator)) return;
    let live = true;
    let r: ServiceWorkerRegistration | undefined;
    void register()
      .then((value) => {
        if (!live) return;
        r = value;
      })
      .catch(() => {});
    const update = () => {
      if (document.visibilityState === "visible") void r?.update().catch(() => {});
    };
    const timer = window.setInterval(update, 60 * 60_000);
    document.addEventListener("visibilitychange", update);
    return () => {
      live = false;
      clearInterval(timer);
      document.removeEventListener("visibilitychange", update);
    };
  }, []);

  return <PushNotice />;
}
