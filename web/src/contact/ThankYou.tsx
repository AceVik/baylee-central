// What the form becomes once a message is sent; it takes the focus, so a
// screen reader says so.

import { useEffect, useRef } from "react";

import { T } from "../i18n";

export function ThankYou({ onAgain }: { onAgain: () => void }) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => ref.current?.focus(), []);
  return (
    <div className="contact-done" tabIndex={-1} ref={ref}>
      <svg className="check" viewBox="0 0 52 52" aria-hidden="true" focusable="false">
        <circle cx="26" cy="26" r="24" />
        <path d="M15 27l7 7 15-16" />
      </svg>
      <h3>
        <T en="Thank you, it is on its way." de="Danke, sie ist unterwegs." />
      </h3>
      <p className="small">
        <T
          en="If you left an address, the answer comes from a person, by mail."
          de="Wenn du eine Adresse dagelassen hast, kommt die Antwort von einem Menschen, per Mail."
        />
      </p>
      <button
        type="button"
        className="button button-ghost"
        onClick={onAgain}
      >
        <T en="Write another" de="Noch eine schreiben" />
      </button>
    </div>
  );
}
