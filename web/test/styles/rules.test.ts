/// <reference types="node" />
// Rules the stylesheets and the page shell must keep, checked as text.

import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

const read = (path: string) => readFileSync(join(process.cwd(), path), "utf8");

/** The declarations of every rule whose selector names `cls`. */
function rulesFor(css: string, cls: RegExp): string[] {
  const out: string[] = [];
  for (const m of css.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
    if (cls.test(m[1] ?? "")) out.push(m[2] ?? "");
  }
  return out;
}

describe("card pictures", () => {
  it("are never filtered, faded, blended or tinted (Baylee's docs/legal.md §3)", () => {
    for (const file of readdirSync("src/styles")) {
      const css = read(join("src/styles", file));
      for (const body of rulesFor(css, /\.(ctile-pic|cp-pic)\b/)) {
        expect(`${file}: ${body}`).not.toMatch(/\b(filter|opacity|mix-blend-mode|backdrop-filter)\s*:/);
      }
    }
  });
});

describe("the page shell", () => {
  it("has no inline script or style, which the CSP would refuse", () => {
    const html = read("index.html");
    expect(html).not.toMatch(/<script(?![^>]*\bsrc=)[^>]*>/);
    expect(html).not.toMatch(/<style\b/);
    expect(html).not.toMatch(/\sstyle=/);
    expect(html).not.toMatch(/\son[a-z]+=/);
  });

  it("reserves text and data mining in machine-readable form", () => {
    const reservation = JSON.parse(read("public/.well-known/tdmrep.json"));
    expect(reservation).toEqual([{ location: "/", "tdm-reservation": 1 }]);
    expect(read("index.html")).toContain('<meta name="tdm-reservation" content="1" />');
    const robots = read("public/robots.txt");
    for (const agent of ["GPTBot", "ClaudeBot", "Google-Extended", "CCBot"]) {
      expect(robots).toContain(`User-agent: ${agent}`);
    }
  });

  it("asks robots to leave the Impressum alone", () => {
    const robots = read("public/robots.txt");
    expect(robots).toContain("Disallow: /impressum");
    expect(robots).toContain("Disallow: /site/impressum/");
  });
});

describe("the palette", () => {
  // docs/website-design.md §"Colour": the artwork keeps its own colours
  const ARTWORK = new Set(["memorial-book.css", "remembrance.css", "hero.css", "world.css", "tokens.css"]);

  it("is named in tokens.css alone, besides the artwork's own colours", () => {
    for (const file of readdirSync("src/styles")) {
      if (ARTWORK.has(file)) continue;
      const css = read(join("src/styles", file)).replace(/\/\*[\s\S]*?\*\//g, "");
      expect(`${file}: ${css.match(/#[0-9a-f]{3,8}\b/gi)?.join(", ") ?? ""}`).toBe(`${file}: `);
    }
  });
});

describe("motion", () => {
  it("stills every animation and transition under prefers-reduced-motion", () => {
    const base = read("src/styles/base.css");
    const still = /@media \(prefers-reduced-motion: reduce\) \{\s*\*, \*::before, \*::after \{ animation: none !important; transition: none !important;/;
    expect(base).toMatch(still);
  });
});
