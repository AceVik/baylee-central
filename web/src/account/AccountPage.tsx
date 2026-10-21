// `/account`: whose account this browser is signed in to, until when, and
// signing out. Changing the password, recovery, e-mail and deletion come
// with M1; until then deletion is by request to the operator.

import { useState } from "react";

import { T, localeOf, useLang, useTitle, type Elems } from "../i18n";
import { Link, PATHS } from "../router";
import "../styles/account.css";
import { useAccount } from "./session";
import { TermsStep } from "./TermsStep";

const LINKS: Elems = {
  signin: (words) => <Link to={PATHS.signIn}>{words}</Link>,
  register: (words) => <Link to={PATHS.register}>{words}</Link>,
  contact: (words) => <Link to={PATHS.contact}>{words}</Link>,
  download: (words) => <Link to={PATHS.download}>{words}</Link>,
};

export default function AccountPage() {
  useTitle("title.account");
  const lang = useLang();
  const { account, signOut } = useAccount();
  // when Central could not end the session: the time it ends by itself
  const [unconfirmed, setUnconfirmed] = useState<string | null>(null);
  const time = (seconds: number) => new Date(seconds * 1000).toLocaleTimeString(localeOf(lang), { hour: "2-digit", minute: "2-digit" });
  const until = account === null ? "" : time(account.expiresAt);
  const leave = () => {
    if (account === null) return;
    const ends = time(account.expiresAt);
    void signOut().then((ended) => setUnconfirmed(ended ? null : ends));
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
            <p className="account-name">
              {account.name === null ? (
                <T id="account.signedIn" />
              ) : (
                <>
                  <span className="small">
                    <T id="account.signedInAs" />
                  </span>{" "}
                  <bdi>{account.name}</bdi>
                </>
              )}
            </p>
            <p>
              <T id="account.session" args={{ time: until }} />
            </p>
            <TermsStep onDone={() => undefined} />
            <p>
              <T id="account.client" elems={LINKS} />
            </p>
            <p className="small">
              <T id="account.recovery" />
            </p>
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
