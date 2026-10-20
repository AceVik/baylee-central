import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import TermsPage, { TERMS } from "../../src/legal/TermsPage";

describe("the terms page", () => {
  it("shows Central's terms from legal/, and says a draft is not in force", () => {
    render(<TermsPage />);
    expect(TERMS.de.version).toBe(TERMS.en.version);
    expect(document.querySelector(".legal-text h2")).not.toBeNull();
    // A draft says it is not in force; a published version names itself.
    const said =
      TERMS.en.version === "draft" ? screen.getByRole("note").textContent : document.querySelector(".legal-date")?.textContent;
    expect(said).toMatch(TERMS.en.version === "draft" ? /not in force|gelten noch nicht/ : /Version|Fassung/);
    const links = Array.from(document.querySelectorAll<HTMLAnchorElement>(".legal-text a"));
    expect(links.length).toBeGreaterThan(0);
    for (const link of links) {
      expect(link.href.startsWith("https://") || link.href.startsWith("mailto:")).toBe(true);
      expect(link.rel).toBe("noopener noreferrer nofollow");
    }
  });
});
