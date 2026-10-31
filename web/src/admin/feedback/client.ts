// The client's details of a report (its `client` part), read in the browser
// (docs/reports.md: Central keeps the gzip as it came and never unpacks it).
// The part is untrusted: a gzip of at most `BOUNDS.clientGzip` bytes is
// inflated through the browser's own `DecompressionStream` and stopped past
// `MAX_CLIENT_JSON` bytes of output, decoded as UTF-8 and only then parsed
// as JSON. Every reader below takes what it finds or nothing: a part that is
// missing or of another shape reads as `null`, and the page says so instead
// of breaking (Baylee `web/feedback/src/dump.ts`, the client's
// `crates/baylee-client-core/src/bugreport.rs`).

import { isObject } from "./api";

/**
 * The most a client part may inflate to: 16 MiB, docs/reports.md pass 2.
 * The client writes a few hundred KiB of JSON and a screenshot of a few
 * MiB as base64; eight times the largest gzip Central takes leaves room for
 * that and stops a decompression bomb long before it costs the tab.
 */
export const MAX_CLIENT_JSON = 16 * 1024 * 1024;

/** A screenshot's largest side, in pixels, as its PNG header says it. */
export const MAX_SHOT_SIDE = 16_384;

export class Unreadable extends Error {
  readonly why: "large" | "gzip" | "json";
  constructor(why: "large" | "gzip" | "json") {
    super(`client part: ${why}`);
    this.why = why;
  }
}

/** `gzip` inflated, refused past `max` bytes of output (counted while it streams, never after). */
export async function inflate(gzip: Uint8Array, max: number = MAX_CLIENT_JSON): Promise<Uint8Array> {
  let reader: ReadableStreamDefaultReader<Uint8Array>;
  try {
    const source = new ReadableStream<BufferSource>({
      start(controller) {
        controller.enqueue(gzip.slice());
        controller.close();
      },
    });
    reader = source.pipeThrough(new DecompressionStream("gzip")).getReader();
  } catch {
    throw new Unreadable("gzip");
  }
  const chunks: Uint8Array[] = [];
  let total = 0;
  for (;;) {
    let step: ReadableStreamReadResult<Uint8Array>;
    try {
      step = await reader.read();
    } catch {
      throw new Unreadable("gzip");
    }
    if (step.done) break;
    total += step.value.byteLength;
    if (total > max) {
      await reader.cancel().catch(() => undefined);
      throw new Unreadable("large");
    }
    chunks.push(step.value);
  }
  const out = new Uint8Array(total);
  let at = 0;
  for (const c of chunks) {
    out.set(c, at);
    at += c.byteLength;
  }
  return out;
}

export type Json = Record<string, unknown>;

/** The client part: inflated within its bound, strict UTF-8, a JSON object. */
export async function readClient(gzip: Uint8Array, max: number = MAX_CLIENT_JSON): Promise<Json> {
  const bytes = await inflate(gzip, max);
  let value: unknown;
  try {
    value = JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(bytes));
  } catch {
    throw new Unreadable("json");
  }
  if (!isObject(value)) throw new Unreadable("json");
  return value;
}

const text = (v: unknown, max = 2_000): string | null => (typeof v === "string" && v.length <= max ? v : null);
const num = (v: unknown): number | null => (typeof v === "number" && Number.isFinite(v) ? v : null);
const part = (client: Json, key: string): Json | null => {
  const v = client[key];
  return isObject(v) ? v : null;
};

export interface Build {
  version: string | null;
  commit: string | null;
}

export function readBuild(client: Json): Build | null {
  const b = part(client, "build");
  return b === null ? null : { version: text(b["version"], 128), commit: text(b["commit"], 64) };
}

export type SystemRow = "platform" | "cpus" | "adapter" | "backend" | "window" | "scale" | "lang";

