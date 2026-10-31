// "Confirm with your password": a change that hands out a credential or acts
// as an administrator needs a password sign-in less than 30 minutes old, so
// the page signs in again (the same OPAQUE sign-in as /signin, in its
// Worker) before it tries. The new sign-in replaces this browser's session.

import { useEffect, useId, useRef, useState, type FormEvent } from "react";

import type { AccountSummary } from "../account/api";
import { FAILURE } from "../account/errors";
import { BusyButton } from "../account/Busy";
import { PasswordInput } from "../account/PasswordInput";
import { FlowError, handleOf, signIn } from "../account/flows";
import { useAccount } from "../account/session";
import { Modal } from "../components/Modal";
import { T, t, useLang, type Key } from "../i18n";

export function Reauth({ summary, onDone, onCancel }: { summary: AccountSummary; onDone(summary: AccountSummary): void; onCancel(): void }) {
  const lang = useLang();
  const heading = useId();
  const { api, crypto, signedIn } = useAccount();
  const [password, setPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState<{ bad: boolean; key: Key } | null>(null);
  // the field is wrong only when it is empty or the password was refused, not when Central is away
  const [wrong, setWrong] = useState(false);
  const field = useRef<HTMLInputElement>(null);
  useEffect(() => field.current?.focus(), []);

  const submit = (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    if (busy) return;
    if (password === "") {
      setStatus({ bad: true, key: "signin.errPassword" });
      setWrong(true);
      field.current?.focus();
      return;
    }
    setBusy(true);
    setWrong(false);
    setStatus({ bad: false, key: "signin.progress" });
    signIn({ username: summary.username, password }, { api, crypto }).then(
      ({ session, summary: now }) => {
        setPassword("");
        signedIn({ handle: handleOf(now), expiresAt: session.expiresAt });
        if (now === null) {
          // Central signed in but could not say who: read the account again
          api.accountSummary().then(onDone, () => {
            setBusy(false);
            setStatus({ bad: true, key: "acct.errUnreachable" });
          });
        } else {
          onDone(now);
        }
      },
      (error: unknown) => {
        setBusy(false);
        setStatus({ bad: true, key: FAILURE[error instanceof FlowError ? error.kind : "unreachable"] });
        if (error instanceof FlowError && error.kind === "wrongLogin") {
          setWrong(true);
          field.current?.select();
        }
      },
    );
  };

  return (
    <Modal className="app-dialog" labelledBy={heading} onClose={onCancel}>
      <form noValidate onSubmit={submit} aria-busy={busy}>
        <h2 id={heading} className="app-dialog-title">
          <T id="app.reauthTitle" />
        </h2>
        <p className="app-dialog-body">
          <T id="app.reauthWhy" />
        </p>
        <div className="field">
          <label htmlFor={`${heading}-password`}>
            <T id="acct.password" />
          </label>
          <PasswordInput
            inputRef={field}
            id={`${heading}-password`}
            autoComplete="current-password"
            value={password}
            onChange={(value) => {
              setPassword(value);
              setWrong(false);
            }}
            invalid={wrong}
            {...(wrong ? { describedBy: `${heading}-status` } : {})}
          />
        </div>
        <output id={`${heading}-status`} className={`form-status${status?.bad === true ? " bad" : ""}`}>
          {status === null ? "" : t(lang, status.key)}
        </output>
        <div className="app-dialog-actions">
          <button type="button" className="button button-ghost" onClick={onCancel}>
            <T id="app.cancel" />
          </button>
          <BusyButton busy={busy} type="submit">
            <T id="app.reauthGo" />
          </BusyButton>
        </div>
      </form>
    </Modal>
  );
}
