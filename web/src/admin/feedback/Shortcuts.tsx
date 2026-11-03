// `?`: the section's keys in a dialog, and the switch that turns the single
// keys off (WCAG 2.1.4); the browser keeps the choice (`keys.ts`).

import { useId, useState } from "react";

import { Modal } from "../../components/Modal";
import { t, useLang, type Key } from "../../i18n";
import { keysOn, setKeysOn } from "./keys";

/** The list's keys, and a report page's. */
export const LIST_KEYS: readonly [string, Key][] = [
  ["j / k", "fb.keyMove"],
  ["Enter", "fb.keyOpen"],
  ["o", "fb.keyPage"],
  ["x", "fb.keySelect"],
  ["a", "fb.keySelectAll"],
  ["1 – 5", "fb.keyStatus"],
  ["m", "fb.keyMine"],
  ["u", "fb.keyRead"],
  ["# / Del", "fb.keyTrash"],
  ["[ / ]", "fb.keyPages"],
  ["/", "fb.keySearch"],
  ["r", "fb.keyRefresh"],
  ["c", "fb.keyCopy"],
  ["e", "fb.keyExport"],
  ["Esc", "fb.keyClose"],
  ["?", "fb.keyHelp"],
];

export const PAGE_KEYS: readonly [string, Key][] = [
  ["k / j", "fb.keySteps"],
  ["?", "fb.keyHelp"],
];

export function Shortcuts({ keys, onClose }: { keys: readonly [string, Key][]; onClose(): void }) {
  const lang = useLang();
  const heading = useId();
  const [on, setOn] = useState(keysOn);
  return (
    <Modal className="app-dialog app-dialog-wide" labelledBy={heading} onClose={onClose}>
      <h2 id={heading} className="app-dialog-title">
        {t(lang, "fb.keysTitle")}
      </h2>
      <dl className="kv fb-keys">
        {keys.map(([k, what]) => (
          <div key={k}>
            <dt>
              <kbd>{k}</kbd>
            </dt>
            <dd>{t(lang, what)}</dd>
          </div>
        ))}
        <div>
          <dt>
            <kbd>Ctrl K</kbd>
          </dt>
          <dd>{t(lang, "fb.keySearch")}</dd>
        </div>
      </dl>
      <label className="check-label">
        <input
          type="checkbox"
          checked={on}
          onChange={(e) => {
            setKeysOn(e.target.checked);
            setOn(e.target.checked);
          }}
        />
        {t(lang, "fb.keysOn")}
      </label>
      <p className="small">{t(lang, "fb.keysWhy")}</p>
      <div className="app-dialog-actions">
        <button type="button" className="button" onClick={onClose}>
          {t(lang, "app.close")}
        </button>
      </div>
    </Modal>
  );
}
