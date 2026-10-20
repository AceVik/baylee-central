import { describe, expect, it } from "vitest";

import { hex, solve } from "../../src/impressum/pow";
import { sha256 } from "../fixtures/impressum";

function hexOf(bytes: Uint8Array): string {
  return Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
}

async function reference(text: string): Promise<string> {
  const bytes = new TextEncoder().encode(text);
  return hexOf(await sha256(new Uint8Array(bytes)));
}

describe("the proof of work", () => {
  it("hashes as SHA-256 does", async () => {
    for (const text of ["", "abc", "nonce:0:12345", "x".repeat(120)]) {
      expect(hex(text)).toBe(await reference(text));
    }
  });

  it("finds the smallest counter with the asked leading zero bits for every part", async () => {
    const nonce = "AAAAAAAAAAAAAAAAAAAAAA";
    const counters = solve(nonce, 3, 8);
    expect(counters).toHaveLength(3);
    for (const [part, c] of counters.entries()) {
      expect((await reference(`${nonce}:${part}:${c}`)).slice(0, 2)).toBe("00");
      for (let smaller = 0; smaller < c; smaller++) {
        expect(hex(`${nonce}:${part}:${smaller}`).slice(0, 2)).not.toBe("00");
      }
    }
  });
});
