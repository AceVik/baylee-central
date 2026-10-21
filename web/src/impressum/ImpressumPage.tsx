// The legal notice (§ 5 DDG). The provider's details are in no file of this
// repository and in nothing a crawler reads as text (README.md §"The
// Impressum"). Not indexed: the page says so, robots.txt disallows it, and
// the server adds X-Robots-Tag.

import { useEffect } from "react";

import { T, useTitle, type Elems } from "../i18n";
import { Link, PATHS } from "../router";
import "../styles/impressum.css";
import "../styles/legal.css";
import { CONTACT, ImpressumBox } from "./ImpressumBox";

const DATA: Elems = {
  notice: (words) => <Link to={PATHS.privacy}>{words}</Link>,
  docs: (words) => (
    <a href="https://github.com/AceVik/baylee/blob/main/docs/privacy.md" rel="noopener">
      {words}
    </a>
  ),
};

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
  useTitle("title.impressum");
  useNoIndex();
  return (
    <header className="page-head legal">
      <div className="section-inner">
        <p className="eyebrow">
          <T id="imp.eyebrow" />
        </p>
        <h1>
          <T id="imp.title" />
        </h1>
        <h2>
          <T id="imp.ddg" />
        </h2>
        <p>
          <T id="imp.private" />
        </p>
        <ImpressumBox />
        <p>
          <T id="imp.write" elems={CONTACT} />
        </p>
        <h2>
          <T id="imp.disputeTitle" />
        </h2>
        <p>
          <T id="imp.dispute" />
        </p>
        <p className="small">
          <T id="imp.data" elems={DATA} />
        </p>
      </div>
    </header>
  );
}
