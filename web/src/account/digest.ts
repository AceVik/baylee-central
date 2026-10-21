// The digest an acceptance of terms names (central-api's `terms::digest`,
// docs/terms.md): SHA-256 over `baylee-terms-v1\0` and, per text in the
// given order, its language tag, its date (empty when none) and its
// Markdown, each preceded by its UTF-8 length as eight big-endian bytes.
// The page accepts a version only when the texts it shows hash to the
// digest it names, so an acceptance is always for exactly what was shown.

import type { TermsText } from "./api";

const DOMAIN = "baylee-terms-v1\0";

export async function termsDigest(texts: readonly TermsText[]): Promise<Uint8Array> {
  const encoder = new TextEncoder();
  const parts: Uint8Array[] = [encoder.encode(DOMAIN)];
  const field = (value: string) => {
    const bytes = encoder.encode(value);
    const length = new Uint8Array(8);
    new DataView(length.buffer).setBigUint64(0, BigInt(bytes.length));
    parts.push(length, bytes);
  };
  for (const text of texts) {
    field(text.lang);
    field(text.updated ?? "");
    field(text.markdown);
  }
  const all = new Uint8Array(parts.reduce((n, p) => n + p.length, 0));
  let at = 0;
  for (const part of parts) {
    all.set(part, at);
    at += part.length;
  }
  return new Uint8Array(await crypto.subtle.digest("SHA-256", all));
}

export function equalBytes(a: Uint8Array, b: Uint8Array): boolean {
  return a.length === b.length && a.every((byte, i) => byte === b[i]);
}
