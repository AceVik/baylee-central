/// <reference types="node" />
// The palette's contrast, measured from tokens.css in both schemes
// (docs/website-design.md §"Colour"): text 4.5:1 on every surface it is
// drawn on, the focus ring and a field's border 3:1. A translucent token is
// laid over the ground it stands on before it is measured.

import { readFileSync } from "node:fs";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

const css = readFileSync(join(process.cwd(), "src/styles/tokens.css"), "utf8");

/** The custom properties declared directly in the block that `opener` starts. */
function block(opener: string): Map<string, string> {
  const at = css.indexOf(opener);
  if (at < 0) throw new Error(`no block ${opener}`);
  const open = css.indexOf("{", at + opener.length - 1);
  const close = css.indexOf("}", open);
  const out = new Map<string, string>();
  for (const m of css.slice(open + 1, close).matchAll(/(--[a-z0-9-]+)\s*:\s*([^;]+);/g)) {
    out.set(m[1] ?? "", (m[2] ?? "").trim());
  }
  return out;
}

const dark = block(":root {");
const light = block(':root[data-theme="light"] {');
const lightByPreference = block(':root:not([data-theme="dark"]) {');

type Rgba = [number, number, number, number];

function parse(value: string): Rgba {
  const hex = /^#([0-9a-f]{6})$/i.exec(value);
  if (hex) {
    const n = Number.parseInt(hex[1] ?? "", 16);
    return [(n >> 16) & 255, (n >> 8) & 255, n & 255, 1];
  }
  const rgba = /^rgba\((\d+),\s*(\d+),\s*(\d+),\s*([\d.]+)\)$/.exec(value);
  if (rgba) return [Number(rgba[1]), Number(rgba[2]), Number(rgba[3]), Number(rgba[4])];
  throw new Error(`not a colour this test reads: ${value}`);
}

function over(top: Rgba, ground: Rgba): Rgba {
  const a = top[3];
  return [top[0] * a + ground[0] * (1 - a), top[1] * a + ground[1] * (1 - a), top[2] * a + ground[2] * (1 - a), 1];
}

function channel(c: number): number {
  const s = c / 255;
  return s <= 0.04045 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
}

function luminance([r, g, b]: Rgba): number {
  return 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b);
}

function ratio(a: Rgba, b: Rgba): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x) as [number, number];
  return (hi + 0.05) / (lo + 0.05);
}

/** A token as the scheme draws it: its own value, or the one `:root` gives every scheme. */
function colour(scheme: Map<string, string>, token: string): Rgba {
  const value = scheme.get(token) ?? dark.get(token);
  if (value === undefined) throw new Error(`no ${token}`);
  return parse(value);
}

const TEXTS = ["--ink", "--ink-soft", "--ink-faint", "--title", "--violet", "--link", "--ok", "--warn", "--bad", "--off", "--untested"];
const SURFACES = ["--bg", "--bg-alt", "--panel-solid", "--panel-2"];

describe.each([
  ["the blue hour", dark],
  ["sunlight", light],
])("%s", (_, scheme) => {
  it.each(TEXTS)("draws %s at 4.5:1 on every surface", (text) => {
    for (const surface of SURFACES) {
      const ground = colour(scheme, surface);
      const measured = ratio(over(colour(scheme, text), ground), ground);
      expect(measured, `${text} on ${surface}`).toBeGreaterThanOrEqual(4.5);
    }
  });

  it("writes on the gold buttons and in codes at 4.5:1", () => {
    expect(ratio(colour(scheme, "--gold-ink"), colour(scheme, "--gold"))).toBeGreaterThanOrEqual(4.5);
    expect(ratio(colour(scheme, "--gold-ink"), colour(scheme, "--gold-2"))).toBeGreaterThanOrEqual(4.5);
    expect(ratio(colour(scheme, "--code-ink"), colour(scheme, "--code-bg"))).toBeGreaterThanOrEqual(4.5);
  });

  it("draws the focus ring and a field's border at 3:1", () => {
    for (const surface of ["--bg", "--bg-alt", "--panel-solid"]) {
      const ground = colour(scheme, surface);
      expect(ratio(colour(scheme, "--focus"), ground), `focus on ${surface}`).toBeGreaterThanOrEqual(3);
    }
    const panel = colour(scheme, "--panel-solid");
    expect(ratio(over(colour(scheme, "--field-edge"), panel), panel)).toBeGreaterThanOrEqual(3);
  });
});

describe("the light scheme", () => {
  it("is the same whether the browser or ?theme= chose it", () => {
    expect([...lightByPreference.entries()]).toEqual([...light.entries()]);
  });
});

describe("a test that measures", () => {
  it("refuses a pair below the line", () => {
    expect(ratio(parse("#777777"), parse("#ffffff"))).toBeLessThan(4.5);
    expect(ratio(over(parse("rgba(255, 255, 255, 0.1)"), parse("#000000")), parse("#000000"))).toBeLessThan(3);
  });
});
