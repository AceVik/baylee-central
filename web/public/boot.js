// Runs before the first paint: in <head>, not deferred, a few hundred bytes,
// a file because the CSP allows no inline script. It says that scripts run
// (data-js), applies ?theme= and picks the language as src/i18n.tsx does, so
// the first frame already has the right colours, the right `lang` and the
// no-script note hidden. Everything else is the app's.
(function () {
  "use strict";
  var root = document.documentElement, q = null, l = null, i;
  try { q = new URLSearchParams(location.search); } catch (e) { /* very old browser: defaults */ }
  root.setAttribute("data-js", "");
  var theme = q && q.get("theme");
  if (theme === "dark" || theme === "light") root.setAttribute("data-theme", theme);
  l = q && q.get("lang");
  if (l !== "de" && l !== "en") { try { l = localStorage.getItem("baylee-site-lang"); } catch (e) { l = null; } }
  if (l !== "de" && l !== "en") {
    l = "en";
    var langs = navigator.languages || [navigator.language || "en"];
    for (i = 0; i < langs.length; i++) {
      var x = String(langs[i]).toLowerCase();
      if (x.indexOf("de") === 0) { l = "de"; break; }
      if (x.indexOf("en") === 0) break;
    }
  }
  root.setAttribute("data-lang", l);
  root.setAttribute("lang", l);
})();
