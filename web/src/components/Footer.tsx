// The footer: the notices Wizards' Fan Content Policy and Scryfall ask for,
// word for word as on the old site (Baylee's docs/legal.md §1 and §3), and
// the links every page carries.

import { DISCORD, GITHUB, ISSUES, RELEASES_PAGE, SITE_SOURCE } from "../config";
import { T, tt, useLang } from "../i18n";
import { Link, PATHS } from "../router";
import { Logo } from "./Logo";

export function Footer() {
  const lang = useLang();
  return (
    <footer className="foot">
      <div className="foot-inner">
        <Logo className="foot-logo" sizes="7rem" alt="" lazy />
        {/* The Fan Content Policy's notice, word for word from Wizards'
            templates (English, and the official German one); Baylee's
            docs/legal.md §1. The English one stands in both languages. */}
        <p className="notice" lang="en">
          Baylee is unofficial Fan Content permitted under the{" "}
          <a href="https://company.wizards.com/en/legal/fancontentpolicy" rel="noopener">
            Fan Content Policy
          </a>
          . Not approved/endorsed by Wizards. Portions of the materials used are property of Wizards of the Coast.
          ©Wizards of the Coast LLC.
        </p>
        {lang === "de" && (
          <p className="notice" lang="de">
            Baylee ist ein inoffizieller Fan-Inhalt im Rahmen der{" "}
            <a href="https://company.wizards.com/de/legal/fancontentpolicy" rel="noopener">
              Richtlinie für Fan-Inhalte
            </a>
            . Nicht von Wizards gefördert/gesponsert. Bestandteile des enthaltenen Materials sind Eigentum von Wizards of
            the Coast. ©Wizards of the Coast LLC. Baylee ist nicht mit Wizards of the Coast verbunden.
          </p>
        )}
        <p className="notice">
          <T
            en="Magic: The Gathering, card names, set names, card images and mana symbols are property of Wizards of the Coast LLC. Every card image names its artist; select a card to see artist and printing on Scryfall."
            de="Magic: The Gathering sowie Kartennamen, Setnamen, Kartenbilder und Manasymbole sind Eigentum von Wizards of the Coast LLC. Jedes Kartenbild nennt seine Künstlerin oder seinen Künstler; ein Klick auf eine Karte zeigt Künstler und Druck bei Scryfall."
          />
        </p>
        <p className="notice">
          <T
            en="Card data and images provided by Scryfall. Baylee is not affiliated with Scryfall."
            de="Kartendaten und -bilder bereitgestellt von Scryfall. Baylee ist nicht mit Scryfall verbunden."
          />
        </p>
        <nav className="foot-links" aria-label={tt(lang, "Footer", "Fußzeile")}>
          <a href={GITHUB} rel="noopener">
            <T en="Source code (AGPL-3.0)" de="Quellcode (AGPL-3.0)" />
          </a>
          <a href={SITE_SOURCE} rel="noopener">
            <T en="This site's source" de="Quellcode dieser Seite" />
          </a>
          <a href={RELEASES_PAGE} rel="noopener">
            Releases
          </a>
          <a href={ISSUES} rel="noopener">
            Issues
          </a>
          <a href={DISCORD} rel="noopener">
            Discord
          </a>
          <Link to={PATHS.contact}>
            <T en="Contact" de="Kontakt" />
          </Link>
          <a href={`${GITHUB}/blob/main/docs/legal.md`} rel="noopener">
            <T en="Legal" de="Rechtliches" />
          </a>
          <a href={`${GITHUB}/blob/main/NOTICE`} rel="noopener">
            NOTICE
          </a>
          <Link to={PATHS.privacy}>
            <T en="Privacy" de="Datenschutz" />
          </Link>
          <Link to={PATHS.impressum}>
            <T en="Legal notice" de="Impressum" />
          </Link>
        </nav>
        <p className="colophon">
          <T
            en="Fonts: Faustina and Alegreya Sans, SIL Open Font License 1.1, served from this site. The painting and the logo are our own."
            de="Schriften: Faustina und Alegreya Sans, SIL Open Font License 1.1, von dieser Seite ausgeliefert. Das Gemälde und das Logo sind unsere eigenen."
          />
        </p>
      </div>
    </footer>
  );
}
