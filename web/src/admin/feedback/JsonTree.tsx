// A JSON value as a tree (Baylee `web/feedback/src/JsonTree.tsx`): objects
// and arrays fold on the browser's own `<details>`, so the keyboard and a
// screen reader know them, and their children are drawn only once unfolded,
// a page of them at a time, so a large client part costs nothing until it
// is read. Long strings show their start until asked; base64 is counted,
// not shown. Every value is text: nothing here is HTML.

import { useState } from "react";

import { t, useLang } from "../../i18n";
import { isObject } from "./api";

/** Strings longer than this show their start and a button for the rest. */
export const LONG_STRING = 240;
/** Children drawn at once; a button draws the next as many. */
export const CHILDREN = 200;
/** Levels drawn at most: deeper ones say how deep they go on. */
export const DEPTH = 32;

/** Keys whose value is not worth reading as text. */
const OPAQUE = new Set(["png_base64", "gzip_base64"]);

function LongString({ value }: { value: string }) {
  const lang = useLang();
  const [all, setAll] = useState(false);
  if (value.length <= LONG_STRING || all) return <span className="fb-j-string">“{value}”</span>;
  return (
    <>
      <span className="fb-j-string">“{value.slice(0, LONG_STRING)}…”</span>{" "}
      <button type="button" className="fb-linkish" onClick={() => setAll(true)}>
        {t(lang, "fb.jsonAll", { n: value.length })}
      </button>
    </>
  );
}

function Leaf({ value, name }: { value: unknown; name: string | null }) {
  const lang = useLang();
  if (typeof value === "string" && name !== null && OPAQUE.has(name)) return <span className="small">{t(lang, "fb.jsonBase64", { n: value.length })}</span>;
  if (typeof value === "string") return <LongString value={value} />;
  if (typeof value === "number" || typeof value === "boolean") return <span className="fb-j-value">{String(value)}</span>;
  if (value === null) return <span className="fb-j-value">null</span>;
  return <span className="small">{typeof value}</span>;
}

function Children({ entries, depth }: { entries: [string, unknown][]; depth: number }) {
  const lang = useLang();
  const [shown, setShown] = useState(CHILDREN);
  return (
    <ul className="fb-j-children">
      {entries.slice(0, shown).map(([k, v]) => (
        <Node key={k} name={k} value={v} depth={depth + 1} />
      ))}
      {entries.length > shown && (
        <li>
          <button type="button" className="fb-linkish" onClick={() => setShown((n) => n + CHILDREN)}>
            {t(lang, "fb.jsonMore", { n: Math.min(CHILDREN, entries.length - shown), left: entries.length - shown })}
          </button>
        </li>
      )}
    </ul>
  );
}

function Node({ name, value, depth, open = false }: { name: string | null; value: unknown; depth: number; open?: boolean }) {
  const lang = useLang();
  const [unfolded, setUnfolded] = useState(open);
  const label = name === null ? null : <span className="fb-j-key">{name}: </span>;
  const entries: [string, unknown][] | null = Array.isArray(value) ? value.map((v, i) => [String(i), v]) : isObject(value) ? Object.entries(value) : null;
  if (entries === null) {
    return (
      <li>
        {label}
        <Leaf value={value} name={name} />
      </li>
    );
  }
  const [a, b] = Array.isArray(value) ? ["[", "]"] : ["{", "}"];
  const count = t(lang, Array.isArray(value) ? "fb.jsonItems" : "fb.jsonKeys", { n: entries.length });
  if (entries.length === 0 || depth >= DEPTH) {
    return (
      <li>
        {label}
        <span className="small">
          {a} {entries.length === 0 ? "" : count} {b}
        </span>
      </li>
    );
  }
  return (
    <li>
      <details open={unfolded} onToggle={(e) => setUnfolded(e.currentTarget.open)}>
        <summary>
          {label}
          <span className="small">
            {a} {count} {b}
          </span>
        </summary>
        {unfolded && <Children entries={entries} depth={depth} />}
      </details>
    </li>
  );
}

/** `value` as a tree, its top level unfolded when `open`. */
export function JsonTree({ value, open = true }: { value: unknown; open?: boolean }) {
  return (
    <ul className="fb-json" translate="no">
      <Node name={null} value={value} depth={0} open={open} />
    </ul>
  );
}
