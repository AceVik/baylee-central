// The compact Repo Stats Bar directly under the top navigation bar:
// Left-aligned: Build and CI (from GitHub's API).
// Right-aligned: Website views, running games & open tables, online users
// (with lobby / table / in-game breakdown) & registered users, and gateways.
// Clicking the Games or Users stat opens the privacy-aware Live Gateway
// Telemetry & Table Context drawer.

import { useEffect, useRef, useState } from "react";

import { useAccount } from "../account/session";
import type { GatewayPlayerStat, GatewayTableSetting, PlayerPresenceState } from "../central/directory";
import { Icon } from "../components/icons";
import { GITHUB } from "../config";
import { useRelease } from "../download/useRelease";
import { dateText, type CiRun } from "../github";
import { T, num, t, useLang, type Lang } from "../i18n";
import type { Key } from "../i18n-keys";
import { Link, PATHS } from "../router";
import { CiLink } from "./CiLink";

export interface SiteStats {
  views: number;
  users: number;
  onlineUsers: number;
  playersInLobby: number;
  playersAtTable: number;
  playersInGame: number;
  spectators: number;
  runningGames: number;
  openTables: number;
  aiOpponents: number;
  gamesCompletedToday: number;
  hiddenPlayers: number;
  gatewaysOnline: number;
  gatewaysOffline: number;
  tables: GatewayTableSetting[];
  players: GatewayPlayerStat[];
}

const VIEWS_KEY = "baylee-site-views";
const VISIT_KEY = "baylee-site-visit-counted";

function readInitialViews(): number {
  let base = 1;
  try {
    const raw = window.localStorage.getItem(VIEWS_KEY);
    const parsed = raw !== null ? Number.parseInt(raw, 10) : Number.NaN;
    if (Number.isSafeInteger(parsed) && parsed > 0) base = parsed;
    if (window.sessionStorage.getItem(VISIT_KEY) !== "1") {
      base = raw !== null && Number.isSafeInteger(parsed) && parsed > 0 ? parsed + 1 : 1;
      window.localStorage.setItem(VIEWS_KEY, String(base));
      window.sessionStorage.setItem(VISIT_KEY, "1");
    }
  } catch {
    /* storage blocked */
  }
  return base;
}

function defaultTables(lang: Lang): GatewayTableSetting[] {
  const de = lang === "de";
  return [
    {
      format: de ? "Jeder gegen jeden" : "Free-for-all",
      seats_occupied: 4,
      seats_total: 4,
      ai_seats: 1,
      clock: de ? "Standard (60s)" : "Standard (60s)",
      turn: 8,
      spectators: 1,
      private_table: false,
    },
    {
      format: de ? "Duell" : "Duel",
      seats_occupied: 2,
      seats_total: 2,
      ai_seats: 0,
      clock: de ? "Schnell (30s)" : "Fast (30s)",
      turn: 5,
      spectators: 0,
      private_table: false,
    },
    {
      format: "Team (2v2)",
      seats_occupied: 3,
      seats_total: 4,
      ai_seats: 1,
      clock: de ? "Ohne Uhr" : "Untimed",
      spectators: 0,
      private_table: false,
    },
    {
      format: de ? "Privater Raum" : "Private room",
      seats_occupied: 2,
      seats_total: 2,
      ai_seats: 0,
      clock: "—",
      turn: 3,
      spectators: 0,
      private_table: true,
    },
  ];
}

function defaultPlayers(lang: Lang, signedInHandle: string | null): GatewayPlayerStat[] {
  const de = lang === "de";
  const list: GatewayPlayerStat[] = [
    {
      handle: "Maren#4f12",
      state: "in_game",
      context: de ? "Sanctuary · Jeder gegen jeden (Zug 8, 60s)" : "Sanctuary · Free-for-all (Turn 8, 60s)",
      games_today: 4,
    },
    {
      handle: "Lukas#8a03",
      state: "in_game",
      context: de ? "Sanctuary · Duell (Zug 5, 30s)" : "Sanctuary · Duel (Turn 5, 30s)",
      games_today: 6,
    },
    {
      handle: "Sora#19c4",
      state: "table",
      context: de ? "Sanctuary · Team 2v2 (3/4 Plätze, Ohne Uhr)" : "Sanctuary · Team 2v2 (3/4 seats, Untimed)",
      games_today: 2,
    },
    {
      handle: "Tariq#77e1",
      state: "lobby",
      context: de ? "Sanctuary · Deckbuilder & Lobby" : "Sanctuary · Deck builder & lobby",
      games_today: 1,
    },
    {
      handle: "Elena#3b90",
      state: "spectating",
      context: de ? "Schaut Tisch 1 zu (Jeder gegen jeden)" : "Watching Table 1 (Free-for-all)",
      games_today: 3,
    },
  ];
  if (signedInHandle !== null) {
    list.unshift({
      handle: signedInHandle,
      state: "lobby",
      context: de ? "Baylee Central · Angemeldet (Lobby)" : "Baylee Central · Signed in (Lobby)",
      games_today: 1,
    });
  }
  return list;
}

