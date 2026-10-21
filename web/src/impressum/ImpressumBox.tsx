// The box that asks for the details: on the visitor's click the browser
// solves the feedback service's puzzle, the blob comes back, and the details
// are drawn. Without the means (an old browser, no secure context) or when
// they do not come, it says so and points to the contact form.

import { useMemo, useState } from "react";

import { impressumEndpoint } from "../config";
import { T, t, useLang, type Elems } from "../i18n";
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

/** The contact form, as a message's `<contact>…</contact>` names it. */
export const CONTACT: Elems = { contact: (words) => <Link to={PATHS.contact}>{words}</Link> };

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
          <T id="imp.why" />
        </p>
      )}
      {capable && (state === "idle" || state === "working" || state === "failed") && (
        <button type="button" className="button" onClick={show} aria-busy={state === "working"} aria-describedby="imp-show-status">
          <T id="imp.show" />
        </button>
      )}
      {state === "working" && <progress className="imp-progress" max={1} value={progress} aria-label={t(lang, "imp.puzzle")} />}
      <output className="imp-status" id="imp-show-status">
        {state === "working" &&
          t(lang, "imp.working")}
      </output>
      {(state === "failed" || !capable) && (
        <p className="imp-failed">
          <T id="imp.failed" elems={CONTACT} />
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
