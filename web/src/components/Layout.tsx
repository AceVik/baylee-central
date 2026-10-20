// The frame every page shares: the skip link, the bar (brand, sections,
// language), and the footer with the notices Wizards' Fan Content Policy and
// Scryfall ask for, word for word as on the old site (Baylee's
// docs/legal.md §1 and §3).

import { useEffect, useRef, useState, type ReactNode } from "react";

import { DISCORD, GITHUB, ISSUES, RELEASES_PAGE, SITE_SOURCE } from "../config";
import { T, tt, useChooseLang, useLang, type Lang } from "../i18n";
import { Link, PATHS, type Page } from "../router";
import { Logo } from "./Logo";

const NAV: { page: Page; en: string; de: string }[] = [
  { page: "cards", en: "Cards", de: "Karten" },
  { page: "download", en: "Download", de: "Download" },
  { page: "gateways", en: "Gateways", de: "Gateways" },
  { page: "contact", en: "Contact", de: "Kontakt" },
];

function LangSwitch() {
  const lang = useLang();
  const choose = useChooseLang();
  const button = (l: Lang, label: string) => (
    <button type="button" className="lang-btn" lang={l} aria-pressed={lang === l} onClick={() => choose(l)}>
      {label}
    </button>
  );
  return (
    <fieldset className="lang">
      <legend className="vh">Language / Sprache</legend>
      {button("en", "English")}
      {button("de", "Deutsch")}
    </fieldset>
  );
}

export function Header({ page }: { page: Page | null }) {
  const lang = useLang();
  const here = page ?? "missing";
  // the menu is open on the page it was opened on, and closes with a new page
  const [openOn, setOpenOn] = useState<string | null>(null);
  const open = openOn === here;
  const [scrolled, setScrolled] = useState(() => window.scrollY > 24);
  const overHero = page === "home";
  const menu = useRef<HTMLButtonElement>(null);

  // over the hero the bar is clear; once the page moves it frosts
  useEffect(() => {
    if (!overHero) return;
    const update = () => setScrolled(window.scrollY > 24);
    window.addEventListener("scroll", update, { passive: true });
    return () => window.removeEventListener("scroll", update);
  }, [overHero]);
  const solid = !overHero || scrolled || open;

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      setOpenOn(null);
      menu.current?.focus();
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [open]);

  return (
    <header className={`top${solid ? " solid" : ""}${overHero ? " over-hero" : ""}`}>
      <div className="top-inner">
        <Link to="/" className="brand" aria-label="Baylee">
          <Logo sizes="7.6rem" alt="Baylee" />
        </Link>
        <button
          ref={menu}
          type="button"
          className="menu-btn"
          aria-expanded={open}
          aria-controls="nav"
          onClick={() => setOpenOn(open ? null : here)}
        >
          <span className="bars" aria-hidden="true" />
          <span className="vh">{tt(lang, "Menu", "Menü")}</span>
        </button>
        <nav className={`nav${open ? " open" : ""}`} id="nav" aria-label={tt(lang, "Sections", "Bereiche")}>
          {NAV.map((item) => (
            <Link key={item.page} to={PATHS[item.page]} aria-current={page === item.page ? "page" : undefined}>
              {tt(lang, item.en, item.de)}
            </Link>
          ))}
        </nav>
        <LangSwitch />
      </div>
    </header>
  );
}

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

export function SkipLink() {
  return (
    <a className="skip" href="#main">
      <T en="Skip to content" de="Zum Inhalt springen" />
    </a>
  );
}

/** A section with its eyebrow and heading; `alt` gives it the other background. */
export function Section({
  id,
  eyebrow,
  title,
  alt,
  children,
}: {
  id?: string;
  eyebrow: ReactNode;
  title: ReactNode;
  alt?: boolean;
  children: ReactNode;
}) {
  const heading = id === undefined ? undefined : `${id}-title`;
  return (
    <section className={`section${alt === true ? " section-alt" : ""}`} id={id} aria-labelledby={heading}>
      <div className="section-inner">
        <p className="eyebrow">{eyebrow}</p>
        <h2 id={heading}>{title}</h2>
        {children}
      </div>
    </section>
  );
}
