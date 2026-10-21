import { render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { forgetLoaded, loadCards } from "../../src/cards/data";
import { SetsBlock } from "../../src/cards/SetsBlock";
import { RELEASE_DATA } from "../../src/config";
import { LangProvider } from "../../src/i18n";
import { serve } from "../support/network";

afterEach(() => forgetLoaded());

const SETS = {
  corpus: 100,
  rev: "9c6eb0649db3e245b2767c97c4d7cf70a71c875f",
  generated_at: "2026-10-21T12:00:00Z",
  totals: { total: 100, implemented: 60, generated: 20, partial: 5, unimplemented: 5, absent: 8, never: 2 },
  sets: [{ code: "lea", name: "Limited Edition Alpha", released: "1993-08-05", position: 0, total: 100, implemented: 60, generated: 20, partial: 5, unimplemented: 5, absent: 8, never: 2 }],
};

const RELEASES = [
  { tag_name: "v0.2.0-draft", draft: true, html_url: "https://github.com/AceVik/baylee/releases/tag/v0.2.0-draft", published_at: "2026-10-22T00:00:00Z", assets: [{ name: "a.zip", size: 1, browser_download_url: "https://github.com/AceVik/baylee/releases/download/v0.2.0-draft/a.zip" }] },
  { tag_name: "v0.1.0-beta.8a", prerelease: true, html_url: "https://github.com/AceVik/baylee/releases/tag/v0.1.0-beta.8a", published_at: "2026-10-21T00:00:00Z", assets: [{ name: "a.zip", size: 1, browser_download_url: "https://github.com/AceVik/baylee/releases/download/v0.1.0-beta.8a/a.zip" }] },
];

function block() {
  return render(
    <LangProvider initial="en">
      <SetsBlock hash="" />
    </LangProvider>,
  );
}

describe("the card status of the newest release", () => {
  it("is read at the tag of the release the download offers, with playable first", async () => {
    const base = RELEASE_DATA("v0.1.0-beta.8a");
    expect(base).toBe("https://raw.githubusercontent.com/AceVik/baylee/v0.1.0-beta.8a/web/site/data/");
    const asked: string[] = [];
    serve((c) => {
      asked.push(c.url);
      if (c.url.startsWith("https://api.github.com/repos/AceVik/baylee/releases")) return { body: RELEASES };
      if (c.url === `${base}sets.json`) return { body: SETS };
      if (c.url === `${base}sets/set-lea.json`) return { body: { cards: [] } };
      return undefined;
    });
    block();
    // playable = full + untested (80 of the 98 that will ever be built), then the parts
    expect(await screen.findByText(/80 of 98 cards playable \(82 %\): 60 full, 20 untested/)).toBeTruthy();
    expect(screen.getByText(/Release v0\.1\.0-beta\.8a, commit 9c6eb0649/)).toBeTruthy();
    await loadCards("lea");
    expect(asked).toContain(`${base}sets/set-lea.json`);
    expect(asked).not.toContain("/data/sets.json");
  });

  it("falls back to the bundled copy when the release carries none", async () => {
    serve((c) => {
      if (c.url.startsWith("https://api.github.com/repos/AceVik/baylee/releases")) return { body: RELEASES };
      if (c.url === "/data/sets.json") return { body: SETS };
      return undefined;
    });
    block();
    expect(await screen.findByText(/80 of 98 cards playable/)).toBeTruthy();
    expect(screen.getByText(/Measured at commit 9c6eb0649/)).toBeTruthy();
  });
});
