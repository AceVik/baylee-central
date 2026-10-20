import { describe, expect, it } from "vitest";

import { imgOf, pageOf, why } from "../../src/cards/card";
import type { Card } from "../../src/cards/model";

function card(over: Partial<Card> = {}): Card {
  return {
    name: "Air Elemental",
    oracleId: "7744bae4-a8b7-44a5-9b4c-0048ad4cc448",
    status: "implemented",
    note: "",
    img: "a27efec0-40c4-48bc-a21a-3af28a6529b5",
    uri: null,
    artist: null,
    ...over,
  };
}

describe("pictures and pages", () => {
  it("pictures only a pool card with a Scryfall printing id, from Scryfall's image host", () => {
    expect(imgOf(card(), "small")).toBe("https://cards.scryfall.io/small/front/a/2/a27efec0-40c4-48bc-a21a-3af28a6529b5.jpg");
    expect(imgOf(card({ status: "absent" }), "small")).toBeNull();
    expect(imgOf(card({ img: null }), "normal")).toBeNull();
    expect(imgOf(card({ img: "../../evil" }), "normal")).toBeNull();
    expect(imgOf(card({ status: "never" }), "normal")).not.toBeNull();
  });

  it("links the printing's page, else Scryfall's search for the card", () => {
    expect(pageOf(card({ uri: "https://scryfall.com/card/lea/1/air-elemental" }))).toBe("https://scryfall.com/card/lea/1/air-elemental");
    expect(pageOf(card({ uri: "https://evil.invalid/" }))).toBe("https://scryfall.com/search?q=oracleid%3A7744bae4-a8b7-44a5-9b4c-0048ad4cc448");
  });

  it("gives the reason in both languages", () => {
    expect(why(card({ status: "never", note: "ante" }), "en")).toContain("CR 407");
    expect(why(card({ status: "partial", note: "the second ability" }), "de")).toContain("the second ability");
  });
});
