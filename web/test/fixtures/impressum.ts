// For tests only: made-up Impressum details and the blob Baylee's
// `baylee-tools site impressum-blob` would write for them (version 2), so the
// decoder is tested against the format and not against itself. Never the
// real details, which are in no file of either repository.

import { FIELDS, MAGIC, NONCE_LEN, SCRIPT_KEY, VERSION, type FieldName } from "../../src/impressum/blob";

export const SYNTHETIC: Record<FieldName, string> = {
  name: "Erika Mustermann",
  street: "Musterweg 1",
  postcode: "12345",
  city: "Musterstadt",
  country_de: "Deutschland",
  country_en: "Germany",
  email: "erika@example.org",
  phone: "+49 30 0000000",
};

export async function sha256(bytes: Uint8Array<ArrayBuffer>): Promise<Uint8Array> {
  return new Uint8Array(await crypto.subtle.digest("SHA-256", bytes));
}

export async function encode(fields: Record<FieldName, string>, nonce: Uint8Array = new Uint8Array(NONCE_LEN).fill(7)): Promise<string> {
  const scattered: Record<string, [number, number][]> = {};
  for (const key of FIELDS) {
    // reversed, as the encoder shuffles: order in the blob means nothing
    scattered[key] = Array.from(fields[key]).map((c, i): [number, number] => [i, c.codePointAt(0) ?? 0]).toReversed();
  }
  const plain = new TextEncoder().encode(JSON.stringify({ v: VERSION, fields: scattered }));
  const out = new Uint8Array(plain.length);
  for (let block = 0; block * 32 < plain.length; block++) {
    const input = new Uint8Array(new ArrayBuffer(SCRIPT_KEY.length + nonce.length + 4));
    input.set(SCRIPT_KEY, 0);
    input.set(nonce, SCRIPT_KEY.length);
    new DataView(input.buffer).setUint32(SCRIPT_KEY.length + nonce.length, block, false);
    const ks = await sha256(input);
    for (let i = 0; i < 32 && block * 32 + i < plain.length; i++) out[block * 32 + i] = (plain[block * 32 + i] ?? 0) ^ (ks[i] ?? 0);
  }
  const all = new Uint8Array(MAGIC.length + 1 + nonce.length + out.length);
  all.set(new TextEncoder().encode(MAGIC), 0);
  all[MAGIC.length] = VERSION;
  all.set(nonce, MAGIC.length + 1);
  all.set(out, MAGIC.length + 1 + nonce.length);
  let bin = "";
  for (const b of all) bin += String.fromCharCode(b);
  return btoa(bin).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}
