// A secret shown once (docs/website-design.md §"The secret box"): mono,
// selectable, a copy button, the warning above it. The dialog that holds it
// closes only when the person says they kept it; nothing of it is stored by
// the page, put in an address or logged.

import { useState } from "react";

import { Icon } from "../components/icons";
import { T, t, useLang } from "../i18n";

export function SecretBox({ secret, label }: { secret: string; label: string }) {
  const lang = useLang();
  const [said, setSaid] = useState("");
  const copy = () => {
    if (!navigator.clipboard?.writeText) {
      setSaid(t(lang, "gw.copyImpossible"));
      return;
    }
    navigator.clipboard.writeText(secret).then(
      () => setSaid(t(lang, "app.secretCopied")),
      () => setSaid(t(lang, "gw.copyFailed")),
    );
  };
  return (
    <div className="secret">
      <p className="secret-warn">
        <Icon name="warn" />
        <span>
          <T id="app.secretOnce" />
        </span>
      </p>
      <code aria-label={label} translate="no">
        {secret}
      </code>
      <div className="secret-actions">
        <button type="button" className="button button-ghost button-sm" onClick={copy}>
          <Icon name="copy" className="button-icon" />
          <T id="gw.copyShort" />
        </button>
        <output className="small">{said}</output>
      </div>
    </div>
  );
}
