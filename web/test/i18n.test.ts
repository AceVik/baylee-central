import { describe, expect, it } from "vitest";

import { pickLang } from "../src/i18n";

describe("the language", () => {
  it("is ?lang=, then the remembered choice, then the browser's", () => {
    expect(pickLang("?lang=de", "en", ["en-GB"])).toBe("de");
    expect(pickLang("?lang=fr", "de", ["en-GB"])).toBe("de");
    expect(pickLang("", null, ["fr-FR", "de-AT", "en"])).toBe("de");
    expect(pickLang("", null, ["fr-FR"])).toBe("en");
  });
});
