// Central's gateway directory, page by page, with what to say when Central
// does not answer or answers something this page does not take. A field
// finds a gateway among those read so far by its name or address
// (Ctrl/Cmd+K reaches it), and a chip keeps to the ones online.

import { useState } from "react";

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

/** Whether `g` is one the words find: in its name, without case, or in its address. */
function found(g: GatewayListing, words: string): boolean {
  const w = words.trim().toLocaleLowerCase();
  return w === "" || g.metadata.name.toLocaleLowerCase().includes(w) || g.origin.toLowerCase().includes(w);
}

export function Directory({ client }: { client: DirectoryClient }) {
  const lang = useLang();
  const { gateways, more, loading, failure, next, retry } = useDirectory(client);
  const [words, setWords] = useState("");
  const [onlineOnly, setOnlineOnly] = useState(false);
  const online = gateways.filter((g) => g.online).length;
  const shown = officialFirst(gateways).filter((g) => found(g, words) && (!onlineOnly || g.online));
  const narrowed = words.trim() !== "" || onlineOnly;
  return (
    <div className="directory" aria-busy={loading}>
      {gateways.length > 1 && (
        <div className="dir-find">
          <label className="vh" htmlFor="dir-find">
            <T id="gw.find" />
          </label>
          <input
            id="dir-find"
            type="search"
            data-find=""
            aria-keyshortcuts="Control+K Meta+K"
            placeholder={t(lang, "gw.findHint")}
            value={words}
            maxLength={80}
            onChange={(e) => setWords(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Escape" && words !== "") {
                e.preventDefault();
                setWords("");
              }
            }}
          />
          <button type="button" className="chip-btn" aria-pressed={onlineOnly} onClick={() => setOnlineOnly(!onlineOnly)}>
            <span className="dot ok" aria-hidden="true" /> <T id="gw.onlineOnly" />
          </button>
        </div>
      )}
      <output className="small dir-count">
        {gateways.length > 0 && !narrowed && t(lang, "gw.count", { n: gateways.length, online })}
        {gateways.length > 0 && narrowed && t(lang, "gw.shown", { n: shown.length, all: gateways.length })}
        {!loading && failure === null && gateways.length === 0 && t(lang, "gw.none")}
      </output>
      {gateways.length > 0 && shown.length === 0 && (
        <p className="dir-none">
          <T id="gw.noMatch" />{" "}
          <button
            type="button"
            className="button button-ghost button-sm"
            onClick={() => {
              setWords("");
              setOnlineOnly(false);
            }}
          >
            <T id="gw.showAll" />
          </button>
        </p>
      )}
      {shown.length > 0 && (
        <>
          <ul className="gateways grid-cards grid-centred stagger">
            {shown.map((g) => (
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
