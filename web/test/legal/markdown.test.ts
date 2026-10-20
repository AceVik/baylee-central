// The website reads legal texts as the Baylee client does
// (`crates/baylee-client-core/src/terms.rs`); these mirror that file's cases,
// with links drawn as links instead of "label (url)".

import { describe, expect, it } from "vitest";

import { inline, parse, readDocument } from "../../src/legal/markdown";

const plain = (text: string) => ({ text, bold: false, italic: false });

describe("legal Markdown", () => {
  it("reads headings, paragraphs, lists and rules", () => {
    const blocks = parse("# Title\n\nOne line\nand the next.\n\n## Part\n\n- one\n- two\n  continued\n\n1. first\n2) second\n\n---\n");
    expect(blocks).toEqual([
      { kind: "heading", level: 1, spans: [plain("Title")] },
      { kind: "paragraph", spans: [plain("One line and the next.")] },
      { kind: "heading", level: 2, spans: [plain("Part")] },
      { kind: "item", number: null, spans: [plain("one")] },
      { kind: "item", number: null, spans: [plain("two continued")] },
      { kind: "item", number: 1, spans: [plain("first")] },
      { kind: "item", number: 2, spans: [plain("second")] },
      { kind: "rule" },
    ]);
  });

  it("drops comment lines and never reads HTML", () => {
    expect(parse("<!-- version: 1 -->\n<b>bold?</b>")).toEqual([{ kind: "paragraph", spans: [plain("<b>bold?</b>")] }]);
  });

  it("styles bold and italic, keeps an underscore inside a word", () => {
    expect(inline("**fair** and *kind*, snake_case")).toEqual([
      { text: "fair", bold: true, italic: false },
      plain(" and "),
      { text: "kind", bold: false, italic: true },
      plain(", snake_case"),
    ]);
    expect(inline("\\*not italic\\*")).toEqual([plain("*not italic*")]);
  });

  it("links only to https and mailto, anything else stays text", () => {
    expect(inline("[Impressum](https://baylee.app/impressum)")).toEqual([
      { text: "Impressum", bold: false, italic: false, href: "https://baylee.app/impressum" },
    ]);
    // As the client reads it: the address ends at the first `)`.
    expect(inline("[x](javascript:alert(1))")).toEqual([plain("x (javascript:alert(1))")]);
    expect(inline("<mailto:a@example.org>")).toEqual([
      { text: "mailto:a@example.org", bold: false, italic: false, href: "mailto:a@example.org" },
    ]);
    expect(inline("<http://plain.example>")).toEqual([plain("http://plain.example")]);
  });

  it("reads a legal file's leading comments", () => {
    const doc = readDocument("<!-- version: draft -->\n<!-- updated: 2026-10-20 -->\n\n# Terms\n");
    expect(doc).toEqual({ version: "draft", updated: "2026-10-20", markdown: "# Terms\n" });
    expect(readDocument("# Terms\n<!-- version: late -->").version).toBeNull();
  });
});
