// Sets and card status. A summary first: the whole corpus with one bar, the
// finished sets, and the next three in release order. "Show all sets" opens
// the full list in eras of four years, whose rows exist only while an era is
// open; a search lists its matches flat (at most 40, the count announced).
// `#set-<code>` in the address opens that set wherever it is.

import { useCallback, useEffect, useMemo, useState, useTransition, type ReactNode } from "react";

import { T, num, tt, useLang } from "../i18n";
import { reducedMotion } from "../motion";
import "../styles/cards.css";
import { CardGrid } from "./CardGrid";
import { loadCards, loadSets } from "./data";
import { begun, buildable, eraOf, eras, finished, matches, pct, sum, type Card, type Counts, type SetSummary, type SetsDoc } from "./model";

const NEXT_UP = 3;
const FOUND_MAX = 40;

/** `undefined` while loading, `null` when it did not load. */
export function useSetsDoc(enabled = true): SetsDoc | null | undefined {
  const [doc, setDoc] = useState<SetsDoc | null | undefined>(undefined);
  useEffect(() => {
    if (!enabled) return;
    let live = true;
    loadSets().then(
      (d) => live && setDoc(d),
      () => live && setDoc(null),
    );
    return () => {
      live = false;
    };
  }, [enabled]);
  return doc;
}

/** Full, untested and partial as three parts of one bar; the rest is the track. */
export function Bar({ counts }: { counts: Counts }) {
  const of = buildable(counts);
  const share = (n: number) => `${of ? (n * 100) / of : 0}%`;
  return (
    <span className="pbar" aria-hidden="true">
      {/* CSSOM widths (React sets `style` property by property), which the CSP allows */}
      <span className="pbar-full" style={{ width: of ? share(counts.implemented) : "100%" }} />
      <span className="pbar-generated" style={{ width: share(counts.generated) }} />
      <span className="pbar-partial" style={{ width: share(counts.partial) }} />
    </span>
  );
}

function CountsLine({ counts }: { counts: Counts }) {
  const lang = useLang();
  const part = (cls: string, n: number, en: string, de: string) => (
    <span className={`c c-${cls}`}>
      {num(lang, n)} {tt(lang, en, de)}
    </span>
  );
  return (
    <span className="set-counts">
      {part("implemented", counts.implemented, "full", "voll")}
      {part("generated", counts.generated, "untested", "ungetestet")}
      {part("partial", counts.partial, "partial", "teilweise")}
      {part("none", counts.unimplemented + counts.absent, "not yet", "noch nicht")}
      {counts.never > 0 && part("never", counts.never, "never", "nie")}
    </span>
  );
}

function Progress({ counts }: { counts: Counts }) {
  const lang = useLang();
  const p = pct(counts.implemented, buildable(counts));
  return (
    <span className="set-progress">
      <span className={`set-pct${p === 100 ? " done" : ""}`} aria-hidden="true">
        {p} %
      </span>
      <span className="vh">{tt(lang, `${p} percent full`, `${p} Prozent voll`)}</span>
      <Bar counts={counts} />
    </span>
  );
}

function SetCards({ set }: { set: SetSummary }) {
  const lang = useLang();
  const [cards, setCards] = useState<Card[] | null | undefined>(undefined);
  const [, startTransition] = useTransition();
  useEffect(() => {
    let live = true;
    loadCards(set.code).then(
      (c) => live && startTransition(() => setCards(c)),
      () => live && setCards(null),
    );
    return () => {
      live = false;
    };
  }, [set.code]);
  if (cards === undefined) {
    return (
      <output className="small block">
        <T en="Loading…" de="Lädt …" />
      </output>
    );
  }
  if (cards === null) {
    return (
      <output className="small block">
        <T
          en="The card list did not load. Close and open the set to try again."
          de="Die Kartenliste wurde nicht geladen. Schließe das Set und öffne es erneut."
        />
      </output>
    );
  }
  return (
    <>
      <CardGrid cards={cards} counts={set} setName={set.name} />
      <p className="small set-foot">
        <a href={`https://scryfall.com/sets/${encodeURIComponent(set.code)}`} rel="noopener">
          {tt(lang, "This set on Scryfall", "Dieses Set bei Scryfall")}
        </a>
        {" · "}
        {tt(lang, "Images: Scryfall · © Wizards of the Coast", "Bilder: Scryfall · © Wizards of the Coast")}
      </p>
    </>
  );
}

