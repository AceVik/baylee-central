import { useEffect, useState, useSyncExternalStore } from "react";
import { useAccount } from "../account/session";
import { T } from "../i18n";
import { Link } from "../router";
import { installation, supported, watchInstallation } from "./client";

/** Browser permission survives logout; a device subscription deliberately does not. */
export function PushNotice() {
  const { account, generation } = useAccount();
  const nonce = useSyncExternalStore(watchInstallation, installation, () => null);
  const [dismissed, setDismissed] = useState<number | null>(null);
  useEffect(() => {
    let hadConsent = installation() !== null;
    return watchInstallation(() => {
      const hasConsent = installation() !== null;
      // An explicit opt-out must not immediately bring the offer back.
      if (hadConsent || hasConsent) setDismissed(generation);
      hadConsent = hasConsent;
    });
  }, [generation]);
  if (!account?.known || dismissed === generation || nonce !== null || !supported() || Notification.permission === "denied") return null;
  const later = () => setDismissed(generation);
  return <aside className="pwa-notice">
    <p><T id="push.prompt" /></p>
    <Link to="/account#notifications" className="button button-ghost" onClick={later}><T id="push.choose" /></Link>
    <button type="button" className="text-link" onClick={later}><T id="push.later" /></button>
  </aside>;
}
