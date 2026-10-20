// Baylee Central's terms of use, from `legal/terms.{de,en}.md` (German
// binding): the same text the service compiles in, read at build time. While
// its version is `draft` it is not in force (`docs/terms.md`), and the page
// says so above it.

import { useLang, useTitle, T } from "../i18n";
import "../styles/legal.css";
import { LegalText } from "./LegalText";
import { DRAFT, readDocument } from "./markdown";
import termsDe from "../../../legal/terms.de.md?raw";
import termsEn from "../../../legal/terms.en.md?raw";

export const TERMS = { de: readDocument(termsDe), en: readDocument(termsEn) };

export default function TermsPage() {
  const lang = useLang();
  useTitle("Terms of use – Baylee", "Nutzungsbedingungen – Baylee");
  const shown = lang === "de" ? TERMS.de : TERMS.en;
  const draft = shown.version === DRAFT || shown.version === null;
  return (
    <header className="page-head legal">
      <div className="section-inner">
        <p className="eyebrow">
          <T en="Terms of use" de="Nutzungsbedingungen" />
        </p>
        {draft && (
          <p className="legal-draft" role="note">
            <T
              en="Draft: these terms are not in force yet. They take effect only once published, and you will be asked to accept them."
              de="Entwurf: Diese Bedingungen gelten noch nicht. Sie treten erst mit ihrer Veröffentlichung in Kraft, und du wirst gefragt, ob du sie annimmst."
            />
          </p>
        )}
        <LegalText text={shown.markdown} />
        {!draft && (
          <p className="small legal-date">
            <T en={`Version ${shown.version}.`} de={`Fassung ${shown.version}.`} />
            {shown.updated && <> {lang === "de" ? `Stand: ${shown.updated}.` : `Updated ${shown.updated}.`}</>}
          </p>
        )}
      </div>
    </header>
  );
}
