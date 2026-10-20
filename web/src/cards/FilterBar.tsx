import { num, tt, useLang } from "../i18n";
import type { Counts, Group } from "./model";

export type Filter = "all" | Group;

export function FilterBar({ counts, filter, onFilter }: { counts: Counts; filter: Filter; onFilter: (f: Filter) => void }) {
  const lang = useLang();
  const groups: [Filter, string, string, number][] = [
    ["all", "All", "Alle", counts.total],
    ["implemented", "Full", "Voll", counts.implemented],
    ["generated", "Untested", "Ungetestet", counts.generated],
    ["partial", "Partial", "Teilweise", counts.partial],
    ["none", "Not yet", "Noch nicht", counts.unimplemented + counts.absent],
    ["never", "Never", "Nie", counts.never],
  ];
  return (
    <fieldset className="cfilter">
      <legend className="vh">{tt(lang, "Show cards", "Karten zeigen")}</legend>
      {groups
        .filter(([g, , , n]) => g === "all" || n > 0)
        .map(([g, en, de, n]) => (
          <button key={g} type="button" className="chip-btn" aria-pressed={filter === g} onClick={() => onFilter(g)}>
            {tt(lang, en, de)} <span className="chip-n">{num(lang, n)}</span>
          </button>
        ))}
    </fieldset>
  );
}
