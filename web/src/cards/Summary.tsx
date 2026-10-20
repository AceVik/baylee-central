// The summary above the full list: the whole corpus in one bar, the finished
// sets as links, and the next sets in release order.

import { T, num, tt, useLang } from "../i18n";
import type { SetsDoc } from "./model";
import { buildable, finished, pct } from "./numbers";
import { Bar } from "./Progress";
import { SetRow, useOpenSet } from "./SetRow";

const NEXT_UP = 3;

export function Summary({ doc }: { doc: SetsDoc }) {
  const lang = useLang();
  const [open, toggle] = useOpenSet();
  const tot = doc.totals;
  const all = doc.corpus - tot.never;
  const p = pct(tot.implemented, all);
  const done = doc.sets.filter(finished);
  const next = doc.sets.filter((s) => buildable(s) > 0 && !finished(s)).slice(0, NEXT_UP);
  const n = (x: number) => num(lang, x);
  return (
    <div className="sets-summary">
      <div className="sets-overall glass">
        <p className="sets-total">
          {tt(
            lang,
            `${n(doc.sets.length)} sets, ${n(doc.corpus)} cards: ${n(tot.implemented)} full (${p} %), ${n(tot.generated)} untested, ${n(tot.partial)} partial, ${n(tot.unimplemented + tot.absent)} not yet, ${n(tot.never)} never. Measured at commit ${doc.rev.slice(0, 9)}.`,
            `${n(doc.sets.length)} Sets, ${n(doc.corpus)} Karten: ${n(tot.implemented)} voll (${p} %), ${n(tot.generated)} ungetestet, ${n(tot.partial)} teilweise, ${n(tot.unimplemented + tot.absent)} noch nicht, ${n(tot.never)} nie. Gemessen an Commit ${doc.rev.slice(0, 9)}.`,
          )}
        </p>
        <Bar counts={{ ...tot, total: doc.corpus }} />
      </div>
      {done.length > 0 && (
        <>
          <h4 className="sets-sub">
            <T en="Finished" de="Fertig" />
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
            <T en="Next, in release order" de="Als Nächstes, in Erscheinungsreihenfolge" />
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
