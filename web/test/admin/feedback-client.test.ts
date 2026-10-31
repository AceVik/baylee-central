/// <reference types="node" />
// A report's client part, read in the browser (`admin/feedback/client.ts`):
// inflated within its output bound, strict JSON, a screenshot only when it
// is a PNG by its signature and header, card pictures only for an id shaped
// like Scryfall's.

import { gzipSync } from "node:zlib";

import { describe, expect, it } from "vitest";

import { cardImage, cardPage, fromBase64, inflate, readClient, readRefs, readScreenshot, Unreadable } from "../../src/admin/feedback/client";

const gz = (text: string | Uint8Array) => new Uint8Array(gzipSync(typeof text === "string" ? Buffer.from(text) : Buffer.from(text)));
const PIXEL = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAIAAACQd1PeAAAADElEQVR4nGNQM3ABAAEaAJsR09CMAAAAAElFTkSuQmCC";
const BOLT = "77c6fa74-5543-42ac-9ead-0e890b188e99";

describe("the client part", () => {
  it("inflates within its bound and stops past it while it streams", async () => {
    expect(new TextDecoder().decode(await inflate(gz("hello"), 5))).toBe("hello");
    // a small gzip of a large output: refused, never held whole
    const bomb = gz(new Uint8Array(4 * 1024 * 1024));
    expect(bomb.length).toBeLessThan(10_000);
    await expect(inflate(bomb, 1024 * 1024)).rejects.toThrow(Unreadable);
    await expect(inflate(new Uint8Array([0x1f, 0x8b, 1, 2, 3]))).rejects.toThrow(Unreadable);
  });

  it("is a JSON object in strict UTF-8, or unreadable", async () => {
    expect(await readClient(gz('{"build":{"version":"1"}}'))).toEqual({ build: { version: "1" } });
    for (const bad of ["[1,2]", "not json", "null"]) await expect(readClient(gz(bad))).rejects.toThrow(Unreadable);
    await expect(readClient(gz(new Uint8Array([0x7b, 0xff, 0x7d])))).rejects.toThrow(Unreadable);
  });

  it("shows a screenshot only when it is a PNG, its size from its own header", () => {
    expect(readScreenshot({ screenshot: { png_base64: PIXEL, width: 9_999, height: 9_999 } })).toMatchObject({ width: 1, height: 1 });
    const gif = Buffer.from("GIF89a....................").toString("base64");
    expect(readScreenshot({ screenshot: { png_base64: gif } })).toBeNull();
    expect(readScreenshot({ screenshot: { png_base64: "not base64!" } })).toBeNull();
    expect(readScreenshot({ screenshot: { png_base64: 7 } })).toBeNull();
    expect(fromBase64("YWJj")).toEqual(new Uint8Array([0x61, 0x62, 0x63]));
    expect(fromBase64("YWJ")).toBeNull();
  });

  it("names a card's picture and page by a Scryfall id alone", () => {
    const refs = readRefs({ refs: { cards: [{ text: "Lightning Bolt", at: [1, 17], print: { scryfall_id: BOLT } }, { text: "Evil", print: { scryfall_id: "../../evil" } }], players: [] } });
    expect(refs?.cards.map((c) => c.scryfallId)).toEqual([BOLT, null]);
    expect(cardImage(BOLT)).toBe(`https://cards.scryfall.io/normal/front/7/7/${BOLT}.jpg`);
    expect(cardPage(BOLT)).toBe(`https://scryfall.com/card/${BOLT}`);
    expect(cardImage("javascript:alert(1)")).toBeNull();
    expect(readRefs({ refs: { cards: [], players: [] } })).toBeNull();
  });
});
