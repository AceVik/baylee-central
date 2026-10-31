// The section's single-key shortcuts (`j`, `k`, `?` and the rest): never
// while a field has the keyboard or a modifier is held, and switched off
// with one choice the browser keeps, so that no key fires by accident for
// someone who speaks or types commands (WCAG 2.1.4, "turn off").

const KEY = "baylee-feedback-keys";

/** Whether a key press belongs to a field, a menu or a dialog rather than to the list. */
export function typing(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  return target.isContentEditable || ["INPUT", "SELECT", "TEXTAREA"].includes(target.tagName) || target.closest("dialog") !== null;
}

/** Whether the single-key shortcuts are on (they are unless switched off). */
export function keysOn(): boolean {
  try {
    return window.localStorage.getItem(KEY) !== "off";
  } catch {
    return true;
  }
}

export function setKeysOn(on: boolean): void {
  try {
    if (on) window.localStorage.removeItem(KEY);
    else window.localStorage.setItem(KEY, "off");
  } catch {
    /* private mode or blocked storage: the choice lasts this visit */
  }
}

/** A single key for the section: no modifier but Shift, not in a field, shortcuts on. */
export function forUs(e: KeyboardEvent): boolean {
  return !e.ctrlKey && !e.metaKey && !e.altKey && !e.defaultPrevented && !typing(e.target) && keysOn();
}
