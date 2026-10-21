import { FluentBundle, FluentResource, type Message } from "@fluent/bundle";
import { isValidElement } from "react";
import { describe, expect, it, test } from "vitest";

import { KEYS, type Key } from "../src/i18n-keys";
import { LANG_LIST, OFFERED, isLang, langOfTag, localeOf, pickLang, rich, t, tagOf } from "../src/i18n";

// Every language's file, found on disk rather than through `OFFERED`, so a
// file nobody wired in is noticed too.
const files = import.meta.glob<string>("../locales/*/site.ftl", { query: "?raw", import: "default", eager: true });
const sources = new Map(Object.entries(files).map(([path, source]) => [path.split("/").at(-2) ?? path, source] as const));

type Pattern = NonNullable<Message["value"]>;
type Expression = Exclude<Exclude<Pattern, string>[number], string>;

/** The arguments a pattern reads, in selectors and variants too. */
function variables(pattern: Pattern | null, into: Set<string> = new Set()): Set<string> {
  if (pattern === null || typeof pattern === "string") return into;
  for (const element of pattern) if (typeof element !== "string") expression(element, into);
  return into;
}

function expression(e: Expression, into: Set<string>): void {
  switch (e.type) {
    case "var":
      into.add(e.name);
      break;
    case "select":
      expression(e.selector, into);
      for (const variant of e.variants) variables(variant.value, into);
      break;
    case "func":
    case "term":
      for (const arg of e.args) if (arg.type !== "narg") expression(arg, into);
      break;
    default:
      break;
  }
}

/** Every plural category a pattern's selectors name. */
function categories(pattern: Pattern | null, into: Set<string> = new Set()): Set<string> {
  if (pattern === null || typeof pattern === "string") return into;
  for (const element of pattern) {
    if (typeof element === "string" || element.type !== "select") continue;
    for (const variant of element.variants) {
      if (variant.key.type === "str") into.add(variant.key.value);
      categories(variant.value, into);
    }
  }
  return into;
}

/** The text of a pattern, each variant of a selector after the other. */
function text(pattern: Pattern | null): string {
  if (pattern === null) return "";
  if (typeof pattern === "string") return pattern;
  return pattern
    .map((element) =>
      typeof element === "string" ? element : element.type === "select" ? element.variants.map((v) => text(v.value)).join(" ") : "",
    )
    .join("");
}

/** The markup tags of a pattern, as a sorted set. */
function tags(pattern: Pattern | null): string[] {
  return [...new Set([...text(pattern).matchAll(/<\/?[a-zA-Z][^>]*>/g)].map((m) => m[0]))].sort();
}

