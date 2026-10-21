// The submit button of an account form: never `disabled` while working,
// which would drop the keyboard's focus to the page; the form ignores a
// second press instead.

import { T, type Key } from "../i18n";

export function SubmitButton({ busy, label, busyLabel }: { busy: boolean; label: Key; busyLabel: Key }) {
  return (
    <button type="submit" className="button button-send" aria-disabled={busy ? true : undefined}>
      {busy && (
        <svg className="button-icon i-spin" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
          <circle cx="12" cy="12" r="8.5" />
        </svg>
      )}
      <T id={busy ? busyLabel : label} />
    </button>
  );
}
