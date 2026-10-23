// `/register` (docs/web-accounts.md §"Flows"): the form; then Central
// reserves the name for the beta key, the crypto Worker enrols the password
// and wraps a new master key under it and under a new recovery code; then
// the code, shown once; then a sign-in (started at once, so the password is
// let go), the terms where they are in force, and the account page. If the
// sign-in fails, the account still exists: the page sends the person to
// `/signin` after the code.

import { useState } from "react";

import { T, useTitle, type Elems, type Key } from "../i18n";
import { Link, PATHS, navigate } from "../router";
import "../styles/account.css";
import { FAILURE } from "./errors";
import { FlowError, register, signIn, type Progress } from "./flows";
import { RecoveryStep } from "./RecoveryStep";
import { RegisterForm, type Registration } from "./RegisterForm";
import { useAccount } from "./session";
import { TermsStep } from "./TermsStep";

const LINKS: Elems = { signin: (words) => <Link to={PATHS.signIn}>{words}</Link> };

const SAYS: Readonly<Record<Progress, Key>> = {
  key: "register.progressKey",
  securing: "register.progressSecuring",
  saving: "register.progressSaving",
  signingIn: "signin.progress",
};

type Stage =
  | { at: "form" }
  | { at: "code"; code: string; name: string; signedIn: Promise<boolean> }
  | { at: "terms" };

export default function RegisterPage() {
  useTitle("title.register");
  const { api, crypto, signedIn } = useAccount();
  const [stage, setStage] = useState<Stage>({ at: "form" });
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState<{ bad: boolean; key: Key } | null>(null);
  const [finishing, setFinishing] = useState(false);

  const submit = (r: Registration) => {
    setBusy(true);
    setStatus({ bad: false, key: SAYS.key });
    register(r, { api, crypto }, (p) => setStatus({ bad: false, key: SAYS[p] })).then(
      (code) => {
        // sign in now, so the password is let go while the code is read
        const session = signIn({ username: r.username, password: r.password }, { api, crypto }).then(
          ({ session: s }) => {
            signedIn({ name: s.username, expiresAt: s.expiresAt });
            return true;
          },
          () => false,
        );
        setBusy(false);
        setStatus(null);
        setStage({ at: "code", code, name: r.username, signedIn: session });
      },
      (error: unknown) => {
        setBusy(false);
        setStatus({ bad: true, key: FAILURE[error instanceof FlowError ? error.kind : "unreachable"] });
      },
    );
  };

  const kept = () => {
    if (stage.at !== "code" || finishing) return;
    setFinishing(true);
    void stage.signedIn.then((ok) => {
      if (ok) setStage({ at: "terms" });
      // the account exists; only the sign-in after it did not work
      else navigate(`${PATHS.signIn}?registered=1`);
    });
  };

  return (
    <header className="page-head account-page">
      <div className="section-inner">
        <p className="eyebrow">
          <T id="acct.eyebrow" />
        </p>
        <h1>
          <T id="register.title" />
        </h1>
        {stage.at === "form" && (
          <>
            <p className="section-lead">
              <T id="register.lead" />
            </p>
            <div className="account-card glass">
              <RegisterForm busy={busy} status={status} onSubmit={submit} />
              <p>
                <T id="register.haveAccount" elems={LINKS} />
              </p>
            </div>
          </>
        )}
        {stage.at === "code" && (
          <>
            <RecoveryStep code={stage.code} name={stage.name} onKept={kept} />
            <output className="form-status">
              {finishing && <T id="signin.progress" />}
            </output>
          </>
        )}
        {stage.at === "terms" && <TermsStep onDone={() => navigate(PATHS.account)} />}
      </div>
    </header>
  );
}
