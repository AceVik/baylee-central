// The box that asks for the details: on the visitor's click the browser
// solves the feedback service's puzzle, the blob comes back, and the details
// are drawn. Without the means (an old browser, no secure context) or when
// they do not come, it says so and points to the contact form.

import { useMemo, useState } from "react";

import { impressumEndpoint } from "../config";
import { T, tt, useLang } from "../i18n";
import { Link, PATHS } from "../router";
import { decode, type Fields } from "./blob";
import { Details, draw } from "./Details";
import { fetchBlob, type Solver } from "./fetch";

type State = "idle" | "working" | "shown" | "failed";

function ready(): boolean {
  return (
    typeof crypto !== "undefined" &&
    typeof crypto.subtle !== "undefined" &&
    typeof crypto.getRandomValues === "function" &&
    typeof Worker === "function" &&
    typeof TextDecoder === "function"
  );
}

export function ContactLink({ children }: { children: string }) {
  return <Link to={PATHS.contact}>{children}</Link>;
}

export function ImpressumBox({ solver }: { solver?: Solver }) {
  const lang = useLang();
  const [state, setState] = useState<State>("idle");
  const [progress, setProgress] = useState(0);
  const [fields, setFields] = useState<Fields | null>(null);
  const drawn = useMemo(() => (fields ? draw(fields) : null), [fields]);
  const capable = useMemo(() => ready(), []);

  const show = () => {
    if (state === "working") return;
    setState("working");
    setProgress(0);
    fetchBlob(impressumEndpoint(), setProgress, solver)
      .then(decode)
      .then((f) => {
        setFields(f);
        setState("shown");
      })
      .catch(() => setState("failed"));
  };

  return (
    <div className="imp-box glass">
      {state !== "shown" && (
        <p className="imp-why">
          <T
            en="To keep address harvesters away, the details load only when you ask for them; your browser works out a small puzzle for it, which takes a moment."
            de="Damit Adresssammler leer ausgehen, werden die Angaben erst auf deinen Wunsch geladen; dein Browser löst dafür ein kleines Rätsel, das einen Moment dauert."
          />
        </p>
      )}
      {capable && (state === "idle" || state === "working" || state === "failed") && (
        <button type="button" className="button" onClick={show} aria-busy={state === "working"} aria-describedby="imp-show-status">
          <T en="Show the details" de="Impressum anzeigen" />
        </button>
      )}
      {state === "working" && <progress className="imp-progress" max={1} value={progress} aria-label={tt(lang, "Puzzle", "Rätsel")} />}
      <output className="imp-status" id="imp-show-status">
        {state === "working" &&
          tt(lang, "One moment, your browser is working out a small puzzle …", "Einen Moment, dein Browser löst ein kleines Rätsel …")}
      </output>
      {(state === "failed" || !capable) && (
        <p className="imp-failed">
          <T
            en={
              <>
                The details could not be loaded. Please reload the page or use the{" "}
                <ContactLink>contact form</ContactLink>.
              </>
            }
            de={
              <>
                Die Angaben konnten nicht geladen werden. Bitte lade die Seite neu oder nutze das{" "}
                <ContactLink>Kontaktformular</ContactLink>.
              </>
            }
          />
        </p>
      )}
      {/* the honeypot: hidden from people, disallowed in robots.txt; an address that follows it gets invented details */}
      <a className="hp" href="/site/impressum/kontakt" tabIndex={-1} aria-hidden="true" rel="nofollow">
        Kontakt
      </a>
      {drawn && <Details drawn={drawn} />}
    </div>
  );
}
