// The site's words, in Project Fluent: one file per language,
// `locales/<lang>/site.ftl`, in which English decides which messages exist
// (Baylee's docs/translations.md, README.md §"The words"). Call sites name a
// message as `section.name`; the file spells the same id `section-name`,
// because a Fluent id takes no dot.
//
// One language at a time. The choice: `?lang=` wins, then what this browser
// chose last time (localStorage, the key the old site at baylee.acevik.de
// used, so a visitor keeps it), then the browser's languages; only a language
// with a file counts, else English. `public/boot.js` makes the same choice
// before the first paint.

import { FluentBundle, FluentResource } from "@fluent/bundle";
import {
  Fragment,
  createContext,
  createElement,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";

import english from "../locales/en/site.ftl?raw";
import type { Key } from "./i18n-keys";

export type { Key } from "./i18n-keys";

/**
 * Every language Magic is printed in, by Scryfall's code for it: its BCP 47
 * tag (the page's `lang`), the locale its numbers, dates and plural forms
 * follow, and, for the language buttons, its own name and its word for
 * "language". Magic's Portuguese is Brazil's. A language is offered once `locales/<code>/site.ftl` exists; the
 * table and that file are all a new one needs (README.md §"The words").
 */
const LANGS = {
  en: { tag: "en", locale: "en-GB", name: "English", word: "Language" },
  de: { tag: "de", locale: "de-DE", name: "Deutsch", word: "Sprache" },
  fr: { tag: "fr", locale: "fr-FR", name: "Français", word: "Langue" },
  it: { tag: "it", locale: "it-IT", name: "Italiano", word: "Lingua" },
  es: { tag: "es", locale: "es-ES", name: "Español", word: "Idioma" },
  pt: { tag: "pt-BR", locale: "pt-BR", name: "Português", word: "Idioma" },
  ja: { tag: "ja", locale: "ja-JP", name: "日本語", word: "言語" },
  ko: { tag: "ko", locale: "ko-KR", name: "한국어", word: "언어" },
  ru: { tag: "ru", locale: "ru-RU", name: "Русский", word: "Язык" },
  zhs: { tag: "zh-Hans", locale: "zh-Hans-CN", name: "简体中文", word: "语言" },
  zht: { tag: "zh-Hant", locale: "zh-Hant-TW", name: "繁體中文", word: "語言" },
} as const;

export type Lang = keyof typeof LANGS;

/** Every language, English first. */
export const LANG_LIST = Object.keys(LANGS) as Lang[];

export function isLang(value: unknown): value is Lang {
  return typeof value === "string" && Object.hasOwn(LANGS, value);
}

// English is the fallback for every message and comes with the page; every
// other language is a chunk of its own, fetched only by a visitor who reads it.
const LOADERS = new Map<Lang, () => Promise<string>>();
for (const [path, load] of Object.entries(
  import.meta.glob<string>(["../locales/*/site.ftl", "!../locales/en/site.ftl"], { query: "?raw", import: "default" }),
)) {
  const code = path.split("/").at(-2);
  if (isLang(code)) LOADERS.set(code, load);
}
const SOURCES = new Map<Lang, string>([["en", english]]);

/** The languages with a file, in the table's order: the ones the buttons offer. */
export const OFFERED: readonly Lang[] = LANG_LIST.filter((l) => l === "en" || LOADERS.has(l));

/** Whether `lang`'s words are here. */
export function isLoaded(lang: Lang): boolean {
  return SOURCES.has(lang);
}

/** Fetches `lang`'s words, once; a language without a file has nothing to fetch. */
export function loadLang(lang: Lang): Promise<void> {
  const load = LOADERS.get(lang);
  if (SOURCES.has(lang) || load === undefined) return Promise.resolve();
  return load().then((source) => {
    SOURCES.set(lang, source);
  });
}

/** The tag for `lang` attributes: zh-Hans for zhs. */
export function tagOf(lang: Lang): string {
  return LANGS[lang].tag;
}

/** The locale `lang` formats numbers and dates in. */
export function localeOf(lang: Lang): string {
  return LANGS[lang].locale;
}

/** The language's name in itself. */
export function nameOf(lang: Lang): string {
  return LANGS[lang].name;
}

/** The language's own word for "language". */
export function wordOf(lang: Lang): string {
  return LANGS[lang].word;
}

/** Ours for a browser's language tag: its base language, Chinese by its script or region. */
export function langOfTag(tag: string): Lang | null {
  const parts = tag.toLowerCase().split(/[-_]/);
  const base = parts[0];
  if (base === "zh") return parts.some((p) => p === "hant" || p === "tw" || p === "hk" || p === "mo") ? "zht" : "zhs";
  return isLang(base) ? base : null;
}

export const LANG_KEY = "baylee-site-lang";

export function pickLang(
  search: string,
  stored: string | null,
  languages: readonly string[],
  offered: readonly Lang[] = OFFERED,
): Lang {
  const ok = (v: unknown): v is Lang => isLang(v) && offered.includes(v);
  const asked = new URLSearchParams(search).get("lang");
  if (ok(asked)) return asked;
  if (ok(stored)) return stored;
  for (const language of languages) {
    const l = langOfTag(language);
    if (ok(l)) return l;
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

// ---------------------------------------------------------------- messages

const bundles = new Map<Lang, FluentBundle>();

/** `lang`'s messages, parsed on first use; `undefined` while they are not here. */
function bundle(lang: Lang): FluentBundle | undefined {
  let found = bundles.get(lang);
  const source = SOURCES.get(lang);
  if (found === undefined && source !== undefined) {
    // No isolation marks around arguments: they would reach the DOM as
    // invisible characters, and every offered language runs left to right.
    found = new FluentBundle(LANGS[lang].locale, { useIsolating: false });
    found.addResource(new FluentResource(source));
    bundles.set(lang, found);
  }
  return found;
}

const fluentId = (key: string) => key.replaceAll(".", "-");

/** A message's arguments: a count as a number (it picks the plural form), anything else as a string. */
export type Args = Readonly<Record<string, string | number>>;

/**
 * `key` in `lang`, its `{ $name }` arguments filled from `args`; in English
 * where `lang` lacks it, and the key itself where English does too. A number
 * is written as `lang` writes it (1,234 or 1.234) and picks the plural form,
 * so a count is passed as a number and an identifier (a year, a version, a
 * commit) as a string. A missing argument shows as `{$name}`.
 */
export function t(lang: Lang, key: Key, args: Args = {}): string {
  const id = fluentId(key);
  for (const each of lang === "en" ? [bundle("en")] : [bundle(lang), bundle("en")]) {
    const pattern = each?.getMessage(id)?.value;
    if (each !== undefined && pattern !== undefined && pattern !== null) return each.formatPattern(pattern, args, []);
  }
  return key;
}

/**
 * What a message's markup draws, by tag: each takes the words between its
 * tags and gives back what stands there, a link for `<notice>…</notice>`.
 * `b`, `i`, `code` and `kbd` need none: they are drawn as themselves.
 */
export type Elems = Readonly<Record<string, (words: string) => ReactNode>>;

const PLAIN: ReadonlySet<string> = new Set(["b", "i", "code", "kbd"]);
const MARKUP = /<([a-z][a-zA-Z]*)>([^<>]*)<\/\1>/g;
// An argument's `<` and `>` while the markup is read, so that no argument (a
// name, an address) can ever open or break an element; given back in the text.
const hold = (text: string) => text.replaceAll("<", "\u{E000}").replaceAll(">", "\u{E001}");
const restore = (text: string) => text.replaceAll("\u{E000}", "<").replaceAll("\u{E001}", ">");

/**
 * `key` in `lang` with its markup drawn: each `<name>words</name>` becomes
 * `b`, `i`, `code` or `kbd` holding the words, or what `elems[name]` makes of
 * them. Nothing in a message or an argument is ever read as HTML, and a tag
 * neither names stays text.
 */
export function rich(lang: Lang, key: Key, elems: Elems = {}, args: Args = {}): ReactNode[] {
  const held: Record<string, string | number> = {};
  for (const [name, value] of Object.entries(args)) held[name] = typeof value === "string" ? hold(value) : value;
  const text = t(lang, key, held);
  const out: ReactNode[] = [];
  let at = 0;
  for (const m of text.matchAll(MARKUP)) {
    const [whole, name = "", inner = ""] = m;
    const draw = Object.hasOwn(elems, name) ? elems[name] : undefined;
    if (draw === undefined && !PLAIN.has(name)) continue;
    if (m.index > at) out.push(restore(text.slice(at, m.index)));
    const words = restore(inner);
    out.push(draw === undefined ? createElement(name, { key: out.length }, words) : <Fragment key={out.length}>{draw(words)}</Fragment>);
    at = m.index + whole.length;
  }
  if (at < text.length) out.push(restore(text.slice(at)));
  return out;
}

// ---------------------------------------------------------------- the page

interface LangState {
  lang: Lang;
  choose: (lang: Lang) => void;
}

const LangContext = createContext<LangState>({ lang: "en", choose: () => undefined });

export function LangProvider({ initial, children }: { initial: Lang; children: ReactNode }) {
  const [lang, setLang] = useState<Lang>(initial);
  // the last language asked for: a slow file that arrives late does not undo a later choice
  const wanted = useRef(initial);
  const choose = useCallback((next: Lang) => {
    wanted.current = next;
    const show = () => {
      if (wanted.current !== next) return;
      setLang(next);
      remember(next);
    };
    if (isLoaded(next)) show();
    else loadLang(next).then(show, () => undefined);
  }, []);
  useEffect(() => {
    const root = document.documentElement;
    root.lang = tagOf(lang);
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

/** One message in the page's language, its markup drawn (`rich`). */
export function T({ id, args, elems }: { id: Key; args?: Args; elems?: Elems }) {
  return <>{rich(useLang(), id, elems, args)}</>;
}

export function num(lang: Lang, n: number): string {
  return n.toLocaleString(localeOf(lang));
}

/** The tab's title for the page that is shown. */
export function useTitle(key: Key): void {
  const lang = useLang();
  useEffect(() => {
    document.title = t(lang, key);
  }, [lang, key]);
}
