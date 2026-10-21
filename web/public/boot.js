// Runs before the first paint: in <head>, not deferred, about a kilobyte,
// a file because the CSP allows no inline script. It says that scripts run
// (data-js), applies ?theme= and picks the language as src/i18n.tsx does, so
// the first frame already has the right colours, the right `lang` and the
// no-script note hidden. Everything else is the app's. The languages with a
// file are in <html data-langs>, which the build writes (vite.config.ts);
// TAGS is src/i18n.tsx's table, and test/boot.test.ts holds the two together.
(function () {
  "use strict";
  var TAGS = { en: "en", de: "de", fr: "fr", it: "it", es: "es", pt: "pt-BR", ja: "ja", ko: "ko", ru: "ru", zhs: "zh-Hans", zht: "zh-Hant" };
  var root = document.documentElement, q = null, l = null, i;
  var have = " " + (root.getAttribute("data-langs") || "en") + " ";
  function offered(code) {
    return typeof code === "string" && Object.prototype.hasOwnProperty.call(TAGS, code) && have.indexOf(" " + code + " ") >= 0;
  }
  function ours(tag) {
    var p = String(tag).toLowerCase().split(/[-_]/);
    if (p[0] !== "zh") return p[0];
    for (var j = 1; j < p.length; j++) if (p[j] === "hant" || p[j] === "tw" || p[j] === "hk" || p[j] === "mo") return "zht";
    return "zhs";
  }
  try { q = new URLSearchParams(location.search); } catch (e) { /* very old browser: defaults */ }
  root.setAttribute("data-js", "");
  var theme = q && q.get("theme");
  if (theme === "dark" || theme === "light") root.setAttribute("data-theme", theme);
  l = q && q.get("lang");
  if (!offered(l)) { try { l = localStorage.getItem("baylee-site-lang"); } catch (e) { l = null; } }
  if (!offered(l)) {
    l = "en";
    var langs = navigator.languages && navigator.languages.length ? navigator.languages : [navigator.language || "en"];
    for (i = 0; i < langs.length; i++) {
      if (offered(ours(langs[i]))) { l = ours(langs[i]); break; }
    }
  }
  root.setAttribute("data-lang", l);
  root.setAttribute("lang", TAGS[l]);
})();
