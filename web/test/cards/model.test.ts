import { describe, expect, it } from "vitest";

import { parseCards, parseSets } from "../../src/cards/model";

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
});
