import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";

import { directoryClient } from "../../src/central/directory";
import Gateways from "../../src/gateways/GatewaysPage";
import { LangProvider } from "../../src/i18n";
import { listing, page } from "../fixtures/directory";
import { serve } from "../support/network";

function show(lang: "en" | "de" = "en") {
  return render(
    <LangProvider initial={lang}>
      <Gateways client={directoryClient({ api: "/api" })} />
    </LangProvider>,
  );
}

describe("the gateway directory", () => {
  it("lists a page, then the next one after the last id", async () => {
    const calls = serve((c) => {
      if (c.url !== "/api/v1/native") return undefined;
      const after = (c.body as { after: number[] | null }).after;
      return { body: page(after === null ? 0 : 1, 130) };
    });
    const user = userEvent.setup();
    show();
    await waitFor(() => expect(screen.getAllByRole("listitem").length).toBeGreaterThanOrEqual(100));
    expect(screen.getByText(/100 gateways listed/)).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "Show the next 100" }));
    await waitFor(() => expect(screen.getByText(/130 gateways listed/)).toBeTruthy());
    expect(screen.queryByRole("button", { name: "Show the next 100" })).toBeNull();
    expect(calls).toHaveLength(2);
    expect(calls[0]?.body).toEqual({ operation: "gateway_directory", after: null });
    expect((calls[1]?.body as { after: number[] } | undefined)?.after).toEqual(listing(100).id);
  });

  it("shows a name as text and links a website only on its own origin", async () => {
    serve(() => ({
      body: { result: "gateway_directory", gateways: [listing(1, { name: "<b>Bold</b> & co", online: true }), listing(3, { online: false })] },
    }));
    const { container } = show();
    const first = await screen.findByRole("heading", { name: "<b>Bold</b> & co" });
    expect(container.querySelector("b")?.textContent).not.toBe("Bold");
    const card = first.closest("li") as HTMLElement;
    expect(within(card).getByText("online")).toBeTruthy();
    expect(within(card).getByRole("link", { name: "Website" }).getAttribute("href")).toBe("https://gw1.example.invalid/");
    expect(within(card).getByRole("link", { name: "Website" }).getAttribute("rel")).toContain("noreferrer");
    expect(screen.getByText("not seen lately")).toBeTruthy();
    expect(screen.getByText("2 gateways listed, 1 online.")).toBeTruthy();
  });

  it("shows nothing of an answer that fails its checks, and says so", async () => {
    const forged = listing(1);
    forged.metadata.website = "https://phishing.invalid/";
    serve(() => ({ body: { result: "gateway_directory", gateways: [forged] } }));
    show();
    expect(await screen.findByText(/did not pass this page's checks/)).toBeTruthy();
    expect(screen.queryByText("Synthetic table 1")).toBeNull();
  });

  it("says when Central is not open to browsers, and tries again on request", async () => {
    let answer = 403;
    const calls = serve(() => (answer === 403 ? { status: 403, text: "request not permitted\n" } : { body: page(0, 1) }));
    const user = userEvent.setup();
    show("de");
    expect(await screen.findByText(/nicht beantwortet/)).toBeTruthy();
    answer = 200;
    await user.click(screen.getByRole("button", { name: "Erneut versuchen" }));
    expect(await screen.findByText("Synthetic table 1")).toBeTruthy();
    expect(calls).toHaveLength(2);
  });

  it("says when no gateway is listed", async () => {
    serve(() => ({ body: { result: "gateway_directory", gateways: [] } }));
    show();
    expect(await screen.findByText("No gateway is listed yet.")).toBeTruthy();
  });
});
