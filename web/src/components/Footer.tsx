// The footer: the brand, dedication & community links alongside the three-column
// sitemap, followed by the required Wizards Fan Content Policy and Scryfall
// notices (word for word, Baylee's docs/legal.md §1 and §3) and colophon.

import type { ReactNode } from "react";

import { CENTRAL_SOURCE, DISCORD, GITHUB, ISSUES, RELEASES_PAGE, SITE_SOURCE, WHATSAPP } from "../config";
import { T, rich, t, tagOf, useLang, type Elems, type Key, type Lang } from "../i18n";
import { Link, PATHS } from "../router";
import { Icon } from "./icons";
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
        <div className="foot-top">
          <div className="foot-brand">
            <Link to={PATHS.home} className="foot-logo-link" aria-label="Baylee">
              <Logo className="foot-logo" sizes="8.5rem" alt="" lazy />
            </Link>
            <p className="footer-dedication">
              <T id="baylee.dedication" />
            </p>
            <p className="foot-summary">
              <T id="foot.summary" />
            </p>
            <div className="foot-social">
              <a className="foot-social-pill" href={DISCORD} target="_blank" rel="noopener">
                <Icon name="discord" />
                <span>Discord</span>
              </a>
              <a className="foot-social-pill" href={WHATSAPP} target="_blank" rel="noopener">
                <Icon name="whatsapp" />
                <span>WhatsApp</span>
              </a>
              <a className="foot-social-pill" href={GITHUB} target="_blank" rel="noopener">
                <Icon name="github" />
                <span>GitHub</span>
              </a>
            </div>
          </div>

          <nav className="foot-map" aria-label={t(lang, "foot.label")}>
            <Column head="foot.play">
              <li>
                <Link to={PATHS.download}>
                  <T id="nav.download" />
                </Link>
              </li>
              <li>
                <Link to={PATHS.gateways}>
                  <T id="nav.gateways" />
                </Link>
              </li>
              <li>
                <Link to={PATHS.cards}>
                  <T id="nav.cards" />
                </Link>
              </li>
              <li>
                <a href={DISCORD} target="_blank" rel="noopener">
                  Discord
                </a>
              </li>
              <li>
                <a href={WHATSAPP} target="_blank" rel="noopener">
                  WhatsApp
                </a>
              </li>
              <li>
                <Link to={PATHS.contact}>
                  <T id="foot.contact" />
                </Link>
              </li>
            </Column>
            <Column head="foot.project">
              <li>
                <a href={GITHUB} target="_blank" rel="noopener">
                  <T id="foot.source" />
                </a>
              </li>
              <li>
                <a href={CENTRAL_SOURCE} target="_blank" rel="noopener">
                  <T id="foot.central" />
                </a>
              </li>
              <li>
                <a href={SITE_SOURCE} target="_blank" rel="noopener">
                  <T id="foot.siteSource" />
                </a>
              </li>
              <li>
                <a href={RELEASES_PAGE} target="_blank" rel="noopener">
                  <T id="foot.releases" />
                </a>
              </li>
              <li>
                <a href={ISSUES} target="_blank" rel="noopener">
                  <T id="foot.issues" />
                </a>
              </li>
            </Column>
            <Column head="foot.legalHead">
              <li>
                <Link to={PATHS.terms}>
                  <T id="foot.terms" />
                </Link>
              </li>
              <li>
                <Link to={PATHS.privacy}>
                  <T id="foot.privacy" />
                </Link>
              </li>
              <li>
                <Link to={PATHS.impressum}>
                  <T id="foot.impressum" />
                </Link>
              </li>
              <li>
                <a href={`${GITHUB}/blob/main/docs/legal.md`} target="_blank" rel="noopener">
                  <T id="foot.legal" />
                </a>
              </li>
              <li>
                <a href={`${GITHUB}/blob/main/NOTICE`} target="_blank" rel="noopener">
                  NOTICE
                </a>
              </li>
            </Column>
          </nav>
        </div>

        <div className="foot-bottom">
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

          <div className="foot-colophon-row">
            <p className="colophon">
              <T id="foot.colophon" />
            </p>
            <div className="foot-badges" aria-hidden="true">
              <span className="foot-badge">AGPL-3.0</span>
              <span className="foot-badge">Rust &amp; Bevy</span>
            </div>
          </div>
        </div>
      </div>
    </footer>
  );
}
