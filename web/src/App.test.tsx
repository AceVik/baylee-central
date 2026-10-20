import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";

import { App } from "./App";
import { LANG_KEY, LangProvider, pickLang } from "./i18n";
import { navigate } from "./router";
import { serve } from "./test-utils";

function app(lang: "en" | "de" = "en") {
  return render(
    <LangProvider initial={lang}>
      <App />
    </LangProvider>,
  );
}

describe("the language", () => {
  it("is ?lang=, then the remembered choice, then the browser's", () => {
    expect(pickLang("?lang=de", "en", ["en-GB"])).toBe("de");
    expect(pickLang("?lang=fr", "de", ["en-GB"])).toBe("de");
    expect(pickLang("", null, ["fr-FR", "de-AT", "en"])).toBe("de");
    expect(pickLang("", null, ["fr-FR"])).toBe("en");
  });

  it("switches the whole page and is remembered", async () => {
    serve(() => undefined);
    const user = userEvent.setup();
    app("en");
    expect(screen.getByRole("heading", { level: 1 }).textContent).toContain("knows the rules");
    await user.click(screen.getByRole("button", { name: "Deutsch" }));
    expect(screen.getByRole("heading", { level: 1 }).textContent).toContain("die Regeln kennt");
    expect(document.documentElement.lang).toBe("de");
    expect(window.localStorage.getItem(LANG_KEY)).toBe("de");
    expect(screen.getByRole("button", { name: "Deutsch" }).getAttribute("aria-pressed")).toBe("true");
  });
});

describe("the pages", () => {
  it("goes from page to page without a reload, marking where it is", async () => {
    serve(() => undefined);
    const user = userEvent.setup();
    app("en");
    const nav = screen.getByRole("navigation", { name: "Sections" });
    await user.click(within(nav).getByRole("link", { name: "Contact" }));
    expect(await screen.findByRole("heading", { level: 1, name: "Write to us" })).toBeTruthy();
    expect(window.location.pathname).toBe("/contact");
    expect(within(nav).getByRole("link", { name: "Contact" }).getAttribute("aria-current")).toBe("page");
    await user.click(screen.getByRole("link", { name: "Legal notice" }));
    expect(await screen.findByRole("heading", { level: 1, name: "Legal notice" })).toBeTruthy();
    expect(document.head.querySelector('meta[name="robots"]')?.getAttribute("content")).toBe("noindex, noarchive, nosnippet");
    await user.click(within(nav).getByRole("link", { name: "Cards" }));
    expect(await screen.findByRole("heading", { level: 1, name: "The cards, honestly" })).toBeTruthy();
    expect(document.head.querySelector('meta[name="robots"]')).toBeNull();
  });

  it("answers an unknown address with a page that says so", async () => {
    serve(() => undefined);
    window.history.replaceState(null, "", "/nowhere");
    app("en");
    expect(await screen.findByRole("heading", { level: 1, name: "There is no such page" })).toBeTruthy();
  });

  it("sends the old site's link to one set on to the card status", async () => {
    serve(() => undefined);
    window.history.replaceState(null, "", "/#set-lea");
    app("en");
    await waitFor(() => expect(window.location.pathname).toBe("/cards"));
    expect(window.location.hash).toBe("#set-lea");
  });

  it("keeps the footer's notices on every page", async () => {
    serve(() => undefined);
    app("de");
    act(() => navigate("/privacy"));
    expect(await screen.findByRole("heading", { level: 1, name: "Datenschutzerklärung dieser Website" })).toBeTruthy();
    const foot = screen.getByRole("contentinfo");
    expect(foot.textContent).toContain("Baylee is unofficial Fan Content permitted under the Fan Content Policy");
    expect(foot.textContent).toContain("Richtlinie für Fan-Inhalte");
    expect(foot.textContent).toContain("Baylee ist nicht mit Scryfall verbunden");
  });
});

describe("the start page's download", () => {
  it("offers the release's file and links only to GitHub", async () => {
    serve((c) => {
      if (c.url.startsWith("https://api.github.com/repos/AceVik/baylee/releases")) {
        return {
          body: [
            {
              tag_name: "v0.1.0-beta.7",
              html_url: "https://github.com/AceVik/baylee/releases/tag/v0.1.0-beta.7",
              published_at: "2026-10-12T10:00:00Z",
              prerelease: true,
              draft: false,
              assets: [
                {
                  name: "Baylee-0.1.0-x86_64.AppImage",
                  size: 104857600,
                  browser_download_url: "https://github.com/AceVik/baylee/releases/download/v0.1.0-beta.7/Baylee-0.1.0-x86_64.AppImage",
                },
              ],
            },
          ],
        };
      }
      return undefined;
    });
    app("en");
    // jsdom's user agent names no system: the page says so instead of guessing
    expect(await screen.findByText(/could not tell your system/)).toBeTruthy();
    const big = screen.getAllByRole("link", { name: /Download Baylee/ })[0];
    expect(big?.getAttribute("href")).toBe("https://github.com/AceVik/baylee/releases/tag/v0.1.0-beta.7");
    expect(screen.getAllByText(/v0\.1\.0-beta\.7/).length).toBeGreaterThan(0);
  });
});
