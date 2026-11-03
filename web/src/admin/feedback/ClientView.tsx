// What the client sent with a report, and the game's record: read from
// Central's part route only when the report has them (`ReportsApi.part`),
// the client part inflated within its bounds (`client.ts`) and shown as
// facts, the window's picture (a checked PNG through a `blob:` address, given
// back when the view goes), the cards the text names as chips with
// Scryfall's pictures, the log, and the whole object as a tree. The record
// is not unpacked here: it is handed to the browser as the `.gz` Central
// keeps, read no further than its bound.

import { useCallback, useEffect, useId, useMemo, useRef, useState, type ReactNode } from "react";

import { ApiError } from "../../account/api";
import { Failed } from "../../app/AppFrame";
import { Icon } from "../../components/icons";
import { Modal } from "../../components/Modal";
import { BusyButton } from "../../account/Busy";
import { num, t, useLang, type Key } from "../../i18n";
import { failureKey } from "../shared";
import { BOUNDS, type RecordOrigin, type ReportsApi } from "./api";
import { cardImage, cardPage, readBuild, readClient, readCrash, readGame, readLog, readScreenshot, readSystem, spaced, Unreadable, type Json, type Refs, type Screenshot, type SystemRow } from "./client";
import { download } from "./csv";
import { JsonTree } from "./JsonTree";
import { useNotices } from "./pieces";

export type ClientLoad = { at: "none" } | { at: "loading" } | { at: "failed"; key: Key } | { at: "ready"; client: Json };

/**
 * The client part of report `hex`, read when it has arrived; `delay` waits
 * that long first, so stepping through a list in the drawer does not read
 * every report it passes.
 */
export function useClientPart(api: ReportsApi, id: Uint8Array, hex: string, received: boolean, delay = 0): [ClientLoad, () => void] {
  const [load, setLoad] = useState<{ hex: string; load: ClientLoad } | null>(null);
  // only the newest read's answer is kept
  const latest = useRef(0);
  const read = useCallback(() => {
    const mine = ++latest.current;
    api
      .part(id, "client", BOUNDS.clientGzip)
      .then((p) => readClient(p.gzip))
      .then(
        (client) => {
          if (mine === latest.current) setLoad({ hex, load: { at: "ready", client } });
        },
        (e: unknown) => {
          if (mine !== latest.current) return;
          const key: Key = e instanceof Unreadable ? (e.why === "large" ? "fb.errClientLarge" : "fb.errClientShape") : failureKey(e instanceof ApiError ? e.kind : "network");
          setLoad({ hex, load: { at: "failed", key } });
        },
      );
  }, [api, id, hex]);
  useEffect(() => {
    if (!received) return;
    const timer = window.setTimeout(read, delay);
    return () => {
      window.clearTimeout(timer);
      latest.current += 1;
    };
  }, [read, received, delay]);
  if (!received) return [{ at: "none" }, read];
  return [load?.hex === hex ? load.load : { at: "loading" }, read];
}

/** A card the text names: its page on Scryfall, its picture beside it under the pointer or the keyboard. */
function CardChip({ name, scryfallId, where }: { name: string; scryfallId: string | null; where: string }) {
  const lang = useLang();
  const tip = useId();
  const [shown, setShown] = useState(false);
  const [broken, setBroken] = useState(false);
  const page = scryfallId === null ? null : cardPage(scryfallId);
  const image = scryfallId === null ? null : cardImage(scryfallId);
  const label = where === "" ? name : `${name} · ${where}`;
  if (page === null) {
    return (
      <span className="fb-chip" title={label}>
        {name}
      </span>
    );
  }
  return (
    <span className="fb-chip-wrap" onPointerEnter={() => setShown(true)} onPointerLeave={() => setShown(false)}>
      <a className="fb-chip fb-chip-card" href={page} target="_blank" rel="noopener noreferrer" title={label} aria-describedby={shown && !broken ? tip : undefined} onFocus={() => setShown(true)} onBlur={() => setShown(false)}>
        {name}
        <span className="vh"> {t(lang, "fb.newTab")}</span>
      </a>
      {shown && image !== null && !broken && (
        <span className="fb-card-preview" role="tooltip" id={tip}>
          <img src={image} alt={t(lang, "fb.cardImage", { name })} width={244} height={340} loading="lazy" referrerPolicy="no-referrer" onError={() => setBroken(true)} />
          <span className="small">{t(lang, "fb.scryfall")}</span>
        </span>
      )}
    </span>
  );
}

