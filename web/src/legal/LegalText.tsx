// Draws `markdown.ts`'s blocks as plain elements: headings one level under
// the page's own, paragraphs, lists, rules, bold, italic and links. Text is
// always text (React escapes it); a link is drawn only for the `https:` and
// `mailto:` addresses `markdown.ts` let through, and leaves the site with
// `rel="noopener noreferrer nofollow"`.

import type { ReactNode } from "react";
import { parse, type Block, type Span } from "./markdown";

function spans(list: Span[]): ReactNode[] {
  return list.map((span, i) => {
    let node: ReactNode = span.text;
    if (span.italic) node = <em>{node}</em>;
    if (span.bold) node = <strong>{node}</strong>;
    if (span.href)
      node = (
        <a href={span.href} rel="noopener noreferrer nofollow">
          {node}
        </a>
      );
    return <span key={i}>{node}</span>;
  });
}

function group(blocks: Block[]): ReactNode[] {
  const out: ReactNode[] = [];
  let items: Extract<Block, { kind: "item" }>[] = [];
  const close = () => {
    if (items.length === 0) return;
    const numbered = items[0]?.number !== null;
    const li = items.map((item, i) => <li key={i}>{spans(item.spans)}</li>);
    const start = items[0]?.number ?? 1;
    out.push(
      numbered ? (
        <ol key={`l${out.length}`} start={start}>
          {li}
        </ol>
      ) : (
        <ul key={`l${out.length}`}>{li}</ul>
      ),
    );
    items = [];
  };
  for (const block of blocks) {
    if (block.kind === "item") {
      if (items.length > 0 && (items[0]?.number === null) !== (block.number === null)) close();
      items.push(block);
      continue;
    }
    close();
    const key = `b${out.length}`;
    switch (block.kind) {
      case "heading":
        out.push(block.level === 1 ? <h2 key={key}>{spans(block.spans)}</h2> : <h3 key={key}>{spans(block.spans)}</h3>);
        break;
      case "paragraph":
        out.push(<p key={key}>{spans(block.spans)}</p>);
        break;
      case "rule":
        out.push(<hr key={key} />);
        break;
    }
  }
  close();
  return out;
}

export function LegalText({ text }: { text: string }) {
  return <div className="legal-text">{group(parse(text))}</div>;
}
