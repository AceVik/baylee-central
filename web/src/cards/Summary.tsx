// The summary above the full list: the whole corpus in one bar, the finished
// sets as links, and the next sets in release order.

import { T, t, useLang } from "../i18n";
import type { SetsDoc } from "./model";
import { buildable, finished, pct, playable } from "./numbers";
import { Bar } from "./Progress";
import { SetRow, useOpenSet } from "./SetRow";

const NEXT_UP = 3;

export function Summary({ doc }: { doc: SetsDoc }) {
  const lang = useLang();
  const [open, toggle] = useOpenSet();
  const tot = doc.totals;
  const all = doc.corpus - tot.never;
  const play = playable(tot);
  const p = pct(play, all);
  const done = doc.sets.filter(finished);
  const next = doc.sets.filter((s) => buildable(s) > 0 && !finished(s)).slice(0, NEXT_UP);
  return (
    <div className="sets-summary">
      <div className="sets-overall glass">
        <p className="sets-total">
          {`${t(lang, "sets.total", {
            play,
            all,
            pct: p,
            full: tot.implemented,
            untested: tot.generated,
            partial: tot.partial,
            none: tot.unimplemented + tot.absent,
            never: tot.never,
            sets: doc.sets.length,
            corpus: doc.corpus,
          })} ${
            doc.release
              ? t(lang, "sets.atRelease", { release: doc.release, rev: doc.rev.slice(0, 9) })
              : t(lang, "sets.atCommit", { rev: doc.rev.slice(0, 9) })
          }`}
        </p>
        <Bar counts={{ ...tot, total: doc.corpus }} />
      </div>
      {done.length > 0 && (
        <>
          <h4 className="sets-sub">
            <T id="sets.finished" />
          </h4>
          <ul className="set-chips">
            {done.map((s) => (
              <li key={s.code}>
                <a className="chip" href={`#set-${s.code}`}>
                  {s.name}
                </a>
              </li>
            ))}
          </ul>
        </>
      )}
      {next.length > 0 && (
        <>
          <h4 className="sets-sub">
            <T id="sets.next" />
          </h4>
          <div className="sets">
            {next.map((s) => (
              <SetRow key={s.code} set={s} open={open.has(s.code)} onOpen={toggle} />
            ))}
          </div>
        </>
      )}
    </div>
  );
}