/** A file's messages by id, after checking nothing in it was skipped. */
function parse(lang: string): Map<string, Message> {
  const source = sources.get(lang);
  if (source === undefined) throw new Error(`no locales/${lang}/site.ftl`);
  // The runtime parser drops a message it cannot read without a word, and
  // ignores a line that starts no message, so both are counted here: every
  // line is blank, a comment, a message's first line or an indented one.
  const stray = source.split("\n").filter((line) => !/^(|#.*|[a-zA-Z][\w-]* *=.*| .*)$/.test(line));
  expect(stray, `lines in ${lang} that start no message, comment or continuation`).toEqual([]);
  const starts = [...source.matchAll(/^[a-zA-Z][\w-]* *=/gm)].map((m) => m[0].replace(/ *=$/, ""));
  const body = new FluentResource(source).body;
  expect(
    body.map((m) => m.id),
    `messages in ${lang} the parser could not read`,
  ).toEqual(starts);
  const byId = new Map(body.map((m) => [m.id, m] as const));
  expect(byId.size, `an id written twice in ${lang}`).toBe(body.length);
  return byId;
}

const english = parse("en");

describe("the language files", () => {
  test("each file is one of Magic's languages, and English and German have one", () => {
    expect([...sources.keys()].filter((code) => !isLang(code))).toEqual([]);
    expect([...sources.keys()].sort()).toEqual([...OFFERED].sort());
    expect(OFFERED).toEqual(["en", "de"]);
  });

  test("the call sites' keys are the English ids, in the file's order", () => {
    expect(KEYS.map((key) => key.replaceAll(".", "-"))).toEqual([...english.keys()]);
    expect(english.size).toBeGreaterThan(350);
  });

  test.each(OFFERED)("%s has every English message, no other, each with a value", (lang) => {
    const messages = parse(lang);
    const ids = [...messages.keys()];
    expect(
      [...english.keys()].filter((id) => !messages.has(id)),
      "missing",
    ).toEqual([]);
    expect(
      ids.filter((id) => !english.has(id)),
      "not in English",
    ).toEqual([]);
    expect(ids.filter((id) => messages.get(id)?.value === null)).toEqual([]);
  });

  test.each(OFFERED)("%s reads the same arguments as English in each message", (lang) => {
    const messages = parse(lang);
    const differing = [...english].flatMap(([id, en]) => {
      const want = [...variables(en.value)].sort();
      const got = [...variables(messages.get(id)?.value ?? null)].sort();
      return want.join() === got.join() ? [] : [`${id} reads [${got.join(", ")}], English [${want.join(", ")}]`];
    });
    expect(differing).toEqual([]);
  });

  test.each(OFFERED)("%s keeps English's markup in each message", (lang) => {
    const messages = parse(lang);
    const differing = [...english].flatMap(([id, en]) => {
      const want = tags(en.value);
      const got = tags(messages.get(id)?.value ?? null);
      return want.join() === got.join() ? [] : [`${id} has [${got.join(" ")}], English [${want.join(" ")}]`];
    });
    expect(differing).toEqual([]);
  });

  test.each(OFFERED)("%s writes markup only as closed pairs, never nested", (lang) => {
    // `rich` draws `<name>words</name>`; anything else would show as text
    const broken = [...parse(lang)].flatMap(([id, m]) =>
      text(m.value).replace(/<([a-z][a-zA-Z]*)>[^<>]*<\/\1>/g, "").includes("<") ? [id] : [],
    );
    expect(broken).toEqual([]);
  });

  test.each(OFFERED)("%s names only plural forms the language has", (lang) => {
    const own = new Set<string>(new Intl.PluralRules(localeOf(lang)).resolvedOptions().pluralCategories);
    const unknown = [...parse(lang)].flatMap(([id, m]) => [...categories(m.value)].filter((c) => !own.has(c)).map((c) => `${id}: ${c}`));
    expect(unknown).toEqual([]);
  });

  test.each(OFFERED)("%s formats every message without an error", (lang) => {
    const bundle = new FluentBundle(localeOf(lang), { useIsolating: false });
    bundle.addResource(new FluentResource(sources.get(lang) ?? ""));
    const failing = [...english].flatMap(([id, en]) => {
      const pattern = bundle.getMessage(id)?.value ?? null;
      if (pattern === null) return [`${id}: no value`];
      const errors: Error[] = [];
      for (const value of [1, 3, "x"]) {
        bundle.formatPattern(pattern, Object.fromEntries([...variables(en.value)].map((name) => [name, value])), errors);
      }
      return errors.map((e) => `${id}: ${e.message}`);
    });
    expect(failing).toEqual([]);
  });
});

describe("t", () => {
  it("fills arguments, numbers as the language writes them", () => {
    expect(t("en", "gw.count", { n: 1234, online: 3 })).toBe("1,234 gateways listed, 3 online.");
    expect(t("de", "gw.count", { n: 1234, online: 3 })).toBe("1.234 Gateways gelistet, 3 online.");
    expect(t("en", "card.illustrated", { artist: "Anson Maddocks" })).toBe("Illustrated by Anson Maddocks");
  });

  it("picks the plural form by each language's own rules", () => {
    expect(t("en", "sets.cards", { n: 1 })).toBe("1 card");
    expect(t("en", "sets.cards", { n: 0 })).toBe("0 cards");
    expect(t("de", "sets.cards", { n: 1 })).toBe("1 Karte");
    expect(t("de", "sets.eraMeta", { sets: 1, cards: 1 })).toBe("1 Set · 1 Karte");
    expect(t("en", "time.daysAgo", { n: 3 })).toBe("3 days ago");
    expect(t("de", "time.daysAgo", { n: 3 })).toBe("vor 3 Tagen");
  });

  it("writes the card status's summary as before", () => {
    const args = { play: 310, all: 376, pct: 82, full: 290, untested: 20, partial: 6, none: 60, never: 4, sets: 3, corpus: 380 };
    expect(t("en", "sets.total", args)).toBe(
      "310 of 376 cards playable (82 %): 290 full, 20 untested. Besides: 6 partial, 60 not yet, 4 never; 3 sets, 380 cards.",
    );
    expect(t("de", "sets.total", args)).toBe(
      "310 von 376 Karten spielbar (82 %): 290 voll, 20 ungetestet. Außerdem: 6 teilweise, 60 noch nicht, 4 nie; 3 Sets, 380 Karten.",
    );
  });

  it("puts no isolation marks around an argument", () => {
    expect(t("de", "grid.label", { set: "The Dark" })).toBe("Karten aus The Dark");
  });

  it("reads a language without a file in English", () => {
    expect(t("fr", "nav.cards")).toBe("Cards");
    expect(t("zhs", "sets.cards", { n: 2 })).toBe("2 cards");
  });

  it("reads a key no file has as itself", () => {
    expect(t("de", "nav.nowhere" as Key)).toBe("nav.nowhere");
  });
});

describe("rich", () => {
  it("draws each tag as the element it names, its words as text", () => {
    const out = rich("en", "contact.f8", { kbd: <kbd /> });
    expect(out[0]).toBe("A card that resolved wrongly is best reported from the game with ");
    const key = out[1];
    expect(isValidElement(key) && key.type).toBe("kbd");
    expect(isValidElement<{ children: string }>(key) && key.props.children).toBe("F8");
    expect(out[2]).toBe(", because the report can carry the whole game.");
  });

  it("never lets an argument open an element", () => {
    const out = rich("en", "privacy.contact", { code: <code /> }, { host: "<code>x</code>" });
    const code = out[1];
    expect(isValidElement<{ children: string }>(code) && code.props.children).toBe("<code>x</code>");
    expect(out.filter((node) => isValidElement(node))).toHaveLength(1);
  });

  it("leaves a tag the call site does not name as text", () => {
    expect(rich("en", "contact.f8", {}).join("")).toContain("<kbd>F8</kbd>");
  });
});

describe("the language", () => {
  it("is ?lang=, then the remembered choice, then the browser's", () => {
    expect(pickLang("?lang=de", "en", ["en-GB"])).toBe("de");
    expect(pickLang("?lang=fr", "de", ["en-GB"])).toBe("de");
    expect(pickLang("", null, ["fr-FR", "de-AT", "en"])).toBe("de");
    expect(pickLang("", null, ["fr-FR"])).toBe("en");
  });

  it("takes only a language that has a file", () => {
    expect(pickLang("?lang=fr", null, ["fr-FR"])).toBe("en");
    expect(pickLang("?lang=fr", null, ["fr-FR"], ["en", "de", "fr"])).toBe("fr");
    expect(pickLang("", "zht", [], ["en", "zht"])).toBe("zht");
    expect(pickLang("?lang=xx", "constructor", ["xx"])).toBe("en");
  });

  it("knows Chinese by its script or region", () => {
    expect(langOfTag("zh-TW")).toBe("zht");
    expect(langOfTag("zh-Hant-HK")).toBe("zht");
    expect(langOfTag("zh-HK")).toBe("zht");
    expect(langOfTag("zh-CN")).toBe("zhs");
    expect(langOfTag("zh-Hans-SG")).toBe("zhs");
    expect(langOfTag("zh")).toBe("zhs");
    expect(langOfTag("pt-PT")).toBe("pt");
    expect(langOfTag("en-US")).toBe("en");
    expect(langOfTag("nl")).toBeNull();
  });

  it("names every one of Magic's languages by its BCP 47 tag", () => {
    expect(LANG_LIST).toEqual(["en", "de", "fr", "it", "es", "pt", "ja", "ko", "ru", "zhs", "zht"]);
    expect(LANG_LIST.map(tagOf)).toEqual(["en", "de", "fr", "it", "es", "pt-BR", "ja", "ko", "ru", "zh-Hans", "zh-Hant"]);
    for (const lang of LANG_LIST) expect(Intl.getCanonicalLocales(tagOf(lang))).toEqual([tagOf(lang)]);
  });
});
