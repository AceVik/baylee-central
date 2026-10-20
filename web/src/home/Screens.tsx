// The screenshots, in a grid, each opening large in a dialog with the
// arrow keys to step. They show the client's text view (card text, no card
// image) and live outside this repository: `npm run content` copies them
// from a Baylee checkout (README.md §"Content from Baylee").

import { useState } from "react";

import { MEDIA_BASE } from "../config";
import { T, tt, useLang } from "../i18n";
import { Icon } from "../components/icons";
import { Modal } from "../components/Modal";
import { Section } from "../components/Section";
import { SHOTS, type Shot } from "./shots";

const SIZES = "(max-width: 600px) 92vw, (max-width: 1000px) 46vw, 380px";

function Picture({ shot, large, alt }: { shot: Shot; large?: boolean; alt: string }) {
  const base = MEDIA_BASE + shot.file;
  if (large === true) {
    return (
      <picture>
        <source type="image/avif" srcSet={`${base}.avif`} />
        <img src={`${base}.webp`} width={1200} height={703} alt={alt} decoding="async" />
      </picture>
    );
  }
  return (
    <picture>
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
      eyebrow={<T en="At the table" de="Am Tisch" />}
      title={<T en="What it looks like" de="So sieht es aus" />}
    >
      <p className="section-lead reveal">
        <T
          en="These pictures show the text view, which draws each card from its rules text rather than the printed card image, because we do not redistribute Scryfall's scans. In the game you see the real card images. The screenshots were taken with 0.1.0-beta.2 on a local test gateway with made-up players; the current build looks a little different."
          de="Die Bilder zeigen die Textansicht, die jede Karte aus ihrem Regeltext zeichnet statt aus dem gedruckten Kartenbild, weil wir Scryfalls Scans nicht weiterverbreiten. Im Spiel siehst du die echten Kartenbilder. Die Aufnahmen entstanden mit 0.1.0-beta.2 auf einem lokalen Test-Gateway mit erfundenen Spielern; die aktuelle Version sieht etwas anders aus."
        />
      </p>
      <ul className="shots">
        {SHOTS.map((s, i) => (
          <li key={s.file} className="glass reveal">
            <figure>
              <button
                type="button"
                className="shot-open"
                onClick={() => setAt(i)}
                aria-label={tt(lang, `Show large: ${s.title[0]}`, `Groß zeigen: ${s.title[1]}`)}
              >
                <Picture shot={s} alt={tt(lang, s.alt[0], s.alt[1])} />
              </button>
              <figcaption>
                <b>{tt(lang, s.title[0], s.title[1])}</b> {tt(lang, s.caption[0], s.caption[1])}
              </figcaption>
            </figure>
          </li>
        ))}
      </ul>
      {shot && (
        <Modal className="lightbox" label={tt(lang, "Screenshot", "Bildschirmfoto")} onClose={() => setAt(null)} onKey={onKey}>
          <Picture shot={shot} large alt={tt(lang, shot.alt[0], shot.alt[1])} />
          <p className="lightbox-cap">
            <b>{tt(lang, shot.title[0], shot.title[1])}</b> {tt(lang, shot.caption[0], shot.caption[1])}
          </p>
          <button type="button" className="round-btn lightbox-prev" onClick={() => step(-1)} aria-label={tt(lang, "Previous", "Zurück")}>
            <Icon name="prev" />
          </button>
          <button type="button" className="round-btn lightbox-next" onClick={() => step(1)} aria-label={tt(lang, "Next", "Weiter")}>
            <Icon name="next" />
          </button>
          <button type="button" className="round-btn lightbox-close" onClick={() => setAt(null)} aria-label={tt(lang, "Close", "Schließen")}>
            <Icon name="close" />
          </button>
        </Modal>
      )}
    </Section>
  );
}
