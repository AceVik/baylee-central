// `/signin` (docs/web-accounts.md §"Flows"): the name and the password,
// then OPAQUE in the crypto Worker; Central sets its session cookie, the
// page remembers only the name and the end. Then the terms, where Central's
// terms in force are not yet accepted; for an account moved here without a
// recovery code, the code the sign-in made (shown, then stored once kept,
// which needs the terms accepted); and the account page.

import { useRef, useState, type FormEvent } from "react";

import { T, useTitle, type Elems, type Key } from "../i18n";
import { Link, PATHS, navigate, useLocation } from "../router";
import "../styles/account.css";
import { SubmitButton } from "./Busy";
import { FAILURE } from "./errors";
import { FlowError, signIn, type NewCode } from "./flows";
import { canonicalName } from "./names";
import { NewCodeStep } from "./NewCodeStep";
import { useAccount } from "./session";
import { TermsStep } from "./TermsStep";

/** After the sign-in: the terms, then the new code when the sign-in made one. */
type After = { at: "terms" | "code"; name: string; code: NewCode | null };

const LINKS: Elems = {
  register: (words) => <Link to={PATHS.register}>{words}</Link>,
  account: (words) => <Link to={PATHS.account}>{words}</Link>,
};

export default function SignInPage() {
  useTitle("title.signIn");
  const { account, api, crypto, signedIn } = useAccount();
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState<{ bad: boolean; key: Key } | null>(null);
  const [after, setAfter] = useState<After | null>(null);
  const registered = new URLSearchParams(useLocation().search).get("registered") === "1";
  const nameRef = useRef<HTMLInputElement>(null);
  const passwordRef = useRef<HTMLInputElement>(null);

  const submit = (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    if (busy) return;
    const name = canonicalName(username);
    if (name === "" || name.length > 64) {
      setStatus({ bad: true, key: "signin.errName" });
      nameRef.current?.focus();
      return;
    }
    if (password === "") {
      setStatus({ bad: true, key: "signin.errPassword" });
      passwordRef.current?.focus();
      return;
    }
    setBusy(true);
    setStatus({ bad: false, key: "signin.progress" });
    signIn({ username: name, password }, { api, crypto }).then(
      ({ session, code }) => {
        setPassword("");
        setBusy(false);
        setStatus(null);
        signedIn({ name: session.username, expiresAt: session.expiresAt });
        setAfter({ at: "terms", name: session.username, code });
      },
      (error: unknown) => {
        setBusy(false);
        setStatus({ bad: true, key: FAILURE[error instanceof FlowError ? error.kind : "unreachable"] });
        if (error instanceof FlowError && error.kind === "wrongLogin") passwordRef.current?.select();
      },
    );
  };

  return (
    <header className="page-head account-page">
      <div className="section-inner">
        <p className="eyebrow">
          <T id="acct.eyebrow" />
        </p>
        <h1>
          <T id="signin.title" />
        </h1>
        {after?.at === "terms" ? (
          <TermsStep onDone={() => (after.code === null ? navigate(PATHS.account) : setAfter({ ...after, at: "code" }))} />
        ) : after?.at === "code" && after.code !== null ? (
          <NewCodeStep code={after.code} name={after.name} onDone={() => navigate(PATHS.account)} />
        ) : account !== null && !busy ? (
          <p className="account-note">
            {account.name === null ? (
              <T id="signin.alreadyUnnamed" elems={LINKS} />
            ) : (
              <T id="signin.already" args={{ name: account.name }} elems={LINKS} />
            )}
          </p>
        ) : (
          <div className="account-card glass">
            {registered && (
              <p className="account-note">
                <T id="signin.registered" />
              </p>
            )}
            <form className="account-form" noValidate onSubmit={submit} aria-busy={busy}>
              <div className="field">
                <label htmlFor="s-name">
                  <T id="acct.username" />
                </label>
                <input
                  ref={nameRef}
                  id="s-name"
                  type="text"
                  autoComplete="username"
                  autoCapitalize="none"
                  spellCheck={false}
                  maxLength={64}
                  value={username}
                  onChange={(e) => setUsername(e.target.value)}
                />
              </div>
              <div className="field">
                <label htmlFor="s-password">
                  <T id="acct.password" />
                </label>
                <input
                  ref={passwordRef}
                  id="s-password"
                  type="password"
                  autoComplete="current-password"
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                />
              </div>
              <div className="form-row">
                <SubmitButton busy={busy} label="signin.submit" busyLabel="signin.submitting" />
                <output className={`form-status${status?.bad ? " bad" : ""}`}>{status !== null && <T id={status.key} />}</output>
              </div>
            </form>
            <p className="small">
              <T id="signin.legacy" />
            </p>
            <p>
              <T id="signin.noAccount" elems={LINKS} />
            </p>
          </div>
        )}
      </div>
    </header>
  );
}
