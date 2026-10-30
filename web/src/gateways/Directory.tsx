// Central's gateway directory, page by page, with what to say when Central
// does not answer or answers something this page does not take.

import { PAGE_SIZE, type DirectoryClient, type DirectoryFailure, type GatewayListing } from "../central/directory";
import { Icon } from "../components/icons";
import { deepLinks, OFFICIAL_GATEWAY } from "../config";
import { T, t, useLang, type Key } from "../i18n";
import { GatewayCard } from "./GatewayCard";
import { useDirectory } from "./useDirectory";

const FAILURE: Record<DirectoryFailure, Key> = {
  unavailable: "gw.unavailable",
  busy: "gw.busy",
  refused: "gw.refused",
  malformed: "gw.malformed",
  network: "gw.network",
};

function Failure({ kind, onRetry }: { kind: DirectoryFailure; onRetry: () => void }) {
  const lang = useLang();
  return (
    <div className="dir-failure glass">
      <output>{t(lang, FAILURE[kind])}</output>
      <button type="button" className="button button-ghost" onClick={onRetry}>
        <Icon name="retry" className="button-icon" />
        <T id="gw.tryAgain" />
      </button>
    </div>
  );
}

/**
 * The official gateway first, the rest in Central's order. Only what is
 * drawn moves: the pages keep Central's order, whose last id is the next
 * page's cursor.
 */
function officialFirst(gateways: readonly GatewayListing[]): GatewayListing[] {
  return [...gateways.filter((g) => g.origin === OFFICIAL_GATEWAY), ...gateways.filter((g) => g.origin !== OFFICIAL_GATEWAY)];
}

export function Directory({ client }: { client: DirectoryClient }) {
  const lang = useLang();
  const { gateways, more, loading, failure, next, retry } = useDirectory(client);
  const online = gateways.filter((g) => g.online).length;
  return (
    <div className="directory" aria-busy={loading}>
      <output className="small dir-count">
        {gateways.length > 0 && t(lang, "gw.count", { n: gateways.length, online })}
        {!loading && failure === null && gateways.length === 0 && t(lang, "gw.none")}
      </output>
      {gateways.length > 0 && (
        <>
          <ul className="gateways grid-cards stagger">
            {officialFirst(gateways).map((g) => (
              <GatewayCard key={g.id} g={g} official={g.origin === OFFICIAL_GATEWAY} />
            ))}
          </ul>
          {deepLinks() && (
            <p className="small dir-hint">
              <T id="gw.openHint" />
            </p>
          )}
        </>
      )}
      {loading && gateways.length === 0 && (
        <ul className="gateways grid-cards" aria-hidden="true">
          {[0, 1, 2].map((i) => (
            <li key={i} className="gw card skeleton gw-skeleton" />
          ))}
        </ul>
      )}
      {failure !== null && <Failure kind={failure} onRetry={retry} />}
      {more && failure === null && (
        <p className="dir-more">
          <button type="button" className="button button-ghost" onClick={next} disabled={loading}>
            <T id="gw.next" args={{ n: PAGE_SIZE }} />
          </button>
        </p>
      )}
    </div>
  );
}
