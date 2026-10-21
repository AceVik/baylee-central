// Central's gateway directory, page by page, with what to say when Central
// does not answer or answers something this page does not take.

import { PAGE_SIZE, type DirectoryClient, type DirectoryFailure } from "../central/directory";
import { Icon } from "../components/icons";
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
        <ul className="gateways">
          {gateways.map((g) => (
            <GatewayCard key={g.id} g={g} />
          ))}
        </ul>
      )}
      {loading && gateways.length === 0 && (
        <ul className="gateways" aria-hidden="true">
          {[0, 1].map((i) => (
            <li key={i} className="gw glass gw-skeleton" />
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
