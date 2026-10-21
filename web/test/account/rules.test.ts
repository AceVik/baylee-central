// The username rule and the strength hint.

import { describe, expect, it } from "vitest";

import { canonicalName, validName } from "../../src/account/names";
import { acceptable, strength } from "../../src/account/strength";

describe("the username rule", () => {
  it("is Central's: 3 to 24 of a–z, 0–9, . _ -, alphanumeric at both ends, no two separators in a row", () => {
    for (const ok of ["abc", "Synthetic_Name", "a.b-c_d", "x".repeat(24), "  padded  "]) {
      expect({ ok, valid: validName(ok) }).toEqual({ ok, valid: true });
    }
    for (const bad of ["ab", "x".repeat(25), "_abc", "abc.", "a..b", "a-_b", "äbc", "a b", "a@b", "a1", ""]) {
      expect({ bad, valid: validName(bad) }).toEqual({ bad, valid: false });
    }
    expect(canonicalName("  Synthetic_Name ")).toBe("synthetic_name");
  });
});

describe("the strength hint", () => {
  it("rates short, guessable, mixed and long passphrases", () => {
    expect(strength("short")).toBe("short");
    expect(strength("aaaaaaaaaa")).toBe("weak");
    expect(strength("abcdefgh")).toBe("weak");
    expect(strength("password123")).not.toBe("strong");
    expect(strength("synthetic_name42", "synthetic_name")).not.toBe("strong");
    expect(strength("k7#Qm2!x")).toBe("fair");
    expect(strength("violet lantern harbour quietly")).toBe("strong");
  });

  it("allows 8 characters up to 1024 bytes, counting characters, not code units", () => {
    expect(acceptable("1234567")).toBe(false);
    expect(acceptable("12345678")).toBe(true);
    expect(acceptable("🂡🂡🂡🂡🂡🂡🂡🂡")).toBe(true);
    expect(acceptable("x".repeat(1024))).toBe(true);
    expect(acceptable("x".repeat(1025))).toBe(false);
    expect(acceptable("ä".repeat(513))).toBe(false);
  });
});