/**
 * The player's words as written, each `[Card]` and `[@Player]` the client
 * resolved drawn as a chip in its place; a reference whose range does not
 * say exactly what the text says there is left as text.
 */
export function RefText({ text, refs }: { text: string; refs: Refs | null }) {
  const lang = useLang();
  if (refs === null) return <>{text}</>;
  const chars = [...text];
  const pieces: { from: number; to: number; node: ReactNode }[] = [];
  const spans = (at: [number, number] | null, want: string): [number, number] | null => {
    if (at === null) return null;
    const [from, to] = at;
    return from >= 0 && to <= chars.length && from < to && chars.slice(from, to).join("") === want ? [from, to] : null;
  };
  refs.cards.forEach((c, i) => {
    const at = spans(c.at, `[${c.name}]`);
    if (at === null) return;
    const where = [c.zone?.replaceAll("_", " "), c.owner === null ? null : t(lang, "fb.seat", { n: c.owner })].filter((p): p is string => typeof p === "string" && p !== "").join(", ");
    pieces.push({ from: at[0], to: at[1], node: <CardChip key={`c${i}`} name={c.name} scryfallId={c.scryfallId} where={where} /> });
  });
  refs.players.forEach((p, i) => {
    const at = spans(p.at, `[@${p.name}]`);
    if (at === null) return;
    pieces.push({
      from: at[0],
      to: at[1],
      node: (
        <span key={`p${i}`} className="fb-chip" title={p.seat === null ? p.name : t(lang, "fb.seat", { n: p.seat })}>
          @{p.name}
        </span>
      ),
    });
  });
  if (pieces.length === 0) return <>{text}</>;
  pieces.sort((a, b) => a.from - b.from);
  const out: ReactNode[] = [];
  let cursor = 0;
  for (const piece of pieces) {
    if (piece.from < cursor) continue;
    if (piece.from > cursor) out.push(chars.slice(cursor, piece.from).join(""));
    out.push(piece.node);
    cursor = piece.to;
  }
  if (cursor < chars.length) out.push(chars.slice(cursor).join(""));
  return <>{out}</>;
}

/** The window's picture through a `blob:` address of its checked bytes, given back when it goes. */
function Shot({ shot }: { shot: Screenshot }) {
  const lang = useLang();
  const [big, setBig] = useState(false);
  const url = useMemo(() => URL.createObjectURL(new Blob([shot.png.slice()], { type: "image/png" })), [shot]);
  useEffect(() => () => URL.revokeObjectURL(url), [url]);
  const alt = t(lang, "fb.shotAlt", { width: shot.width, height: shot.height });
  return (
    <>
      <button type="button" className="fb-shot" onClick={() => setBig(true)}>
        <img src={url} alt={alt} width={shot.width} height={shot.height} decoding="async" />
        <span className="vh">{t(lang, "fb.enlarge")}</span>
      </button>
      {big && (
        <Modal className="app-dialog fb-lightbox" label={t(lang, "fb.screenshot")} onClose={() => setBig(false)}>
          <img src={url} alt={alt} width={shot.width} height={shot.height} />
          <div className="app-dialog-actions">
            <button type="button" className="button" onClick={() => setBig(false)}>
              {t(lang, "app.close")}
            </button>
          </div>
        </Modal>
      )}
    </>
  );
}

const SYSTEM_WORD: Record<SystemRow, Key> = {
  platform: "fb.sysPlatform",
  cpus: "fb.sysCpus",
  adapter: "fb.sysAdapter",
  backend: "fb.sysBackend",
  window: "fb.sysWindow",
  scale: "fb.sysScale",
  lang: "fb.sysLang",
};

function Fold({ title, children, open = false }: { title: string; children: ReactNode; open?: boolean }) {
  return (
    <details className="fb-more" open={open}>
      <summary>{title}</summary>
      {children}
    </details>
  );
}

