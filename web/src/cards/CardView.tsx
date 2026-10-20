// A card shown large: beside the pointer or the focused tile (the preview,
// for the eye only; the tile says the same in words), or in a dialog. The
// picture is Scryfall's "normal" image, fetched only then, unaltered, and
// nothing is ever drawn on it: the status sits beside it.

import { useLayoutEffect, useRef } from "react";
import { createPortal } from "react-dom";

import { tt, useLang } from "../i18n";
import { CARD_CREDIT, imgOf, why } from "./card";
import { STATUS, type Card } from "./model";

export function Badge({ card }: { card: Card }) {
  const lang = useLang();
  const st = STATUS[card.status];
  return <span className={`badge b-${st.group}`}>{tt(lang, st.en, st.de)}</span>;
}

export function Credit() {
  const lang = useLang();
  return <p className="cp-credit">{tt(lang, CARD_CREDIT.en, CARD_CREDIT.de)}</p>;
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
        <span className="cp-artist">
          {tt(lang, "Illustrated by ", "Illustriert von ")}
          {card.artist}
        </span>
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

/** The card large beside the pointer or the focused card; for the eye only (the tile says the same in words). */
export function Preview({ shown }: { shown: Shown }) {
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
