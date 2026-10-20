import { describe, expect, it } from "vitest";

import { DirectoryError, parsePage } from "../../src/central/directory";
import { uuidText } from "../../src/central/validate";
import { idBytes, listing, page } from "../fixtures/directory";

describe("parsePage", () => {
  it("reads a page in UUID order", () => {
    const listings = parsePage(page(0, 3), null);
    expect(listings.map((g) => g.origin)).toEqual([
      "https://gw1.example.invalid",
      "https://gw2.example.invalid",
      "https://gw3.example.invalid",
    ]);
    expect(listings[0]?.id).toBe(uuidText(idBytes(1)));
  });

  it("refuses an unordered page, an id at or before the cursor, and unknown fields", () => {
    const swapped = { result: "gateway_directory", gateways: [listing(2), listing(1)] };
    expect(() => parsePage(swapped, null)).toThrow(DirectoryError);
    expect(() => parsePage(page(0, 2), idBytes(1))).toThrow(DirectoryError);
    expect(() => parsePage({ ...page(0, 1), extra: 1 }, null)).toThrow(DirectoryError);
    const g = { ...listing(1), owner: "someone" };
    expect(() => parsePage({ result: "gateway_directory", gateways: [g] }, null)).toThrow(DirectoryError);
    expect(() => parsePage({ result: "object", gateways: [] }, null)).toThrow(DirectoryError);
  });

  it("refuses more than a page and a listing that fails its checks", () => {
    const big = { result: "gateway_directory", gateways: Array.from({ length: 101 }, (_, i) => listing(i + 1)) };
    expect(() => parsePage(big, null)).toThrow(DirectoryError);
    const foreign = listing(1);
    foreign.metadata.website = "https://elsewhere.invalid/";
    expect(() => parsePage({ result: "gateway_directory", gateways: [foreign] }, null)).toThrow(DirectoryError);
  });

  it("leaves markup in a name as text for the page to escape", () => {
    const g = listing(1, { name: "<img src=x onerror=alert(1)>" });
    expect(parsePage({ result: "gateway_directory", gateways: [g] }, null)[0]?.metadata.name).toBe("<img src=x onerror=alert(1)>");
  });
});