/** A row that opens; `children` are rendered only while it is open. */
function Row({
  id,
  className,
  open,
  onOpen,
  title,
  meta,
  counts,
  children,
}: {
  id?: string;
  className: string;
  open: boolean;
  onOpen: (open: boolean) => void;
  title: string;
  meta: ReactNode;
  counts: Counts;
  children: () => ReactNode;
}) {
  return (
    <details className={className} id={id} open={open} onToggle={(e) => onOpen(e.currentTarget.open)}>
      <summary className="set-sum">
        <span className="set-title">
          <span className="set-name">{title}</span>
          <span className="set-meta">{meta}</span>
        </span>
        <Progress counts={counts} />
        <CountsLine counts={counts} />
      </summary>
      <div className="set-body">{open && children()}</div>
    </details>
  );
}

function SetRow({ set, withId, open, onOpen }: { set: SetSummary; withId?: boolean; open: boolean; onOpen: (code: string, open: boolean) => void }) {
  const lang = useLang();
  const year = set.released.slice(0, 4);
  return (
    <Row
      {...(withId === true ? { id: `set-${set.code}` } : {})}
      className="set"
      open={open}
      onOpen={(o) => onOpen(set.code, o)}
      title={set.name}
      meta={`${set.code.toUpperCase()}${year ? ` · ${year}` : ""} · ${num(lang, set.total)} ${set.total === 1 ? tt(lang, "card", "Karte") : tt(lang, "cards", "Karten")}`}
      counts={set}
    >
      {() => <SetCards set={set} />}
    </Row>
  );
}

function useOpenSet(): [ReadonlySet<string>, (code: string, open: boolean) => void, (codes: string[]) => void] {
  const [open, setOpen] = useState<ReadonlySet<string>>(new Set());
  const toggle = useCallback(
    (code: string, o: boolean) =>
      setOpen((prev) => {
        if (prev.has(code) === o) return prev;
        const next = new Set(prev);
        if (o) next.add(code);
        else next.delete(code);
        return next;
      }),
    [],
  );
  const add = useCallback((codes: string[]) => setOpen((prev) => new Set([...prev, ...codes])), []);
  return [open, toggle, add];
}

function Summary({ doc }: { doc: SetsDoc }) {
  const lang = useLang();
  const [open, toggle] = useOpenSet();
  const tot = doc.totals;
  const all = doc.corpus - tot.never;
  const p = pct(tot.implemented, all);
  const done = doc.sets.filter(finished);
  const next = doc.sets.filter((s) => buildable(s) > 0 && !finished(s)).slice(0, NEXT_UP);
  const n = (x: number) => num(lang, x);
  return (
    <div className="sets-summary">
      <div className="sets-overall glass">
        <p className="sets-total">
          {tt(
            lang,
            `${n(doc.sets.length)} sets, ${n(doc.corpus)} cards: ${n(tot.implemented)} full (${p} %), ${n(tot.generated)} untested, ${n(tot.partial)} partial, ${n(tot.unimplemented + tot.absent)} not yet, ${n(tot.never)} never. Measured at commit ${doc.rev.slice(0, 9)}.`,
            `${n(doc.sets.length)} Sets, ${n(doc.corpus)} Karten: ${n(tot.implemented)} voll (${p} %), ${n(tot.generated)} ungetestet, ${n(tot.partial)} teilweise, ${n(tot.unimplemented + tot.absent)} noch nicht, ${n(tot.never)} nie. Gemessen an Commit ${doc.rev.slice(0, 9)}.`,
          )}
        </p>
        <Bar counts={{ ...tot, total: doc.corpus }} />
      </div>
      {done.length > 0 && (
        <>
          <h4 className="sets-sub">
            <T en="Finished" de="Fertig" />
          </h4>
          <ul className="set-chips">
            {done.map((s) => (
              <li key={s.code}>
                <a className="chip" href={`#set-${s.code}`}>
                  {s.name}
                </a>
              </li>
            ))}
          </ul>
        </>
      )}
      {next.length > 0 && (
        <>
          <h4 className="sets-sub">
            <T en="Next, in release order" de="Als Nächstes, in Erscheinungsreihenfolge" />
          </h4>
          <div className="sets">
            {next.map((s) => (
              <SetRow key={s.code} set={s} open={open.has(s.code)} onOpen={toggle} />
            ))}
          </div>
        </>
      )}
    </div>
  );
}

