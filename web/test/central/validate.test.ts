import { describe, expect, it } from "vitest";

import { isUuidV7, uuidBytes, uuidText, validMetadata, validOrigin } from "../../src/central/validate";
import { idBytes } from "../fixtures/directory";

describe("ids", () => {
  it("round-trips a UUIDv7 between Central's bytes and text", () => {
    const bytes = idBytes(7);
    const text = uuidText(bytes);
    expect(text).toBe("019a2b3c-4d5e-7000-8000-000000000007");
    expect(uuidBytes(text)).toEqual(bytes);
    expect(isUuidV7(bytes)).toBe(true);
  });

  it("refuses another version or variant, as central-client does", () => {
    const v4 = idBytes(1);
    v4[6] = 0x40;
    expect(isUuidV7(v4)).toBe(false);
    const ncs = idBytes(1);
    ncs[8] = 0x00;
    expect(isUuidV7(ncs)).toBe(false);
    expect(uuidBytes("019A2B3C-4D5E-7000-8000-000000000007")).toBeNull();
  });
});

describe("origins and metadata", () => {
  it("takes only canonical HTTPS origins", () => {
    expect(validOrigin("https://gateway.invalid")).toBe(true);
    for (const bad of [
      "http://gateway.invalid",
      "https://u@gateway.invalid",
      "https://gateway.invalid/",
      "https://GATEWAY.invalid",
      "https://gateway.invalid/path",
      "https://gateway.invalid:443",
      "https://gateway.invalid?x",
      "javascript:alert(1)",
      "",
    ]) {
      expect({ bad, valid: validOrigin(bad) }).toEqual({ bad, valid: false });
    }
  });

  it("takes numeric loopback only on a page on loopback", () => {
    expect(validOrigin("http://127.0.0.1:9876")).toBe(false);
    expect(validOrigin("http://127.0.0.1:9876", true)).toBe(true);
    expect(validOrigin("http://[::1]:9876", true)).toBe(true);
    expect(validOrigin("http://localhost:9876", true)).toBe(false);
    expect(validOrigin("http://127.0.0.1", true)).toBe(false);
    expect(validOrigin("http://127.0.0.1:0", true)).toBe(false);
    expect(validOrigin("http://10.0.0.1:9876", true)).toBe(false);
    expect(validOrigin("https://gateway.invalid", true)).toBe(true);
  });

  it("keeps a website on its gateway's origin, without query or credentials", () => {
    const meta = { name: "Synthetic", version: "1.2.3-test", website: "https://gateway.invalid/status" };
    expect(validMetadata(meta, "https://gateway.invalid")).toBe(true);
    for (const website of [
      "https://foreign.invalid/",
      "https://u@gateway.invalid/",
      "https://gateway.invalid/?next=elsewhere",
      "https://gateway.invalid/#x",
      "https://gateway.invalid",
      "javascript:alert(1)",
    ]) {
      expect({ website, valid: validMetadata({ ...meta, website }, "https://gateway.invalid") }).toEqual({ website, valid: false });
    }
  });

  it("bounds the name and the version", () => {
    const ok = { name: "Synthetic", version: "1.2.3", website: "https://g.invalid/" };
    expect(validMetadata({ ...ok, name: "" }, "https://g.invalid")).toBe(false);
    expect(validMetadata({ ...ok, name: " padded" }, "https://g.invalid")).toBe(false);
    expect(validMetadata({ ...ok, name: "bell\u0007" }, "https://g.invalid")).toBe(false);
    expect(validMetadata({ ...ok, name: "é".repeat(41) }, "https://g.invalid")).toBe(false);
    expect(validMetadata({ ...ok, name: "é".repeat(40) }, "https://g.invalid")).toBe(true);
    expect(validMetadata({ ...ok, version: "1.0 beta" }, "https://g.invalid")).toBe(false);
    expect(validMetadata({ ...ok, version: "x".repeat(65) }, "https://g.invalid")).toBe(false);
  });
});
