// The footer: the notices Wizards' Fan Content Policy and Scryfall ask for,
// word for word as on the old site (Baylee's docs/legal.md §1 and §3), and
// the site's map in three columns.

import { CENTRAL_SOURCE, DISCORD, GITHUB, ISSUES, RELEASES_PAGE, SITE_SOURCE } from "../config";
import type { ReactNode } from "react";

import { T, rich, t, tagOf, useLang, type Elems, type Key, type Lang } from "../i18n";
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

/** One column of the footer's map: a heading and its links. */
function Column({ head, children }: { head: Key; children: ReactNode }) {
  const lang = useLang();
  const id = `foot-${head.replace(".", "-")}`;
  return (
    <section className="foot-col" aria-labelledby={id}>
      <h2 className="foot-head" id={id}>
        {t(lang, head)}
      </h2>
      <ul className="foot-list">{children}</ul>
    </section>
  );
}

export function Footer() {
  const lang = useLang();
  return (
    <footer className="foot">
      <div className="foot-inner">
        <Logo className="foot-logo" sizes="7rem" alt="" lazy />
        <p className="footer-dedication"><T id="baylee.dedication" /></p>
        {/* Three equal columns, the site's map: what to play, the project,
            the legal pages (docs/website-design.md §"Space and layout"). */}
        <nav className="foot-map" aria-label={t(lang, "foot.label")}>
          <Column head="foot.play">
            <li><Link to={PATHS.download}><T id="nav.download" /></Link></li>
            <li><Link to={PATHS.gateways}><T id="nav.gateways" /></Link></li>
            <li><Link to={PATHS.cards}><T id="nav.cards" /></Link></li>
            <li><a href={DISCORD} rel="noopener">Discord</a></li>
            <li><Link to={PATHS.contact}><T id="foot.contact" /></Link></li>
          </Column>
          <Column head="foot.project">
            <li><a href={GITHUB} rel="noopener"><T id="foot.source" /></a></li>
            <li><a href={CENTRAL_SOURCE} rel="noopener"><T id="foot.central" /></a></li>
            <li><a href={SITE_SOURCE} rel="noopener"><T id="foot.siteSource" /></a></li>
            <li><a href={RELEASES_PAGE} rel="noopener"><T id="foot.releases" /></a></li>
            <li><a href={ISSUES} rel="noopener"><T id="foot.issues" /></a></li>
          </Column>
          <Column head="foot.legalHead">
            <li><Link to={PATHS.terms}><T id="foot.terms" /></Link></li>
            <li><Link to={PATHS.privacy}><T id="foot.privacy" /></Link></li>
            <li><Link to={PATHS.impressum}><T id="foot.impressum" /></Link></li>
            <li><a href={`${GITHUB}/blob/main/docs/legal.md`} rel="noopener"><T id="foot.legal" /></a></li>
            <li><a href={`${GITHUB}/blob/main/NOTICE`} rel="noopener">NOTICE</a></li>
          </Column>
        </nav>
        {/* The Fan Content Policy's notice, word for word from Wizards'
            templates (English, and Wizards' official translation of it);
            Baylee's docs/legal.md §1. The English one stands in every
            language, the page's own beside it. */}
        <div className="foot-notices">
          <FanContent lang="en" />
          {lang !== "en" && <FanContent lang={lang} />}
          <p className="notice">
            <T id="foot.wotc" />
          </p>
          <p className="notice">
            <T id="foot.scryfall" />
          </p>
        </div>
        <p className="colophon">
          <T id="foot.colophon" />
        </p>
      </div>
    </footer>
  );
}