function AllSets({ doc, target, onTargetShown }: { doc: SetsDoc; target: string | null; onTargetShown: () => void }) {
  const lang = useLang();
  const [query, setQuery] = useState("");
  const [needle, setNeedle] = useState("");
  const [onlyBegun, setOnlyBegun] = useState(false);
  const [openEras, setOpenEras] = useState<ReadonlySet<number>>(new Set());
  const [openSets, toggleSet, addSets] = useOpenSet();

  useEffect(() => {
    const t = window.setTimeout(() => setNeedle(query), 150);
    return () => window.clearTimeout(t);
  }, [query]);

  // a link to one set: its era open, then its row (set while rendering, so
  // the rows are there in the same commit), then scrolled to and focused
  const [handled, setHandled] = useState<string | null>(null);
  if (target !== null && target !== handled) {
    setHandled(target);
    const s = doc.sets.find((x) => x.code === target);
    if (s) {
      setQuery("");
      setNeedle("");
      if (!begun(s)) setOnlyBegun(false);
      setOpenEras((prev) => new Set([...prev, eraOf(s)]));
      addSets([s.code]);
    }
  }
  useEffect(() => {
    if (target === null || handled !== target) return;
    const frame = requestAnimationFrame(() => {
      const row = document.getElementById(`set-${target}`);
      if (row) {
        row.scrollIntoView({ block: "start", behavior: reducedMotion() ? "auto" : "smooth" });
        row.querySelector("summary")?.focus({ preventScroll: true });
      }
      onTargetShown();
    });
    return () => cancelAnimationFrame(frame);
  }, [target, handled, onTargetShown]);

  const chosen = useMemo(() => doc.sets.filter((s) => !onlyBegun || begun(s)), [doc, onlyBegun]);
  const hits = useMemo(() => (needle.trim() ? chosen.filter((s) => matches(s, needle)) : null), [chosen, needle]);

  let found = "";
  if (hits) {
    const n = hits.length;
    if (n === 0) found = tt(lang, "No set matches.", "Kein Set passt.");
    else if (n > FOUND_MAX)
      found = tt(
        lang,
        `${n} sets match; the first ${FOUND_MAX} are shown. Type more to narrow it.`,
        `${n} Sets passen; die ersten ${FOUND_MAX} stehen hier. Tippe mehr, um einzugrenzen.`,
      );
    else found = n === 1 ? tt(lang, "1 set matches.", "1 Set passt.") : tt(lang, `${n} sets match.`, `${n} Sets passen.`);
  }

  return (
    <div className="sets-all" id="sets-all">
      <div className="sets-tools">
        <label className="field sets-search">
          <span>
            <T en="Find a set" de="Set suchen" />
          </span>
          <input
            type="search"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            autoComplete="off"
            spellCheck={false}
            aria-describedby="sets-found"
          />
        </label>
        <label className="check-label">
          <input type="checkbox" checked={onlyBegun} onChange={(e) => setOnlyBegun(e.target.checked)} />{" "}
          <T en="only sets that have begun" de="nur begonnene Sets" />
        </label>
      </div>
      <output className="small sets-found" id="sets-found">
        {found}
      </output>
      <div className="sets">
        {hits
          ? hits.slice(0, FOUND_MAX).map((s) => <SetRow key={s.code} set={s} withId open={openSets.has(s.code)} onOpen={toggleSet} />)
          : eras(chosen).map((era) => {
              const counts = sum(era.sets);
              return (
                <Row
                  key={era.from}
                  className="set set-era"
                  open={openEras.has(era.from)}
                  onOpen={(o) =>
                    setOpenEras((prev) => {
                      if (prev.has(era.from) === o) return prev;
                      const next = new Set(prev);
                      if (o) next.add(era.from);
                      else next.delete(era.from);
                      return next;
                    })
                  }
                  title={era.from === era.to ? String(era.from) : `${era.from}–${era.to}`}
                  meta={`${num(lang, era.sets.length)} ${era.sets.length === 1 ? tt(lang, "set", "Set") : "Sets"} · ${num(lang, counts.total)} ${tt(lang, "cards", "Karten")}`}
                  counts={counts}
                >
                  {() => (
                    <div className="sets era-body">
                      {era.sets.map((s) => (
                        <SetRow key={s.code} set={s} withId open={openSets.has(s.code)} onOpen={toggleSet} />
                      ))}
                    </div>
                  )}
                </Row>
              );
            })}
      </div>
    </div>
  );
}

