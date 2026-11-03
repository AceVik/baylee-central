// Start page Gateway Showcase & Interactive Directory (`#gateways`):
// - Sleek, fast, scannable list of registered community gateways (all servers,
//   including Baylee Sanctuary hosted by the Baylee operator, are community
//   gateways — there are no "official" servers).
// - Shows live status, version & compatibility, running games, open tables,
//   online player breakdown, table formats & available AI/LLM seats.
// - Actions per gateway:
//   1. Primary button: "Hinzufügen" / "Entfernen" (persisted in Central's local
//      gateway list, with smooth state transition).
//   2. Deep-Link button: "Verbinden" (`baylee://gateway/connect?origin=...`)
//      which opens the Baylee client and connects directly (or after login),
//      enabled only when the gateway is online and version-compatible.
//   3. External Website button: "Gateway-Webseite" (`target="_blank"`).

import { useEffect, useState } from "react";

import { useAccount } from "../account/session";
import { gatewayConnectLink } from "../central/deeplink";
import { Icon } from "../components/icons";
import { Section } from "../components/Section";
import { num, T, t, useLang, type Lang } from "../i18n";
import { Link, PATHS } from "../router";

const CLIENT_VERSION = "v0.1.0-beta.8b";

export interface ShowcaseGateway {
  origin: string;
  name: string;
  homeUrl: string;
  region: string;
  hostNoteDe: string;
  hostNoteEn: string;
  online: boolean;
  version: string;
  compatible: boolean;
  runningGames: number;
  openTables: number;
  playersOnline: number;
  inLobby: number;
  atTable: number;
  inGame: number;
  formatsDe: string;
  formatsEn: string;
  aiModels: string;
  descDe: string;
  descEn: string;
}

const STORAGE_KEY = "baylee-central-added-gateways";
const DEFAULT_ADDED = ["https://gateway.baylee.app"];

export const SHOWCASE_GATEWAYS: ShowcaseGateway[] = [
  {
    origin: "https://gateway.baylee.app",
    name: "Baylee Sanctuary",
    homeUrl: "https://gateway.baylee.app",
    region: "Frankfurt · EU-Central",
    hostNoteDe: "Community-Gateway (Baylee-Betreiber)",
    hostNoteEn: "Community gateway (Baylee operator)",
    online: true,
    version: CLIENT_VERSION,
    compatible: true,
    runningGames: 3,
    openTables: 2,
    playersOnline: 10,
    inLobby: 2,
    atTable: 3,
    inGame: 5,
    formatsDe: "Jeder gegen jeden (4–8), Duell, Team 2v2",
    formatsEn: "Free-for-All (4–8), Duel, Team 2v2",
    aiModels: "Haus-KI (5 Stufen), Gemini 2.5 Pro, Claude Sonnet",
    descDe:
      "Offenes Community-Gateway mit 19-Sprachen-Kartenkatalog, Haus-KI und gehosteten LLM-Plätzen für gemeinsame Runden.",
    descEn:
      "Open community gateway with 19-language card catalog, built-in House AI, and hosted LLM seats for casual and competitive tables.",
  },
  {
    origin: "https://play.nordlicht-tabletop.de",
    name: "Nordlicht Tabletop Hub",
    homeUrl: "https://play.nordlicht-tabletop.de",
    region: "Hamburg · EU-North",
    hostNoteDe: "Community-Gateway",
    hostNoteEn: "Community gateway",
    online: true,
    version: CLIENT_VERSION,
    compatible: true,
    runningGames: 2,
    openTables: 1,
    playersOnline: 7,
    inLobby: 1,
    atTable: 2,
    inGame: 4,
    formatsDe: "Mehrspieler-Ring (6 Plätze), Duell",
    formatsEn: "Multiplayer Ring (6 seats), Duel",
    aiModels: "Haus-KI, DeepSeek-R1, ChatGPT (GPT-4o)",
    descDe:
      "Community-Server für entspannte Abendrunden bis 8 Personen sowie Partien gegen lokale DeepSeek- und ChatGPT-Modelle.",
    descEn:
      "Community server for relaxed evening tables up to 8 players and matches against hosted DeepSeek and ChatGPT models.",
  },
  {
    origin: "https://aetherworks.gg",
    name: "Aetherworks Guild Server",
    homeUrl: "https://aetherworks.gg",
    region: "Amsterdam · EU-West",
    hostNoteDe: "Community-Gateway",
    hostNoteEn: "Community gateway",
    online: true,
    version: CLIENT_VERSION,
    compatible: true,
    runningGames: 1,
    openTables: 1,
    playersOnline: 4,
    inLobby: 2,
    atTable: 0,
    inGame: 2,
    formatsDe: "Turnier-Duell (30s), Team 2v2",
    formatsEn: "Tournament Duel (30s), Team 2v2",
    aiModels: "Haus-KI (Scharf & Experte), Claude",
    descDe:
      "Schnelles Trainings- und Turnier-Gateway mit Fokus auf 1v1-Duelle und 2v2-Teamrunden.",
    descEn:
      "Fast training and tournament gateway focused on 1v1 duels and 2v2 team matches.",
  },
  {
    origin: "https://citadel.tabletop-node.org",
    name: "Old Citadel Node",
    homeUrl: "https://citadel.tabletop-node.org",
    region: "Virginia · US-East",
    hostNoteDe: "Community-Gateway",
    hostNoteEn: "Community gateway",
    online: true,
    version: "v0.1.0-beta.7",
    compatible: false,
    runningGames: 0,
    openTables: 0,
    playersOnline: 1,
    inLobby: 1,
    atTable: 0,
    inGame: 0,
    formatsDe: "Wartet auf Protokoll-Update",
    formatsEn: "Awaiting protocol update",
    aiModels: "Haus-KI",
    descDe:
      "Community-Node auf älterer Protokollversion (direkte Client-Verbindung bis zum Server-Update deaktiviert).",
    descEn:
      "Community node running an older protocol version (direct client connection disabled until server update).",
  },
];

