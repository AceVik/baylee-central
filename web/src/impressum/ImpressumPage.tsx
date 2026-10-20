// The legal notice (§ 5 DDG). The provider's details are in no file of this
// repository and in nothing a crawler reads as text (README.md §"The
// Impressum"). Not indexed: the page says so, robots.txt disallows it, and
// the server adds X-Robots-Tag.

import { useEffect } from "react";

import { T, useTitle } from "../i18n";
import { Link, PATHS } from "../router";
import "../styles/impressum.css";
import "../styles/legal.css";
import { ContactLink, ImpressumBox } from "./ImpressumBox";

function useNoIndex(): void {
  useEffect(() => {
    const meta = document.createElement("meta");
    meta.name = "robots";
    meta.content = "noindex, noarchive, nosnippet";
    document.head.append(meta);
    return () => meta.remove();
  }, []);
}

export default function Impressum() {
  useTitle("Legal notice – Baylee", "Impressum – Baylee");
  useNoIndex();
  return (
    <header className="page-head legal">
      <div className="section-inner">
        <p className="eyebrow">
          <T en="Legal notice" de="Impressum" />
        </p>
        <h1>
          <T en="Legal notice" de="Impressum" />
        </h1>
        <h2>
          <T en="Information pursuant to § 5 DDG" de="Angaben gemäß § 5 DDG" />
        </h2>
        <p>
          <T
            en="This website is a private, non-commercial project of a private individual. Baylee is free and open source; there is no advertising and nothing to buy."
            de="Diese Website ist ein privates, nicht kommerzielles Projekt einer Privatperson. Baylee ist kostenlos und quelloffen; es gibt keine Werbung und nichts zu kaufen."
          />
        </p>
        <ImpressumBox />
        <p>
          <T
            en={
              <>
                You can also write to me with the <ContactLink>contact form</ContactLink>.
              </>
            }
            de={
              <>
                Du erreichst mich außerdem über das <ContactLink>Kontaktformular</ContactLink>.
              </>
            }
          />
        </p>
        <h2>
          <T en="Dispute resolution" de="Streitbeilegung" />
        </h2>
        <p>
          <T
            en="This is a free, non-commercial offer of a private individual. I am neither obliged nor willing to take part in dispute resolution proceedings before a consumer arbitration board."
            de="Dies ist ein kostenloses, nicht kommerzielles Angebot einer Privatperson. Ich bin weder verpflichtet noch bereit, an Streitbeilegungsverfahren vor einer Verbraucherschlichtungsstelle teilzunehmen."
          />
        </p>
        <p className="small">
          <T en="How this site handles your data: " de="Wie diese Seite mit deinen Daten umgeht: " />
          <Link to={PATHS.privacy}>
            <T en="the privacy notice" de="die Datenschutzerklärung" />
          </Link>
          .{" "}
          <T en="What the client, the gateway and the feedback service keep about you: " de="Was Client, Gateway und Feedback-Dienst über dich speichern: " />
          <a href="https://github.com/AceVik/baylee/blob/main/docs/privacy.md" rel="noopener">
            docs/privacy.md
          </a>
          .
        </p>
      </div>
    </header>
  );
}
