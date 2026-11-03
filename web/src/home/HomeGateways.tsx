// The start page's gateways: the first few that Baylee Central lists, as the
// directory draws them (the official one first; an "Open in Baylee" link
// only behind the deep-link switch), with the way to all of them and to
// running one's own. Nothing here is made up: what Central does not list
// is not shown, and while it does not answer the section says so.

import { deepLinks, OFFICIAL_GATEWAY } from "../config";
import type { DirectoryFailure, GatewayListing } from "../central/directory";
import { Section } from "../components/Section";
import { Failure, officialFirst } from "../gateways/Directory";
import { GatewayCard } from "../gateways/GatewayCard";
import { T, t, useLang } from "../i18n";
import { Link, PATHS } from "../router";
import "../styles/gateways.css";

/** How many cards the start page shows; the directory has the rest. */
const SHOWN = 4;

export function HomeGateways({ gateways, loading, failure, retry }: {
  gateways: readonly GatewayListing[];
  loading: boolean;
  failure: DirectoryFailure | null;
  retry(): void;
}) {
  const lang = useLang();
  const online = gateways.filter((g) => g.online).length;
  const shown = officialFirst(gateways).slice(0, SHOWN);
  return (
    <Section id="gateways" eyebrow={<T id="gw.eyebrow" />} title={<T id="gw.title" />}>
      <p className="section-lead">
        <T id="home.gwLead" />
      </p>
      <output className="small dir-count hgw-count">
        {gateways.length > 0 && t(lang, "gw.count", { n: gateways.length, online })}
        {!loading && failure === null && gateways.length === 0 && t(lang, "gw.none")}
      </output>
      {shown.length > 0 && (
        <ul className="gateways grid-cards grid-centred stagger">
          {shown.map((g) => (
            <GatewayCard key={g.id} g={g} official={g.origin === OFFICIAL_GATEWAY} />
          ))}
        </ul>
      )}
      {loading && gateways.length === 0 && (
        <ul className="gateways grid-cards" aria-hidden="true">
          {[0, 1].map((i) => (
            <li key={i} className="gw card skeleton gw-skeleton" />
          ))}
        </ul>
      )}
      {failure !== null && <Failure kind={failure} onRetry={retry} />}
      {shown.length > 0 && deepLinks() && (
        <p className="small dir-hint">
          <T id="gw.openHint" />
        </p>
      )}
      <p className="hgw-foot">
        <Link className="button" to={PATHS.gateways}>
          <T id="beta.open" />
        </Link>
        <Link className="button button-ghost" to={`${PATHS.gateways}#selfhost`}>
          <T id="beta.own" />
        </Link>
      </p>
    </Section>
  );
}
