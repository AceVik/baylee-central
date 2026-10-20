import type { ReactNode } from "react";

import { GITHUB } from "../config";
import { T, num, useLang, useTitle } from "../i18n";
import { SetsBlock, useSetsDoc } from "../cards/CardStatus";
import { useCountUp } from "../motion";
import { useLocation } from "../router";

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
  const t = doc?.totals;
  return (
    <dl className="numbers">
      <NumberTile
        value={t ? t.implemented : null}
        label={<T en="cards that play exactly as printed" de="Karten, die genau wie gedruckt spielen" />}
      />
      <NumberTile
        value={t ? t.partial : null}
        label={<T en="more that are partly implemented and marked as such" de="weitere, teilweise umgesetzt und so markiert" />}
      />
      <NumberTile
        value={doc && t ? doc.corpus - t.never : null}
        label={<T en="cards in the whole corpus we are working towards" de="Karten im gesamten Korpus, auf den wir hinarbeiten" />}
      />
    </dl>
  );
}

export default function Cards() {
  useTitle("Cards – Baylee", "Karten – Baylee");
  const { hash } = useLocation();
  return (
    <>
      <header className="page-head">
        <div className="section-inner">
          <p className="eyebrow">
            <T en="Cards" de="Karten" />
          </p>
          <h1>
            <T en="The cards, honestly" de="Die Karten, ehrlich" />
          </h1>
          <div className="two-col">
            <div>
              <p>
                <T
                  en='Every card in Baylee is code: one file per card, written in a small rules language and played in a test. There is no "close enough" interpreter guessing at card text. That is why the pool grows batch by batch instead of all at once.'
                  de="Jede Karte in Baylee ist Code: eine Datei pro Karte, in einer kleinen Regelsprache geschrieben und in einem Test gespielt. Es gibt keinen Interpreter, der Kartentext ungefähr errät. Deshalb wächst der Pool Stapel für Stapel statt auf einmal."
                />
              </p>
              <p className="small">
                <T
                  en={
                    <>
                      Counted from the repository by Baylee&apos;s own tools (
                      <a href={`${GITHUB}/blob/main/data/card-pool.txt`} rel="noopener">
                        the pool list
                      </a>
                      ,{" "}
                      <a href={`${GITHUB}#what-baylee-is`} rel="noopener">
                        the README
                      </a>
                      ). The deck builder can show only playable cards, and it warns when a deck holds cards that will not
                      play as printed.
                    </>
                  }
                  de={
                    <>
                      Gezählt im Repository von Baylees eigenen Werkzeugen (
                      <a href={`${GITHUB}/blob/main/data/card-pool.txt`} rel="noopener">
                        die Pool-Liste
                      </a>
                      ,{" "}
                      <a href={`${GITHUB}/blob/main/README.de.md#was-baylee-ist`} rel="noopener">
                        die README
                      </a>
                      ). Der Deckbauer kann nur spielbare Karten zeigen und warnt, wenn ein Deck Karten enthält, die nicht
                      wie gedruckt spielen.
                    </>
                  }
                />
              </p>
            </div>
            <div>
              <h2 className="h3">
                <T en="The plan" de="Der Plan" />
              </h2>
              <p>
                <T
                  en="Sets go in release order, starting with Alpha, because every set introduces its mechanics on top of the older ones. Alpha is complete: every playable card of it plays as printed. Arabian Nights and Antiquities come next, and the rest follows set by set. Bugs found at the table come before new cards, every time."
                  de="Die Sets kommen in Erscheinungsreihenfolge, beginnend mit Alpha, weil jedes Set seine Mechaniken auf die älteren setzt. Alpha ist vollständig: Jede spielbare Karte daraus spielt wie gedruckt. Als Nächstes kommen Arabian Nights und Antiquities, der Rest folgt Set für Set. Fehler, die am Tisch auffallen, gehen immer vor neuen Karten."
                />
              </p>
              <p>
                <T
                  en="That means a modern deck will often hold cards Baylee does not know yet. We would rather tell you that in the deck builder than let a card resolve wrongly. Cards that cannot be played at all (ante, dexterity, subgames) are on a named skip list and never silently dropped."
                  de="Das heißt: Ein modernes Deck enthält oft Karten, die Baylee noch nicht kennt. Wir sagen dir das lieber im Deckbauer, als eine Karte falsch verrechnen zu lassen. Karten, die gar nicht spielbar sind (Ante, Geschicklichkeit, Teilspiele), stehen auf einer benannten Auslassungsliste und fallen nie stillschweigend weg."
                />
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
