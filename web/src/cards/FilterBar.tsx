import { num, t, useLang, type Key } from "../i18n";
import { GROUP_LABEL, type Counts, type Group } from "./model";

export type Filter = "all" | Group;

export function FilterBar({ counts, filter, onFilter }: { counts: Counts; filter: Filter; onFilter: (f: Filter) => void }) {
  const lang = useLang();
  const groups: [Filter, Key, number][] = [
    ["all", "filter.all", counts.total],
    ["implemented", GROUP_LABEL.implemented, counts.implemented],
    ["generated", GROUP_LABEL.generated, counts.generated],
    ["partial", GROUP_LABEL.partial, counts.partial],
    ["none", GROUP_LABEL.none, counts.unimplemented + counts.absent],
    ["never", GROUP_LABEL.never, counts.never],
  ];
  return (
    <fieldset className="cfilter">
      <legend className="vh">{t(lang, "filter.label")}</legend>
      {groups
        .filter(([g, , n]) => g === "all" || n > 0)
        .map(([g, label, n]) => (
          <button key={g} type="button" className="chip-btn" aria-pressed={filter === g} onClick={() => onFilter(g)}>
            {t(lang, label)} <span className="chip-n">{num(lang, n)}</span>
          </button>
        ))}
    </fieldset>
  );
}
