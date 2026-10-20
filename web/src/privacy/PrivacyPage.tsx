// The website's own privacy notice (Art. 13 GDPR), drawn from the old
// site's (Baylee's web/site/datenschutz.html, written from the code on
// 13.10.2026) and from what this app does. A draft for the owner's lawyer to
// check before it is relied on (Baylee's docs/legal.md §4). It must change
// with the site: a new host in the CSP, a cookie, an access log or a new form
// means a new sentence here (README.md §"Privacy"). Never put the operator's
// private details here: the Impressum names the controller.

import { GITHUB } from "../config";
import { T, useLang, useTitle } from "../i18n";
import "../styles/legal.css";
import { PERSONAL } from "./rights";
import { SERVING } from "./serving";

const PARTS = [...SERVING, ...PERSONAL];

export default function Privacy() {
  const lang = useLang();
  const de = lang === "de";
  useTitle("Privacy – Baylee", "Datenschutz – Baylee");
  return (
    <header className="page-head legal">
      <div className="section-inner">
        <p className="eyebrow">
          <T en="Privacy" de="Datenschutz" />
        </p>
        <h1>
          <T en="Privacy notice for this website" de="Datenschutzerklärung dieser Website" />
        </h1>
        <p className="section-lead">
          <T
            en={
              <>
                This page says what happens with your data when you visit this website. The game itself (the client and the
                online gateway) keeps its own records, listed in full in{" "}
                <a href={`${GITHUB}/blob/main/docs/privacy.md`} rel="noopener">
                  docs/privacy.md
                </a>
                .
              </>
            }
            de={
              <>
                Diese Seite sagt, was mit deinen Daten geschieht, wenn du diese Website besuchst. Das Spiel selbst (Client und
                Online-Gateway) führt eigene Aufzeichnungen; die vollständige Liste steht in{" "}
                <a href={`${GITHUB}/blob/main/docs/privacy.md`} rel="noopener">
                  docs/privacy.md
                </a>
                .
              </>
            }
          />
        </p>
        <nav className="toc" aria-label={de ? "Inhalt" : "Contents"}>
          <ol>
            {PARTS.map((p) => (
              <li key={p.id}>
                <a href={`#${p.id}`}>{de ? p.title[1] : p.title[0]}</a>
              </li>
            ))}
          </ol>
        </nav>
        {PARTS.map((p) => (
          <section key={p.id} id={p.id} className="legal-part" aria-labelledby={`${p.id}-title`}>
            <h2 id={`${p.id}-title`}>{de ? p.title[1] : p.title[0]}</h2>
            {p.id === "rights" ? de ? p.body[1] : p.body[0] : <p>{de ? p.body[1] : p.body[0]}</p>}
            {p.basis && <p className="small">{de ? p.basis[1] : p.basis[0]}</p>}
          </section>
        ))}
        <p className="small legal-date">
          <T en="As of 20 October 2026." de="Stand: 20. Oktober 2026." />
        </p>
      </div>
    </header>
  );
}
