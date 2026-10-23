// `/account`: whose account this browser is signed in to (the handle other
// players see, and the private sign-in name small beneath it), until when,
// where the account came from and whether it has a recovery code
// (`AccountSummary`), and signing out. An account without a display name
// chooses one first (ChooseName); a moved one whose name and tag the
// official gateway has not carried yet is told where they come from. An
// account moved here has no code; "Make a recovery code" signs in again (the
// master key exists only inside a sign-in's Worker) and shows the code the
// sign-in made, stored once kept (NewCodeStep). Changing the password,
// e-mail and deletion come with M1; until then deletion is by request to the
// operator.

import { useEffect, useRef, useState, type FormEvent } from "react";

import { T, localeOf, useLang, useTitle, type Elems, type Key } from "../i18n";
import { Link, PATHS } from "../router";
import "../styles/account.css";
import type { AccountSummary } from "./api";
import { SubmitButton } from "./Busy";
import { ChooseName } from "./ChooseName";
import { FAILURE } from "./errors";
import { FlowError, handleOf, signIn, type NewCode } from "./flows";
import { handle } from "./names";
import { NewCodeStep } from "./NewCodeStep";
import { useAccount } from "./session";
import { TermsStep } from "./TermsStep";

const LINKS: Elems = {
  signin: (words) => <Link to={PATHS.signIn}>{words}</Link>,
  register: (words) => <Link to={PATHS.register}>{words}</Link>,
  contact: (words) => <Link to={PATHS.contact}>{words}</Link>,
  download: (words) => <Link to={PATHS.download}>{words}</Link>,
};

/** Making a recovery code: the password again, then the code. */
type Making = { at: "password" } | { at: "code"; code: NewCode; name: string };