type FilterMode = "all" | "online" | "compatible" | "added";

function readAddedGateways(): string[] {
  if (typeof window === "undefined") return DEFAULT_ADDED;
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) return DEFAULT_ADDED;
    const parsed: unknown = JSON.parse(raw);
    if (Array.isArray(parsed) && parsed.every((x) => typeof x === "string")) {
      return parsed;
    }
  } catch {
    // ignore storage errors
  }
  return DEFAULT_ADDED;
}

function writeAddedGateways(origins: string[]): void {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(origins));
  } catch {
    // ignore storage errors
  }
}

export function HomeGateways() {
  const lang = useLang();
  const account = useAccount();
  const [addedOrigins, setAddedOrigins] = useState<string[]>(readAddedGateways);
  const [filter, setFilter] = useState<FilterMode>("all");
  const [query, setQuery] = useState("");
  const [copiedOrigin, setCopiedOrigin] = useState<string | null>(null);
  const [justToggled, setJustToggled] = useState<string | null>(null);

  useEffect(() => {
    writeAddedGateways(addedOrigins);
  }, [addedOrigins]);

  const toggleAdded = (origin: string) => {
    setAddedOrigins((prev) =>
      prev.includes(origin) ? prev.filter((o) => o !== origin) : [...prev, origin],
    );
    setJustToggled(origin);
    window.setTimeout(() => {
      setJustToggled((curr) => (curr === origin ? null : curr));
    }, 650);
  };

  const copyOrigin = async (origin: string) => {
    try {
      await navigator.clipboard.writeText(origin);
      setCopiedOrigin(origin);
      window.setTimeout(() => {
        setCopiedOrigin((curr) => (curr === origin ? null : curr));
      }, 1800);
    } catch {
      // clipboard unavailable
    }
  };

  const de = lang === "de";
  const q = query.trim().toLocaleLowerCase();

  const filtered = SHOWCASE_GATEWAYS.filter((g) => {
    if (filter === "online" && !g.online) return false;
    if (filter === "compatible" && (!g.online || !g.compatible)) return false;
    if (filter === "added" && !addedOrigins.includes(g.origin)) return false;
    if (!q) return true;
    return (
      g.name.toLocaleLowerCase().includes(q) ||
      g.origin.toLocaleLowerCase().includes(q) ||
      g.region.toLocaleLowerCase().includes(q) ||
      g.aiModels.toLocaleLowerCase().includes(q)
    );
  });

  return (
    <Section
      id="gateways"
      alt
      eyebrow={de ? "COMMUNITY-SERVER & LIVE-TELEMETRIE" : "COMMUNITY SERVERS & LIVE TELEMETRY"}
      title={de ? "Registrierte Gateways" : "Registered Gateways"}
    >
      <p className="section-lead reveal">
        {de
          ? "Alle Gateways werden von der Community betrieben – auch das Baylee Sanctuary. Füge Server deiner Central-Liste hinzu oder verbinde dich per Deep-Link direkt im Baylee-Client."
          : "Every gateway is community-run — including Baylee Sanctuary. Add servers to your Central list or connect directly in the Baylee client via deep-link."}
      </p>

      <div className="hgw-toolbar glass reveal">
        <div className="hgw-search-wrap">
          <label className="vh" htmlFor="hgw-search">
            <T id="gw.find" />
          </label>
          <input
            id="hgw-search"
            type="search"
            className="hgw-search"
            placeholder={
              de
                ? "Gateway, Region oder KI-Modell suchen …"
                : "Search gateway, region, or AI model …"
            }
            value={query}
            maxLength={80}
            onChange={(e) => setQuery(e.target.value)}
          />
        </div>
        <fieldset className="hgw-filters" aria-label={tFilterLabel(lang)}>
          {(
            [
              ["all", de ? "Alle" : "All", SHOWCASE_GATEWAYS.length],
              [
                "online",
                de ? "Online" : "Online",
                SHOWCASE_GATEWAYS.filter((g) => g.online).length,
              ],
              [
                "compatible",
                de ? "Kompatibel" : "Compatible",
                SHOWCASE_GATEWAYS.filter((g) => g.online && g.compatible).length,
              ],
              [
                "added",
                de ? "Gespeichert" : "Saved",
                SHOWCASE_GATEWAYS.filter((g) => addedOrigins.includes(g.origin)).length,
              ],
            ] as const
          ).map(([mode, text, count]) => (
            <button
              key={mode}
              type="button"
              className={`chip-btn hgw-filter-chip${filter === mode ? " is-active" : ""}`}
              aria-pressed={filter === mode}
              onClick={() => setFilter(mode)}
            >
              <span>{text}</span>
              <span className="hgw-chip-count num">{count}</span>
            </button>
          ))}
        </fieldset>
      </div>

      {filtered.length === 0 ? (
        <div className="hgw-empty glass reveal">
          <p>
            {de
              ? "Kein Gateway entspricht deinem aktuellen Filter."
              : "No gateway matches your current filter."}
          </p>
          <button
            type="button"
            className="button button-ghost"
            onClick={() => {
              setFilter("all");
              setQuery("");
            }}
          >
            {de ? "Filter zurücksetzen" : "Reset filters"}
          </button>
        </div>
      ) : (
        <ul className="hgw-list stagger">
          {filtered.map((g) => {
            const isAdded = addedOrigins.includes(g.origin);
            const canConnect = g.online && g.compatible;
            const connectHref = canConnect ? gatewayConnectLink(g.origin, false) : null;
            const isPulsing = justToggled === g.origin;

            return (
              <li
                key={g.origin}
                className={`hgw-card glass reveal${isAdded ? " is-added" : ""}${isPulsing ? " is-pulsing" : ""}`}
              >
                <div className="hgw-card-head">
                  <div className="hgw-title-wrap">
                    <h3 className="hgw-name">{g.name}</h3>
                    <span className="hgw-submeta">
                      {de ? g.hostNoteDe : g.hostNoteEn} · {g.region}
                    </span>
                  </div>
                  <div className="hgw-badges">
                    <span className={`pill hgw-state-pill ${g.online ? "ok" : "off"}`}>
                      <span className={`dot ${g.online ? "ok live" : "off"}`} aria-hidden="true" />
                      {g.online ? t(lang, "gw.online") : t(lang, "status.offline")}
                    </span>
                    <span
                      className={`pill hgw-version-pill ${g.compatible ? "ok" : "warn"}`}
                      title={
                        g.compatible
                          ? de
                            ? `Kompatibel mit Client ${CLIENT_VERSION}`
                            : `Compatible with client ${CLIENT_VERSION}`
                          : de
                            ? `Inkompatibel: erfordert ${CLIENT_VERSION}`
                            : `Incompatible: requires ${CLIENT_VERSION}`
                      }
                    >
                      <code className="num">{g.version}</code>
                      {!g.compatible && (
                        <span> · {de ? "Inkompatibel" : "Incompatible"}</span>
                      )}
                    </span>
                  </div>
                </div>

                <div className="hgw-origin-bar">
                  <code className="hgw-origin">{g.origin}</code>
                  <button
                    type="button"
                    className="hgw-copy-btn"
                    onClick={() => void copyOrigin(g.origin)}
                    data-copied={copiedOrigin === g.origin ? "" : undefined}
                  >
                    <Icon name={copiedOrigin === g.origin ? "check" : "copy"} />
                    <span>
                      {copiedOrigin === g.origin
                        ? t(lang, "gw.copied")
                        : t(lang, "gw.copy")}
                    </span>
                  </button>
                </div>

                <p className="hgw-desc">{de ? g.descDe : g.descEn}</p>

                <dl className="hgw-stats-strip">
                  <div className="hgw-stat-cell">
                    <dt>{de ? "Spiele & Tische" : "Games & Tables"}</dt>
                    <dd className="num">
                      <strong>{num(lang, g.runningGames)}</strong>{" "}
                      {de ? "laufend" : "running"} ·{" "}
                      <strong>{num(lang, g.openTables)}</strong>{" "}
                      {de ? "offen" : "open"}
                    </dd>
                  </div>
                  <div className="hgw-stat-cell">
                    <dt>{de ? "Spieler online" : "Players online"}</dt>
                    <dd className="num">
                      <strong>{num(lang, g.playersOnline)}</strong>{" "}
                      <span className="hgw-stat-detail">
                        ({num(lang, g.inLobby)} {de ? "Lobby" : "lobby"} ·{" "}
                        {num(lang, g.atTable)} {de ? "Tisch" : "table"} ·{" "}
                        {num(lang, g.inGame)} {de ? "Spiel" : "in game"})
                      </span>
                    </dd>
                  </div>
                  <div className="hgw-stat-cell hgw-stat-wide">
                    <dt>{de ? "Formate & KI" : "Formats & AI"}</dt>
                    <dd>
                      <span>{de ? g.formatsDe : g.formatsEn}</span>
                      <span className="hgw-sep" aria-hidden="true">
                        {" · "}
                      </span>
                      <span className="hgw-ai-inline">{g.aiModels}</span>
                    </dd>
                  </div>
                </dl>

                <div className="hgw-actions">
                  <button
                    type="button"
                    className={`button ${isAdded ? "button-ghost hgw-btn-remove" : "hgw-btn-add"}`}
                    aria-pressed={isAdded}
                    onClick={() => toggleAdded(g.origin)}
                  >
                    <Icon name={isAdded ? "trash" : "plus"} />
                    <span>{isAdded ? (de ? "Entfernen" : "Remove") : (de ? "Hinzufügen" : "Add")}</span>
                  </button>

                  {connectHref !== null ? (
                    <a
                      className="button button-ghost hgw-btn-connect"
                      href={connectHref}
                      title={
                        account
                          ? de
                            ? "Öffnet den Baylee-Client und verbindet direkt mit diesem Gateway"
                            : "Opens the Baylee client and connects directly to this gateway"
                          : de
                            ? "Öffnet den Baylee-Client (verbindet nach dem Login direkt mit diesem Gateway)"
                            : "Opens the Baylee client (connects to this gateway right after sign-in)"
                      }
                    >
                      <Icon name="play" />
                      <span>{de ? "Verbinden" : "Connect"}</span>
                    </a>
                  ) : (
                    <button
                      type="button"
                      className="button button-ghost hgw-btn-connect is-disabled"
                      disabled
                      title={
                        de
                          ? `Verbindung deaktiviert (Server nutzt ${g.version}, Client erwartet ${CLIENT_VERSION})`
                          : `Connection disabled (server runs ${g.version}, client expects ${CLIENT_VERSION})`
                      }
                    >
                      <Icon name="lock" />
                      <span>{de ? "Inkompatibel" : "Incompatible"}</span>
                    </button>
                  )}

                  <a
                    className="button button-ghost hgw-btn-web"
                    href={g.homeUrl}
                    target="_blank"
                    rel="noopener noreferrer nofollow"
                  >
                    <Icon name="external" />
                    <span>{de ? "Gateway-Webseite" : "Gateway Website"}</span>
                  </a>
                </div>
              </li>
            );
          })}
        </ul>
      )}

      <div className="hgw-foot reveal">
        <Link className="button button-ghost" to={PATHS.gateways}>
          <Icon name="globe" />
          <span>
            {de ? "Vollständiges Gateway-Verzeichnis" : "Full Gateway Directory"}
          </span>
        </Link>
        <Link className="button button-ghost" to={`${PATHS.gateways}#selfhost`}>
          <Icon name="server" />
          <span>
            {de ? "Eigenes Gateway selbst hosten" : "Self-host Your Own Gateway"}
          </span>
        </Link>
      </div>
    </Section>
  );
}

function tFilterLabel(lang: Lang): string {
  return lang === "de" ? "Gateways filtern" : "Filter gateways";
}
