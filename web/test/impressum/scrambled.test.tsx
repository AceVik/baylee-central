import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { readRun, scatter, Scrambled } from "../../src/impressum/Scrambled";

describe("the scrambled details", () => {
  it("reads right on screen and wrong in the DOM", () => {
    const field = Array.from("erika@example.org").map((c, i) => [i, c.codePointAt(0) ?? 0] as [number, number]);
    const { container } = render(<Scrambled glyphs={scatter(field)} />);
    const run = container.querySelector(".scr");
    expect(run).not.toBeNull();
    expect(run?.textContent).not.toContain("erika@example.org");
    expect(run?.textContent?.length).toBeGreaterThan("erika@example.org".length);
    expect(readRun(run as Element)).toBe("erika@example.org");
  });
});
