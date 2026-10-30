// One listed gateway. Its name and version are the operator's words, shown
// as text; its website is a link only because the directory's checks put it
// on the gateway's own origin. The official gateway carries a mark, which
// this page sets by comparing the listed origin with the one it knows
// (`OFFICIAL_GATEWAY`), never by a name or anything else an operator sends.
// The first action opens the client on the gateway through its deep link
// (`central/deeplink.ts`); copying the address works with every client.

import { useState } from "react";

import type { GatewayListing } from "../central/directory";
import { gatewayLink } from "../central/deeplink";
import { Icon } from "../components/icons";
import { onLoopback } from "../config";
import { T, t, useLang } from "../i18n";

export function GatewayCard({ g, official = false }: { g: GatewayListing; official?: boolean }) {
  const lang = useLang();
  const [said, setSaid] = useState("");
  const link = gatewayLink(g.origin, onLoopback());
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
    <li className={`gw card card-link reveal${official ? " gw-is-official" : ""}`} aria-label={t(lang, "gw.cardLabel", { name: g.metadata.name })}>
      <div className="gw-head">
        <h3 className="gw-name">
          <bdi>{g.metadata.name}</bdi>
        </h3>
        {official && (
          <span className="pill gold gw-official">
            <T id="gw.official" />
          </span>
        )}
      </div>
      <p className={`gw-state pill ${g.online ? "ok" : "off"}`}>
        <span className={`dot ${g.online ? "ok live" : "off"}`} aria-hidden="true" />
        {g.online ? <T id="gw.online" /> : <T id="gw.notSeen" />}
      </p>
      <dl className="gw-facts">
        <div>
          <dt>
            <T id="gw.address" />
          </dt>
          <dd className="gw-address">
            <code translate="no">{g.origin}</code>
            <button type="button" className="round-btn gw-copy" onClick={copy} title={t(lang, "gw.copy")}>
              <Icon name="copy" />
              <span className="vh">
                <T id="gw.copy" />
              </span>
            </button>
          </dd>
        </div>
        <div>
          <dt>
            <T id="gw.version" />
          </dt>
          <dd>
            <code translate="no">{g.metadata.version}</code>
          </dd>
        </div>
      </dl>
      <div className="gw-actions">
        {link !== null && (
          <a className="button gw-open" href={link}>
            <Icon name="play" className="button-icon" />
            <T id="gw.open" />
          </a>
        )}
        <a className="button button-ghost" href={g.metadata.website} rel="noopener noreferrer nofollow" target="_blank">
          <Icon name="external" className="button-icon" />
          <T id="gw.website" />
        </a>
      </div>
      <output className="gw-said">{said}</output>
    </li>
  );
}
