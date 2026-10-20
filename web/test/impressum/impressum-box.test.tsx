import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { LangProvider } from "../../src/i18n";
import type { Solver } from "../../src/impressum/fetch";
import { ImpressumBox } from "../../src/impressum/ImpressumBox";
import { readLines } from "../../src/impressum/Scrambled";
import { SYNTHETIC, encode } from "../fixtures/impressum";

/** jsdom has no Worker; the page needs one before it offers the button (the solver is stubbed). */
class FakeWorker {
  terminate(): void {}
}

describe("the Impressum", () => {
  it("shows the details only after the puzzle, scrambled, and copies them right", async () => {
    const blob = await encode(SYNTHETIC);
    const calls: { url: string; init?: RequestInit }[] = [];
    vi.stubGlobal(
      "fetch",
      vi.fn<(url: string, init?: RequestInit) => Promise<Response>>(async (url, init) => {
        calls.push({ url, ...(init ? { init } : {}) });
        if (url.includes("/challenge")) {
          return new Response(JSON.stringify({ nonce: "AAAA", parts: 2, difficulty: 4, expires: 1, sig: "s" }), { status: 200 });
        }
        return new Response(blob, { status: 200 });
      }),
    );
    vi.stubGlobal("Worker", FakeWorker);
    const solver = vi.fn<Solver>(async () => [1, 2]);
    const user = userEvent.setup();
    const { container } = render(
      <LangProvider initial="en">
        <ImpressumBox solver={solver} />
      </LangProvider>,
    );
    // nothing about the provider before the click
    expect(container.textContent).not.toContain("Mustermann");
    await user.click(screen.getByRole("button", { name: "Show the details" }));
    await waitFor(() => expect(screen.getByRole("button", { name: "Copy address" })).toBeTruthy());
    expect(solver).toHaveBeenCalledWith({ nonce: "AAAA", parts: 2, difficulty: 4, expires: 1, sig: "s" }, expect.any(Function));
    // jsdom's page is on localhost, which talks to a local feedback service
    expect(calls[0]?.url).toMatch(/^http:\/\/127\.0\.0\.1:28780\/site\/impressum\/challenge\?s=\d+$/);
    expect(calls[1]?.url).toBe("http://127.0.0.1:28780/site/impressum");
    expect(JSON.parse(String(calls[1]?.init?.body))).toEqual({ nonce: "AAAA", parts: 2, difficulty: 4, expires: 1, sig: "s", counters: [1, 2] });
    // the DOM's text is a jumble; no href holds an address
    for (const value of Object.values(SYNTHETIC)) expect(container.textContent).not.toContain(value);
    expect(container.querySelector("a[href^='mailto:'], a[href^='tel:']")).toBeNull();
    const address = container.querySelector(".imp-address");
    expect(address?.getAttribute("aria-hidden")).toBe("true");
    expect(readLines(address as Element)).toBe("Erika Mustermann\nMusterweg 1\n12345 Musterstadt\nGermany");
    // user-event stands in for the clipboard
    await user.click(screen.getByRole("button", { name: "Copy address" }));
    await waitFor(async () => expect(await navigator.clipboard.readText()).toBe("Erika Mustermann\nMusterweg 1\n12345 Musterstadt\nGermany"));
    await user.click(screen.getByRole("button", { name: "Copy e-mail address" }));
    await waitFor(async () => expect(await navigator.clipboard.readText()).toBe("erika@example.org"));
  });

  it("says so when the details do not come", async () => {
    vi.stubGlobal("fetch", vi.fn<() => Promise<Response>>(async () => new Response("", { status: 503 })));
    vi.stubGlobal("Worker", FakeWorker);
    const user = userEvent.setup();
    render(
      <LangProvider initial="de">
        <ImpressumBox solver={async () => []} />
      </LangProvider>,
    );
    await user.click(screen.getByRole("button", { name: "Impressum anzeigen" }));
    await waitFor(() => expect(screen.getByText(/konnten nicht geladen werden/)).toBeTruthy());
  });
});
