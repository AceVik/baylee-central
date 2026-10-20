// One set's cards. A grid is one stop for Tab: the arrow keys, Home and End
// move in it (a roving tabindex), Space shows the card large, Enter opens
// its page on Scryfall. With a mouse, resting on a card shows it large
// beside the pointer after a moment (focus does the same beside the card);
// on a touch screen the first tap shows it in a dialog, with the link. The
// large picture is Scryfall's "normal" image, fetched only then, unaltered:
// nothing is ever drawn on a picture, the status sits under it.

import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState, type FocusEvent, type KeyboardEvent, type MouseEvent, type PointerEvent } from "react";
import { createPortal } from "react-dom";

import { T, num, tt, useLang } from "../i18n";
import { Icon } from "../components/icons";
import { Modal } from "../components/Modal";
import { CARD_CREDIT, STATUS, imgOf, pageOf, why, type Card, type Counts, type Group } from "./model";

type Filter = "all" | Group;

function hoverable(): boolean {
  return typeof window.matchMedia === "function" && window.matchMedia("(hover: hover) and (pointer: fine)").matches;
}

function Badge({ card }: { card: Card }) {
  const lang = useLang();
  const st = STATUS[card.status];
  return <span className={`badge b-${st.group}`}>{tt(lang, st.en, st.de)}</span>;
}

function Credit() {
  const lang = useLang();
  return <p className="cp-credit">{tt(lang, CARD_CREDIT.en, CARD_CREDIT.de)}</p>;
}

function CardText({ card, nameId }: { card: Card; nameId?: string }) {
  const lang = useLang();
  return (
    <div className="cp-cap">
      <strong className="cp-name" id={nameId}>
        {card.name}
      </strong>
      <Badge card={card} />
      <span className="cp-why">{why(card, lang)}</span>
      {card.artist !== null && (
        <span className="cp-artist">
          {tt(lang, "Illustrated by ", "Illustriert von ")}
          {card.artist}
        </span>
      )}
    </div>
  );
}

function BigPicture({ card }: { card: Card }) {
  const src = imgOf(card, "normal");
  if (src === null) return null;
  return (
    <div className="cp-pic">
      <img src={src} alt="" width={488} height={680} decoding="async" referrerPolicy="no-referrer" />
    </div>
  );
}

interface Shown {
  card: Card;
  tile: HTMLElement;
  x: number | null;
  y: number | null;
}

/** The card large beside the pointer or the focused card; for the eye only (the tile says the same in words). */
function Preview({ shown }: { shown: Shown }) {
  const lang = useLang();
  const box = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    const el = box.current;
    if (!el) return;
    const w = el.offsetWidth;
    const h = el.offsetHeight;
    const W = window.innerWidth;
    const H = window.innerHeight;
    let { x, y } = shown;
    let gap = 18;
    if (x === null || y === null) {
      const r = shown.tile.getBoundingClientRect();
      x = r.right;
      y = r.top + r.height / 2;
      gap = 12;
    }
    let left = x + gap + w > W - 8 ? x - gap - w : x + gap;
    if (left < 8) left = 8;
    const top = Math.max(8, Math.min(H - h - 8, y - h / 2));
    // CSSOM, which the CSP allows (a style attribute it would not)
    el.style.transform = `translate3d(${Math.round(left)}px, ${Math.round(top)}px, 0)`;
  }, [shown]);
  return createPortal(
    <div className="cpreview" ref={box} aria-hidden="true">
      <BigPicture card={shown.card} />
      <CardText card={shown.card} />
      <p className="cp-go">{tt(lang, "Click for the artist and printing on Scryfall.", "Klick: Künstler und Druck bei Scryfall.")}</p>
      <Credit />
    </div>,
    document.body,
  );
}

function FilterBar({ counts, filter, onFilter }: { counts: Counts; filter: Filter; onFilter: (f: Filter) => void }) {
  const lang = useLang();
  const groups: [Filter, string, string, number][] = [
    ["all", "All", "Alle", counts.total],
    ["implemented", "Full", "Voll", counts.implemented],
    ["generated", "Untested", "Ungetestet", counts.generated],
    ["partial", "Partial", "Teilweise", counts.partial],
    ["none", "Not yet", "Noch nicht", counts.unimplemented + counts.absent],
    ["never", "Never", "Nie", counts.never],
  ];
  return (
    <fieldset className="cfilter">
      <legend className="vh">{tt(lang, "Show cards", "Karten zeigen")}</legend>
      {groups
        .filter(([g, , , n]) => g === "all" || n > 0)
        .map(([g, en, de, n]) => (
          <button key={g} type="button" className="chip-btn" aria-pressed={filter === g} onClick={() => onFilter(g)}>
            {tt(lang, en, de)} <span className="chip-n">{num(lang, n)}</span>
          </button>
        ))}
    </fieldset>
  );
}

