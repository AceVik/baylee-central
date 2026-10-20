// The card status as Baylee measures it: `data/sets.json` lists every set
// in release order with its counts, `data/sets/set-<code>.json` a set's cards
// (both written by Baylee's `baylee-tools site card-status`, from one git
// revision). The `set-` prefix keeps a code such as `con` from naming a
// reserved Windows file. Data written before `generated` existed lacks the
// field; it counts as 0.

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

const CODE = /^[a-z0-9]{2,8}$/;

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
