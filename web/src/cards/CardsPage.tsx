import type { ReactNode } from "react";

import { GITHUB } from "../config";
import { T, num, t, useLang, useTitle, type Elems, type Lang } from "../i18n";
import { useCountUp } from "../motion";
import { useLocation } from "../router";
import { SetsBlock } from "./SetsBlock";
import { useSetsDoc } from "./useSetsDoc";

/** Where the counts come from: the pool list, and the README of `lang`'s language. */
const sources = (lang: Lang): Elems => ({
  pool: (words) => (
    <a href={`${GITHUB}/blob/main/data/card-pool.txt`} rel="noopener">
      {words}
    </a>
  ),
  readme: (words) => (
    <a href={`${GITHUB}${t(lang, "link.readmeWhat")}`} rel="noopener">
      {words}
    </a>
  ),
});

function NumberTile({ value, label }: { value: number | null; label: ReactNode }) {
  const lang = useLang();
  const shown = useCountUp(value);
  return (
    <div className="number glass">
      <dt>
        <span aria-hidden="true">{shown === null ? "–" : num(lang, shown)}</span>
        <span className="vh">{value === null ? "–" : num(lang, value)}</span>
      </dt>
      <dd>{label}</dd>
    </div>
  );
}

/** The three numbers of the card status, counted up once they are loaded. */
export function CardNumbers() {
  const doc = useSetsDoc();
  const totals = doc?.totals;
  return (
    <dl className="numbers">
      <NumberTile
        value={totals ? totals.implemented : null}
        label={<T id="cards.implemented" />}
      />
      <NumberTile
        value={totals ? totals.partial : null}
        label={<T id="cards.partial" />}
      />
      <NumberTile
        value={doc && totals ? doc.corpus - totals.never : null}
        label={<T id="cards.corpus" />}
      />
    </dl>
  );
}

export default function Cards() {
  useTitle("title.cards");
  const lang = useLang();
  const { hash } = useLocation();
  return (
    <>
      <header className="page-head">
        <div className="section-inner">
          <p className="eyebrow">
            <T id="cards.eyebrow" />
          </p>
          <h1>
            <T id="cards.title" />
          </h1>
          <div className="two-col">
            <div>
              <p>
                <T id="cards.intro" />
              </p>
              <p className="small">
                <T id="cards.counted" elems={sources(lang)} />
              </p>
            </div>
            <div>
              <h2 className="h3">
                <T id="cards.planTitle" />
              </h2>
              <p>
                <T id="cards.plan" />
              </p>
              <p>
                <T id="cards.modern" />
              </p>
            </div>
          </div>
          <CardNumbers />
        </div>
      </header>
      <section className="section section-alt" aria-labelledby="sets-title">
        <div className="section-inner">
          <SetsBlock hash={hash} />
        </div>
      </section>
    </>
  );
}