function targetOf(hash: string): string | null {
  const m = /^#set-([a-z0-9]+)$/.exec(hash);
  return m?.[1] ?? null;
}

export function SetsBlock({ hash }: { hash: string }) {
  const lang = useLang();
  const doc = useSetsDoc();
  const [target, setTarget] = useState<string | null>(() => targetOf(hash));
  const [expanded, setExpanded] = useState(() => targetOf(hash) !== null);
  const shown = useCallback(() => setTarget(null), []);
  // a new `#set-<code>` opens the full list at that set
  const [seen, setSeen] = useState(hash);
  if (hash !== seen) {
    setSeen(hash);
    const t = targetOf(hash);
    if (t !== null) {
      setTarget(t);
      setExpanded(true);
    }
  }

  return (
    <div className="sets-block" id="sets">
      <h2 id="sets-title">
        <T en="Sets and card status" de="Sets und Kartenstatus" />
      </h2>
      <p className="section-lead">
        <T
          en="Every set in release order, each card counted under the set it first appeared in. Open a set to see its cards: whether each plays as printed, plays but is not yet tested, plays in part, is not playable yet, or will never be built. Rest the pointer on a card, or focus it, to see it large with the reason; a click opens its page on Scryfall, with artist and printing. On a touch screen a tap shows it large first."
          de="Jedes Set in Erscheinungsreihenfolge, jede Karte unter dem Set gezählt, in dem sie zuerst erschien. Öffne ein Set, um seine Karten zu sehen: ob jede wie gedruckt spielt, spielt, aber noch ungetestet ist, teilweise spielt, noch nicht spielbar ist oder nie gebaut wird. Lass den Zeiger auf einer Karte ruhen oder fokussiere sie, um sie groß mit dem Grund zu sehen; ein Klick öffnet ihre Seite bei Scryfall, mit Künstler und Druck. Auf einem Touchscreen zeigt ein Tippen sie zuerst groß."
        />
      </p>
      <ul className="legend" aria-label={tt(lang, "Legend", "Legende")}>
        <li>
          <span className="swatch s-implemented" aria-hidden="true" />
          <T en="Full: plays as printed" de="Voll: spielt wie gedruckt" />
        </li>
        <li>
          <span className="swatch s-generated" aria-hidden="true" />
          <T
            en="Untested: playable, every sentence is read by the rules engine, but no test plays it yet"
            de="Ungetestet: spielbar, jeder Satz wird von der Regel-Engine gelesen, aber noch kein Test spielt sie"
          />
        </li>
        <li>
          <span className="swatch s-partial" aria-hidden="true" />
          <T
            en="Partial: plays, one part missing (the reason is shown with the card)"
            de="Teilweise: spielt, ein Teil fehlt (der Grund steht bei der Karte)"
          />
        </li>
        <li>
          <span className="swatch s-none" aria-hidden="true" />
          <T en="Not yet: not playable yet" de="Noch nicht: noch nicht spielbar" />
        </li>
        <li>
          <span className="swatch s-never" aria-hidden="true" />
          <T
            en="Never: ante, dexterity or a subgame; a set counts as complete without it"
            de="Nie: Ante, Geschicklichkeit oder ein Teilspiel; ein Set gilt ohne sie als vollständig"
          />
        </li>
      </ul>
      <p className="small">
        <T
          en="The percentage is full cards out of all cards of the set that will ever be built. Untested and partial cards are the other parts of the bar and are not counted as done."
          de="Der Prozentwert ist der Anteil voller Karten an allen Karten des Sets, die je gebaut werden. Ungetestete und teilweise umgesetzte Karten sind die übrigen Teile des Balkens und zählen nicht als fertig."
        />
      </p>
      {doc === undefined && (
        <output className="small block">
          <T en="Loading the set list …" de="Lade die Set-Liste …" />
        </output>
      )}
      {doc === null && (
        <output className="small block">
          <T
            en="The set list did not load just now. Reload the page to try again."
            de="Die Set-Liste wurde gerade nicht geladen. Lade die Seite neu, um es noch einmal zu versuchen."
          />
        </output>
      )}
      {doc && (
        <>
          <Summary doc={doc} />
          <p className="sets-more">
            <button
              type="button"
              className="button button-ghost"
              aria-expanded={expanded}
              aria-controls="sets-all"
              onClick={() => setExpanded((e) => !e)}
            >
              {expanded
                ? tt(lang, "Hide the full list", "Vollständige Liste ausblenden")
                : tt(lang, `Show all ${num(lang, doc.sets.length)} sets`, `Alle ${num(lang, doc.sets.length)} Sets anzeigen`)}
            </button>
          </p>
          {expanded && <AllSets doc={doc} target={target} onTargetShown={shown} />}
        </>
      )}
      {/* Attribution and notices for the card status and its pictures: add
          what the legal review asks for here, in both languages (and in
          CARD_CREDIT in model.ts for the line under a card shown large). */}
      <div className="sets-notes">
        <p className="small">
          <T
            en={
              <>
                Card images are loaded from Scryfall only when you open a set or show a card large, unaltered. Card data
                and images provided by Scryfall; Baylee is not affiliated with Scryfall. The counts are measured from the
                repository by <code>baylee-tools site card-status</code>.
              </>
            }
            de={
              <>
                Kartenbilder werden erst beim Öffnen eines Sets oder beim Großzeigen einer Karte und unverändert von
                Scryfall geladen. Kartendaten und -bilder bereitgestellt von Scryfall; Baylee ist nicht mit Scryfall
                verbunden. Die Zahlen misst <code>baylee-tools site card-status</code> im Repository.
              </>
            }
          />
        </p>
        <p className="small">
          <T
            en="Card images © Wizards of the Coast, shown unaltered from Scryfall. Cards not yet in Baylee's pool, and Universes Beyond cards, are listed by name only."
            de="Kartenbilder © Wizards of the Coast, unverändert von Scryfall geladen. Karten, die noch nicht in Baylees Pool sind, und Universes-Beyond-Karten stehen nur mit Namen hier."
          />
        </p>
      </div>
    </div>
  );
}
