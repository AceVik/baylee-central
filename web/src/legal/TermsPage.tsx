// Baylee Central's terms of use, from `legal/terms.{de,en}.md` (German
// binding): the same text the service compiles in, read at build time; a
// language without its own file reads the English one. While its version is
// `draft` it is not in force (`docs/terms.md`), and the page says so above it.

import { useLang, useTitle, T } from "../i18n";
import "../styles/legal.css";
import { LegalText } from "./LegalText";
import { DRAFT, readDocument } from "./markdown";
import termsDe from "../../../legal/terms.de.md?raw";
import termsEn from "../../../legal/terms.en.md?raw";

export const TERMS = { de: readDocument(termsDe), en: readDocument(termsEn) };

export default function TermsPage() {
  const lang = useLang();
  useTitle("title.terms");
  const shown = lang === "de" ? TERMS.de : TERMS.en;
  const draft = shown.version === DRAFT || shown.version === null;
  return (
    <header className="page-head legal">
      <div className="section-inner">
        <p className="eyebrow">
          <T id="terms.eyebrow" />
        </p>
        {draft && (
          <p className="legal-draft" role="note">
            <T id="terms.draft" />
          </p>
        )}
        <LegalText text={shown.markdown} />
        {!draft && (
          <p className="small legal-date">
            <T id="terms.version" args={{ version: shown.version ?? "" }} />
            {shown.updated && (
              <>
                {" "}
                <T id="terms.updated" args={{ date: shown.updated }} />
              </>
            )}
          </p>
        )}
      </div>
    </header>
  );
}
