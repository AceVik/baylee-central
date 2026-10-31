// The button of an action that takes a moment: never `disabled` while
// working, which would drop the keyboard's focus to the page; it says it is
// busy (`aria-disabled`, a spinner) and ignores a second press instead.

import type { ReactNode } from "react";

import { T, type Key } from "../i18n";

export function BusyButton({
  busy,
  className = "button",
  type = "button",
  onClick,
  children,
}: {
  busy: boolean;
  className?: string;
  type?: "button" | "submit";
  onClick?: () => void;
  children: ReactNode;
}) {
  return (
    <button
      type={type}
      className={className}
      aria-disabled={busy ? true : undefined}
      onClick={(e) => {
        // a form's submit handler checks `busy` itself; a plain button stops here
        if (busy && type === "button") e.preventDefault();
        else onClick?.();
      }}
    >
      {busy && (
        <svg className="button-icon i-spin" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
          <circle cx="12" cy="12" r="8.5" />
        </svg>
      )}
      {children}
    </button>
  );
}

export function SubmitButton({ busy, label, busyLabel, className = "button button-send" }: { busy: boolean; label: Key; busyLabel: Key; className?: string }) {
  return (
    <BusyButton busy={busy} type="submit" className={className}>
      <T id={busy ? busyLabel : label} />
    </BusyButton>
  );
}
