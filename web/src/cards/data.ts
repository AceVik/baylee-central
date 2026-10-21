// Fetching the card status: `sets.json` once per visit when asked for, a
// set's cards when that set is opened. It is the status of the newest
// release, read at that release's tag (`RELEASE_DATA`), the release the
// download button offers, so the numbers are those of the client a visitor
// gets. When GitHub does not answer, or that tag carries no status, it is
// the copy bundled with the site (`DATA_BASE`, `npm run content`). No
// credentials either way.

import { DATA_BASE, RELEASE_DATA } from "../config";
import { pickRelease } from "../download/release";
import { github, RELEASES_PATH } from "../github";
import { parseCards, parseSets, type Card, type SetsDoc } from "./model";

/** Where the status was read: its base URL, and the release it belongs to. */
interface Source {
  base: string;
  release: string | null;
}

let sets: Promise<SetsDoc> | null = null;
let source: Source | null = null;
const cardsBySet = new Map<string, Promise<Card[]>>();

async function json(url: string): Promise<unknown> {
  const r = await fetch(url, { credentials: "omit", headers: { Accept: "application/json" }, referrerPolicy: "no-referrer" });
  if (!r.ok) throw new Error(`HTTP ${r.status}`);
  return r.json();
}

/** The newest release's status, else the bundled one. */
async function load(): Promise<SetsDoc> {
  const release = pickRelease(await github(RELEASES_PATH, "releases"));
  if (release !== null) {
    const base = RELEASE_DATA(release.tag_name);
    try {
      const doc = parseSets(await json(`${base}sets.json`));
      if (doc) {
        source = { base, release: release.tag_name };
        return { ...doc, release: release.tag_name };
      }
    } catch {
      /* an older release without a status, or GitHub away: the bundled copy */
    }
  }
  const doc = parseSets(await json(`${DATA_BASE}sets.json`));
  if (!doc) throw new Error("not a set list");
  source = { base: DATA_BASE, release: null };
  return doc;
}

export function loadSets(): Promise<SetsDoc> {
  sets ??= load();
  // a failure may be asked again later
  sets.catch(() => {
    sets = null;
  });
  return sets;
}

export function loadCards(code: string): Promise<Card[]> {
  let asked = cardsBySet.get(code);
  if (!asked) {
    // a set's cards come from where its overview came from
    asked = loadSets()
      .then(() => json(`${source?.base ?? DATA_BASE}sets/set-${encodeURIComponent(code)}.json`))
      .then((raw) => {
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
  source = null;
  cardsBySet.clear();
}
