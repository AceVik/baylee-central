// One card: its picture, its page on Scryfall, and why it has its status.
//
// Pictures (Baylee's docs/legal.md §3): only a card in the pool or on the
// never-built list, never a Universes Beyond printing, and only where the
// data names a Scryfall printing; straight from Scryfall's image host,
// unaltered, lazily, credited. The page never calls Scryfall's API.

import type { Lang } from "../i18n";
import type { Card, Status } from "./model";

const NEVER: Record<string, [string, string]> = {
  ante: ["it plays for ante (CR 407), and no table here plays for ante", "sie spielt um Ante (CR 407), und kein Tisch hier spielt um Ante"],
  dexterity: ["it asks a physical act of the player, which no game state describes", "sie verlangt eine körperliche Handlung, die kein Spielzustand beschreibt"],
  subgame: ["it starts a game within the game (CR 728), which the engine does not host", "sie startet ein Spiel im Spiel (CR 728), das die Engine nicht ausrichtet"],
};

/** The line under a card shown large. The legal review may ask for more (Baylee's docs/legal.md §3). */
export const CARD_CREDIT = {
  en: "Card image provided by Scryfall, unaltered. Baylee is not affiliated with Scryfall or Wizards of the Coast.",
  de: "Kartenbild bereitgestellt von Scryfall, unverändert. Baylee ist weder mit Scryfall noch mit Wizards of the Coast verbunden.",
};

export const IMG_HOST = "https://cards.scryfall.io";

const SCRYFALL_ID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;

const POOL: ReadonlySet<Status> = new Set(["implemented", "generated", "partial", "unimplemented", "never"]);

export function imgOf(c: Card, size: "small" | "normal"): string | null {
  if (!POOL.has(c.status) || c.img === null || !SCRYFALL_ID.test(c.img)) return null;
  return `${IMG_HOST}/${size}/front/${c.img[0]}/${c.img[1]}/${c.img}.jpg`;
}

/** The card's page on Scryfall (artist and printing): the printing's own, else a search for the card. */
export function pageOf(c: Card): string {
  return c.uri !== null && c.uri.startsWith("https://scryfall.com/card/")
    ? c.uri
    : `https://scryfall.com/search?q=oracleid%3A${encodeURIComponent(c.oracleId)}`;
}

export function why(c: Card, lang: Lang): string {
  const de = lang === "de";
  switch (c.status) {
    case "implemented":
      return de ? "Spielt wie gedruckt." : "Plays as printed.";
    case "generated":
      return de
        ? "Spielbar: jeder Satz wird von der Regel-Engine gelesen, aber noch kein Test spielt sie."
        : "Playable: every sentence is read by the rules engine, but no test plays it yet.";
    case "partial":
      return (de ? "Teilweise umgesetzt. Es fehlt (Notiz auf Englisch): " : "Partly implemented. Missing: ") + c.note;
    case "unimplemented":
      return de
        ? "Im Kartenpool, aber ihre Fähigkeiten sind noch nicht geschrieben: nicht spielbar."
        : "In the card pool, but its abilities are not written yet: not playable.";
    case "absent":
      return de ? "Noch nicht im Kartenpool." : "Not in the card pool yet.";
    case "never": {
      const reason = NEVER[c.note];
      return (de ? "Wird nie gebaut: " : "Never built: ") + (reason ? reason[de ? 1 : 0] : c.note);
    }
  }
}