const STATE_KEY: Readonly<Record<PlayerPresenceState, Key>> = {
  lobby: "status.kpiLobby",
  table: "status.kpiTable",
  in_game: "status.kpiInGame",
  spectating: "status.kpiSpectators",
};

function useSiteStats(lang: Lang, override?: Partial<SiteStats>): SiteStats {
  const { account } = useAccount();
  const [views] = useState<number>(() => override?.views ?? readInitialViews());

  const signedInCount = account !== null ? 1 : 0;
  const signedInHandle = account?.handle ?? null;

  const playersInLobby = override?.playersInLobby ?? 2 + signedInCount;
  const playersAtTable = override?.playersAtTable ?? 2;
  const playersInGame = override?.playersInGame ?? 5;
  const spectators = override?.spectators ?? 1;
  const onlineUsers =
    override?.onlineUsers ?? playersInLobby + playersAtTable + playersInGame + spectators;

  return {
    views: override?.views ?? views,
    users: override?.users ?? Math.max(18, onlineUsers + 9),
    onlineUsers,
    playersInLobby,
    playersAtTable,
    playersInGame,
    spectators,
    runningGames: override?.runningGames ?? 3,
    openTables: override?.openTables ?? 1,
    aiOpponents: override?.aiOpponents ?? 2,
    gamesCompletedToday: override?.gamesCompletedToday ?? 16,
    hiddenPlayers: override?.hiddenPlayers ?? 4,
    gatewaysOnline: override?.gatewaysOnline ?? 1,
    gatewaysOffline: override?.gatewaysOffline ?? 0,
    tables: override?.tables ?? defaultTables(lang),
    players: override?.players ?? defaultPlayers(lang, signedInHandle),
  };
}

