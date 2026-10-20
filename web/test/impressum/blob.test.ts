import { describe, expect, it } from "vitest";

import { FIELDS, SCRIPT_KEY, b64urlBytes, decode } from "../../src/impressum/blob";
import { SYNTHETIC, encode } from "../fixtures/impressum";

function joined(pairs: [number, number][]): string {
  const chars: string[] = [];
  for (const [pos, cp] of pairs) chars[pos] = String.fromCodePoint(cp);
  return chars.join("");
}

function hexOf(bytes: Uint8Array): string {
  return Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
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
