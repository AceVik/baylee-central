// A set's progress: its percentage, one bar in three parts (full, untested,
// partial; the rest is the track) and its counts by status.

import { num, tt, useLang } from "../i18n";
import type { Counts } from "./model";
import { buildable, pct } from "./numbers";

/** Full, untested and partial as three parts of one bar; the rest is the track. */
export function Bar({ counts }: { counts: Counts }) {
  const of = buildable(counts);
  const share = (n: number) => `${of ? (n * 100) / of : 0}%`;
  return (
    <span className="pbar" aria-hidden="true">
      {/* CSSOM widths (React sets `style` property by property), which the CSP allows */}
      <span className="pbar-full" style={{ width: of ? share(counts.implemented) : "100%" }} />
      <span className="pbar-generated" style={{ width: share(counts.generated) }} />
      <span className="pbar-partial" style={{ width: share(counts.partial) }} />
    </span>
  );
}

export function CountsLine({ counts }: { counts: Counts }) {
  const lang = useLang();
  const part = (cls: string, n: number, en: string, de: string) => (
    <span className={`c c-${cls}`}>
      {num(lang, n)} {tt(lang, en, de)}
    </span>
  );
  return (
    <span className="set-counts">
      {part("implemented", counts.implemented, "full", "voll")}
      {part("generated", counts.generated, "untested", "ungetestet")}
      {part("partial", counts.partial, "partial", "teilweise")}
      {part("none", counts.unimplemented + counts.absent, "not yet", "noch nicht")}
      {counts.never > 0 && part("never", counts.never, "never", "nie")}
    </span>
  );
}

export function Progress({ counts }: { counts: Counts }) {
  const lang = useLang();
  const p = pct(counts.implemented, buildable(counts));
  return (
    <span className="set-progress">
      <span className={`set-pct${p === 100 ? " done" : ""}`} aria-hidden="true">
        {p} %
      </span>
      <span className="vh">{tt(lang, `${p} percent full`, `${p} Prozent voll`)}</span>
      <Bar counts={counts} />
    </span>
  );
}
