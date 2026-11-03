// The bar every page shares: the brand, the sections, the Discord community
// and the account (Sign in, or the signed-in handle; a menu on narrow
// screens), the colour switch (device, light, dark; in the menu on narrow
// screens) and the language switch: one button per language with a file,
// each in its own name. Over the start page's hero it is clear; once the
// page moves, or on any other page, it frosts.

import { useEffect, useId, useRef, useState } from "react";

import { useAccount } from "../account/session";
import { DISCORD } from "../config";
import { OFFERED, nameOf, t, tagOf, useChooseLang, useLang, wordOf, type Key } from "../i18n";
import { Link, PATHS, type Page } from "../router";
import { NEXT_THEME, useTheme, type Theme } from "../theme";
import { Icon, type IconName } from "./icons";

const NAV: { page: Page; label: Key }[] = [
  { page: "cards", label: "nav.cards" },
  { page: "download", label: "nav.download" },
  { page: "gateways", label: "nav.gateways" },
  { page: "contact", label: "nav.contact" },
];

const LANG_CODE: Readonly<Record<typeof OFFERED[number], string>> = {
  en: "EN",
  de: "DE",
  fr: "FR",
  it: "IT",
  es: "ES",
  pt: "PT",
  ja: "JA",
  ko: "KO",
  ru: "RU",
  zhs: "ZH",
  zht: "ZH",
};

/** Decorated vector flag emblem for each supported language. */
function LangFlag({ lang }: { lang: typeof OFFERED[number] }) {
  return (
    <span className="lang-flag-frame" aria-hidden="true">
      <svg className="lang-flag-svg" viewBox="0 0 24 16" width="21" height="14" focusable="false">
        {lang === "de" && (
          <>
            <rect width="24" height="5.34" y="0" fill="#18181b" />
            <rect width="24" height="5.34" y="5.33" fill="#dc2626" />
            <rect width="24" height="5.34" y="10.66" fill="#facc15" />
          </>
        )}
        {lang === "en" && (
          <>
            <rect width="24" height="16" fill="#1e3a8a" />
            <path d="M0 0L24 16M24 0L0 16" stroke="#ffffff" strokeWidth="3.2" />
            <path d="M0 0L24 16M24 0L0 16" stroke="#dc2626" strokeWidth="1.6" />
            <path d="M12 0V16M0 8H24" stroke="#ffffff" strokeWidth="5.2" />
            <path d="M12 0V16M0 8H24" stroke="#dc2626" strokeWidth="3" />
          </>
        )}
        {lang === "fr" && (
          <>
            <rect width="8" height="16" x="0" fill="#1d4ed8" />
            <rect width="8" height="16" x="8" fill="#ffffff" />
            <rect width="8" height="16" x="16" fill="#dc2626" />
          </>
        )}
        {lang === "it" && (
          <>
            <rect width="8" height="16" x="0" fill="#15803d" />
            <rect width="8" height="16" x="8" fill="#ffffff" />
            <rect width="8" height="16" x="16" fill="#dc2626" />
          </>
        )}
        {lang === "es" && (
          <>
            <rect width="24" height="4" y="0" fill="#dc2626" />
            <rect width="24" height="8" y="4" fill="#facc15" />
            <rect width="24" height="4" y="12" fill="#dc2626" />
          </>
        )}
        {lang === "pt" && (
          <>
            <rect width="24" height="16" fill="#15803d" />
            <polygon points="12,2 21,8 12,14 3,8" fill="#facc15" />
            <circle cx="12" cy="8" r="3.2" fill="#1d4ed8" />
          </>
        )}
        {lang === "ja" && (
          <>
            <rect width="24" height="16" fill="#ffffff" />
            <circle cx="12" cy="8" r="4.5" fill="#dc2626" />
          </>
        )}
        {lang === "ko" && (
          <>
            <rect width="24" height="16" fill="#ffffff" />
            <circle cx="12" cy="8" r="4.2" fill="#dc2626" />
            <path d="M7.8 8a4.2 4.2 0 0 0 8.4 0 2.1 2.1 0 0 0-4.2 0 2.1 2.1 0 0 1-4.2 0z" fill="#1d4ed8" />
          </>
        )}
        {lang === "ru" && (
          <>
            <rect width="24" height="5.34" y="0" fill="#ffffff" />
            <rect width="24" height="5.34" y="5.33" fill="#1d4ed8" />
            <rect width="24" height="5.34" y="10.66" fill="#dc2626" />
          </>
        )}
        {(lang === "zhs" || lang === "zht") && (
          <>
            <rect width="24" height="16" fill="#dc2626" />
            <circle cx="6" cy="5.5" r="2.2" fill="#facc15" />
          </>
        )}
        <rect
          width="24"
          height="16"
          fill="none"
          stroke="rgba(255, 255, 255, 0.28)"
          strokeWidth="1.2"
        />
      </svg>
    </span>
  );
}

