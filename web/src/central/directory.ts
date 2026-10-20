// Central's public gateway directory (docs/gateway-registry.md), typed. The
// wire vocabulary is central-api's: a tagged JSON request
// `{"operation": "gateway_directory", "after": null | [16 bytes]}` posted with
// exactly one `Content-Type: application/json` and no credential, answered
// `{"result": "gateway_directory", "gateways": [...]}`, at most 100 approved
// gateways in ascending UUID order; the last id is the next page's `after`.
// A refusal is plain text with a status (401, 409, 429, 503), never JSON.
//
// Every answer is checked as central-client's `gateway_directory` checks it
// (ids are UUIDv7 and strictly ascending, origins canonical HTTPS, metadata
// bounded and on its own origin), and nothing that fails is shown. Metadata
// is untrusted display text: the page renders it as text, never HTML, and
// links a website only after these checks.

import { compare, isByteArray, isUuidV7, uuidBytes, uuidText, validMetadata, validOrigin } from "./validate";

export interface GatewayMetadata {
  name: string;
  version: string;
  website: string;
}

export interface GatewayListing {
  /** The gateway's UUIDv7 as text. */
  id: string;
  origin: string;
  metadata: GatewayMetadata;
  /** Central accepted a heartbeat within the last 90 seconds; no availability guarantee. */
  online: boolean;
}

export const PAGE_SIZE = 100;

export type DirectoryFailure = "unavailable" | "busy" | "refused" | "malformed" | "network";

export class DirectoryError extends Error {
  readonly kind: DirectoryFailure;
  constructor(kind: DirectoryFailure) {
    super(`gateway directory: ${kind}`);
    this.kind = kind;
  }
}

function exactKeys(o: Record<string, unknown>, keys: readonly string[]): boolean {
  const own = Object.keys(o);
  return own.length === keys.length && keys.every((k) => Object.hasOwn(o, k));
}

function isObject(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

const bad = () => new DirectoryError("malformed");

/** The answer to one page, checked; throws `DirectoryError("malformed")` on anything else. */
export function parsePage(raw: unknown, after: readonly number[] | null, development = false): GatewayListing[] {
  if (!isObject(raw) || !exactKeys(raw, ["result", "gateways"]) || raw["result"] !== "gateway_directory") throw bad();
  const list = raw["gateways"];
  if (!Array.isArray(list) || list.length > PAGE_SIZE) throw bad();
  let previous = after;
  const out: GatewayListing[] = [];
  for (const g of list) {
    if (!isObject(g) || !exactKeys(g, ["id", "origin", "metadata", "online"])) throw bad();
    const { id, origin, metadata, online } = g;
    if (!isByteArray(id) || !isUuidV7(id) || (previous !== null && compare(previous, id) >= 0)) throw bad();
    if (typeof origin !== "string" || !validOrigin(origin, development)) throw bad();
    if (typeof online !== "boolean" || !isObject(metadata) || !exactKeys(metadata, ["name", "version", "website"])) throw bad();
    const { name, version, website } = metadata;
    if (typeof name !== "string" || typeof version !== "string" || typeof website !== "string") throw bad();
    const meta = { name, version, website };
    if (!validMetadata(meta, origin)) throw bad();
    out.push({ id: uuidText(id), origin, metadata: meta, online });
    previous = id;
  }
  return out;
}

export interface DirectoryClient {
  /** One page after `after` (a listing's id), or the first. */
  page(after: string | null): Promise<GatewayListing[]>;
}

export interface ClientOptions {
  /** Central's API root on this origin, e.g. `/api`. */
  api: string;
  /** Accept Central's development origins (a page served from loopback). */
  development?: boolean;
  fetch?: typeof fetch;
}

export function directoryClient({ api, development = false, fetch: doFetch = (...a) => fetch(...a) }: ClientOptions): DirectoryClient {
  return {
    async page(after) {
      const cursor = after === null ? null : uuidBytes(after);
      if (after !== null && (cursor === null || !isUuidV7(cursor))) throw new DirectoryError("malformed");
      let r: Response;
      try {
        r = await doFetch(`${api}/v1/native`, {
          method: "POST",
          credentials: "omit",
          referrerPolicy: "no-referrer",
          cache: "no-store",
          headers: { "Content-Type": "application/json", Accept: "application/json" },
          body: JSON.stringify({ operation: "gateway_directory", after: cursor }),
        });
      } catch {
        throw new DirectoryError("network");
      }
      if (r.status === 429) throw new DirectoryError("busy");
      if (r.status === 503) throw new DirectoryError("unavailable");
      if (!r.ok) throw new DirectoryError(r.status >= 500 ? "unavailable" : "refused");
      let body: unknown;
      try {
        body = await r.json();
      } catch {
        throw new DirectoryError("malformed");
      }
      return parsePage(body, cursor, development);
    },
  };
}
