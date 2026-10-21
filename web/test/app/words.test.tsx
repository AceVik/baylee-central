// Every page in every offered language: no message reads as its id, no
// argument is left unfilled, and no markup is left over as text (a tag the
// call site does not draw), in the text or in a label.

import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { App } from "../../src/App";
import { KEYS } from "../../src/i18n-keys";
import { SESSION_KEY } from "../../src/account/session";
import { LangProvider, OFFERED } from "../../src/i18n";
import { serve } from "../support/network";

const PAGES = ["/", "/download", "/cards", "/gateways", "/contact", "/impressum", "/privacy", "/terms", "/register", "/signin", "/account", "/nowhere"];
/** The pages that change when an account is signed in (the header on every one). */
const SIGNED_IN = ["/", "/signin", "/account"];

/** The words a visitor meets: the text, and what labels, alternatives and titles say. */
function words(): string {
  const labels = Array.from(document.querySelectorAll("[aria-label], [alt], [title]")).flatMap((el) =>
    ["aria-label", "alt", "title"].map((name) => el.getAttribute(name) ?? ""),
  );
  return [document.title, document.body.textContent, ...labels].join("\n");
}

describe("the words on every page", () => {
  it.each([
    ...OFFERED.flatMap((lang) => PAGES.map((path) => [lang, path, false] as const)),
    ...OFFERED.flatMap((lang) => SIGNED_IN.map((path) => [lang, path, true] as const)),
  ])("%s %s (signed in: %s)", async (lang, path, signedIn) => {
    serve(() => undefined);
    if (signedIn) window.localStorage.setItem(SESSION_KEY, JSON.stringify({ expiresAt: Math.floor(Date.now() / 1000) + 600 }));
    window.history.replaceState(null, "", path);
    render(
      <LangProvider initial={lang}>
        <App />
      </LangProvider>,
    );
    await screen.findAllByRole("heading");
    const text = words();
    expect(text.match(/<\/?[a-z][a-zA-Z]*>/g), "markup left as text").toBeNull();
    expect(text.match(/\{ ?\$[a-zA-Z]+ ?\}/g), "an argument left unfilled").toBeNull();
    expect(
      KEYS.filter((key) => text.includes(key)),
      "a message shown as its id",
    ).toEqual([]);
  });
});
