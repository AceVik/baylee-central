import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it } from "vitest";

import { App } from "../../src/App";
import { LangProvider } from "../../src/i18n";
import { THEME_KEY } from "../../src/theme";
import { serve } from "../support/network";

afterEach(() => {
  document.documentElement.removeAttribute("data-theme");
  window.localStorage.removeItem(THEME_KEY);
});

describe("the colour switch", () => {
  it("goes from the device's colours to light, dark and back, and is remembered", async () => {
    serve(() => undefined);
    const user = userEvent.setup();
    render(
      <LangProvider initial="en">
        <App />
      </LangProvider>,
    );
    // the bar's button (the menu holds a second one for narrow screens)
    const bar = within(document.querySelector<HTMLElement>(".top-tools") ?? document.body);
    await user.click(bar.getByRole("button", { name: "Colours as on this device. Switch to light" }));
    expect(document.documentElement.getAttribute("data-theme")).toBe("light");
    expect(window.localStorage.getItem(THEME_KEY)).toBe("light");
    await user.click(bar.getByRole("button", { name: "Light colours. Switch to dark" }));
    expect(document.documentElement.getAttribute("data-theme")).toBe("dark");
    expect(window.localStorage.getItem(THEME_KEY)).toBe("dark");
    await user.click(bar.getByRole("button", { name: "Dark colours. Switch back to this device's" }));
    expect(document.documentElement.hasAttribute("data-theme")).toBe(false);
    expect(window.localStorage.getItem(THEME_KEY)).toBeNull();
    // both copies say the same
    expect(screen.getAllByRole("button", { name: "Colours as on this device. Switch to light" })).toHaveLength(2);
  });
});
