// public/boot.js chooses the language before the app runs, from the list the
// build writes into <html data-langs>; the app chooses again with pickLang.
// Both must agree for every visitor, or the first frame says one language
// and the page another.

import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { afterEach, describe, expect, it } from "vitest";

import { LANG_LIST, OFFERED, pickLang, tagOf, type Lang } from "../src/i18n";
import { languagesOnDisk } from "../vite.config";

// (vitest runs in web/, and its jsdom gives modules no file: address)
const BOOT = readFileSync(resolve("public/boot.js"), "utf8");

function boot(search: string, stored: string | null, languages: readonly string[], offered: readonly Lang[]) {
  const root = document.documentElement;
  for (const name of ["data-js", "data-theme", "data-lang", "lang"]) root.removeAttribute(name);
  root.setAttribute("data-langs", offered.join(" "));
  window.history.replaceState(null, "", `/${search}`);
  if (stored === null) window.localStorage.removeItem("baylee-site-lang");
  else window.localStorage.setItem("baylee-site-lang", stored);
  Object.defineProperty(window.navigator, "languages", { value: languages, configurable: true });
  new Function(BOOT)();
  return { code: root.getAttribute("data-lang"), lang: root.getAttribute("lang"), theme: root.getAttribute("data-theme"), js: root.hasAttribute("data-js") };
}

afterEach(() => {
  document.documentElement.removeAttribute("data-langs");
  Reflect.deleteProperty(window.navigator, "languages");
});

describe("boot.js", () => {
  const offers: Lang[][] = [["en", "de"], ["en", "de", "fr", "pt", "zhs", "zht"], ["en"]];
  const searches = ["", "?lang=de", "?lang=fr", "?lang=zht", "?lang=constructor", "?lang=EN"];
  const stored = [null, "de", "fr", "zhs", "toString"];
  const browsers = [[], ["fr-FR", "de-AT", "en"], ["zh-TW"], ["zh-HK"], ["zh-Hans-CN"], ["pt-PT", "de"], ["ja-JP"], ["en-US", "de"]];

  it("chooses as the app does, for every visitor", () => {
    const disagree: string[] = [];
    for (const offered of offers)
      for (const search of searches)
        for (const s of stored)
          for (const languages of browsers) {
            // an empty list is the browser's one language, as initialLang reads it
            const app = pickLang(search, s, languages.length > 0 ? languages : [navigator.language || "en"], offered);
            const got = boot(search, s, languages, offered);
            if (got.code !== app || got.lang !== tagOf(app))
              disagree.push(`${offered.join("+")} ${search} ${String(s)} [${languages.join(",")}]: boot ${got.code}/${got.lang}, app ${app}/${tagOf(app)}`);
          }
    expect(disagree).toEqual([]);
  });

  it("knows every one of Magic's languages by the app's tag", () => {
    for (const lang of LANG_LIST) expect(boot(`?lang=${lang}`, null, [], LANG_LIST).lang).toBe(tagOf(lang));
  });

  it("says that scripts run, and takes the forced colour scheme", () => {
    expect(boot("?theme=dark", null, [], ["en"])).toMatchObject({ js: true, theme: "dark", code: "en" });
    expect(boot("?theme=pink", null, [], ["en"]).theme).toBeNull();
  });

  it("takes English alone when the build named no language", () => {
    document.documentElement.removeAttribute("data-langs");
    Object.defineProperty(window.navigator, "languages", { value: ["de-DE"], configurable: true });
    new Function(BOOT)();
    expect(document.documentElement.getAttribute("data-lang")).toBe("en");
  });
});

describe("the build", () => {
  it("names for boot.js exactly the languages the app offers", () => {
    expect(languagesOnDisk(resolve("locales"))).toEqual([...OFFERED].sort());
  });
});