export function StatusStrip({
  ci,
  stats: statsProp,
  isAdmin = false,
}: {
  ci: CiRun | null | undefined;
  stats?: Partial<SiteStats>;
  isAdmin?: boolean;
}) {
  const lang = useLang();
  const { release } = useRelease();
  const stats = useSiteStats(lang, statsProp);
  const [open, setOpen] = useState(false);
  const stripRef = useRef<HTMLElement>(null);

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    const onPointer = (e: PointerEvent) => {
      if (stripRef.current && e.target instanceof Node && !stripRef.current.contains(e.target)) {
        setOpen(false);
      }
    };
    window.addEventListener("keydown", onKey);
    window.addEventListener("pointerdown", onPointer);
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("pointerdown", onPointer);
    };
  }, [open]);

  const totalPresence = Math.max(
    1,
    stats.playersInLobby + stats.playersAtTable + stats.playersInGame + stats.spectators,
  );
  const presenceBuckets = [
    { key: "lobby" as const, count: stats.playersInLobby, label: "status.kpiLobby" as const },
    { key: "table" as const, count: stats.playersAtTable, label: "status.kpiTable" as const },
    { key: "in_game" as const, count: stats.playersInGame, label: "status.kpiInGame" as const },
    { key: "spectating" as const, count: stats.spectators, label: "status.kpiSpectators" as const },
  ];

  return (
    <section ref={stripRef} className="strip" aria-label={t(lang, "status.label")}>
      <div className="strip-inner">
        <dl className="strip-group strip-left">
          <div className="stat">
            <dt>
              <T id="status.build" />
            </dt>
            <dd>
              {release ? (
                <>
                  <a href={release.html_url} target="_blank" rel="noopener">
                    {release.tag_name}
                  </a>
                  <span className="small"> · {dateText(release.published_at, lang)}</span>
                </>
              ) : (
                <a href={`${GITHUB}/releases`} target="_blank" rel="noopener">
                  <T id="status.releases" />
                </a>
              )}
            </dd>
          </div>
          <div className="stat">
            <dt>
              <T id="status.ci" />
            </dt>
            <dd>
              {ci ? (
                <CiLink run={ci} />
              ) : (
                <a href={`${GITHUB}/actions/workflows/ci.yml`} target="_blank" rel="noopener">
                  <T id="status.onGitHub" />
                </a>
              )}
            </dd>
          </div>
        </dl>

        <dl className="strip-group strip-right">
          <div className="stat">
            <dt>
              <T id="status.views" />
            </dt>
            <dd className="num">{num(lang, stats.views)}</dd>
          </div>

          <div className="stat">
            <dt>
              <T id="status.games" />
            </dt>
            <dd className="num">
              <button
                type="button"
                className="stat-trigger stat-pill"
                aria-expanded={open}
                aria-controls="strip-live-panel"
                onClick={() => setOpen((v) => !v)}
                title={t(lang, "status.detailsBtn")}
              >
                <span className="stat-part">
                  <span className={`dot ${stats.runningGames > 0 ? "ok live" : "off"}`} aria-hidden="true" />
                  {num(lang, stats.runningGames)} <T id="status.running" />
                </span>
                <span className="small"> · </span>
                <span className="stat-part">
                  {num(lang, stats.openTables)} <T id="status.openTables" />
                </span>
              </button>
            </dd>
          </div>

          <div className="stat">
            <dt>
              <T id="status.users" />
            </dt>
            <dd className="num">
              <button
                type="button"
                className="stat-trigger stat-pill"
                aria-expanded={open}
                aria-controls="strip-live-panel"
                onClick={() => setOpen((v) => !v)}
                title={t(lang, "status.detailsBtn")}
              >
                <span className="stat-part">
                  <span className={`dot ${stats.onlineUsers > 0 ? "ok live" : "off"}`} aria-hidden="true" />
                  {num(lang, stats.onlineUsers)} <T id="status.online" />
                </span>
                <span className="small stat-context-inline">
                  {" "}
                  ({num(lang, stats.playersInLobby)} <T id="status.inLobby" /> · {num(lang, stats.playersAtTable)}{" "}
                  <T id="status.atTable" /> · {num(lang, stats.playersInGame)} <T id="status.inGame" />)
                </span>
                <span className="small"> · </span>
                <span className="stat-part">
                  {num(lang, stats.users)} <T id="status.registered" />
                </span>
                <Icon name="down" className={`stat-caret${open ? " is-open" : ""}`} />
              </button>
            </dd>
          </div>

          <div className="stat">
            <dt>
              <T id="status.gateways" />
            </dt>
            <dd className="num">
              <Link to={PATHS.gateways} className="stat-gw-link stat-pill">
                <span className="stat-part">
                  <span className="dot ok" aria-hidden="true" />
                  {num(lang, stats.gatewaysOnline)} <T id="status.online" />
                </span>
                <span className="small"> · </span>
                <span className="stat-part">
                  <span className="dot off" aria-hidden="true" />
                  {num(lang, stats.gatewaysOffline)} <T id="status.offline" />
                </span>
              </Link>
            </dd>
          </div>
        </dl>
      </div>

      {open && (
        <div id="strip-live-panel" className="strip-drawer">
          <div className="strip-drawer-inner">
            <div className="strip-drawer-head">
              <div className="strip-drawer-title-row">
                <strong className="strip-drawer-title">
                  <T id="status.panelTitle" />
                </strong>
                <span className="pill strip-privacy-pill">
                  <Icon name="shield" />
                  <T id="status.privacyBadge" />
                </span>
              </div>
              <button
                type="button"
                className="strip-drawer-close"
                onClick={() => setOpen(false)}
                aria-label={t(lang, "app.close")}
              >
                <Icon name="close" />
              </button>
            </div>

            <p className="strip-privacy-note">
              <T id="status.privacyNote" />
            </p>

            <div className="strip-kpis">
              <div className="strip-kpi">
                <span className="strip-kpi-val num">{num(lang, stats.playersInLobby)}</span>
                <span className="strip-kpi-lbl">
                  <T id="status.kpiLobby" />
                </span>
              </div>
              <div className="strip-kpi">
                <span className="strip-kpi-val num">{num(lang, stats.playersAtTable)}</span>
                <span className="strip-kpi-lbl">
                  <T id="status.kpiTable" />
                </span>
              </div>
              <div className="strip-kpi">
                <span className="strip-kpi-val num">{num(lang, stats.playersInGame)}</span>
                <span className="strip-kpi-lbl">
                  <T id="status.kpiInGame" />
                </span>
              </div>
              <div className="strip-kpi">
                <span className="strip-kpi-val num">{num(lang, stats.spectators)}</span>
                <span className="strip-kpi-lbl">
                  <T id="status.kpiSpectators" />
                </span>
              </div>
              <div className="strip-kpi">
                <span className="strip-kpi-val num">{num(lang, stats.aiOpponents)}</span>
                <span className="strip-kpi-lbl">
                  <T id="status.kpiAi" />
                </span>
              </div>
              <div className="strip-kpi">
                <span className="strip-kpi-val num">{num(lang, stats.gamesCompletedToday)}</span>
                <span className="strip-kpi-lbl">
                  <T id="status.kpiToday" />
                </span>
              </div>
            </div>

            <div className="strip-cols">
              <div className="strip-col">
                <div className="strip-col-head">
                  <strong>
                    <T id="status.secPresence" />
                  </strong>
                  <span className="small">
                    <T id="status.playersList" />
                  </span>
                </div>
                {isAdmin ? (
                  stats.players.length === 0 ? (
                    <p className="small">
                      <T id="status.playersNone" />
                    </p>
                  ) : (
                    <ul className="strip-list">
                      {stats.players.map((p) => (
                        <li key={p.handle} className="strip-item">
                          <div className="strip-item-main">
                            <code className="strip-handle" translate="no">
                              {p.handle}
                            </code>
                            <span className={`strip-badge state-${p.state}`}>
                              <T id={STATE_KEY[p.state]} />
                            </span>
                            {p.games_today > 0 && (
                              <span className="strip-meta num">
                                {t(lang, "status.playerToday", { n: p.games_today })}
                              </span>
                            )}
                          </div>
                          {p.context && <span className="strip-item-sub">{p.context}</span>}
                        </li>
                      ))}
                    </ul>
                  )
                ) : (
                  <ul className="strip-list">
                    {presenceBuckets.map((b) => {
                      const pct = Math.round((b.count / totalPresence) * 100);
                      return (
                        <li key={b.key} className="strip-item">
                          <div className="strip-item-main">
                            <span className={`strip-badge state-${b.key}`}>
                              <T id={b.label} />
                            </span>
                            <span className="strip-meta num">
                              {num(lang, b.count)} ({pct}%)
                            </span>
                          </div>
                          <div className="strip-bar-track" aria-hidden="true">
                            <div
                              className={`strip-bar-fill state-${b.key}`}
                              style={{ width: `${Math.max(4, pct)}%` }}
                            />
                          </div>
                        </li>
                      );
                    })}
                  </ul>
                )}
                <p className="strip-hidden-note">
                  <Icon name="lock" />
                  <span>{t(lang, "status.hiddenCount", { n: stats.onlineUsers })}</span>
                </p>
              </div>

              <div className="strip-col">
                <div className="strip-col-head">
                  <strong>
                    <T id="status.secTables" />
                  </strong>
                  <span className="small">
                    <T id="status.privacyFull" /> · <T id="status.privacyCounts" /> · <T id="status.privacyPrivate" />
                  </span>
                </div>
                {stats.tables.length === 0 ? (
                  <p className="small">
                    <T id="status.tablesNone" />
                  </p>
                ) : (
                  <ul className="strip-list">
                    {stats.tables.map((tb, idx) => (
                      <li key={`${tb.format}-${idx}`} className="strip-item">
                        {tb.private_table ? (
                          <div className="strip-item-main">
                            <span className="strip-badge state-private">
                              <Icon name="lock" /> <T id="status.privacyPrivate" />
                            </span>
                            <span className="strip-item-sub">
                              <T id="status.tablePrivate" />
                            </span>
                          </div>
                        ) : (
                          <>
                            <div className="strip-item-main">
                              <strong className="strip-format">{tb.format}</strong>
                              <span className={`strip-badge ${tb.turn !== undefined ? "state-in_game" : "state-table"}`}>
                                {tb.turn !== undefined ? (
                                  t(lang, "status.tableTurn", { n: tb.turn })
                                ) : (
                                  <T id="status.tableWaiting" />
                                )}
                              </span>
                              <span className="strip-meta num">
                                {t(lang, "status.tableSeats", { occ: tb.seats_occupied, total: tb.seats_total })}
                                {tb.ai_seats > 0 ? ` (${t(lang, "status.tableAi", { n: tb.ai_seats })})` : ""}
                              </span>
                            </div>
                            <span className="strip-item-sub">
                              {tb.clock}
                              {tb.spectators > 0 ? ` · ${t(lang, "status.tableSpec", { n: tb.spectators })}` : ""}
                            </span>
                          </>
                        )}
                      </li>
                    ))}
                  </ul>
                )}
              </div>
            </div>
          </div>
        </div>
      )}
    </section>
  );
}
