import { describe, expect, it } from "vitest";

import { begun, eras, finished, pct } from "../../src/cards/numbers";

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

  it("groups sets into eras of four years in release order", () => {
    const out = eras([set("lea", "1993-08-05"), set("mir", "1996-10-08"), set("tmp", "1997-10-14")]);
    expect(out.map((e) => [e.from, e.to, e.sets.length])).toEqual([
      [1993, 1996, 2],
      [1997, 1997, 1],
    ]);
  });
});
