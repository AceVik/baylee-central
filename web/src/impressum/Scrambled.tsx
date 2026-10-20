// A field drawn so the screen reads right and the DOM does not. Each
// character is a span of its own carrying its position as an order class
// (.o0 … .o63), and the spans go into the DOM shuffled, among decoys (.scr-x)
// that take no room on screen but are in textContent and innerText like the
// rest. The flexbox puts the real characters back in order for the eye; a
// crawler that reads the DOM as text, the accessibility tree included (the
// block is aria-hidden), gets a jumble. The characters are put back together
// only in a click handler, for the clipboard or a mailto:/tel: navigation,
// and dropped with it: no href ever holds an address.

import { MAX_LEN, type Field } from "./blob";

let DECOYS = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ@.-+";
for (let d = 0; d < 10; d++) DECOYS += String(d); // (no digit run in this file)

function rand(n: number): number {
  const a = new Uint32Array(1);
  crypto.getRandomValues(a);
  return (a[0] ?? 0) % n;
}

function shuffle<T>(a: T[]): T[] {
  for (let i = a.length - 1; i > 0; i--) {
    const j = rand(i + 1);
    const x = a[i] as T;
    a[i] = a[j] as T;
    a[j] = x;
  }
  return a;
}

export interface Glyph {
  order: number;
  ch: string;
  decoy: boolean;
}

/** One field, shuffled among decoys: computed once, when the details arrive. */
export function scatter(field: Field): Glyph[] {
  const glyphs: Glyph[] = field.map(([pos, cp]) => ({ order: pos, ch: String.fromCodePoint(cp), decoy: false }));
  const decoys = 2 + Math.ceil(field.length / 2) + rand(3);
  for (let d = 0; d < decoys; d++) {
    glyphs.push({ order: rand(MAX_LEN), ch: DECOYS.charAt(rand(DECOYS.length)), decoy: true });
  }
  return shuffle(glyphs);
}

export function Scrambled({ glyphs }: { glyphs: Glyph[] }) {
  return (
    <span className="scr">
      {glyphs.map((g, i) => (
        <span key={i} className={g.decoy ? `scr-x o${g.order}` : `o${g.order}`}>
          {g.ch}
        </span>
      ))}
    </span>
  );
}

/** A run read back in its visual order, for a click handler only. */
export function readRun(run: Element): string {
  const chars: string[] = [];
  for (const c of Array.from(run.children)) {
    if (c.classList.contains("scr-x")) continue;
    const m = /(?:^|\s)o(\d+)(?:\s|$)/.exec(c.className);
    if (m?.[1] !== undefined) chars[Number(m[1])] = c.textContent ?? "";
  }
  return chars.join("");
}

/** A block's lines as they show: the runs of one line joined by a space. */
export function readLines(block: Element): string {
  return Array.from(block.querySelectorAll(".imp-line"))
    .map((line) => Array.from(line.querySelectorAll(".scr")).map(readRun).join(" "))
    .join("\n");
}