function LangSwitch() {
  const lang = useLang();
  const choose = useChooseLang();
  const [menuOpen, setMenuOpen] = useState(false);
  const wrapRef = useRef<HTMLDivElement>(null);
  const mainRef = useRef<HTMLButtonElement>(null);
  const menuId = useId();
  const switcherLabel = OFFERED.map(wordOf).join(" / ");

  // the list closes on a press outside, on Escape and on a choice (the
  // keyboard back on the switch), and when the keyboard leaves it
  useEffect(() => {
    const wrap = wrapRef.current;
    if (!menuOpen || wrap === null) return;
    const onPointerDown = (e: PointerEvent) => {
      if (e.target instanceof Node && !wrap.contains(e.target)) setMenuOpen(false);
    };
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      setMenuOpen(false);
      mainRef.current?.focus();
    };
    const onFocusOut = (e: FocusEvent) => {
      if (e.relatedTarget instanceof Node && !wrap.contains(e.relatedTarget)) setMenuOpen(false);
    };
    document.addEventListener("pointerdown", onPointerDown);
    document.addEventListener("keydown", onKeyDown);
    wrap.addEventListener("focusout", onFocusOut);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown);
      document.removeEventListener("keydown", onKeyDown);
      wrap.removeEventListener("focusout", onFocusOut);
    };
  }, [menuOpen]);

  const currentCode = LANG_CODE[lang] ?? lang.toUpperCase();
  const currentName = nameOf(lang);

  return (
    <div className="lang-split-wrap" ref={wrapRef}>
      <fieldset className="lang-split" aria-label={switcherLabel}>
        <button
          ref={mainRef}
          type="button"
          className="lang-main"
          lang={tagOf(lang)}
          aria-expanded={menuOpen}
          aria-controls={menuId}
          aria-label={switcherLabel}
          onClick={() => setMenuOpen((o) => !o)}
          title={`${currentName} (${currentCode}) — ${switcherLabel}`}
        >
          <LangFlag lang={lang} />
          <span className="lang-code">{currentCode}</span>
          <span className="lang-current-name">{currentName}</span>
        </button>
        {/* the same switch for the pointer; the keyboard has the one before it */}
        <button
          type="button"
          className="lang-split-toggle"
          tabIndex={-1}
          aria-hidden="true"
          title={switcherLabel}
          onClick={() => setMenuOpen((o) => !o)}
        >
          <Icon name="down" className={`lang-chevron${menuOpen ? " open" : ""}`} />
        </button>
      </fieldset>
      {menuOpen && (
        <div id={menuId} className="lang-menu">
          <ul className="lang-menu-list">
            {OFFERED.map((l) => {
              const active = lang === l;
              const code = LANG_CODE[l] ?? l.toUpperCase();
              const name = nameOf(l);
              return (
                <li key={l} className="lang-menu-item">
                  <button
                    type="button"
                    className={`lang-menu-btn${active ? " is-active" : ""}`}
                    lang={tagOf(l)}
                    aria-pressed={active}
                    aria-label={name}
                    onClick={() => {
                      choose(l);
                      setMenuOpen(false);
                      mainRef.current?.focus();
                    }}
                  >
                    <LangFlag lang={l} />
                    <span className="lang-code">{code}</span>
                    <span className="lang-menu-name">{name}</span>
                    {active && <Icon name="check" className="lang-menu-check" />}
                  </button>
                </li>
              );
            })}
          </ul>
        </div>
      )}
    </div>
  );
}

