// The password, twice, with the strength hint under the first: a hint, no
// composition rule. Eight characters at least, 1024 bytes at most (the
// crypto's bound).

import { useEffect, useRef } from "react";

import { T, t, useLang, type Key } from "../i18n";
import { strength, type Strength } from "./strength";

const LEVEL: Readonly<Record<Strength, Key>> = {
  short: "register.strengthShort",
  weak: "register.strengthWeak",
  fair: "register.strengthFair",
  strong: "register.strengthStrong",
};

export interface PasswordProps {
  password: string;
  again: string;
  username: string;
  /** The field the form found wrong; each new request (`n`) focuses it. */
  invalid: { field: "password" | "again"; n: number } | null;
  onPassword(value: string): void;
  onAgain(value: string): void;
}

export function PasswordFields(p: PasswordProps) {
  const lang = useLang();
  const level = strength(p.password, p.username);
  const passwordRef = useRef<HTMLInputElement>(null);
  const againRef = useRef<HTMLInputElement>(null);
  const { invalid } = p;
  const field = invalid?.field ?? null;
  const request = invalid?.n ?? 0;
  // only a new complaint moves the focus, never a keystroke
  useEffect(() => {
    if (field === null || request === 0) return;
    (field === "password" ? passwordRef : againRef).current?.focus();
  }, [field, request]);
  return (
    <>
      <div className="field">
        <label htmlFor="r-password">
          <T id="register.password" />
        </label>
        <input
          ref={passwordRef}
          id="r-password"
          type="password"
          autoComplete="new-password"
          value={p.password}
          aria-invalid={invalid?.field === "password" ? true : undefined}
          aria-describedby="r-password-hint r-strength"
          onChange={(e) => p.onPassword(e.target.value)}
        />
        <p id="r-password-hint" className="field-hint">
          <T id="register.passwordHint" />
        </p>
        <div className="strength" data-level={p.password === "" ? "none" : level}>
          <span className="strength-bar" aria-hidden="true" />
          <span id="r-strength" className="strength-text" aria-live="polite">
            {p.password === "" ? "" : t(lang, LEVEL[level])}
          </span>
        </div>
      </div>
      <div className="field">
        <label htmlFor="r-again">
          <T id="register.passwordAgain" />
        </label>
        <input
          ref={againRef}
          id="r-again"
          type="password"
          autoComplete="new-password"
          value={p.again}
          aria-invalid={invalid?.field === "again" ? true : undefined}
          onChange={(e) => p.onAgain(e.target.value)}
        />
      </div>
    </>
  );
}