/** "To make the code, sign in again": the password, then a sign-in that makes the code. */
function SignInAgain({ name, onSignedIn, onCancel }: { name: string; onSignedIn(code: NewCode | null, summary: AccountSummary | null): void; onCancel(): void }) {
  const { api, crypto, signedIn } = useAccount();
  const [password, setPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState<{ bad: boolean; key: Key } | null>(null);
  const field = useRef<HTMLInputElement>(null);
  useEffect(() => field.current?.focus(), []);

  const submit = (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    if (busy) return;
    if (password === "") {
      setStatus({ bad: true, key: "signin.errPassword" });
      field.current?.focus();
      return;
    }
    setBusy(true);
    setStatus({ bad: false, key: "signin.progress" });
    signIn({ username: name, password }, { api, crypto }).then(
      ({ session, summary, code }) => {
        setPassword("");
        // the new sign-in replaced this browser's session
        signedIn({ handle: handleOf(summary), expiresAt: session.expiresAt });
        onSignedIn(code, summary);
      },
      (error: unknown) => {
        setBusy(false);
        setStatus({ bad: true, key: FAILURE[error instanceof FlowError ? error.kind : "unreachable"] });
        if (error instanceof FlowError && error.kind === "wrongLogin") field.current?.select();
      },
    );
  };

  return (
    <form className="account-form" noValidate onSubmit={submit} aria-busy={busy}>
      <p>
        <T id="account.recoverySignIn" />
      </p>
      <div className="field">
        <label htmlFor="a-password">
          <T id="acct.password" />
        </label>
        <input ref={field} id="a-password" type="password" autoComplete="current-password" value={password} onChange={(e) => setPassword(e.target.value)} />
      </div>
      <div className="form-row">
        <SubmitButton busy={busy} label="signin.submit" busyLabel="signin.submitting" />
        <button type="button" className="button button-ghost" aria-disabled={busy ? true : undefined} onClick={busy ? undefined : onCancel}>
          <T id="account.recoveryCancel" />
        </button>
        <output className={`form-status${status?.bad ? " bad" : ""}`}>{status !== null && <T id={status.key} />}</output>
      </div>
    </form>
  );
}

/** Where the account came from, and its recovery code. */
function Recovery({ summary, offer, onMake }: { summary: AccountSummary; offer: boolean; onMake(): void }) {
  if (!summary.recovery)
    return (
      <>
        <p className="small">
          <T id="account.noRecovery" />
        </p>
        {offer && (
          <p>
            <button type="button" className="button" onClick={onMake}>
              <T id="account.makeRecovery" />
            </button>
          </p>
        )}
      </>
    );
  return (
    <>
      <p className="small">
        <T id={summary.source === "legacy" ? "account.sourceLegacy" : "account.sourceRegistered"} />
      </p>
      <p className="small">
        <T id="account.recovery" />
      </p>
    </>
  );
}

export default function AccountPage() {
  useTitle("title.account");
  const lang = useLang();
  const { account, api, signOut, signedIn } = useAccount();
  // when Central could not end the session: the time it ends by itself
  const [unconfirmed, setUnconfirmed] = useState<string | null>(null);
  const [summary, setSummary] = useState<AccountSummary | null>(null);
  const [unread, setUnread] = useState(false);
  // a code is offered once the terms are settled: storing one needs them accepted
  const [settled, setSettled] = useState(false);
  const [making, setMaking] = useState<Making | null>(null);
  const time = (seconds: number) => new Date(seconds * 1000).toLocaleTimeString(localeOf(lang), { hour: "2-digit", minute: "2-digit" });
  const until = account === null ? "" : time(account.expiresAt);
  const leave = () => {
    if (account === null) return;
    const ends = time(account.expiresAt);
    void signOut().then((ended) => setUnconfirmed(ended ? null : ends));
  };

  const present = account !== null;
  useEffect(() => {
    if (!present) return;
    let live = true;
    api.accountSummary().then(
      (s) => {
        if (live) setSummary(s);
      },
      () => {
        if (live) setUnread(true);
      },
    );
    return () => {
      live = false;
    };
  }, [api, present]);

  // the header and this page show Central's handle as soon as it changes
  const shown = handleOf(summary) ?? account?.handle ?? null;
  // a name chosen elsewhere meanwhile refused this page's: read the account again
  const reread = () => {
    api.accountSummary().then(setSummary, () => setUnread(true));
  };
  const named = (name: string) => {
    if (summary === null || summary.tag === null || account === null) return;
    setSummary({ ...summary, displayName: name });
    signedIn({ handle: handle(name, summary.tag), expiresAt: account.expiresAt });
  };
  useEffect(() => {
    if (account === null || summary === null) return;
    const drawn = handleOf(summary);
    if (drawn !== null && drawn !== account.handle) signedIn({ handle: drawn, expiresAt: account.expiresAt });
  }, [account, summary, signedIn]);

  const signedInAgain = (code: NewCode | null, now: AccountSummary | null) => {
    if (now !== null) setSummary(now);
    // an account that has a code by now (made elsewhere) shows that it has one
    setMaking(code === null ? null : { at: "code", code, name: now?.username ?? summary?.username ?? "" });
  };
  const stored = (done: boolean) => {
    setMaking(null);
    if (done && summary !== null) setSummary({ ...summary, recovery: true });
  };

  return (
    <header className="page-head account-page">
      <div className="section-inner">
        <p className="eyebrow">
          <T id="acct.eyebrow" />
        </p>
        <h1>
          <T id="account.title" />
        </h1>
        {unconfirmed !== null && (
          <p className="account-note bad" role="alert">
            <T id="account.signOutUnconfirmed" args={{ time: unconfirmed }} />
          </p>
        )}
        {account === null ? (
          <p className="account-note">
            <T id="account.notSignedIn" elems={LINKS} />
          </p>
        ) : (
          <div className="account-card glass">
            {/* first, once the terms are settled: choosing a name needs them accepted */}
            {settled && summary !== null && summary.displayName === null && summary.tag !== null && (
              <ChooseName tag={summary.tag} onChosen={named} onStale={reread} />
            )}
            <p className="account-name">
              {shown === null ? (
                <T id="account.signedIn" />
              ) : (
                <>
                  <span className="small">
                    <T id="account.signedInAs" />
                  </span>{" "}
                  <bdi translate="no">{shown}</bdi>
                </>
              )}
            </p>
            {summary !== null && (
              <p className="small account-signin-name">
                <T id="account.signInName" args={{ name: summary.username }} />
              </p>
            )}
            {summary !== null && summary.source === "legacy" && summary.tag === null && (
              <p className="small">
                <T id="account.nameFromGateway" />
              </p>
            )}
            <p>
              <T id="account.session" args={{ time: until }} />
            </p>
            <TermsStep onDone={() => setSettled(true)} />
            <p>
              <T id="account.client" elems={LINKS} />
            </p>
            {making?.at === "password" && summary !== null ? (
              <SignInAgain name={summary.username} onSignedIn={signedInAgain} onCancel={() => setMaking(null)} />
            ) : making?.at === "code" ? (
              <NewCodeStep code={making.code} name={making.name} onDone={stored} />
            ) : summary !== null ? (
              <Recovery summary={summary} offer={settled} onMake={() => setMaking({ at: "password" })} />
            ) : (
              unread && (
                <p className="small">
                  <T id="acct.errUnreachable" />
                </p>
              )
            )}
            <p className="small">
              <T id="account.later" elems={LINKS} />
            </p>
            <button type="button" className="button button-ghost" onClick={leave}>
              <T id="account.signOut" />
            </button>
          </div>
        )}
      </div>
    </header>
  );
}
