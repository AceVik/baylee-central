// The policy the site is served with (scripts/csp.ts): GitHub only at the
// repository the site reads, and nothing it does not need.

import { describe, expect, it } from "vitest";

import { REPO as SITE_REPO } from "../src/config";
import { csp, PERMISSIONS, REPO } from "../scripts/csp";

describe("the content security policy", () => {
  const directives = new Map(csp().split("; ").map((d) => [d.split(" ")[0], d.split(" ").slice(1)]));

  it("names the repository the site reads, and GitHub nowhere else", () => {
    expect(REPO).toBe(SITE_REPO);
    const github = (directives.get("connect-src") ?? []).filter((s) => s.includes("github"));
    expect(github).toEqual([`https://api.github.com/repos/${REPO}`, `https://api.github.com/repos/${REPO}/`, `https://raw.githubusercontent.com/${REPO}/`]);
  });

  it("takes no picture from a data: URL, no script but its own, and no frame", () => {
    // blob: is a picture the page made from bytes it checked (a report's screenshot, a PNG)
    expect(directives.get("img-src")).toEqual(["'self'", "blob:", "https://cards.scryfall.io"]);
    expect(csp()).not.toContain("data:");
    expect(directives.get("script-src")).toEqual(["'self'", "'wasm-unsafe-eval'"]);
    expect(directives.get("frame-ancestors")).toEqual(["'none'"]);
  });

  it("switches off the browser features the site never uses", () => {
    for (const feature of ["camera", "microphone", "geolocation", "payment"]) expect(PERMISSIONS).toContain(`${feature}=()`);
  });
});
