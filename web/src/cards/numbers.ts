// Sums, shares and eras over the card status.

import type { Counts, SetSummary } from "./model";

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
