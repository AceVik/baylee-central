// Every set: in eras of four years, or, while a search is typed, its matches
// flat (at most 40, the count announced). A link to one set opens its era and
// its row and scrolls there.

import { useEffect, useMemo, useState } from "react";

import { T, t, useLang } from "../i18n";
import { reducedMotion } from "../motion";
import type { SetsDoc } from "./model";
import { begun, eraOf, eras, matches, sum } from "./numbers";
import { Row, SetRow, useOpenSet } from "./SetRow";

const FOUND_MAX = 40;

export function AllSets({ doc, target, onTargetShown }: { doc: SetsDoc; target: string | null; onTargetShown: () => void }) {
  const lang = useLang();
  const [query, setQuery] = useState("");
  const [needle, setNeedle] = useState("");
  const [onlyBegun, setOnlyBegun] = useState(false);
  const [openEras, setOpenEras] = useState<ReadonlySet<number>>(new Set());
  const [openSets, toggleSet, addSets] = useOpenSet();

  useEffect(() => {
    const wait = window.setTimeout(() => setNeedle(query), 150);
    return () => window.clearTimeout(wait);
  }, [query]);

  // a link to one set: its era open, then its row (set while rendering, so
  // the rows are there in the same commit), then scrolled to and focused
  const [handled, setHandled] = useState<string | null>(null);
  if (target !== null && target !== handled) {
    setHandled(target);
    const s = doc.sets.find((x) => x.code === target);
    if (s) {
      setQuery("");
      setNeedle("");
      if (!begun(s)) setOnlyBegun(false);
      setOpenEras((prev) => new Set([...prev, eraOf(s)]));
      addSets([s.code]);
    }
  }
  useEffect(() => {
    if (target === null || handled !== target) return;
    const frame = requestAnimationFrame(() => {
      const row = document.getElementById(`set-${target}`);
      if (row) {
        row.scrollIntoView({ block: "start", behavior: reducedMotion() ? "auto" : "smooth" });
        row.querySelector("summary")?.focus({ preventScroll: true });
      }
      onTargetShown();
    });
    return () => cancelAnimationFrame(frame);
  }, [target, handled, onTargetShown]);

  const chosen = useMemo(() => doc.sets.filter((s) => !onlyBegun || begun(s)), [doc, onlyBegun]);
  const hits = useMemo(() => (needle.trim() ? chosen.filter((s) => matches(s, needle)) : null), [chosen, needle]);

  let found = "";
  if (hits) {
    const n = hits.length;
    if (n === 0) found = t(lang, "sets.foundNone");
    else if (n > FOUND_MAX) found = t(lang, "sets.foundMany", { n, max: FOUND_MAX });
    else found = t(lang, "sets.found", { n });
  }

  return (
    <div className="sets-all" id="sets-all">
      <div className="sets-tools">
        <label className="field sets-search">
          <span>
            <T id="sets.find" />
          </span>
          <input
            type="search"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            autoComplete="off"
            spellCheck={false}
            aria-describedby="sets-found"
          />
        </label>
        <label className="check-label">
          <input type="checkbox" checked={onlyBegun} onChange={(e) => setOnlyBegun(e.target.checked)} />{" "}
          <T id="sets.onlyBegun" />
        </label>
      </div>
      <output className="small sets-found" id="sets-found">
        {found}
      </output>
      <div className="sets">
        {hits
          ? hits.slice(0, FOUND_MAX).map((s) => <SetRow key={s.code} set={s} withId open={openSets.has(s.code)} onOpen={toggleSet} />)
          : eras(chosen).map((era) => {
              const counts = sum(era.sets);
              return (
                <Row
                  key={era.from}
                  className="set set-era"
                  open={openEras.has(era.from)}
                  onOpen={(o) =>
                    setOpenEras((prev) => {
                      if (prev.has(era.from) === o) return prev;
                      const next = new Set(prev);
                      if (o) next.add(era.from);
                      else next.delete(era.from);
                      return next;
                    })
                  }
                  title={era.from === era.to ? String(era.from) : `${era.from}–${era.to}`}
                  meta={t(lang, "sets.eraMeta", { sets: era.sets.length, cards: counts.total })}
                  counts={counts}
                >
                  {() => (
                    <div className="sets era-body">
                      {era.sets.map((s) => (
                        <SetRow key={s.code} set={s} withId open={openSets.has(s.code)} onOpen={toggleSet} />
                      ))}
                    </div>
                  )}
                </Row>
              );
            })}
      </div>
    </div>
  );
}
