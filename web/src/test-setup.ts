import { cleanup } from "@testing-library/react";
import { afterEach } from "vitest";

// jsdom draws no canvas; the Impressum asks one for its renderer and copes with none
HTMLCanvasElement.prototype.getContext = (() => null) as typeof HTMLCanvasElement.prototype.getContext;

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
