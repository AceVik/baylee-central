// One listed gateway. Its name and version are the operator's words, shown
// as text; its website is a link only because the directory's checks put it
// on the gateway's own origin.

import { useState } from "react";

import type { GatewayListing } from "../central/directory";
import { Icon } from "../components/icons";
import { T, t, useLang } from "../i18n";

export function GatewayCard({ g }: { g: GatewayListing }) {
  const lang = useLang();
  const [said, setSaid] = useState("");
  const copy = () => {
    if (!navigator.clipboard?.writeText) {
      setSaid(t(lang, "gw.copyImpossible"));
      return;
    }
    navigator.clipboard.writeText(g.origin).then(
      () => setSaid(t(lang, "gw.copied")),
      () => setSaid(t(lang, "gw.copyFailed")),
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
          {g.online ? <T id="gw.online" /> : <T id="gw.notSeen" />}
        </span>
      </div>
      <dl className="kv gw-kv">
        <div>
          <dt>
            <T id="gw.address" />
          </dt>
          <dd>
            <code>{g.origin}</code>
          </dd>
        </div>
        <div>
          <dt>
            <T id="gw.version" />
          </dt>
          <dd>
            <code>{g.metadata.version}</code>
          </dd>
        </div>
      </dl>
      <p className="gw-actions">
        <button type="button" className="button button-ghost" onClick={copy}>
          <Icon name="copy" className="button-icon" />
          <T id="gw.copy" />
        </button>
        <a className="button button-ghost" href={g.metadata.website} rel="noopener noreferrer nofollow" target="_blank">
          <T id="gw.website" />
        </a>
      </p>
      <output className="gw-said">{said}</output>
    </li>
  );
}
