// English and German, one at a time. The choice: `?lang=de|en` wins, then
// what this browser chose last time (localStorage, the key the old site at
// baylee.acevik.de used, so a visitor keeps it), then the browser's
// languages. `public/boot.js` makes the same choice before the first paint.
// No library: the text lives in the components, both languages side by side
// (`<T en={…} de={…} />`), so a sentence and its translation change together.

import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";

export type Lang = "en" | "de";

export const LANG_KEY = "baylee-site-lang";
export const LOCALE: Record<Lang, string> = { en: "en-GB", de: "de-DE" };

export function isLang(value: unknown): value is Lang {
  return value === "en" || value === "de";
}

export function pickLang(search: string, stored: string | null, languages: readonly string[]): Lang {
  const asked = new URLSearchParams(search).get("lang");
  if (isLang(asked)) return asked;
  if (isLang(stored)) return stored;
  for (const language of languages) {
    const l = language.toLowerCase();
    if (l.startsWith("de")) return "de";
    if (l.startsWith("en")) return "en";
  }
  return "en";
}

export function storedLang(): string | null {
  try {
    return window.localStorage.getItem(LANG_KEY);
  } catch {
    return null;
  }
}

function remember(lang: Lang): void {
  try {
    window.localStorage.setItem(LANG_KEY, lang);
  } catch {
    /* private mode or blocked storage: the choice lasts this visit */
  }
}

export function initialLang(): Lang {
  const languages = navigator.languages.length > 0 ? navigator.languages : [navigator.language || "en"];
  return pickLang(window.location.search, storedLang(), languages);
}

interface LangState {
  lang: Lang;
  choose: (lang: Lang) => void;
}

const LangContext = createContext<LangState>({ lang: "en", choose: () => undefined });

export function LangProvider({ initial, children }: { initial: Lang; children: ReactNode }) {
  const [lang, setLang] = useState<Lang>(initial);
  const choose = useCallback((next: Lang) => {
    setLang(next);
    remember(next);
  }, []);
  useEffect(() => {
    const root = document.documentElement;
    root.lang = lang;
    root.dataset["lang"] = lang;
  }, [lang]);
  const value = useMemo(() => ({ lang, choose }), [lang, choose]);
  return <LangContext value={value}>{children}</LangContext>;
}

export function useLang(): Lang {
  return useContext(LangContext).lang;
}

export function useChooseLang(): (lang: Lang) => void {
  return useContext(LangContext).choose;
}

/** One run of text in the page's language. */
export function T({ en, de }: { en: ReactNode; de: ReactNode }) {
  return <>{useLang() === "de" ? de : en}</>;
}

export function tt(lang: Lang, en: string, de: string): string {
  return lang === "de" ? de : en;
}

export function num(lang: Lang, n: number): string {
  return n.toLocaleString(LOCALE[lang]);
}

/** The tab's title for the page that is shown. */
export function useTitle(en: string, de: string): void {
  const lang = useLang();
  useEffect(() => {
    document.title = tt(lang, en, de);
  }, [lang, en, de]);
}
