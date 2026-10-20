// The card status as Baylee measures it: `data/sets.json` lists every set
// in release order with its counts, `data/sets/set-<code>.json` a set's cards
// (both written by Baylee's `baylee-tools site card-status`, from one git
// revision). The `set-` prefix keeps a code such as `con` from naming a
// reserved Windows file. Data written before `generated` existed lacks the
// field; it counts as 0.
//
// Pictures (Baylee's docs/legal.md §3): only a card in the pool or on the
// never-built list, never a Universes Beyond printing, and only where the
// data names a Scryfall printing; straight from Scryfall's image host,
// unaltered, lazily, credited. The page never calls Scryfall's API.

import type { Lang } from "../i18n";

export type Status = "implemented" | "generated" | "partial" | "unimplemented" | "absent" | "never";
export type Group = "implemented" | "generated" | "partial" | "none" | "never";

export interface Counts {
  total: number;
  implemented: number;
  generated: number;
  partial: number;
  unimplemented: number;
  absent: number;
  never: number;
}

export interface SetSummary extends Counts {
  code: string;
  name: string;
  released: string;
  position: number;
}

export interface SetsDoc {
  corpus: number;
  rev: string;
  generatedAt: string;
  totals: Counts;
  sets: SetSummary[];
}

export interface Card {
  name: string;
  oracleId: string;
  status: Status;
  note: string;
  img: string | null;
  uri: string | null;
  artist: string | null;
}

export const STATUS: Record<Status, { group: Group; en: string; de: string }> = {
  implemented: { group: "implemented", en: "Full", de: "Voll" },
  generated: { group: "generated", en: "Untested", de: "Ungetestet" },
  partial: { group: "partial", en: "Partial", de: "Teilweise" },
  unimplemented: { group: "none", en: "Not yet", de: "Noch nicht" },
  absent: { group: "none", en: "Not yet", de: "Noch nicht" },
  never: { group: "never", en: "Never", de: "Nie" },
};

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
const CODE = /^[a-z0-9]{2,8}$/;

// ---------------------------------------------------------------- parsing

function isObject(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}
function count(v: unknown): number {
  return typeof v === "number" && Number.isFinite(v) && v >= 0 ? Math.floor(v) : 0;
}
function counts(o: Record<string, unknown>): Counts {
  return {
    total: count(o["total"]),
    implemented: count(o["implemented"]),
    generated: count(o["generated"]),
    partial: count(o["partial"]),
    unimplemented: count(o["unimplemented"]),
    absent: count(o["absent"]),
    never: count(o["never"]),
  };
}

export function parseSets(raw: unknown): SetsDoc | null {
  if (!isObject(raw) || !Array.isArray(raw["sets"]) || !isObject(raw["totals"])) return null;
  const sets: SetSummary[] = [];
  for (const s of raw["sets"]) {
    if (!isObject(s)) continue;
    const code = s["code"];
    const name = s["name"];
    if (typeof code !== "string" || !CODE.test(code) || typeof name !== "string") continue;
    sets.push({
      ...counts(s),
      code,
      name,
      released: typeof s["released"] === "string" ? s["released"] : "",
      position: count(s["position"]),
    });
  }
  return {
    corpus: count(raw["corpus"]),
    rev: typeof raw["rev"] === "string" ? raw["rev"] : "",
    generatedAt: typeof raw["generated_at"] === "string" ? raw["generated_at"] : "",
    totals: counts(raw["totals"]),
    sets,
  };
}

const STATUSES: ReadonlySet<string> = new Set(Object.keys(STATUS));

export function parseCards(raw: unknown): Card[] | null {
  if (!isObject(raw) || !Array.isArray(raw["cards"])) return null;
  const cards: Card[] = [];
  for (const c of raw["cards"]) {
    if (!isObject(c)) continue;
    const { name, oracle_id: oracleId, status } = c;
    if (typeof name !== "string" || typeof oracleId !== "string" || typeof status !== "string" || !STATUSES.has(status)) {
      continue;
    }
    cards.push({
      name,
      oracleId,
      status: status as Status,
      note: typeof c["note"] === "string" ? c["note"] : "",
      img: typeof c["img"] === "string" ? c["img"] : null,
      uri: typeof c["uri"] === "string" ? c["uri"] : null,
      artist: typeof c["artist"] === "string" ? c["artist"] : null,
    });
  }
  return cards;
}

// ---------------------------------------------------------------- numbers

/** Percent done, never rounded up to 100 or down to 0 by a hair. */
export function pct(done: number, of: number): number {
  if (!of) return 100;
  const p = (done * 100) / of;
  return p > 0 && p < 1 ? 1 : p > 99 && p < 100 ? 99 : Math.round(p);
}

/** The cards of `s` that will ever be built. */
export function buildable(s: Counts): number {
  return s.total - s.never;
}

export function begun(s: Counts): boolean {
  return s.implemented + s.generated + s.partial > 0;
}

export function finished(s: Counts): boolean {
  const of = buildable(s);
  return of > 0 && s.implemented === of;
}

export function sum(list: readonly Counts[]): Counts {
  const a: Counts = { total: 0, implemented: 0, generated: 0, partial: 0, unimplemented: 0, absent: 0, never: 0 };
  for (const s of list) {
    a.total += s.total;
    a.implemented += s.implemented;
    a.generated += s.generated;
    a.partial += s.partial;
    a.unimplemented += s.unimplemented;
    a.absent += s.absent;
    a.never += s.never;
  }
  return a;
}

export const ERA = 4;
export const FIRST_YEAR = 1993;

export function yearOf(s: SetSummary): number {
  return Number.parseInt(s.released.slice(0, 4), 10) || FIRST_YEAR;
}

export function eraOf(s: SetSummary): number {
  return FIRST_YEAR + Math.floor((yearOf(s) - FIRST_YEAR) / ERA) * ERA;
}

/** The sets in eras of four years, in release order. */
export function eras(sets: readonly SetSummary[]): { from: number; to: number; sets: SetSummary[] }[] {
  const out: { from: number; to: number; sets: SetSummary[] }[] = [];
  for (const s of sets) {
    const from = eraOf(s);
    let era = out.find((e) => e.from === from);
    if (!era) {
      era = { from, to: from, sets: [] };
      out.push(era);
    }
    era.sets.push(s);
    era.to = Math.max(era.to, yearOf(s));
  }
  return out;
}

export function matches(s: SetSummary, needle: string): boolean {
  return `${s.name} ${s.code}`.toLowerCase().includes(needle.trim().toLowerCase());
}

// ---------------------------------------------------------------- one card

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
