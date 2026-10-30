import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";

import { directoryClient } from "../../src/central/directory";
import Gateways from "../../src/gateways/GatewaysPage";
import { LangProvider } from "../../src/i18n";
import { listing, officialListing, page } from "../fixtures/directory";
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
      if (c.url !== "/api/v1") return undefined;
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
    expect(calls[0]?.init?.credentials).toBe("omit");
    expect(calls[0]?.init?.headers).toMatchObject({ "Content-Type": "application/json", "X-Baylee-Central": "1" });
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

  it("shows the official gateway first, with its mark, and the rest in Central's order", async () => {
    serve(() => ({
      body: { result: "gateway_directory", gateways: [listing(1), listing(2), officialListing(3, { name: "Sanctuary" }), listing(4)] },
    }));
    show();
    const names = (await screen.findAllByRole("heading", { level: 3 })).map((h) => h.textContent);
    expect(names).toEqual(["Sanctuary", "Synthetic table 1", "Synthetic table 2", "Synthetic table 4"]);
    const official = screen.getByRole("heading", { name: "Sanctuary" }).closest("li") as HTMLElement;
    expect(within(official).getByText("Official").className).toContain("gw-official");
    expect(within(official).getByText("https://baylee.acevik.de")).toBeTruthy();
    expect(screen.getAllByText("Official")).toHaveLength(1);
  });

  it("pages after Central's last id while the official gateway is drawn first", async () => {
    const calls = serve((c) => {
      const after = (c.body as { after: number[] | null }).after;
      if (after !== null) return { body: page(1, 130) };
      const first = page(0, 130);
      first.gateways[49] = officialListing(50, { name: "Sanctuary" });
      return { body: first };
    });
    const user = userEvent.setup();
    show();
    await waitFor(() => expect(screen.getAllByRole("heading", { level: 3 })).toHaveLength(100));
    expect(screen.getAllByRole("heading", { level: 3 })[0]?.textContent).toBe("Sanctuary");
    await user.click(screen.getByRole("button", { name: "Show the next 100" }));
    await waitFor(() => expect(screen.getByText(/130 gateways listed/)).toBeTruthy());
    expect((calls[1]?.body as { after: number[] } | undefined)?.after).toEqual(listing(100).id);
    expect(screen.getAllByRole("heading", { level: 3 })[0]?.textContent).toBe("Sanctuary");
  });

  it("marks the official gateway in German too, and only by its origin", async () => {
    serve(() => ({ body: { result: "gateway_directory", gateways: [officialListing(1, { name: "Baylee" })] } }));
    const { unmount } = show("de");
    expect(await screen.findByText("Offiziell")).toBeTruthy();
    unmount();
    serve(() => ({ body: { result: "gateway_directory", gateways: [listing(1, { name: "Official" })] } }));
    show();
    expect(await screen.findByRole("heading", { name: "Official" })).toBeTruthy();
    expect(document.querySelector(".gw-official")).toBeNull();
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
