// The screenshots, in a grid, each opening large in a dialog with the
// arrow keys to step. They show the client's text view (card text, no card
// image) and live outside this repository: `npm run content` copies them
// from a Baylee checkout (README.md §"Content from Baylee").

import { useState } from "react";

import { MEDIA_BASE } from "../config";
import { T, tt, useLang } from "../i18n";
import { Icon } from "./icons";
import { Section } from "./Layout";
import { Modal } from "./Modal";

interface Shot {
  file: string;
  alt: [string, string];
  title: [string, string];
  caption: [string, string];
}

const SHOTS: Shot[] = [
  {
    file: "shot-table-duel",
    alt: ["A duel against the house AI; a hovered card is shown large in the text view", "Ein Duell gegen die Haus-KI; eine Karte unter dem Zeiger groß in der Textansicht"],
    title: ["A duel", "Ein Duell"],
    caption: ["against the house AI. Hovering a card shows it large.", "gegen die Haus-KI. Eine Karte unter dem Zeiger wird groß gezeigt."],
  },
  {
    file: "shot-game-log",
    alt: ["The game log beside the table, each card name a link", "Das Spielprotokoll neben dem Tisch, jeder Kartenname ein Link"],
    title: ["The game log", "Das Spielprotokoll"],
    caption: ["(L) reads like a chat; every card name opens its preview.", "(L) liest sich wie ein Chat; jeder Kartenname öffnet die Vorschau."],
  },
  {
    file: "shot-table-ring",
    alt: ["A four-seat table, the seats in a ring around the felt", "Ein Tisch mit vier Plätzen im Ring um den Filz"],
    title: ["Four seats.", "Vier Plätze."],
    caption: ["Larger tables sit in a ring around the felt.", "Größere Tische sitzen im Ring um den Filz."],
  },
  {
    file: "shot-lobby",
    alt: ["The lobby: decks on the left, open tables on the right", "Die Lobby: Decks links, offene Tische rechts"],
    title: ["The lobby.", "Die Lobby."],
    caption: ["Your decks, the open tables, and Play the house for a quick game.", "Deine Decks, die offenen Tische und Play the house für ein schnelles Spiel."],
  },
  {
    file: "shot-room",
    alt: ["A room being arranged: seats, AI levels, starting life", "Ein Raum wird eingerichtet: Plätze, KI-Stufen, Startleben"],
    title: ["Arranging a table.", "Einen Tisch einrichten."],
    caption: ["Chairs, starting life, mulligans, and a level for each AI chair.", "Stühle, Startleben, Mulligans und eine Stufe für jeden KI-Stuhl."],
  },
  {
    file: "shot-deck-builder",
    alt: ["The deck builder with statistics and a search over the pool", "Der Deckbauer mit Statistik und einer Suche über den Pool"],
    title: ["The deck builder.", "Der Deckbauer."],
    caption: [
      "Mana curve, land statistics, a search over the whole pool, and a warning for cards that will not play as printed.",
      "Manakurve, Länderstatistik, eine Suche über den ganzen Pool und eine Warnung für Karten, die nicht wie gedruckt spielen.",
    ],
  },
];

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
