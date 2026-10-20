// The Impressum blob: base64url of `MAGIC`, a version byte, a 16-byte nonce
// and the details as JSON XORed with a SHA-256 keystream. It is written by
// Baylee's `baylee-tools site impressum-blob` (crates/baylee-tools/src/site/
// impressum.rs) from a private file on the owner's machine and handed out
// only by the feedback service, for a solved proof of work.
//
// That is obfuscation, not security, and says so: there is no "@" and no "."
// in the blob and none of the details as text, so a harvester grepping
// pages for addresses finds nothing, and anybody who runs the page reads
// them, which is what an Impressum is for. The key is split between
// SCRIPT_KEY here and the nonce in the blob.

export const MAGIC = "BYIM";
export const VERSION = 2;
export const NONCE_LEN = 16;
/**
 * The script's half of the obfuscation key: published, not a secret, written
 * as Baylee's crates/baylee-tools/src/site/impressum.rs writes it. It must
 * stay byte for byte that value (and the old web/site/impressum.js's), or
 * existing blobs no longer decode.
 */
export const SCRIPT_KEY: readonly number[] = [
  0x6b, 0x1f, 0x0c, 0x9e, 0x52, 0xd8, 0x4a, 0x37, 0xb0, 0xe1, 0xf6, 0xa2, 0xc9, 0xd3, 0x5e, 0x48,
];

export const FIELDS = ["name", "street", "postcode", "city", "country_de", "country_en", "email", "phone"] as const;
export type FieldName = (typeof FIELDS)[number];
/** A field as `[position, code point]` pairs: never one string. */
export type Field = [number, number][];
export type Fields = Record<FieldName, Field>;

/** The longest field a page draws (one order class per position). */
export const MAX_LEN = 64;

function bytes(length: number): Uint8Array<ArrayBuffer> {
  return new Uint8Array(new ArrayBuffer(length));
}

export function b64urlBytes(text: string): Uint8Array<ArrayBuffer> {
  let s = text.trim().replace(/-/g, "+").replace(/_/g, "/");
  while (s.length % 4) s += "=";
  const bin = atob(s);
  const out = bytes(bin.length);
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
  return out;
}

/** Keystream block i = SHA-256(SCRIPT_KEY | nonce | i as 4 bytes big-endian). */
export async function keystream(nonce: Uint8Array, length: number): Promise<Uint8Array> {
  const key = Uint8Array.from(SCRIPT_KEY);
  const blocks: Promise<ArrayBuffer>[] = [];
  for (let i = 0; i * 32 < length; i++) {
    const input = bytes(key.length + nonce.length + 4);
    input.set(key, 0);
    input.set(nonce, key.length);
    new DataView(input.buffer).setUint32(key.length + nonce.length, i, false);
    blocks.push(crypto.subtle.digest("SHA-256", input));
  }
  const parts = await Promise.all(blocks);
  const out = bytes(parts.length * 32);
  parts.forEach((p, j) => out.set(new Uint8Array(p), j * 32));
  return out.subarray(0, length);
}

function isField(v: unknown): v is Field {
  return (
    Array.isArray(v) &&
    v.length <= MAX_LEN &&
    v.every(
      (pair: unknown) =>
        Array.isArray(pair) &&
        pair.length === 2 &&
        Number.isInteger(pair[0]) &&
        Number.isInteger(pair[1]) &&
        (pair[0] as number) >= 0 &&
        (pair[0] as number) < MAX_LEN &&
        (pair[1] as number) > 0 &&
        (pair[1] as number) <= 0x10ffff,
    )
  );
}

/** The details in a blob, or a rejection for anything that is not one. */
export async function decode(text: string): Promise<Fields> {
  const raw = b64urlBytes(text);
  const head = MAGIC.length + 1 + NONCE_LEN;
  if (raw.length <= head || String.fromCharCode(...raw.subarray(0, MAGIC.length)) !== MAGIC || raw[MAGIC.length] !== VERSION) {
    throw new Error("not an Impressum blob");
  }
  const nonce = raw.subarray(MAGIC.length + 1, head);
  const body = raw.subarray(head);
  const ks = await keystream(nonce, body.length);
  const plain = bytes(body.length);
  for (let i = 0; i < body.length; i++) plain[i] = (body[i] ?? 0) ^ (ks[i] ?? 0);
  const doc = JSON.parse(new TextDecoder("utf-8").decode(plain)) as { v?: unknown; fields?: unknown };
  if (doc.v !== VERSION || typeof doc.fields !== "object" || doc.fields === null) throw new Error("unexpected document");
  const fields = doc.fields as Record<string, unknown>;
  const out = {} as Fields;
  for (const name of FIELDS) {
    const f = fields[name];
    if (!isField(f)) throw new Error("bad field");
    out[name] = f;
  }
  return out;
}
