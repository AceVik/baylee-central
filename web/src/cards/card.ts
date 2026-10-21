// One card: its picture, its page on Scryfall, and why it has its status.
//
// Pictures (Baylee's docs/legal.md §3): only a card in the pool or on the
// never-built list, never a Universes Beyond printing, and only where the
// data names a Scryfall printing; straight from Scryfall's image host,
// unaltered, lazily, credited. The page never calls Scryfall's API.

import { t, type Key, type Lang } from "../i18n";
import type { Card, Status } from "./model";

/** Why a card on the never-built list is there, by the note the data gives; another note is shown as it is. */
const NEVER: Readonly<Record<string, Key>> = {
  ante: "card.neverAnte",
  dexterity: "card.neverDexterity",
  subgame: "card.neverSubgame",
};

/** The line under a card shown large (`card.credit`). The legal review may ask for more (Baylee's docs/legal.md §3). */
export const CARD_CREDIT: Key = "card.credit";

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

/** Why the card has its status; a partial card's note is Baylee's, in English. */
export function why(c: Card, lang: Lang): string {
  switch (c.status) {
    case "implemented":
      return t(lang, "card.implemented");
    case "generated":
      return t(lang, "card.generated");
    case "partial":
      return t(lang, "card.partial", { note: c.note });
    case "unimplemented":
      return t(lang, "card.unimplemented");
    case "absent":
      return t(lang, "card.absent");
    case "never": {
      const reason = Object.hasOwn(NEVER, c.note) ? NEVER[c.note] : undefined;
      return t(lang, "card.never", { reason: reason === undefined ? c.note : t(lang, reason) });
    }
  }
}
