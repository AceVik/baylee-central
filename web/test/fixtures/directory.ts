// A synthetic directory for tests and the end-to-end smoke tests: made-up
// gateways under `.invalid` (RFC 2606), never a real service. `page(n)` is
// what Central would answer for the n-th page of a directory of `total`.

import { PAGE_SIZE } from "../../src/central/directory";

/** `OFFICIAL_GATEWAY` of src/config.ts, spelt out: e2e/ reads this file without Vite's types. */
const OFFICIAL_GATEWAY = "https://gateway.baylee.app";

/** A UUIDv7 as Central's wire carries it: 16 integers. */
export function idBytes(n: number): number[] {
  const b = [0x01, 0x9a, 0x2b, 0x3c, 0x4d, 0x5e, 0x70, 0x00, 0x80, 0x00, 0, 0, 0, 0, 0, 0];
  b[12] = (n >> 24) & 0xff;
  b[13] = (n >> 16) & 0xff;
  b[14] = (n >> 8) & 0xff;
  b[15] = n & 0xff;
  return b;
}

export function listing(n: number, over: { name?: string; online?: boolean } = {}) {
  const origin = `https://gw${n}.example.invalid`;
  return {
    id: idBytes(n),
    origin,
    metadata: { name: over.name ?? `Synthetic table ${n}`, version: `0.1.0-beta.${n % 9}`, website: `${origin}/` },
    online: over.online ?? n % 3 !== 0,
  };
}

/** The official gateway's entry, as the n-th id: Central lists it like any other. */
export function officialListing(n: number, over: { name?: string; online?: boolean } = {}) {
  return { ...listing(n, over), origin: OFFICIAL_GATEWAY, metadata: { ...listing(n, over).metadata, website: `${OFFICIAL_GATEWAY}/` } };
}

/** Page `index` (from 0) of a directory of `total` gateways, numbered from 1. */
export function page(index: number, total: number) {
  const from = index * PAGE_SIZE + 1;
  const to = Math.min(total, from + PAGE_SIZE - 1);
  const gateways = [];
  for (let n = from; n <= to; n++) gateways.push(listing(n));
  return { result: "gateway_directory", gateways };
}
