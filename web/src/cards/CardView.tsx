// A card shown large: beside the pointer or the focused tile (the preview,
// for the eye only; the tile says the same in words), or in a dialog. The
// picture is Scryfall's "normal" image, fetched only then, unaltered, and
// nothing is ever drawn on it: the status sits beside it.

import { useEffect, useImperativeHandle, useLayoutEffect, useRef, type Ref } from "react";
import { createPortal } from "react-dom";

import { t, useLang } from "../i18n";
import { CARD_CREDIT, imgOf, why } from "./card";
import { GROUP_LABEL, STATUS, type Card } from "./model";

export function Badge({ card }: { card: Card }) {
  const lang = useLang();
  const st = STATUS[card.status];
  return <span className={`badge b-${st.group}`}>{t(lang, GROUP_LABEL[st.group])}</span>;
}

export function Credit() {
  const lang = useLang();
  return <p className="cp-credit">{t(lang, CARD_CREDIT)}</p>;
}

export function CardText({ card, nameId }: { card: Card; nameId?: string }) {
  const lang = useLang();
  return (
    <div className="cp-cap">
      <strong className="cp-name" id={nameId}>
        {card.name}
      </strong>
      <Badge card={card} />
      <span className="cp-why">{why(card, lang)}</span>
      {card.artist !== null && (
        <span className="cp-artist">{t(lang, "card.illustrated", { artist: card.artist })}</span>
      )}
    </div>
  );
}

export function BigPicture({ card }: { card: Card }) {
  const src = imgOf(card, "normal");
  if (src === null) return null;
  return (
    <div className="cp-pic">
      <img src={src} alt="" width={488} height={680} decoding="async" referrerPolicy="no-referrer" />
    </div>
  );
}

export interface Shown {
  card: Card;
  tile: HTMLElement;
  x: number | null;
  y: number | null;
}

/** Puts the preview `el` beside the point (x, y), or beside `tile` without one, inside the window. */
function place(el: HTMLElement, tile: HTMLElement, at: { x: number | null; y: number | null }): void {
  const w = el.offsetWidth;
  const h = el.offsetHeight;
  const W = window.innerWidth;
  const H = window.innerHeight;
  let { x, y } = at;
  let gap = 18;
  if (x === null || y === null) {
    const r = tile.getBoundingClientRect();
    x = r.right;
    y = r.top + r.height / 2;
    gap = 12;
  }
  let left = x + gap + w > W - 8 ? x - gap - w : x + gap;
  if (left < 8) left = 8;
  const top = Math.max(8, Math.min(H - h - 8, y - h / 2));
  // CSSOM, which the CSP allows (a style attribute it would not)
  el.style.transform = `translate3d(${Math.round(left)}px, ${Math.round(top)}px, 0)`;
}

export interface PreviewHandle {
  /** Moves the preview beside the pointer at (x, y), at most once a frame. */
  follow(x: number, y: number): void;
}

/**
 * The card large beside the pointer or the focused card; for the eye only
 * (the tile says the same in words). The pointer moves it through its
 * handle, without drawing the grid again.
 */
export function Preview({ shown, ref }: { shown: Shown; ref?: Ref<PreviewHandle> }) {
  const lang = useLang();
  const box = useRef<HTMLDivElement>(null);
  const frame = useRef(0);
  const next = useRef<{ x: number; y: number } | null>(null);
  useLayoutEffect(() => {
    if (box.current) place(box.current, shown.tile, shown);
  }, [shown]);
  useEffect(() => () => cancelAnimationFrame(frame.current), []);
  useImperativeHandle(
    ref,
    () => ({
      follow(x, y) {
        next.current = { x, y };
        if (frame.current !== 0) return;
        frame.current = requestAnimationFrame(() => {
          frame.current = 0;
          if (box.current && next.current) place(box.current, shown.tile, next.current);
        });
      },
    }),
    [shown],
  );
  return createPortal(
    <div className="cpreview" ref={box} aria-hidden="true">
      <BigPicture card={shown.card} />
      <CardText card={shown.card} />
      <p className="cp-go">{t(lang, "card.clickHint")}</p>
      <Credit />
    </div>,
    document.body,
  );
}
