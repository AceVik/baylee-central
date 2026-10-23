// The registration form (docs/web-accounts.md §"Web pages"): the beta key,
// the username with its rules, the display name other players see with its
// rules, the password twice with a strength hint, the minimum age of 16
// (Central's terms §2), and the acceptance of Central's terms and the
// privacy notice, linked. The page checks what it can say early; Central
// decides.

import { useRef, useState, type FormEvent } from "react";

import { T, type Elems, type Key } from "../i18n";
import { PATHS } from "../router";
import { SubmitButton } from "./Busy";
import { canonicalName, reservedDisplayName, validDisplayName, validName } from "./names";
import { PasswordFields } from "./PasswordFields";
import { acceptable, MAX_BYTES } from "./strength";

export interface Registration {
  betaKey: string;
  username: string;
  /** Kept as typed; other players see it beside the tag Central draws. */
  displayName: string;
  password: string;
}

type Problem = "key" | "username" | "displayName" | "reserved" | "password" | "again" | "age" | "consent";

const SAY: Readonly<Record<Problem, Key>> = {
  key: "register.errKey",
  username: "register.errUsername",
  displayName: "register.errDisplayName",
  reserved: "register.errDisplayNameReserved",
  password: "register.errPassword",
  again: "register.errMismatch",
  age: "register.errAge",
  consent: "register.errConsent",
};

// in a new tab, so the form keeps what was typed
const LINKS: Elems = {
  terms: (words) => (
    <a href={PATHS.terms} target="_blank" rel="noopener">
      {words}
    </a>
  ),
  privacy: (words) => (
    <a href={PATHS.privacy} target="_blank" rel="noopener">
      {words}
    </a>
  ),
};

export function RegisterForm({ busy, status, onSubmit }: { busy: boolean; status: { bad: boolean; key: Key } | null; onSubmit(r: Registration): void }) {
  const [betaKey, setBetaKey] = useState("");
  const [username, setUsername] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [password, setPassword] = useState("");
  const [again, setAgain] = useState("");
  const [age, setAge] = useState(false);
  const [consent, setConsent] = useState(false);
  const [problem, setProblem] = useState<Problem | null>(null);
  // counts the form's complaints, so the same one focuses its field again
  const [complaints, setComplaints] = useState(0);
  const keyRef = useRef<HTMLInputElement>(null);
  const nameRef = useRef<HTMLInputElement>(null);
  const displayRef = useRef<HTMLInputElement>(null);
  const ageRef = useRef<HTMLInputElement>(null);
  const consentRef = useRef<HTMLInputElement>(null);

  const submit = (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    if (busy) return;
    const found: Problem | null =
      betaKey.trim() === "" || betaKey.trim().length > 64
        ? "key"
        : !validName(username)
          ? "username"
          : !validDisplayName(displayName.trim())
            ? "displayName"
            : reservedDisplayName(displayName.trim())
              ? "reserved"
              : !acceptable(password)
                ? "password"
                : password !== again
                  ? "again"
                  : !age
                    ? "age"
                    : !consent
                      ? "consent"
                      : null;
    setProblem(found);
    if (found !== null) {
      setComplaints((n) => n + 1);
      if (found === "key") keyRef.current?.focus();
      else if (found === "username") nameRef.current?.focus();
      else if (found === "displayName" || found === "reserved") displayRef.current?.focus();
      else if (found === "age") ageRef.current?.focus();
      else if (found === "consent") consentRef.current?.focus();
      return;
    }
    onSubmit({ betaKey: betaKey.trim(), username: canonicalName(username), displayName: displayName.trim(), password });
  };

  const invalid = (p: Problem) => (problem === p ? true : undefined);
  const shown = problem !== null ? { bad: true, key: SAY[problem] } : status;
  return (
    <form className="account-form" noValidate onSubmit={submit} aria-busy={busy}>
      <div className="field">
        <label htmlFor="r-key">
          <T id="register.betaKey" />
        </label>
        <input
          ref={keyRef}
          id="r-key"
          type="text"
          autoComplete="off"
          autoCapitalize="characters"
          spellCheck={false}
          maxLength={64}
          value={betaKey}
          aria-invalid={invalid("key")}
          aria-describedby="r-key-hint"
          onChange={(e) => {
            setBetaKey(e.target.value);
            if (problem === "key") setProblem(null);
          }}
        />
        <p id="r-key-hint" className="field-hint">
          <T id="register.betaKeyHint" />
        </p>
      </div>
      <div className="field">
        <label htmlFor="r-name">
          <T id="acct.username" />
        </label>
        <input
          ref={nameRef}
          id="r-name"
          type="text"
          autoComplete="username"
          autoCapitalize="none"
          spellCheck={false}
          maxLength={24}
          value={username}
          aria-invalid={invalid("username")}
          aria-describedby="r-name-rules"
          onChange={(e) => {
            setUsername(e.target.value);
            if (problem === "username") setProblem(null);
          }}
        />
        <p id="r-name-rules" className="field-hint">
          <T id="register.usernameRules" />
        </p>
      </div>
      <div className="field">
        <label htmlFor="r-display">
          <T id="register.displayName" />
        </label>
        <input
          ref={displayRef}
          id="r-display"
          type="text"
          autoComplete="off"
          autoCapitalize="none"
          spellCheck={false}
          maxLength={16}
          value={displayName}
          aria-invalid={problem === "displayName" || problem === "reserved" ? true : undefined}
          aria-describedby="r-display-rules"
          onChange={(e) => {
            setDisplayName(e.target.value);
            if (problem === "displayName" || problem === "reserved") setProblem(null);
          }}
        />
        <p id="r-display-rules" className="field-hint">
          <T id="register.displayNameRules" />
        </p>
      </div>
      <PasswordFields
        password={password}
        again={again}
        username={username}
        invalid={problem === "password" || problem === "again" ? { field: problem, n: complaints } : null}
        onPassword={(value) => {
          setPassword(value);
          if (problem === "password") setProblem(null);
        }}
        onAgain={(value) => {
          setAgain(value);
          if (problem === "again") setProblem(null);
        }}
      />
      <p className="field">
        <label className="check-label">
          <input ref={ageRef} type="checkbox" checked={age} aria-invalid={invalid("age")} onChange={(e) => setAge(e.target.checked)} />
          <span>
            <T id="register.age" />
          </span>
        </label>
      </p>
      <p className="field">
        <label className="check-label">
          <input ref={consentRef} type="checkbox" checked={consent} aria-invalid={invalid("consent")} onChange={(e) => setConsent(e.target.checked)} />
          <span>
            <T id="register.consent" elems={LINKS} />
          </span>
        </label>
      </p>
      <div className="form-row">
        <SubmitButton busy={busy} label="register.submit" busyLabel="register.submitting" />
        <output id="r-status" className={`form-status${shown?.bad ? " bad" : ""}`}>
          {shown !== null && <T id={shown.key} args={{ max: MAX_BYTES }} />}
        </output>
      </div>
    </form>
  );
}
