// The screenshots, in a grid, each opening large in a dialog with the
// arrow keys to step. They show the game with Scryfall's card images, so the
// grid and the dialog each carry the credit (`shots.credit`; Baylee's
// docs/legal.md §3): each AVIF is a short recording that loops, each WebP a
// still of it, which `prefers-reduced-motion` gets (WCAG 2.3.3) and a browser
// without AVIF shows. They live outside this repository: `npm run content`
// copies them from a Baylee checkout (README.md §"Content from Baylee").

import { useState } from "react";

import { MEDIA_BASE } from "../config";
import { T, t, useLang } from "../i18n";
import { Icon } from "../components/icons";
import { Modal } from "../components/Modal";
import { Section } from "../components/Section";
import { SHOTS, type Shot } from "./shots";

const SIZES = "(max-width: 600px) 92vw, (max-width: 1000px) 46vw, 380px";
// A browser takes the first <source> that matches, so the still stands first.
const STILL = "(prefers-reduced-motion: reduce)";

function Picture({ shot, large, alt }: { shot: Shot; large?: boolean; alt: string }) {
  const base = MEDIA_BASE + shot.file;
  if (large === true) {
    return (
      <picture>
        <source media={STILL} type="image/webp" srcSet={`${base}.webp`} />
        <source type="image/avif" srcSet={`${base}.avif`} />
        <img src={`${base}.webp`} width={1200} height={703} alt={alt} decoding="async" />
      </picture>
    );
  }
  return (
    <picture>
      <source media={STILL} type="image/webp" srcSet={`${base}-640.webp 640w, ${base}.webp 1200w`} sizes={SIZES} />
      <source type="image/avif" srcSet={`${base}-640.avif 640w, ${base}.avif 1200w`} sizes={SIZES} />
      <source type="image/webp" srcSet={`${base}-640.webp 640w, ${base}.webp 1200w`} sizes={SIZES} />
      <img src={`${base}.webp`} width={1200} height={703} loading="lazy" decoding="async" alt={alt} />
    </picture>
  );
}

export function Screens() {
  const lang = useLang();
  const [at, setAt] = useState<number | null>(null);
  const step = (by: number) => setAt((i) => (i === null ? null : (i + by + SHOTS.length) % SHOTS.length));
  const onKey = (e: KeyboardEvent) => {
    if (e.key === "ArrowRight") step(1);
    else if (e.key === "ArrowLeft") step(-1);
  };
  const shot = at === null ? null : SHOTS[at];

  return (
    <Section
      id="screens"
      alt
      eyebrow={<T id="shots.eyebrow" />}
      title={<T id="shots.title" />}
    >
      <p className="section-lead reveal">
        <T id="shots.lead" />
      </p>
      <ul className="shots">
        {SHOTS.map((s, i) => (
          <li key={s.file} className="glass reveal">
            <figure>
              <button
                type="button"
                className="shot-open"
                onClick={() => setAt(i)}
                aria-label={t(lang, "shots.showLarge", { title: t(lang, s.title) })}
              >
                <Picture shot={s} alt={t(lang, s.alt)} />
              </button>
              <figcaption>
                <b>{t(lang, s.title)}</b> {t(lang, s.caption)}
              </figcaption>
            </figure>
          </li>
        ))}
      </ul>
      <p className="small shots-credit">
        <T id="shots.credit" />
      </p>
      {shot && (
        <Modal className="lightbox" label={t(lang, "shots.dialog")} onClose={() => setAt(null)} onKey={onKey}>
          <Picture shot={shot} large alt={t(lang, shot.alt)} />
          <p className="lightbox-cap">
            <b>{t(lang, shot.title)}</b> {t(lang, shot.caption)}
            <span className="lightbox-credit">
              <T id="shots.credit" />
            </span>
          </p>
          <button type="button" className="round-btn lightbox-prev" onClick={() => step(-1)} aria-label={t(lang, "shots.previous")}>
            <Icon name="prev" />
          </button>
          <button type="button" className="round-btn lightbox-next" onClick={() => step(1)} aria-label={t(lang, "shots.next")}>
            <Icon name="next" />
          </button>
          <button type="button" className="round-btn lightbox-close" onClick={() => setAt(null)} aria-label={t(lang, "app.close")}>
            <Icon name="close" />
          </button>
        </Modal>
      )}
    </Section>
  );
}
