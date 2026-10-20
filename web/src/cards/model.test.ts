import { describe, expect, it } from "vitest";

import { begun, eras, finished, imgOf, pageOf, parseCards, parseSets, pct, why, type Card } from "./model";

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

function set(code: string, released: string) {
  return { code, name: code, released, position: 0, total: 1, implemented: 0, generated: 0, partial: 0, unimplemented: 0, absent: 1, never: 0 };
}

describe("numbers", () => {
  it("never rounds a percentage up to 100 or down to 0 by a hair", () => {
    expect(pct(0, 0)).toBe(100);
    expect(pct(1, 1000)).toBe(1);
    expect(pct(999, 1000)).toBe(99);
    expect(pct(1000, 1000)).toBe(100);
    expect(pct(0, 10)).toBe(0);
  });

  it("counts a set as finished without its never-built cards", () => {
    const s = { total: 290, implemented: 286, generated: 0, partial: 0, unimplemented: 0, absent: 0, never: 4 };
    expect(finished(s)).toBe(true);
    expect(finished({ ...s, implemented: 285, partial: 1 })).toBe(false);
    expect(begun({ ...s, implemented: 0, generated: 1 })).toBe(true);
  });
});

describe("parsing", () => {
  it("reads sets.json, counting a missing `generated` as 0 and skipping a bad code", () => {
    const doc = parseSets({
      corpus: 10,
      rev: "4e984e84a14e",
      generated_at: "2026-10-14T22:15:34Z",
      totals: { total: 10, implemented: 3, partial: 1, unimplemented: 1, absent: 5, never: 0 },
      sets: [
        { code: "lea", name: "Limited Edition Alpha", released: "1993-08-05", total: 5, implemented: 3, partial: 1, unimplemented: 1, absent: 0, never: 0, position: 0 },
        { code: "../x", name: "Bad", released: "1994-01-01", total: 5, implemented: 0, partial: 0, unimplemented: 0, absent: 5, never: 0, position: 1 },
      ],
    });
    expect(doc?.totals.generated).toBe(0);
    expect(doc?.sets.map((s) => s.code)).toEqual(["lea"]);
  });

  it("reads a set's cards and drops one with an unknown status", () => {
    const cards = parseCards({
      cards: [
        { name: "A", oracle_id: "o1", status: "implemented", img: "a27efec0-40c4-48bc-a21a-3af28a6529b5" },
        { name: "B", oracle_id: "o2", status: "weird" },
      ],
    });
    expect(cards?.map((c) => c.name)).toEqual(["A"]);
  });

  it("groups sets into eras of four years in release order", () => {
    const out = eras([set("lea", "1993-08-05"), set("mir", "1996-10-08"), set("tmp", "1997-10-14")]);
    expect(out.map((e) => [e.from, e.to, e.sets.length])).toEqual([
      [1993, 1996, 2],
      [1997, 1997, 1],
    ]);
  });
});

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
