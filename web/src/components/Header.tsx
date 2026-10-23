// The bar every page shares: the brand, the sections, the Discord community
// and the account (Sign in, or the signed-in handle; a menu on narrow
// screens), and the language switch: one button
// per language with a file, each in its own name. Over the start page's hero
// it is clear; once the page moves, or on any other page, it frosts.

import { useEffect, useRef, useState } from "react";

import { useAccount } from "../account/session";
import { DISCORD } from "../config";
import { OFFERED, nameOf, t, tagOf, useChooseLang, useLang, wordOf, type Key } from "../i18n";
import { Link, PATHS, type Page } from "../router";
import { Logo } from "./Logo";

const NAV: { page: Page; label: Key }[] = [
  { page: "cards", label: "nav.cards" },
  { page: "download", label: "nav.download" },
  { page: "gateways", label: "nav.gateways" },
  { page: "contact", label: "nav.contact" },
];

function LangSwitch() {
  const lang = useLang();
  const choose = useChooseLang();
  return (
    <fieldset className="lang">
      {/* the word in every offered language, for a visitor who reads none of the page's */}
      <legend className="vh">{OFFERED.map(wordOf).join(" / ")}</legend>
      {OFFERED.map((l) => (
        <button key={l} type="button" className="lang-btn" lang={tagOf(l)} aria-pressed={lang === l} onClick={() => choose(l)}>
          {nameOf(l)}
        </button>
      ))}
    </fieldset>
  );
}

export function Header({ page }: { page: Page | null }) {
  const lang = useLang();
  const { account } = useAccount();
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
          <span className="vh">{t(lang, "nav.menu")}</span>
        </button>
        <nav className={`nav${open ? " open" : ""}`} id="nav" aria-label={t(lang, "nav.label")}>
          {NAV.map((item) => (
            <Link key={item.page} to={PATHS[item.page]} aria-current={page === item.page ? "page" : undefined}>
              {t(lang, item.label)}
            </Link>
          ))}
          {/* The community, in sight on every page (owner, 20.10.2026). */}
          <a className="nav-discord" href={DISCORD} rel="noopener">
            Discord
          </a>
          {account === null ? (
            <Link className="nav-account" to={PATHS.signIn} aria-current={page === "signIn" || page === "register" ? "page" : undefined}>
              {t(lang, "nav.signIn")}
            </Link>
          ) : (
            <Link
              className="nav-account"
              to={PATHS.account}
              aria-label={account.handle === null ? undefined : t(lang, "nav.account", { name: account.handle })}
              aria-current={page === "account" ? "page" : undefined}
            >
              {/* the handle others see, never the private sign-in name; until Central names it, "your account" */}
              {account.handle === null ? t(lang, "nav.accountUnnamed") : <bdi translate="no">{account.handle}</bdi>}
            </Link>
          )}
        </nav>
        <LangSwitch />
      </div>
    </header>
  );
}
