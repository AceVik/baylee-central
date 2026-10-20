// The gateway directory: every gateway Baylee Central lists, read from its
// public `GatewayDirectory` call page by page, and how to run one of your
// own. Read-only; nothing here signs anybody in.

import { useCallback, useEffect, useMemo, useState } from "react";

import { CENTRAL_API, GITHUB, onLoopback } from "../config";
import { directoryClient, DirectoryError, PAGE_SIZE, type DirectoryClient, type DirectoryFailure, type GatewayListing } from "../central/directory";
import { Icon } from "../components/icons";
import { Section } from "../components/Layout";
import { T, num, tt, useLang, useTitle } from "../i18n";
import "../styles/gateways.css";

interface Listing {
  gateways: GatewayListing[];
  /** More may follow: the last page was full. */
  more: boolean;
  loading: boolean;
  failure: DirectoryFailure | null;
}

export function useDirectory(client: DirectoryClient) {
  const [state, setState] = useState<Listing>({ gateways: [], more: false, loading: true, failure: null });

  /** Asks for the page after `after`; the caller has marked the state as loading. */
  const fetchPage = useCallback(
    (after: string | null) =>
      client.page(after).then(
        (page) =>
          setState((s) => ({
            gateways: after === null ? page : [...s.gateways, ...page],
            more: page.length === PAGE_SIZE,
            loading: false,
            failure: null,
          })),
        (e: unknown) =>
          setState((s) => ({ ...s, loading: false, failure: e instanceof DirectoryError ? e.kind : "network" })),
      ),
    [client],
  );

  useEffect(() => {
    void fetchPage(null);
  }, [fetchPage]);

  const last = state.gateways.at(-1)?.id ?? null;
  const again = () => {
    setState((s) => ({ ...s, loading: true, failure: null }));
    void fetchPage(last);
  };
  return { ...state, next: again, retry: again };
}

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

