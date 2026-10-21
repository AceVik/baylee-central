// The footer: the notices Wizards' Fan Content Policy and Scryfall ask for,
// word for word as on the old site (Baylee's docs/legal.md §1 and §3), and
// the links every page carries.

import { CENTRAL_SOURCE, DISCORD, GITHUB, ISSUES, RELEASES_PAGE, SITE_SOURCE } from "../config";
import { T, rich, t, tagOf, useLang, type Elems, type Lang } from "../i18n";
import { Link, PATHS } from "../router";
import { Logo } from "./Logo";

/** The policy itself, in `lang`. */
const policy = (lang: Lang): Elems => ({
  policy: (words) => (
    <a href={t(lang, "foot.fanContentUrl")} rel="noopener">
      {words}
    </a>
  ),
});

/** The Fan Content Policy's notice in `lang`, Wizards' own words, linked to the policy in that language. */
function FanContent({ lang }: { lang: Lang }) {
  return (
    <p className="notice" lang={tagOf(lang)}>
      {rich(lang, "foot.fanContent", policy(lang))}
    </p>
  );
}

export function Footer() {
  const lang = useLang();
  return (
    <footer className="foot">
      <div className="foot-inner">
        <Logo className="foot-logo" sizes="7rem" alt="" lazy />
        {/* The Fan Content Policy's notice, word for word from Wizards'
            templates (English, and Wizards' official translation of it);
            Baylee's docs/legal.md §1. The English one stands in every
            language, the page's own beside it. */}
        <FanContent lang="en" />
        {lang !== "en" && <FanContent lang={lang} />}
        <p className="notice">
          <T id="foot.wotc" />
        </p>
        <p className="notice">
          <T id="foot.scryfall" />
        </p>
        <nav className="foot-links" aria-label={t(lang, "foot.label")}>
          <a href={GITHUB} rel="noopener">
            <T id="foot.source" />
          </a>
          <a href={CENTRAL_SOURCE} rel="noopener">
            <T id="foot.central" />
          </a>
          <a href={SITE_SOURCE} rel="noopener">
            <T id="foot.siteSource" />
          </a>
          <a href={RELEASES_PAGE} rel="noopener">
            <T id="foot.releases" />
          </a>
          <a href={ISSUES} rel="noopener">
            <T id="foot.issues" />
          </a>
          <a href={DISCORD} rel="noopener">
            Discord
          </a>
          <Link to={PATHS.contact}>
            <T id="foot.contact" />
          </Link>
          <a href={`${GITHUB}/blob/main/docs/legal.md`} rel="noopener">
            <T id="foot.legal" />
          </a>
          <a href={`${GITHUB}/blob/main/NOTICE`} rel="noopener">
            NOTICE
          </a>
          <Link to={PATHS.terms}>
            <T id="foot.terms" />
          </Link>
          <Link to={PATHS.privacy}>
            <T id="foot.privacy" />
          </Link>
          <Link to={PATHS.impressum}>
            <T id="foot.impressum" />
          </Link>
        </nav>
        <p className="colophon">
          <T id="foot.colophon" />
        </p>
      </div>
    </footer>
  );
}
