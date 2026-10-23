// The username rule, the display name's and the tag's, and the strength hint.

import { describe, expect, it } from "vitest";

import { canonicalName, handle, reservedDisplayName, tagText, validDisplayName, validName, validTag } from "../../src/account/names";
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

describe("the display name and the tag", () => {
  it("keep the gateway's rule, the table central-api and Baylee test too", () => {
    for (const [name, valid] of [
      ["Alice", true],
      ["player_one-99", true],
      ["ab", false],
      ["has space", false],
      ["emoji🎉", false],
      ["sixteen_chars_yz", true],
      ["seventeen_chars_x", false],
      ["alice@example.com", false],
      ["Alice#af03", false],
      ["_Alice", false],
      ["Alice-", false],
      ["abc", true],
      ["a_-b", true],
      ["Zoë", false],
      ["abc\n", false],
    ] as const) {
      expect({ name, valid: validDisplayName(name) }).toEqual({ name, valid });
    }
  });

  it("keep Central's reserved names for Baylee, read without case, _ and -", () => {
    for (const name of ["admin", "Administrator", "Bay-lee", "c_e_n_t_r_a_l", "MODERATOR", "Official", "sup-port", "System"]) {
      expect({ name, reserved: reservedDisplayName(name) }).toEqual({ name, reserved: true });
    }
    for (const name of ["admins", "Admin1", "Baylee2", "Alice"]) {
      expect({ name, reserved: reservedDisplayName(name) }).toEqual({ name, reserved: false });
    }
  });

  it("draw a tag as the client does: lowercase hex, four digits at least", () => {
    expect(tagText(7)).toBe("0007");
    expect(tagText(0x3fa2)).toBe("3fa2");
    expect(tagText(0x10000)).toBe("10000");
    expect(tagText(2 ** 31 - 1)).toBe("7fffffff");
    expect(handle("Alice", 7)).toBe("Alice#0007");
    expect([0, 1, 2 ** 31 - 1, 2 ** 31, 1.5].map(validTag)).toEqual([false, true, true, false, false]);
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
