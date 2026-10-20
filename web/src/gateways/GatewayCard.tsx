// One listed gateway. Its name and version are the operator's words, shown
// as text; its website is a link only because the directory's checks put it
// on the gateway's own origin.

import { useState } from "react";

import type { GatewayListing } from "../central/directory";
import { Icon } from "../components/icons";
import { T, tt, useLang } from "../i18n";

export function GatewayCard({ g }: { g: GatewayListing }) {
  const lang = useLang();
  const [said, setSaid] = useState("");
  const copy = () => {
    if (!navigator.clipboard?.writeText) {
      setSaid(tt(lang, "Copying is not possible in this browser.", "Kopieren geht in diesem Browser nicht."));
      return;
    }
    navigator.clipboard.writeText(g.origin).then(
      () => setSaid(tt(lang, "Address copied.", "Adresse kopiert.")),
      () => setSaid(tt(lang, "Copying did not work.", "Kopieren hat nicht geklappt.")),
    );
  };
  return (
    <li className="gw glass">
      <div className="gw-head">
        <h3 className="gw-name">
          <bdi>{g.metadata.name}</bdi>
        </h3>
        <span className={`gw-state${g.online ? " on" : ""}`}>
          <span className={`dot ${g.online ? "ok" : "off"}`} aria-hidden="true" />
          {g.online ? <T en="online" de="online" /> : <T en="not seen lately" de="zuletzt nicht gesehen" />}
        </span>
      </div>
      <dl className="kv gw-kv">
        <div>
          <dt>
            <T en="Address" de="Adresse" />
          </dt>
          <dd>
            <code>{g.origin}</code>
          </dd>
        </div>
        <div>
          <dt>
            <T en="Version" de="Version" />
          </dt>
          <dd>
            <code>{g.metadata.version}</code>
          </dd>
        </div>
      </dl>
      <p className="gw-actions">
        <button type="button" className="button button-ghost" onClick={copy}>
          <Icon name="copy" className="button-icon" />
          <T en="Copy address" de="Adresse kopieren" />
        </button>
        <a className="button button-ghost" href={g.metadata.website} rel="noopener noreferrer nofollow" target="_blank">
          <T en="Website" de="Website" />
        </a>
      </p>
      <output className="gw-said">{said}</output>
    </li>
  );
}
