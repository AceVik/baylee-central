// A password field with what makes typing one easier: a button that shows
// what was typed (pressed state, the field's own name) and a hint while Caps
// Lock is on. Shown as text, the field still never goes to a spelling
// service (spellcheck off) and its autocomplete stays the password's; once
// the field is emptied (after a sign-in) it hides again.

import { useState, type Ref } from "react";

import { Icon } from "../components/icons";
import { t, useLang } from "../i18n";

export function PasswordInput({
  id,
  inputRef,
  value,
  onChange,
  autoComplete,
  invalid = false,
  describedBy,
}: {
  id: string;
  inputRef?: Ref<HTMLInputElement>;
  value: string;
  onChange(value: string): void;
  autoComplete: "current-password" | "new-password";
  invalid?: boolean;
  describedBy?: string;
}) {
  const lang = useLang();
  const [show, setShow] = useState(false);
  const [caps, setCaps] = useState(false);
  const shown = show && value !== "";
  const capsId = `${id}-caps`;
  const watch = (e: { getModifierState(key: string): boolean }) => setCaps(e.getModifierState("CapsLock"));
  const described = [describedBy, caps ? capsId : null].filter((d) => d !== undefined && d !== null).join(" ");
  return (
    <>
      <div className="password-box">
        <input
          ref={inputRef}
          id={id}
          type={shown ? "text" : "password"}
          autoComplete={autoComplete}
          autoCapitalize="none"
          autoCorrect="off"
          spellCheck={false}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          onKeyDown={watch}
          onKeyUp={watch}
          onBlur={() => setCaps(false)}
          aria-invalid={invalid ? true : undefined}
          aria-describedby={described === "" ? undefined : described}
        />
        <button type="button" className="password-eye" aria-controls={id} aria-pressed={shown} onClick={() => setShow(!shown)} title={t(lang, "pw.show")}>
          <Icon name={shown ? "eye" : "watch"} />
          <span className="vh">{t(lang, "pw.show")}</span>
        </button>
      </div>
      {caps && (
        <p id={capsId} className="field-hint caps-hint">
          <Icon name="warn" className="pill-icon" /> {t(lang, "pw.caps")}
        </p>
      )}
    </>
  );
}