function GatewayCard({ g }: { g: GatewayListing }) {
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

export default function Gateways({ client }: { client?: DirectoryClient }) {
  useTitle("Gateways – Baylee", "Gateways – Baylee");
  const own = useMemo(() => client ?? directoryClient({ api: CENTRAL_API, development: onLoopback() }), [client]);
  return (
    <>
      <header className="page-head">
        <div className="section-inner">
          <p className="eyebrow">
            <T en="Online" de="Online" />
          </p>
          <h1>
            <T en="Gateways" de="Gateways" />
          </h1>
          <p className="section-lead">
            <T
              en="A gateway is a server where online tables live: accounts, decks, rooms and one engine process per game. These are the ones registered with Baylee Central, whose operator checked who runs each before listing it. Baylee runs Sanctuary; the others are run by other people for their groups, and their own rules apply there."
              de="Ein Gateway ist ein Server für Online-Tische: Konten, Decks, Räume und ein Engine-Prozess pro Spiel. Hier stehen die bei Baylee Central registrierten; dessen Betreiber hat bei jedem geprüft, wer es betreibt, bevor er es aufgenommen hat. Baylee betreibt Sanctuary; die anderen betreiben andere für ihre Runde, und dort gelten deren eigene Regeln."
            />
          </p>
          <p className="small">
            <T
              en="To play on one, copy its address into the client's front door. Online means Central heard from the gateway within the last minute and a half; it is no promise that it answers. Names and versions are what each operator reports."
              de="Um auf einem zu spielen, kopiere seine Adresse in die Eingangstür des Clients. Online heißt, Central hat in den letzten anderthalb Minuten von dem Gateway gehört; ein Versprechen, dass es antwortet, ist das nicht. Namen und Versionen sind die Angaben der Betreiber."
            />
          </p>
          <Directory client={own} />
        </div>
      </header>

      <Section
        id="selfhost"
        alt
        eyebrow={<T en="Self-hosting" de="Selbst hosten" />}
        title={<T en="Run your own gateway" de="Ein eigenes Gateway betreiben" />}
      >
        <div className="two-col">
          <div>
            <p>
              <T
                en={
                  <>
                    You do not need our server. A gateway is three kinds of process: <code>baylee-gateway</code> handles
                    accounts, decks, rooms and routing and needs PostgreSQL; <code>baylee-agent</code> starts one engine
                    process per game; <code>baylee-engine-server</code> is one game. The first two run no rules at all.
                  </>
                }
                de={
                  <>
                    Du brauchst unseren Server nicht. Ein Gateway sind drei Arten von Prozessen: <code>baylee-gateway</code>{" "}
                    verwaltet Konten, Decks, Räume und das Routing und braucht PostgreSQL; <code>baylee-agent</code> startet
                    pro Spiel einen Engine-Prozess; <code>baylee-engine-server</code> ist ein Spiel. Die ersten beiden
                    kennen keine Regeln.
                  </>
                }
              />
            </p>
            <p>
              <T
                en={
                  <>
                    Put a TLS proxy in front, set <code>BAYLEE_REGISTRATION=invite</code> if you want your own closed
                    circle, create keys with <code>baylee-gateway invite create</code>, and the client&apos;s front door
                    takes your address like any other. Card text in 19 languages is an optional ingest of Scryfall&apos;s
                    bulk data into the same database.
                  </>
                }
                de={
                  <>
                    Setz einen TLS-Proxy davor, setz <code>BAYLEE_REGISTRATION=invite</code>, wenn du einen eigenen
                    geschlossenen Kreis willst, erzeuge Schlüssel mit <code>baylee-gateway invite create</code>, und die
                    Eingangstür des Clients nimmt deine Adresse wie jede andere. Kartentext in 19 Sprachen ist ein
                    optionaler Import von Scryfalls Bulk-Daten in dieselbe Datenbank.
                  </>
                }
              />
            </p>
            <p>
              <T
                en={
                  <>
                    <a href={`${GITHUB}#running-your-own-gateway`} rel="noopener">
                      The self-hosting guide
                    </a>{" "}
                    walks through it;{" "}
                    <a href={`${GITHUB}/blob/main/docs/protocol.md`} rel="noopener">
                      docs/protocol.md
                    </a>{" "}
                    is the reference.
                  </>
                }
                de={
                  <>
                    <a href={`${GITHUB}/blob/main/README.de.md#ein-eigenes-gateway-betreiben`} rel="noopener">
                      Die Anleitung
                    </a>{" "}
                    geht alles durch;{" "}
                    <a href={`${GITHUB}/blob/main/docs/protocol.md`} rel="noopener">
                      docs/protocol.md
                    </a>{" "}
                    ist die Referenz.
                  </>
                }
              />
            </p>
          </div>
          <div>
            <pre className="code" aria-label="Shell">
              <code>
                {`docker compose up -d
export DATABASE_URL=postgres://baylee:baylee@127.0.0.1:5432/baylee
cargo build --release -p baylee-gateway -p baylee-agent -p baylee-engine-server

export BAYLEE_AGENT_TOKEN=$(openssl rand -hex 32)
RUST_LOG=info BAYLEE_GATEWAY_NAME="My table" ./target/release/baylee-gateway
RUST_LOG=info ./target/release/baylee-agent`}
              </code>
            </pre>
            <p className="small">
              <T
                en={
                  <>
                    Every gateway answers <code>GET /source</code> with its licence, version and commit, without sign-in.
                    The AGPL asks you to offer your source to the people who use your server; that route does it for you.
                  </>
                }
                de={
                  <>
                    Jedes Gateway beantwortet <code>GET /source</code> mit Lizenz, Version und Commit, ohne Anmeldung. Die
                    AGPL verlangt, dass du den Leuten, die deinen Server nutzen, den Quellcode anbietest; diese Route
                    erledigt das für dich.
                  </>
                }
              />
            </p>
          </div>
        </div>
      </Section>
    </>
  );
}
