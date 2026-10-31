// Making a gateway token: a name, what it allows, how long it lives; then,
// once, the token itself with where it goes. Minting needs a password
// sign-in less than 30 minutes old, so a stale one signs in again first.

import { useEffect, useId, useRef, useState, type FormEvent } from "react";

import { Reauth } from "../app/Reauth";
import { SecretBox } from "../app/SecretBox";
import { day } from "../app/format";
import { fresh } from "../app/useSummary";
import { Modal } from "../components/Modal";
import { T, t, useLang, type Key } from "../i18n";
import { ApiError, type AccountSummary } from "./api";
import { BusyButton } from "./Busy";
import { shownName, validTokenName, type CreatedToken, type MyGateway, type OwnerApi, type Scopes } from "./owner-api";

const LIFETIMES = [30, 90, 180, 365] as const;

type Step = { at: "form" } | { at: "password" } | { at: "token"; token: CreatedToken };

export function TokenDialog({
  gateway,
  owner,
  summary,
  onSummary,
  onClose,
}: {
  gateway: MyGateway;
  owner: OwnerApi;
  summary: AccountSummary;
  onSummary(summary: AccountSummary): void;
  onClose(made: boolean): void;
}) {
  const lang = useLang();
  const heading = useId();
  const [name, setName] = useState("");
  const [scopes, setScopes] = useState<Scopes>({ sso: true, listing: true });
  const [days, setDays] = useState<number>(90);
  const [step, setStep] = useState<Step>({ at: "form" });
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState<Key | null>(null);
  const field = useRef<HTMLInputElement>(null);
  const scopeBox = useRef<HTMLFieldSetElement>(null);
  useEffect(() => field.current?.focus(), []);

  const create = (now: AccountSummary) => {
    setBusy(true);
    setStatus(null);
    owner.createToken(gateway.id, name.trim(), scopes, days).then(
      (token) => {
        setBusy(false);
        setStep({ at: "token", token });
      },
      (error: unknown) => {
        setBusy(false);
        const kind = error instanceof ApiError ? error.kind : "network";
        // a sign-in that went stale on the way is asked for once more
        if (kind === "rejected" && !fresh(now)) setStep({ at: "password" });
        else setStatus(kind === "busy" ? "tok.errBusy" : kind === "rejected" ? "tok.errRefused" : "acct.errUnreachable");
      },
    );
  };

  const submit = (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    if (busy) return;
    if (!validTokenName(name.trim()) || !shownName(name.trim())) {
      setStatus("tok.errName");
      field.current?.focus();
      return;
    }
    if (!scopes.sso && !scopes.listing) {
      setStatus("tok.errScope");
      scopeBox.current?.querySelector("input")?.focus();
      return;
    }
    if (fresh(summary)) create(summary);
    else setStep({ at: "password" });
  };

  if (step.at === "password") {
    return (
      <Reauth
        summary={summary}
        onDone={(now) => {
          onSummary(now);
          setStep({ at: "form" });
          create(now);
        }}
        onCancel={() => setStep({ at: "form" })}
      />
    );
  }

  if (step.at === "token") {
    return (
      // its own dialog (the key), so the screen reader hears the new title and focus starts inside it
      <Modal key="token" className="app-dialog app-dialog-wide" labelledBy={heading} onClose={() => {}}>
        <h2 id={heading} className="app-dialog-title">
          <T id="tok.madeTitle" />
        </h2>
        <p className="app-dialog-body">
          <T id="tok.madeText" args={{ gateway: gateway.metadata.name, date: day(lang, step.token.expiresAt) }} />
        </p>
        <SecretBox secret={step.token.text} label={t(lang, "tok.label")} />
        <ol className="steps-small">
          <li>
            <T id="tok.where1" />
          </li>
          <li>
            <T id="tok.where2" />
          </li>
          <li>
            <T id="tok.where3" />
          </li>
        </ol>
        <div className="app-dialog-actions">
          <button type="button" className="button" onClick={() => onClose(true)}>
            <T id="tok.kept" />
          </button>
        </div>
      </Modal>
    );
  }

  return (
    <Modal key="form" className="app-dialog" labelledBy={heading} onClose={() => onClose(false)}>
      <form noValidate onSubmit={submit} aria-busy={busy}>
        <h2 id={heading} className="app-dialog-title">
          <T id="tok.title" args={{ gateway: gateway.metadata.name }} />
        </h2>
        <div className="field">
          <label htmlFor={`${heading}-name`}>
            <T id="tok.name" />
          </label>
          <input
            ref={field}
            id={`${heading}-name`}
            maxLength={64}
            value={name}
            onChange={(e) => {
              setName(e.target.value);
              if (status === "tok.errName") setStatus(null);
            }}
            autoComplete="off"
            aria-invalid={status === "tok.errName" ? true : undefined}
            aria-describedby={status === "tok.errName" ? `${heading}-hint ${heading}-status` : `${heading}-hint`}
          />
          <p id={`${heading}-hint`} className="field-hint">
            <T id="tok.nameHint" />
          </p>
        </div>
        <fieldset ref={scopeBox} className="field" aria-describedby={status === "tok.errScope" ? `${heading}-status` : undefined}>
          <legend>
            <T id="tok.scopes" />
          </legend>
          <label className="check-label">
            <input type="checkbox" checked={scopes.sso} onChange={(e) => setScopes({ ...scopes, sso: e.target.checked })} />
            <span>
              <T id="tok.sso" />
            </span>
          </label>
          <label className="check-label">
            <input type="checkbox" checked={scopes.listing} onChange={(e) => setScopes({ ...scopes, listing: e.target.checked })} />
            <span>
              <T id="tok.listing" />
            </span>
          </label>
        </fieldset>
        <div className="field">
          <label htmlFor={`${heading}-days`}>
            <T id="tok.lifetime" />
          </label>
          <select id={`${heading}-days`} value={days} onChange={(e) => setDays(Number(e.target.value))}>
            {LIFETIMES.map((d) => (
              <option key={d} value={d}>
                {t(lang, "tok.days", { n: d })}
              </option>
            ))}
          </select>
        </div>
        <output id={`${heading}-status`} className="form-status bad">
          {status === null ? "" : t(lang, status)}
        </output>
        <div className="app-dialog-actions">
          <button type="button" className="button button-ghost" onClick={() => onClose(false)}>
            <T id="app.cancel" />
          </button>
          <BusyButton busy={busy} type="submit">
            <T id="tok.create" />
          </BusyButton>
        </div>
      </form>
    </Modal>
  );
}