/** Everything the client sent, as facts first and as a tree last. */
export function ClientDetails({ load, onRetry, compact }: { load: ClientLoad; onRetry(): void; compact: boolean }) {
  const lang = useLang();
  if (load.at === "none") return <p className="small">{t(lang, "fb.noClientPart")}</p>;
  if (load.at === "loading") return <div className="skeleton fb-chart-skeleton" aria-busy="true" />;
  if (load.at === "failed") return <Failed reason={load.key} onRetry={onRetry} />;
  const c = load.client;
  const build = readBuild(c);
  const system = readSystem(c);
  const shot = readScreenshot(c);
  const game = readGame(c);
  const crash = readCrash(c);
  const log = readLog(c);
  const settings = c["settings"];
  return (
    <div className="fb-client">
      {shot !== null ? <Shot shot={shot} /> : <p className="small">{t(lang, "fb.noShot")}</p>}
      <dl className="kv fb-kv">
        {build !== null && (
          <div>
            <dt>{t(lang, "fb.clientBuild")}</dt>
            <dd>
              <code className="fb-code">{[build.version, build.commit?.slice(0, 12)].filter(Boolean).join(" · ") || "—"}</code>
            </dd>
          </div>
        )}
        {game !== null && (
          <>
            {game.when !== null && (
              <div>
                <dt>{t(lang, "fb.gameWhen")}</dt>
                <dd>{game.when}</dd>
              </div>
            )}
            {game.turn !== null && (
              <div>
                <dt>{t(lang, "fb.gameTurn")}</dt>
                <dd>{game.active === null ? num(lang, game.turn) : t(lang, game.active === game.seat ? "fb.gameOwnTurn" : "fb.gameSeatTurn", { n: game.turn, seat: game.active })}</dd>
              </div>
            )}
            {game.seat !== null && (
              <div>
                <dt>{t(lang, "fb.gameSeat")}</dt>
                <dd>{game.players === null ? num(lang, game.seat) : t(lang, "fb.gameSeatOf", { seat: game.seat, n: game.players })}</dd>
              </div>
            )}
            <div>
              <dt>{t(lang, "fb.gamePending")}</dt>
              <dd>{game.pending === null ? t(lang, "fb.gameNothing") : <code className="fb-code">{spaced(game.pending)}</code>}</dd>
            </div>
          </>
        )}
      </dl>
      {crash !== null && crash.backtrace !== null && (
        <Fold title={t(lang, "fb.backtrace")}>
          <pre className="fb-pre">{crash.backtrace}</pre>
        </Fold>
      )}
      {log !== null && (
        <Fold title={t(lang, "fb.log", { n: log.lines.length })}>
          {log.roster.length > 0 && <p className="small">{log.roster.map((p) => `${p.seat ?? "?"}: ${p.name}${p.ai ? " (AI)" : ""}`).join(" · ")}</p>}
          <ol className="fb-log">
            {log.lines.map((l, i) => (
              <li key={i}>
                {l.turn !== null && <span className="small num">T{l.turn} </span>}
                {l.text}
              </li>
            ))}
          </ol>
        </Fold>
      )}
      {system !== null && system.length > 0 && (
        <Fold title={t(lang, "fb.system")} open={!compact}>
          <dl className="kv fb-kv">
            {system.map(([row, value]) => (
              <div key={row}>
                <dt>{t(lang, SYSTEM_WORD[row])}</dt>
                <dd>{value}</dd>
              </div>
            ))}
          </dl>
        </Fold>
      )}
      {settings !== undefined && (
        <Fold title={t(lang, "fb.settings")}>
          <JsonTree value={settings} />
        </Fold>
      )}
      <Fold title={t(lang, "fb.everything")}>
        <JsonTree value={c} />
      </Fold>
    </div>
  );
}

/** The record as Central keeps it, a `.gz` handed to the browser; read no further than the bound of its origin. */
export function RecordDownload({ api, id, hex, origin }: { api: ReportsApi; id: Uint8Array; hex: string; origin: RecordOrigin | null }) {
  const lang = useLang();
  const notices = useNotices();
  const [busy, setBusy] = useState(false);
  const fetchIt = () => {
    if (busy) return;
    setBusy(true);
    api
      .part(id, "record", BOUNDS.recordGzip)
      .then(
        (p) => {
          download(`baylee-report-${hex.slice(-8)}-record.jsonl.gz`, p.gzip.slice(), "application/gzip");
          notices.say(t(lang, p.origin === "client" ? "fb.recordSavedClient" : "fb.recordSaved"));
        },
        (e: unknown) => notices.say(t(lang, failureKey(e instanceof ApiError ? e.kind : "network")), { bad: true }),
      )
      .finally(() => setBusy(false));
  };
  return (
    <BusyButton busy={busy} className="button button-ghost button-sm" onClick={fetchIt}>
      <Icon name="download" className="button-icon" />
      {t(lang, origin === "client" ? "fb.downloadClientRecord" : "fb.downloadRecord")}
    </BusyButton>
  );
}
