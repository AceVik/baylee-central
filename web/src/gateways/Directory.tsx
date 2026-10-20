// Central's gateway directory, page by page, with what to say when Central
// does not answer or answers something this page does not take.

import { PAGE_SIZE, type DirectoryClient, type DirectoryFailure } from "../central/directory";
import { Icon } from "../components/icons";
import { T, num, tt, useLang } from "../i18n";
import { GatewayCard } from "./GatewayCard";
import { useDirectory } from "./useDirectory";

function Failure({ kind, onRetry }: { kind: DirectoryFailure; onRetry: () => void }) {
  const lang = useLang();
  const words: Record<DirectoryFailure, [string, string]> = {
    unavailable: ["Baylee Central cannot list gateways just now.", "Baylee Central kann die Gateways gerade nicht auflisten."],
    busy: ["Baylee Central is busy; please try again in a moment.", "Baylee Central ist ausgelastet; bitte versuch es gleich noch einmal."],
    refused: [
      "Baylee Central did not answer this page's request. The directory may not be open to browsers yet.",
      "Baylee Central hat die Anfrage dieser Seite nicht beantwortet. Vielleicht ist das Verzeichnis für Browser noch nicht offen.",
    ],
    malformed: [
      "Baylee Central's answer did not pass this page's checks, so nothing of it is shown.",
      "Die Antwort von Baylee Central hat die Prüfungen dieser Seite nicht bestanden, deshalb wird nichts davon gezeigt.",
    ],
    network: ["Baylee Central could not be reached.", "Baylee Central war nicht erreichbar."],
  };
  return (
    <div className="dir-failure glass">
      <output>{tt(lang, words[kind][0], words[kind][1])}</output>
      <button type="button" className="button button-ghost" onClick={onRetry}>
        <Icon name="retry" className="button-icon" />
        <T en="Try again" de="Erneut versuchen" />
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
        {gateways.length > 0 &&
          tt(
            lang,
            `${num(lang, gateways.length)} ${gateways.length === 1 ? "gateway" : "gateways"} listed, ${num(lang, online)} online.`,
            `${num(lang, gateways.length)} ${gateways.length === 1 ? "Gateway" : "Gateways"} gelistet, ${num(lang, online)} online.`,
          )}
        {!loading && failure === null && gateways.length === 0 && tt(lang, "No gateway is listed yet.", "Noch ist kein Gateway gelistet.")}
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
            <T en={`Show the next ${PAGE_SIZE}`} de={`Die nächsten ${PAGE_SIZE} zeigen`} />
          </button>
        </p>
      )}
    </div>
  );
}
