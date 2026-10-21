// One set's cards. A grid is one stop for Tab: the arrow keys, Home and End
// move in it (a roving tabindex), Space shows the card large, Enter opens
// its page on Scryfall. With a mouse, resting on a card shows it large
// beside the pointer after a moment (focus does the same beside the card);
// on a touch screen the first tap shows it in a dialog, with the link.

import { useCallback, useEffect, useMemo, useRef, useState, type FocusEvent, type KeyboardEvent, type MouseEvent, type PointerEvent } from "react";

import { T, t, useLang } from "../i18n";
import { Icon } from "../components/icons";
import { Modal } from "../components/Modal";
import { imgOf, pageOf, why } from "./card";
import { Badge, BigPicture, CardText, Credit, Preview, type Shown } from "./CardView";
import { FilterBar, type Filter } from "./FilterBar";
import { GROUP_LABEL, STATUS, type Card, type Counts } from "./model";

function hoverable(): boolean {
  return typeof window.matchMedia === "function" && window.matchMedia("(hover: hover) and (pointer: fine)").matches;
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
        <T id="grid.hint" />
      </p>
      <ul className="cgrid" ref={grid} aria-label={t(lang, "grid.label", { set: setName })}>
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
                  {t(lang, "grid.tile", { status: t(lang, GROUP_LABEL[st.group]), why: why(card, lang) })}
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
                <T id="grid.scryfall" />
              </a>
            </p>
            <Credit />
            <button type="button" className="round-btn cardview-close" aria-label={t(lang, "app.close")} onClick={() => setOpened(null)}>
              <Icon name="close" />
            </button>
          </div>
        </Modal>
      )}
    </>
  );
}
