import { cleanup } from "@testing-library/react";
import { afterEach, beforeAll } from "vitest";

import { OFFERED, loadLang } from "../src/i18n";

// jsdom draws no canvas; the Impressum asks one for its renderer and copes with none
HTMLCanvasElement.prototype.getContext = (() => null) as typeof HTMLCanvasElement.prototype.getContext;

// every language's words, as main.tsx loads the chosen one before the first render
beforeAll(async () => {
  await Promise.all(OFFERED.map(loadLang));
});

afterEach(() => {
  cleanup();
  window.history.replaceState(null, "", "/");
  try {
    window.localStorage.clear();
    window.sessionStorage.clear();
  } catch {
    /* no storage in this environment */
  }
});