const THEME_ICON: Readonly<Record<Theme, IconName>> = { system: "half", light: "sun", dark: "moon" };
const THEME_WORD: Readonly<Record<Theme, Key>> = { system: "theme.system", light: "theme.light", dark: "theme.dark" };
const THEME_SHORT: Readonly<Record<Theme, Key>> = { system: "theme.systemShort", light: "theme.lightShort", dark: "theme.darkShort" };

/**
 * One button through the three schemes; its name says the scheme now and
 * what a click does. In the menu it shows the scheme's word beside the icon
 * (the name begins with that word, WCAG 2.5.3).
 */
function ThemeSwitch({ inMenu }: { inMenu: boolean }) {
  const lang = useLang();
  const [theme, choose] = useTheme();
  const label = t(lang, THEME_WORD[theme]);
  return (
    <button
      type="button"
      className={`theme-btn ${inMenu ? "theme-in-menu" : "theme-in-bar"}`}
      onClick={() => choose(NEXT_THEME[theme])}
      aria-label={label}
      title={label}
    >
      <Icon name={THEME_ICON[theme]} className="theme-icon" />
      {inMenu && <span className="theme-word">{t(lang, THEME_SHORT[theme])}</span>}
    </button>
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
  const bar = useRef<HTMLElement>(null);

  // over the hero the bar is clear; once the page moves it frosts
  useEffect(() => {
    if (!overHero) return;
    const update = () => setScrolled(window.scrollY > 24);
    window.addEventListener("scroll", update, { passive: true });
    return () => window.removeEventListener("scroll", update);
  }, [overHero]);
  const solid = !overHero || scrolled || open;

  // the open menu closes with Escape (focus back on its button), and when
  // focus or a tap goes elsewhere, so it never covers what has focus
  useEffect(() => {
    if (!open) return;
    const inside = (target: EventTarget | null) => target instanceof Node && (bar.current?.contains(target) ?? false);
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      setOpenOn(null);
      menu.current?.focus();
    };
    const onAway = (e: Event) => {
      if (!inside(e.target)) setOpenOn(null);
    };
    document.addEventListener("keydown", onKey);
    document.addEventListener("focusin", onAway);
    document.addEventListener("pointerdown", onAway);
    return () => {
      document.removeEventListener("keydown", onKey);
      document.removeEventListener("focusin", onAway);
      document.removeEventListener("pointerdown", onAway);
    };
  }, [open]);

  return (
    <header ref={bar} className={`top${solid ? " solid" : ""}${overHero ? " over-hero" : ""}`}>
      <div className="top-inner">
        <Link to="/" className="brand" aria-label="Baylee">
          <img src="/img/baylee-sunlight-icon.webp" width={192} height={192} alt="" />
          <span>Baylee</span>
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
            <Link key={item.page} to={PATHS[item.page]} aria-current={page === item.page ? "page" : undefined} onClick={() => setOpenOn(null)}>
              {t(lang, item.label)}
            </Link>
          ))}
          {/* The community, in sight on every page (owner, 20.10.2026). */}
          <a className="nav-discord" href={DISCORD} target="_blank" rel="noopener">
            Discord
          </a>
          {account === null ? (
            <Link className="nav-account" to={PATHS.signIn} aria-current={page === "signIn" || page === "register" ? "page" : undefined} onClick={() => setOpenOn(null)}>
              {t(lang, "nav.signIn")}
            </Link>
          ) : (
            <Link
              className="nav-account"
              to={PATHS.account}
              aria-label={account.handle === null ? undefined : t(lang, "nav.account", { name: account.handle })}
              aria-current={page !== null && (page.startsWith("account") || page.startsWith("admin")) ? "page" : undefined}
              onClick={() => setOpenOn(null)}
            >
              {/* the handle others see, never the private sign-in name; until Central names it, "your account" */}
              {account.handle === null ? t(lang, "nav.accountUnnamed") : <bdi translate="no">{account.handle}</bdi>}
            </Link>
          )}
          <ThemeSwitch inMenu />
        </nav>
        <div className="top-tools">
          <ThemeSwitch inMenu={false} />
          <LangSwitch />
        </div>
      </div>
    </header>
  );
}
