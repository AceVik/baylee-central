// Fetching the card status: `sets.json` once per visit when asked for, a
// set's cards when that set is opened. Same origin, no credentials.

import { DATA_BASE } from "../config";
import { parseCards, parseSets, type Card, type SetsDoc } from "./model";

let sets: Promise<SetsDoc> | null = null;
const cardsBySet = new Map<string, Promise<Card[]>>();

async function json(path: string): Promise<unknown> {
  const r = await fetch(DATA_BASE + path, { credentials: "omit", headers: { Accept: "application/json" } });
  if (!r.ok) throw new Error(`HTTP ${r.status}`);
  return r.json();
}

export function loadSets(): Promise<SetsDoc> {
  sets ??= json("sets.json").then((raw) => {
    const doc = parseSets(raw);
    if (!doc) throw new Error("not a set list");
    return doc;
  });
  // a failure may be asked again later
  sets.catch(() => {
    sets = null;
  });
  return sets;
}

export function loadCards(code: string): Promise<Card[]> {
  let asked = cardsBySet.get(code);
  if (!asked) {
    asked = json(`sets/set-${encodeURIComponent(code)}.json`).then((raw) => {
      const cards = parseCards(raw);
      if (!cards) throw new Error("not a card list");
      return cards;
    });
    asked.catch(() => cardsBySet.delete(code));
    cardsBySet.set(code, asked);
  }
  return asked;
}

/** For tests: forget what was fetched. */
export function forgetLoaded(): void {
  sets = null;
  cardsBySet.clear();
}
