import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";

import { App } from "../../src/App";
import { LANG_KEY, LangProvider } from "../../src/i18n";
import { serve } from "../support/network";

function app(lang: "en" | "de" = "en") {
  return render(
    <LangProvider initial={lang}>
      <App />
    </LangProvider>,
  );
}

describe("the language switch", () => {
  it("switches the whole page and is remembered", async () => {
    serve(() => undefined);
    const user = userEvent.setup();
    app("en");
    expect(screen.getByRole("heading", { level: 1 }).textContent).toContain("Build your decks");
    await user.click(screen.getByRole("button", { name: "Language / Sprache" }));
    await user.click(screen.getByRole("button", { name: "Deutsch" }));
    expect(screen.getByRole("heading", { level: 1 }).textContent).toContain("Decks bauen");
    expect(document.documentElement.lang).toBe("de");
    expect(window.localStorage.getItem(LANG_KEY)).toBe("de");
    await user.click(screen.getByRole("button", { name: "Language / Sprache" }));
    expect(screen.getByRole("button", { name: "Deutsch" }).getAttribute("aria-pressed")).toBe("true");
  });
});
