import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it } from "vitest";

import { forgetLoaded } from "../../src/cards/data";
import { SetsBlock } from "../../src/cards/SetsBlock";
import { LangProvider } from "../../src/i18n";
import { serve } from "../support/network";

afterEach(() => forgetLoaded());

const SETS = {
  corpus: 380,
  rev: "4e984e84a14ed7b3307144af5eb0e9b8188f8930",
  generated_at: "2026-10-14T22:15:34Z",
  totals: { total: 380, implemented: 290, generated: 20, partial: 6, unimplemented: 0, absent: 60, never: 4 },
  sets: [
    { code: "lea", name: "Limited Edition Alpha", released: "1993-08-05", position: 0, total: 290, implemented: 286, generated: 0, partial: 0, unimplemented: 0, absent: 0, never: 4 },
    { code: "arn", name: "Arabian Nights", released: "1993-12-17", position: 1, total: 10, implemented: 4, generated: 0, partial: 6, unimplemented: 0, absent: 0, never: 0 },
    { code: "drk", name: "The Dark", released: "1994-08-01", position: 2, total: 80, implemented: 0, generated: 20, partial: 0, unimplemented: 0, absent: 60, never: 0 },
  ],
};

const ARN = {
  cards: [
    { name: "Aladdin", oracle_id: "o-aladdin", status: "implemented", img: "0b1c2d3e-0000-4000-8000-000000000001" },
    { name: "Ali from Cairo", oracle_id: "o-ali", status: "partial", note: "the damage prevention", img: "0b1c2d3e-0000-4000-8000-000000000002" },
    { name: "Shahrazad", oracle_id: "o-shah", status: "never", note: "subgame" },
  ],
};

function block(hash = "") {
  serve((c) => {
    if (c.url === "/data/sets.json") return { body: SETS };
    if (c.url === "/data/sets/set-arn.json") return { body: ARN };
    return undefined;
  });
  return render(
    <LangProvider initial="en">
      <SetsBlock hash={hash} />
    </LangProvider>,
  );
}

describe("the card status", () => {
  it("summarises the corpus, the finished sets and the next ones", async () => {
    block();
    // playable (full + untested) of the cards that will ever be built, then the parts
    expect(await screen.findByText(/310 of 376 cards playable \(82 %\): 290 full, 20 untested/)).toBeTruthy();
    expect(screen.getByText(/3 sets, 380 cards/)).toBeTruthy();
    expect(screen.getByRole("link", { name: "Limited Edition Alpha" }).getAttribute("href")).toBe("#set-lea");
    expect(screen.getByText("Arabian Nights")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Show all 3 sets" })).toBeTruthy();
  });

  it("opens a set's cards, filters them and pictures only pool cards from Scryfall", async () => {
    const user = userEvent.setup();
    const { container } = block();
    await user.click(await screen.findByText("Arabian Nights"));
    const grid = await screen.findByRole("list", { name: "Cards of Arabian Nights" });
    expect(within(grid).getAllByRole("listitem")).toHaveLength(3);
    const imgs = Array.from(grid.querySelectorAll("img")).map((i) => i.getAttribute("src"));
    expect(imgs).toEqual([
      "https://cards.scryfall.io/small/front/0/b/0b1c2d3e-0000-4000-8000-000000000001.jpg",
      "https://cards.scryfall.io/small/front/0/b/0b1c2d3e-0000-4000-8000-000000000002.jpg",
    ]);
    for (const img of grid.querySelectorAll("img")) expect(img.getAttribute("referrerpolicy")).toBe("no-referrer");
    // one stop for Tab: exactly one tile is reachable
    expect(container.querySelectorAll('.ctile-link[tabindex="0"]')).toHaveLength(1);
    await user.click(screen.getByRole("button", { name: /Partial/ }));
    expect(within(grid).getAllByRole("listitem")).toHaveLength(1);
    expect(within(grid).getByText(/Missing: the damage prevention/)).toBeTruthy();
  });

  it("opens the full list at a set named in the address", async () => {
    block("#set-drk");
    await waitFor(() => expect(document.getElementById("set-drk")?.hasAttribute("open")).toBe(true));
    expect(screen.getByRole("button", { name: "Hide the full list" }).getAttribute("aria-expanded")).toBe("true");
  });

  it("finds sets by name or code", async () => {
    const user = userEvent.setup();
    block();
    await user.click(await screen.findByRole("button", { name: "Show all 3 sets" }));
    await user.type(screen.getByLabelText("Find a set"), "dark");
    expect(await screen.findByText("1 set matches.")).toBeTruthy();
  });
});
