// A set's progress: its percentage, one bar in three parts (full, untested,
// partial; the rest is the track) and its counts by status.

import { t, useLang, type Key } from "../i18n";
import type { Counts } from "./model";
import { buildable, pct, playable } from "./numbers";

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
  const part = (cls: string, n: number, key: Key) => <span className={`c c-${cls}`}>{t(lang, key, { n })}</span>;
  return (
    <span className="set-counts">
      {part("playable", playable(counts), "count.playable")}
      {part("implemented", counts.implemented, "count.full")}
      {part("generated", counts.generated, "count.untested")}
      {part("partial", counts.partial, "count.partial")}
      {part("none", counts.unimplemented + counts.absent, "count.none")}
      {counts.never > 0 && part("never", counts.never, "count.never")}
    </span>
  );
}

export function Progress({ counts }: { counts: Counts }) {
  const lang = useLang();
  const p = pct(playable(counts), buildable(counts));
  return (
    <span className="set-progress">
      <span className={`set-pct${p === 100 ? " done" : ""}`} aria-hidden="true">
        {t(lang, "sets.pct", { pct: p })}
      </span>
      <span className="vh">{t(lang, "sets.pctPlayable", { pct: p })}</span>
      <Bar counts={counts} />
    </span>
  );
}
