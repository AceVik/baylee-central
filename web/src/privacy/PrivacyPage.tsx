// The website's own privacy notice (Art. 13 GDPR), drawn from the old
// site's (Baylee's web/site/datenschutz.html, written from the code on
// 13.10.2026) and from what this app does; its words are the `privacy-…`
// messages. A draft for the owner's lawyer to check before it is relied on
// (Baylee's docs/legal.md §4). It must change with the site: a new host in
// the CSP, a cookie, an access log or a new form means a new sentence here
// (README.md §"Privacy"); the account pages brought the session cookie,
// the account data and the counters (accounts.tsx). Never put the operator's private details here: the
// Impressum names the controller.

import { FEEDBACK_ORIGIN, GITHUB } from "../config";
import { T, t, useLang, useTitle, type Elems } from "../i18n";
import { Link, PATHS } from "../router";
import "../styles/legal.css";
import { ACCOUNTS } from "./accounts";
import { PERSONAL } from "./rights";
import { SERVING } from "./serving";

const PARTS = [...SERVING, ...ACCOUNTS, ...PERSONAL];

/** What the feedback service is called in the contact form's part. */
const ARGS = { host: new URL(FEEDBACK_ORIGIN).host };

/** The links the notice's markup names. */
const LINKS: Elems = {
  docs: (words) => (
    <a href={`${GITHUB}/blob/main/docs/privacy.md`} rel="noopener">
      {words}
    </a>
  ),
  impressum: (words) => <Link to={PATHS.impressum}>{words}</Link>,
  contact: (words) => <Link to={PATHS.contact}>{words}</Link>,
  directory: (words) => <Link to={PATHS.gateways}>{words}</Link>,
  statement: (words) => (
    <a href="https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement" rel="noopener">
      {words}
    </a>
  ),
  policy: (words) => (
    <a href="https://scryfall.com/docs/privacy" rel="noopener">
      {words}
    </a>
  ),
  ldi: (words) => (
    <a href="https://www.ldi.nrw.de/" rel="noopener">
      {words}
    </a>
  ),
};

export default function Privacy() {
  const lang = useLang();
  useTitle("title.privacy");
  return (
    <header className="page-head legal">
      <div className="section-inner">
        <p className="eyebrow">
          <T id="privacy.eyebrow" />
        </p>
        <h1>
          <T id="privacy.title" />
        </h1>
        <p className="section-lead">
          <T id="privacy.lead" elems={LINKS} />
        </p>
        <nav className="toc" aria-label={t(lang, "privacy.contents")}>
          <ol>
            {PARTS.map((p) => (
              <li key={p.id}>
                <a href={`#${p.id}`}>{t(lang, p.title)}</a>
              </li>
            ))}
          </ol>
        </nav>
        {PARTS.map((p) => (
          <section key={p.id} id={p.id} className="legal-part" aria-labelledby={`${p.id}-title`}>
            <h2 id={`${p.id}-title`}>{t(lang, p.title)}</h2>
            {p.body.map((key) => (
              <p key={key}>
                <T id={key} elems={LINKS} args={ARGS} />
              </p>
            ))}
            {p.basis && (
              <p className="small">
                <T id={p.basis} />
              </p>
            )}
          </section>
        ))}
        <p className="small legal-date">
          <T id="privacy.date" />
        </p>
      </div>
    </header>
  );
}
