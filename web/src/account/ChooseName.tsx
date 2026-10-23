// "Choose your display name" (docs/web-accounts.md §"Display name and tag"):
// for an account that has a tag but no name yet (one registered before
// display names, or moved from the official gateway with a name that did
// not keep the rule). It is chosen once (`SetDisplayName`); renaming comes
// later. The page says early what Central would refuse; Central decides.

import { useRef, useState, type FormEvent } from "react";

import { T, type Key } from "../i18n";
import { SubmitButton } from "./Busy";
import { FAILURE } from "./errors";
import { FlowError, chooseName } from "./flows";
import { reservedDisplayName, tagText, validDisplayName } from "./names";
import { useAccount } from "./session";

export function ChooseName({ tag, onChosen, onStale }: { tag: number; onChosen(name: string): void; onStale(): void }) {
  const { api } = useAccount();
  const [name, setName] = useState("");
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState<{ bad: boolean; key: Key } | null>(null);
  const field = useRef<HTMLInputElement>(null);

  const submit = (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    if (busy) return;
    const chosen = name.trim();
    const problem: Key | null = !validDisplayName(chosen)
      ? "register.errDisplayName"
      : reservedDisplayName(chosen)
        ? "register.errDisplayNameReserved"
        : null;
    if (problem !== null) {
      setStatus({ bad: true, key: problem });
      field.current?.focus();
      return;
    }
    setBusy(true);
    setStatus({ bad: false, key: "account.chooseNameSaving" });
    chooseName(chosen, { api }).then(
      () => onChosen(chosen),
      (error: unknown) => {
        setBusy(false);
        const kind = error instanceof FlowError ? error.kind : "unreachable";
        setStatus({ bad: true, key: FAILURE[kind] });
        // a name chosen elsewhere meanwhile: the page reads it
        if (kind === "nameTaken") onStale();
      },
    );
  };

  return (
    <section className="choose-name" aria-labelledby="choose-name-title">
      <h2 id="choose-name-title">
        <T id="account.chooseName" />
      </h2>
      <form className="account-form" noValidate onSubmit={submit} aria-busy={busy}>
        <p>
          <T id="account.chooseNameLead" args={{ tag: tagText(tag) }} />
        </p>
        <div className="field">
          <label htmlFor="a-display">
            <T id="register.displayName" />
          </label>
          <input
            ref={field}
            id="a-display"
            type="text"
            autoComplete="off"
            autoCapitalize="none"
            spellCheck={false}
            maxLength={16}
            value={name}
            aria-invalid={status?.bad ? true : undefined}
            onChange={(e) => {
              setName(e.target.value);
              if (status?.bad) setStatus(null);
            }}
          />
        </div>
        <div className="form-row">
          <SubmitButton busy={busy} label="account.chooseNameSubmit" busyLabel="account.chooseNameSaving" />
          <output className={`form-status${status?.bad ? " bad" : ""}`}>{status !== null && <T id={status.key} />}</output>
        </div>
      </form>
    </section>
  );
}
