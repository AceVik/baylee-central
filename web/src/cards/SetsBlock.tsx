// Sets and card status: the legend, the summary, and the full list behind a
// button. `#set-<code>` in the address opens that set wherever it is.

import { useCallback, useState } from "react";

import { T, num, tt, useLang } from "../i18n";
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
    const t = targetOf(hash);
    if (t !== null) {
      setTarget(t);
      setExpanded(true);
    }
  }

  return (
    <div className="sets-block" id="sets">
      <h2 id="sets-title">
        <T en="Sets and card status" de="Sets und Kartenstatus" />
      </h2>
      <p className="section-lead">
        <T
          en="Every set in release order, each card counted under the set it first appeared in. Open a set to see its cards: whether each plays as printed, plays but is not yet tested, plays in part, is not playable yet, or will never be built. Rest the pointer on a card, or focus it, to see it large with the reason; a click opens its page on Scryfall, with artist and printing. On a touch screen a tap shows it large first."
          de="Jedes Set in Erscheinungsreihenfolge, jede Karte unter dem Set gezählt, in dem sie zuerst erschien. Öffne ein Set, um seine Karten zu sehen: ob jede wie gedruckt spielt, spielt, aber noch ungetestet ist, teilweise spielt, noch nicht spielbar ist oder nie gebaut wird. Lass den Zeiger auf einer Karte ruhen oder fokussiere sie, um sie groß mit dem Grund zu sehen; ein Klick öffnet ihre Seite bei Scryfall, mit Künstler und Druck. Auf einem Touchscreen zeigt ein Tippen sie zuerst groß."
        />
      </p>
      <ul className="legend" aria-label={tt(lang, "Legend", "Legende")}>
        <li>
          <span className="swatch s-implemented" aria-hidden="true" />
          <T en="Full: plays as printed" de="Voll: spielt wie gedruckt" />
        </li>
        <li>
          <span className="swatch s-generated" aria-hidden="true" />
          <T
            en="Untested: playable, every sentence is read by the rules engine, but no test plays it yet"
            de="Ungetestet: spielbar, jeder Satz wird von der Regel-Engine gelesen, aber noch kein Test spielt sie"
          />
        </li>
        <li>
          <span className="swatch s-partial" aria-hidden="true" />
          <T
            en="Partial: plays, one part missing (the reason is shown with the card)"
            de="Teilweise: spielt, ein Teil fehlt (der Grund steht bei der Karte)"
          />
        </li>
        <li>
          <span className="swatch s-none" aria-hidden="true" />
          <T en="Not yet: not playable yet" de="Noch nicht: noch nicht spielbar" />
        </li>
        <li>
          <span className="swatch s-never" aria-hidden="true" />
          <T
            en="Never: ante, dexterity or a subgame; a set counts as complete without it"
            de="Nie: Ante, Geschicklichkeit oder ein Teilspiel; ein Set gilt ohne sie als vollständig"
          />
        </li>
      </ul>
      <p className="small">
        <T
          en="The percentage is full cards out of all cards of the set that will ever be built. Untested and partial cards are the other parts of the bar and are not counted as done."
          de="Der Prozentwert ist der Anteil voller Karten an allen Karten des Sets, die je gebaut werden. Ungetestete und teilweise umgesetzte Karten sind die übrigen Teile des Balkens und zählen nicht als fertig."
        />
      </p>
      {doc === undefined && (
        <output className="small block">
          <T en="Loading the set list …" de="Lade die Set-Liste …" />
        </output>
      )}
      {doc === null && (
        <output className="small block">
          <T
            en="The set list did not load just now. Reload the page to try again."
            de="Die Set-Liste wurde gerade nicht geladen. Lade die Seite neu, um es noch einmal zu versuchen."
          />
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
              {expanded
                ? tt(lang, "Hide the full list", "Vollständige Liste ausblenden")
                : tt(lang, `Show all ${num(lang, doc.sets.length)} sets`, `Alle ${num(lang, doc.sets.length)} Sets anzeigen`)}
            </button>
          </p>
          {expanded && <AllSets doc={doc} target={target} onTargetShown={shown} />}
        </>
      )}
      {/* Attribution and notices for the card status and its pictures: add
          what the legal review asks for here, in both languages (and in
          CARD_CREDIT in model.ts for the line under a card shown large). */}
      <div className="sets-notes">
        <p className="small">
          <T
            en={
              <>
                Card images are loaded from Scryfall only when you open a set or show a card large, unaltered. Card data
                and images provided by Scryfall; Baylee is not affiliated with Scryfall. The counts are measured from the
                repository by <code>baylee-tools site card-status</code>.
              </>
            }
            de={
              <>
                Kartenbilder werden erst beim Öffnen eines Sets oder beim Großzeigen einer Karte und unverändert von
                Scryfall geladen. Kartendaten und -bilder bereitgestellt von Scryfall; Baylee ist nicht mit Scryfall
                verbunden. Die Zahlen misst <code>baylee-tools site card-status</code> im Repository.
              </>
            }
          />
        </p>
        <p className="small">
          <T
            en="Card images © Wizards of the Coast, shown unaltered from Scryfall. Cards not yet in Baylee's pool, and Universes Beyond cards, are listed by name only."
            de="Kartenbilder © Wizards of the Coast, unverändert von Scryfall geladen. Karten, die noch nicht in Baylees Pool sind, und Universes-Beyond-Karten stehen nur mit Namen hier."
          />
        </p>
      </div>
    </div>
  );
}