export function CardGrid({ cards, counts, setName }: { cards: Card[]; counts: Counts; setName: string }) {
  const lang = useLang();
  const [filter, setFilter] = useState<Filter>("all");
  const [active, setActive] = useState(0);
  const [shown, setShown] = useState<Shown | null>(null);
  const [opened, setOpened] = useState<{ card: Card } | null>(null);
  const grid = useRef<HTMLUListElement>(null);
  const timer = useRef(0);
  const canHover = useMemo(() => hoverable(), []);

  const visible = useMemo(
    () => (filter === "all" ? cards : cards.filter((c) => STATUS[c.status].group === filter)),
    [cards, filter],
  );

  const hide = useCallback(() => {
    window.clearTimeout(timer.current);
    setShown(null);
  }, []);

  useEffect(() => {
    if (!shown) return;
    const onScroll = () => hide();
    const onKey = (e: globalThis.KeyboardEvent) => {
      if (e.key === "Escape") hide();
    };
    window.addEventListener("scroll", onScroll, { passive: true });
    document.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("scroll", onScroll);
      document.removeEventListener("keydown", onKey);
    };
  }, [shown, hide]);
  useEffect(() => () => window.clearTimeout(timer.current), []);

  const open = (card: Card) => {
    hide();
    setOpened({ card });
  };

  const tiles = () => Array.from(grid.current?.querySelectorAll<HTMLAnchorElement>(".ctile-link") ?? []);

  const onKey = (e: KeyboardEvent<HTMLAnchorElement>) => {
    const links = tiles();
    const at = links.indexOf(e.target as HTMLAnchorElement);
    if (at < 0) return;
    const card = visible[at];
    if (e.key === " " && card) {
      e.preventDefault();
      open(card);
      return;
    }
    let cols = 1;
    const top = links[0]?.offsetTop ?? 0;
    while (cols < links.length && links[cols]?.offsetTop === top) cols++;
    const to: number | undefined = {
      ArrowRight: at + 1,
      ArrowLeft: at - 1,
      ArrowDown: at + cols,
      ArrowUp: at - cols,
      Home: 0,
      End: links.length - 1,
    }[e.key];
    if (to === undefined) return;
    e.preventDefault();
    const next = Math.max(0, Math.min(links.length - 1, to));
    setActive(next);
    links[next]?.focus();
  };

  const onTileClick = (e: MouseEvent<HTMLAnchorElement>, card: Card) => {
    // with a mouse the click follows the link; without a hover the first tap shows the card large
    if (canHover) return;
    e.preventDefault();
    open(card);
  };

  const onOver = (e: PointerEvent<HTMLAnchorElement>, card: Card) => {
    if (!canHover || e.pointerType !== "mouse") return;
    window.clearTimeout(timer.current);
    const tile = e.currentTarget;
    const x = e.clientX;
    const y = e.clientY;
    timer.current = window.setTimeout(() => setShown({ card, tile, x, y }), shown ? 0 : 220);
  };

  const onFocus = (e: FocusEvent<HTMLAnchorElement>, card: Card, i: number) => {
    setActive(i);
    if (e.currentTarget.matches(":focus-visible")) {
      window.clearTimeout(timer.current);
      setShown({ card, tile: e.currentTarget, x: null, y: null });
    }
  };

  const roving = Math.min(active, Math.max(0, visible.length - 1));

  return (
    <>
      <FilterBar
        counts={counts}
        filter={filter}
        onFilter={(f) => {
          setFilter(f);
          setActive(0);
        }}
      />
      <p className="small cgrid-hint">
        <T
          en="Arrow keys move between cards; Space shows one large, Enter opens it on Scryfall."
          de="Pfeiltasten wechseln die Karte; die Leertaste zeigt sie groß, Enter öffnet sie bei Scryfall."
        />
      </p>
      <ul className="cgrid" ref={grid} aria-label={tt(lang, `Cards of ${setName}`, `Karten aus ${setName}`)}>
        {visible.map((card, i) => {
          const src = imgOf(card, "small");
          const st = STATUS[card.status];
          return (
            <li key={card.oracleId} className="ctile" data-group={st.group}>
              <a
                className="ctile-link"
                href={pageOf(card)}
                target="_blank"
                rel="noopener"
                tabIndex={i === roving ? 0 : -1}
                onClick={(e) => onTileClick(e, card)}
                onKeyDown={onKey}
                onPointerOver={(e) => onOver(e, card)}
                onPointerLeave={hide}
                onPointerMove={(e) => {
                  if (shown?.card === card) setShown({ ...shown, x: e.clientX, y: e.clientY });
                }}
                onFocus={(e) => onFocus(e, card, i)}
                onBlur={hide}
              >
                <span className="ctile-pic">
                  {src !== null ? (
                    <img src={src} alt="" loading="lazy" decoding="async" width={146} height={204} referrerPolicy="no-referrer" />
                  ) : (
                    <span className="ctile-blank" aria-hidden="true">
                      {card.name}
                    </span>
                  )}
                </span>
                <span className="ctile-name">{card.name}</span>
                <span className="vh">
                  {" "}
                  {tt(lang, st.en, st.de)}. {why(card, lang)}
                </span>
              </a>
              <div className="ctile-cap" aria-hidden="true">
                <Badge card={card} />
              </div>
            </li>
          );
        })}
      </ul>
      {shown && <Preview shown={shown} />}
      {opened && (
        <Modal className="cardview" labelledBy="cardview-name" onClose={() => setOpened(null)}>
          <div className="cardview-body">
            {imgOf(opened.card, "normal") !== null && (
              <a href={pageOf(opened.card)} target="_blank" rel="noopener" tabIndex={-1} className="cp-piclink">
                <BigPicture card={opened.card} />
              </a>
            )}
            <CardText card={opened.card} nameId="cardview-name" />
            <p className="cp-go">
              <a className="button button-ghost" href={pageOf(opened.card)} target="_blank" rel="noopener">
                <T en="Artist and printing on Scryfall" de="Künstler und Druck bei Scryfall" />
              </a>
            </p>
            <Credit />
            <button type="button" className="round-btn cardview-close" aria-label={tt(lang, "Close", "Schließen")} onClick={() => setOpened(null)}>
              <Icon name="close" />
            </button>
          </div>
        </Modal>
      )}
    </>
  );
}
