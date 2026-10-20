import { describe, expect, it } from "vitest";

import { FIELDS, SCRIPT_KEY, b64urlBytes, decode } from "./blob";
import { hex, solve } from "./pow";
import { SYNTHETIC, encode, sha256 } from "./testing";

function joined(pairs: [number, number][]): string {
  const chars: string[] = [];
  for (const [pos, cp] of pairs) chars[pos] = String.fromCodePoint(cp);
  return chars.join("");
}

function hexOf(bytes: Uint8Array): string {
  return Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
}

async function reference(text: string): Promise<string> {
  const bytes = new TextEncoder().encode(text);
  return hexOf(await sha256(new Uint8Array(bytes)));
}

describe("the Impressum blob", () => {
  it("decodes what Baylee's encoder writes", async () => {
    const fields = await decode(await encode(SYNTHETIC));
    for (const key of FIELDS) expect(joined(fields[key])).toBe(SYNTHETIC[key]);
  });

  it("keeps the script's key byte for byte (Baylee's crates/baylee-tools/src/site/impressum.rs)", () => {
    expect(hexOf(Uint8Array.from(SCRIPT_KEY))).toBe("6b1f0c9e52d84a37b0e1f6a2c9d35e48");
  });

  it("holds no address as text: no @, no dot, none of the details", async () => {
    const blob = await encode(SYNTHETIC);
    expect(blob).not.toMatch(/[@.]/);
    for (const value of Object.values(SYNTHETIC)) expect(blob).not.toContain(value);
  });

  it("refuses anything that is no blob", async () => {
    await expect(decode("bm90IGEgYmxvYg")).rejects.toThrow("not an Impressum blob");
    const wrong = b64urlBytes(await encode(SYNTHETIC));
    wrong[4] = 1;
    let bin = "";
    for (const b of wrong) bin += String.fromCharCode(b);
    await expect(decode(btoa(bin))).rejects.toThrow("not an Impressum blob");
  });
});

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
