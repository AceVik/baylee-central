// The Markdown subset of legal texts (`legal/README.md`), read as the Baylee
// client reads it (Baylee's `crates/baylee-client-core/src/terms.rs`, `parse`
// and `inline`), so a text looks alike on the website and in the game:
// `#` and `##`..`######` headings, paragraphs (lines joined until a blank
// line), `-`/`*`/`+` and `1.`/`1)` list items (an indented line continues
// one), `---`/`***`/`___` rules, `**bold**`, `*italic*`/`_italic_` (an `_`
// inside a word is a letter), backslash escapes, `[label](url)` and
// `<https://…>`/`<mailto:…>`. Comment lines are dropped. No HTML is ever
// interpreted: everything else is text. Where the client writes a link as
// "label (url)", the website makes it a link, and only to `https:` or
// `mailto:`; any other address stays text.

export interface Span {
  text: string;
  bold: boolean;
  italic: boolean;
  /** The address of a link: `https:` or `mailto:` only. */
  href?: string;
}

export type Block =
  | { kind: "heading"; level: 1 | 2; spans: Span[] }
  | { kind: "paragraph"; spans: Span[] }
  | { kind: "item"; number: number | null; spans: Span[] }
  | { kind: "rule" };

function isRule(line: string): boolean {
  const compact = line.replace(/\s/g, "");
  return compact.length >= 3 && (/^-+$/.test(compact) || /^\*+$/.test(compact) || /^_+$/.test(compact));
}

function heading(line: string): [1 | 2, string] | null {
  const hashes = /^#*/.exec(line)?.[0].length ?? 0;
  if (hashes === 0 || hashes > 6 || line[hashes] !== " ") return null;
  const rest = line
    .slice(hashes + 1)
    .trim()
    .replace(/#+$/, "")
    .trim();
  return [hashes === 1 ? 1 : 2, rest];
}

function ordered(line: string): [number, string] | null {
  const m = /^(\d{1,9})(?:\. |\) )(.*)$/.exec(line);
  if (!m) return null;
  return [Number.parseInt(m[1] ?? "0", 10), (m[2] ?? "").trim()];
}

function safeHref(url: string): string | undefined {
  return url.startsWith("https://") || url.startsWith("mailto:") ? url : undefined;
}

const ASCII_PUNCTUATION = /[!-/:-@[-`{-~]/;
const ALNUM = /[\p{L}\p{N}]/u;

/** The inline styles of one line. */
export function inline(text: string): Span[] {
  const spans: Span[] = [];
  let bold = false;
  let italic = false;
  let word = "";
  const push = (href?: string) => {
    if (word === "") return;
    const last = spans[spans.length - 1];
    if (last && last.bold === bold && last.italic === italic && last.href === undefined && href === undefined) {
      last.text += word;
    } else {
      spans.push(href === undefined ? { text: word, bold, italic } : { text: word, bold, italic, href });
    }
    word = "";
  };
  const chars = Array.from(text);
  let i = 0;
  while (i < chars.length) {
    const c = chars[i] ?? "";
    const next = chars[i + 1];
    if (c === "\\" && next !== undefined && ASCII_PUNCTUATION.test(next)) {
      word += next;
      i += 2;
    } else if (c === "*" && next === "*") {
      push();
      bold = !bold;
      i += 2;
    } else if (
      (c === "*" || c === "_") &&
      (c === "*" || !(i > 0 && ALNUM.test(chars[i - 1] ?? "") && next !== undefined && ALNUM.test(next)))
    ) {
      push();
      italic = !italic;
      i += 1;
    } else if (c === "[") {
      const close = chars.indexOf("]", i + 1);
      const end = close > 0 && chars[close + 1] === "(" ? chars.indexOf(")", close + 2) : -1;
      if (end > 0) {
        const label = chars.slice(i + 1, close).join("");
        const url = chars
          .slice(close + 2, end)
          .join("")
          .trim();
        const href = safeHref(url);
        push();
        word = label;
        if (href) {
          push(href);
        } else if (url !== "" && url !== label) {
          word += ` (${url})`;
        }
        i = end + 1;
      } else {
        word += c;
        i += 1;
      }
    } else if (c === "<") {
      const end = chars.indexOf(">", i + 1);
      const inner = end > 0 ? chars.slice(i + 1, end).join("") : "";
      if (end > 0 && (inner.startsWith("http://") || inner.startsWith("https://") || inner.startsWith("mailto:"))) {
        const href = safeHref(inner);
        push();
        word = inner;
        if (href) push(href);
        i = end + 1;
      } else {
        word += c;
        i += 1;
      }
    } else {
      word += c;
      i += 1;
    }
  }
  push();
  return spans;
}

/** The text of `markdown` as blocks. */
export function parse(markdown: string): Block[] {
  const blocks: Block[] = [];
  let paragraph: string[] = [];
  const flush = () => {
    if (paragraph.length > 0) {
      blocks.push({ kind: "paragraph", spans: inline(paragraph.join(" ")) });
      paragraph = [];
    }
  };
  for (const raw of markdown.split(/\r?\n/)) {
    const line = raw.trim();
    if ((line.startsWith("<!--") && line.endsWith("-->")) || line === "") {
      flush();
      continue;
    }
    if (isRule(line)) {
      flush();
      blocks.push({ kind: "rule" });
      continue;
    }
    const head = heading(line);
    if (head) {
      flush();
      blocks.push({ kind: "heading", level: head[0], spans: inline(head[1]) });
      continue;
    }
    const bullet = /^[-*+] (.*)$/.exec(line);
    if (bullet) {
      flush();
      blocks.push({ kind: "item", number: null, spans: inline((bullet[1] ?? "").trim()) });
      continue;
    }
    const number = ordered(line);
    if (number) {
      flush();
      blocks.push({ kind: "item", number: number[0], spans: inline(number[1]) });
      continue;
    }
    const last = blocks[blocks.length - 1];
    if (/^[ \t]/.test(raw) && paragraph.length === 0 && last?.kind === "item") {
      for (const span of [{ text: " ", bold: false, italic: false } as Span, ...inline(line)]) {
        const tail = last.spans[last.spans.length - 1];
        if (tail && tail.bold === span.bold && tail.italic === span.italic && !tail.href && !span.href) {
          tail.text += span.text;
        } else {
          last.spans.push(span);
        }
      }
      continue;
    }
    paragraph.push(line);
  }
  flush();
  return blocks;
}

/** A legal file's leading `<!-- key: value -->` lines, and the text after them. */
export interface LegalDocument {
  version: string | null;
  updated: string | null;
  markdown: string;
}

export function readDocument(source: string): LegalDocument {
  let version: string | null = null;
  let updated: string | null = null;
  const lines = source.split(/\r?\n/);
  let first = 0;
  for (; first < lines.length; first++) {
    const m = /^\s*<!--\s*(.*?)\s*-->\s*$/.exec(lines[first] ?? "");
    if (!m) break;
    const [key, ...rest] = (m[1] ?? "").split(":");
    const value = rest.join(":").trim();
    if (key?.trim() === "version") version = value;
    if (key?.trim() === "updated") updated = value;
  }
  return {
    version,
    updated,
    markdown: lines.slice(first).join("\n").replace(/^\n+/, ""),
  };
}

/** The version that is never in force (`docs/terms.md`). */
export const DRAFT = "draft";
