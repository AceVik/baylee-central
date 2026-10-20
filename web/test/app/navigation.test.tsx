import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";

import { App } from "../../src/App";
import { LangProvider } from "../../src/i18n";
import { navigate } from "../../src/router";
import { serve } from "../support/network";

function app(lang: "en" | "de" = "en") {
  return render(
    <LangProvider initial={lang}>
      <App />
    </LangProvider>,
  );
}

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
