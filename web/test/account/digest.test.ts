// The terms digest, against a value computed apart from the page (Python's
// hashlib over central-api's layout): SHA-256 of `baylee-terms-v1\0` and,
// per text, its tag, date and Markdown, each after its length as eight
// big-endian bytes.

import { describe, expect, it } from "vitest";

import { equalBytes, termsDigest } from "../../src/account/digest";

const hex = (b: Uint8Array) => Array.from(b, (x) => x.toString(16).padStart(2, "0")).join("");

describe("the terms digest", () => {
  it("matches the independent value, UTF-8 and a missing date included", async () => {
    const digest = await termsDigest([
      { lang: "de", updated: "2026-10-20", markdown: "# Nutzungsbedingungen\n\nGrüße" },
      { lang: "en", updated: null, markdown: "# Terms\n" },
    ]);
    expect(hex(digest)).toBe("ce6970aaace431ca2ffda25357e3beeeb807585eb694b74a735d794647554569");
  });

  it("follows every byte and the order of the texts", async () => {
    const a = { lang: "de", updated: null, markdown: "# A" };
    const b = { lang: "en", updated: null, markdown: "# B" };
    const base = await termsDigest([a, b]);
    expect(equalBytes(base, await termsDigest([a, b]))).toBe(true);
    expect(equalBytes(base, await termsDigest([b, a]))).toBe(false);
    expect(equalBytes(base, await termsDigest([a, { ...b, markdown: "# C" }]))).toBe(false);
    expect(equalBytes(base, await termsDigest([{ ...a, markdown: "# A#" }, { ...b, markdown: " B" }]))).toBe(false);
    expect(equalBytes(base, await termsDigest([a, { ...b, updated: "" }]))).toBe(true);
  });
});
