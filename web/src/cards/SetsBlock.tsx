// Sets and card status: the legend, the summary, and the full list behind a
// button. `#set-<code>` in the address opens that set wherever it is.

import { useCallback, useState } from "react";

import { T, t, useLang } from "../i18n";
import "../styles/cards.css";
import { AllSets } from "./AllSets";
import { Summary } from "./Summary";
import { useSetsDoc } from "./useSetsDoc";

function targetOf(hash: string): string | null {
  const m = /^#set-([a-z0-9]+)$/.exec(hash);
  return m?.[1] ?? null;
}

export function SetsBlock({ hash }: { hash: string }) {
  const lang = useLang();
  const doc = useSetsDoc();
  const [target, setTarget] = useState<string | null>(() => targetOf(hash));
  const [expanded, setExpanded] = useState(() => targetOf(hash) !== null);
  const shown = useCallback(() => setTarget(null), []);
  // a new `#set-<code>` opens the full list at that set
  const [seen, setSeen] = useState(hash);
  if (hash !== seen) {
    setSeen(hash);
    const next = targetOf(hash);
    if (next !== null) {
      setTarget(next);
      setExpanded(true);
    }
  }

  return (
    <div className="sets-block" id="sets">
      <h2 id="sets-title">
        <T id="sets.title" />
      </h2>
      <p className="section-lead">
        <T id="sets.lead" />
      </p>
      <ul className="legend" aria-label={t(lang, "sets.legend")}>
        <li>
          <span className="swatch s-implemented" aria-hidden="true" />
          <T id="sets.legendFull" />
        </li>
        <li>
          <span className="swatch s-generated" aria-hidden="true" />
          <T id="sets.legendUntested" />
        </li>
        <li>
          <span className="swatch s-partial" aria-hidden="true" />
          <T id="sets.legendPartial" />
        </li>
        <li>
          <span className="swatch s-none" aria-hidden="true" />
          <T id="sets.legendNone" />
        </li>
        <li>
          <span className="swatch s-never" aria-hidden="true" />
          <T id="sets.legendNever" />
        </li>
      </ul>
      <p className="small">
        <T id="sets.percent" />
      </p>
      {doc === undefined && (
        <output className="small block">
          <T id="sets.loading" />
        </output>
      )}
      {doc === null && (
        <output className="small block">
          <T id="sets.failed" />
        </output>
      )}
      {doc && (
        <>
          <Summary doc={doc} />
          <p className="sets-more">
            <button
              type="button"
              className="button button-ghost"
              aria-expanded={expanded}
              aria-controls="sets-all"
              onClick={() => setExpanded((e) => !e)}
            >
              {expanded ? t(lang, "sets.hide") : t(lang, "sets.showAll", { n: doc.sets.length })}
            </button>
          </p>
          {expanded && <AllSets doc={doc} target={target} onTargetShown={shown} />}
        </>
      )}
      {/* Attribution and notices for the card status and its pictures: add
          what the legal review asks for here, in every language file (and
          in card.credit for the line under a card shown large). */}
      <div className="sets-notes">
        <p className="small">
          <T id="sets.imagesNote" />
        </p>
        <p className="small">
          <T id="sets.ub" />
        </p>
      </div>
    </div>
  );
}