/** The system part's rows, in the order the client's form shows them. */
export function readSystem(client: Json): [SystemRow, string][] | null {
  const s = part(client, "system");
  if (s === null) return null;
  const rows: [SystemRow, string][] = [];
  const add = (row: SystemRow, value: string | null) => {
    if (value !== null && value !== "") rows.push([row, value]);
  };
  add("platform", text(s["platform"], 256));
  add("cpus", num(s["cpus"])?.toString() ?? null);
  add("adapter", text(s["adapter"], 256));
  add("backend", text(s["backend"], 64));
  const w = s["window"];
  if (Array.isArray(w) && w.length === 2) {
    const [x, y] = w.map(num);
    if (x != null && y != null) add("window", `${x} × ${y}`);
  }
  add("scale", num(s["scale"])?.toString() ?? null);
  add("lang", text(s["lang"], 32));
  return rows;
}

const PNG = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];

/** `text` as bytes, when it is base64 and nothing else. */
export function fromBase64(b64: string): Uint8Array | null {
  if (b64.length === 0 || b64.length % 4 !== 0 || !/^[A-Za-z0-9+/]+={0,2}$/.test(b64)) return null;
  let raw: string;
  try {
    raw = atob(b64);
  } catch {
    return null;
  }
  const out = new Uint8Array(raw.length);
  for (let i = 0; i < raw.length; i++) out[i] = raw.charCodeAt(i);
  return out;
}

export interface Screenshot {
  /** A PNG: its signature checked, its size read from its own header. */
  png: Uint8Array;
  width: number;
  height: number;
}

/**
 * The window's picture, when it is a PNG: the eight bytes of its signature,
 * then an `IHDR` whose sides are 1 to `MAX_SHOT_SIDE`. Anything else is no
 * picture: the page shows bytes only as a PNG it checked, through a `blob:`
 * address, never a `data:` one.
 */
export function readScreenshot(client: Json): Screenshot | null {
  const shot = part(client, "screenshot");
  const b64 = shot === null ? null : shot["png_base64"];
  if (typeof b64 !== "string") return null;
  const png = fromBase64(b64);
  if (png === null || png.length < 24 || !PNG.every((b, i) => png[i] === b)) return null;
  // the first chunk is IHDR: its length (13), its name, then width and height, big-endian
  const view = new DataView(png.buffer, png.byteOffset, png.byteLength);
  if (String.fromCharCode(...png.slice(12, 16)) !== "IHDR") return null;
  const width = view.getUint32(16);
  const height = view.getUint32(20);
  if (width < 1 || height < 1 || width > MAX_SHOT_SIDE || height > MAX_SHOT_SIDE) return null;
  return { png, width, height };
}

const SCRYFALL_ID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;

/** One card the player's text names in brackets (`client.refs.cards`). */
export interface CardRef {
  name: string;
  /** Where in the text, `[start, end)` in characters, brackets included. */
  at: [number, number] | null;
  zone: string | null;
  owner: number | null;
  /** The printing's Scryfall id, only one shaped like an id: all that goes into an address. */
  scryfallId: string | null;
}

/** One player the text names (`client.refs.players`): a seat, never an account. */
export interface PlayerRef {
  name: string;
  at: [number, number] | null;
  seat: number | null;
}

export interface Refs {
  cards: CardRef[];
  players: PlayerRef[];
}

function range(v: unknown): [number, number] | null {
  if (!Array.isArray(v) || v.length !== 2) return null;
  const [a, b] = v.map(num);
  return a != null && b != null && Number.isInteger(a) && Number.isInteger(b) ? [a, b] : null;
}

/** The first 200 objects of `v`, when it is an array. */
const objects = (v: unknown): Json[] => (Array.isArray(v) ? v.slice(0, 200).filter(isObject) : []);

/** References the client resolved in the text, at most 200 of each; null when it names none. */
export function readRefs(client: Json): Refs | null {
  const r = part(client, "refs");
  if (r === null) return null;
  const refs: Refs = {
    cards: objects(r["cards"]).map((c) => {
      const print = isObject(c["print"]) ? c["print"] : null;
      const id = print === null ? null : text(print["scryfall_id"], 36);
      return {
        name: text(c["text"], 200) ?? "?",
        at: range(c["at"]),
        zone: text(c["zone"], 32),
        owner: num(c["owner"]),
        scryfallId: id !== null && SCRYFALL_ID.test(id) ? id : null,
      };
    }),
    players: objects(r["players"]).map((p) => ({ name: text(p["text"], 200) ?? "?", at: range(p["at"]), seat: num(p["seat"]) })),
  };
  return refs.cards.length + refs.players.length > 0 ? refs : null;
}

export interface Crash {
  message: string | null;
  location: string | null;
  backtrace: string | null;
  thread: string | null;
  platform: string | null;
}

export function readCrash(client: Json): Crash | null {
  const c = part(client, "crash");
  if (c === null) return null;
  return { message: text(c["message"]), location: text(c["location"], 512), backtrace: text(c["backtrace"], 200_000), thread: text(c["thread"], 128), platform: text(c["platform"], 256) };
}

export interface LogLine {
  turn: number | null;
  text: string;
}

export interface SeatLog {
  roster: { seat: number | null; name: string; ai: boolean }[];
  lines: LogLine[];
}

/** The lines a log shows at most: the latest ones. */
export const LOG_LINES = 500;

export function readLog(client: Json): SeatLog | null {
  const l = part(client, "log");
  if (l === null) return null;
  const roster = Array.isArray(l["roster"]) ? l["roster"].slice(0, 16).filter(isObject) : [];
  const lines = Array.isArray(l["lines"]) ? l["lines"].filter(isObject).slice(-LOG_LINES) : [];
  return {
    roster: roster.map((p) => ({ seat: num(p["seat"]), name: text(p["name"], 64) ?? "?", ai: p["is_ai"] === true })),
    lines: lines.map((x) => ({ turn: num(x["turn"]), text: text(x["text"]) ?? "" })),
  };
}

/** The name of an enum serde writes externally tagged: a string, or `{"Name": …}`. */
export function variant(v: unknown): string | null {
  if (typeof v === "string") return v.length <= 64 ? v : null;
  if (!isObject(v)) return null;
  const keys = Object.keys(v);
  return keys.length === 1 && (keys[0]?.length ?? 99) <= 64 ? (keys[0] ?? null) : null;
}

export interface GameSummary {
  when: string | null;
  turn: number | null;
  seat: number | null;
  active: number | null;
  players: number | null;
  /** The question the engine was asking, by its variant's name. */
  pending: string | null;
  seq: number | null;
}

/** Where in the game the report was written, in a few facts. */
export function readGame(client: Json): GameSummary | null {
  const g = part(client, "game");
  if (g === null) return null;
  const table = isObject(g["table"]) ? g["table"] : {};
  const view = isObject(g["view"]) ? g["view"] : {};
  return {
    when: text(table["when"], 128),
    turn: num(view["turn"]),
    seat: num(table["seat"]),
    active: num(view["active"]),
    players: Array.isArray(view["players"]) ? view["players"].length : null,
    pending: variant(g["pending"]),
    seq: num(table["seq"]),
  };
}

/** `ChooseAttackers` as `Choose attackers`. */
export const spaced = (name: string): string => name.replace(/([a-z])([A-Z])/g, (_, a: string, b: string) => `${a} ${b.toLowerCase()}`);

/** Where a printing's picture is on Scryfall's image host (the one the CSP names), and its page; from its id alone. */
export function cardImage(id: string): string | null {
  return SCRYFALL_ID.test(id) ? `https://cards.scryfall.io/normal/front/${id[0] ?? ""}/${id[1] ?? ""}/${id}.jpg` : null;
}

export function cardPage(id: string): string | null {
  return SCRYFALL_ID.test(id) ? `https://scryfall.com/card/${id}` : null;
}
