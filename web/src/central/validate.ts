// The checks central-client's `gateway_directory` makes on an answer, and
// central-api's on origins and metadata (crates/central-api/src/gateway.rs),
// for the browser: ids are UUIDv7, origins canonical HTTPS, metadata bounded
// and on its own origin.

import type {
  GatewayMetadata,
  GatewayPlayerStat,
  GatewayPrivacy,
  GatewayStats,
  GatewayTableSetting,
  PlayerPresenceState,
} from "./directory";

export function isByteArray(v: unknown): v is number[] {
  return Array.isArray(v) && v.length === 16 && v.every((b) => Number.isInteger(b) && b >= 0 && b <= 255);
}

/** central-client's `valid_id`: version 7, RFC 9562 variant. */
export function isUuidV7(bytes: readonly number[]): boolean {
  return bytes.length === 16 && (bytes[6] ?? 0) >> 4 === 7 && (bytes[8] ?? 0) >> 6 === 2;
}

export function uuidText(bytes: readonly number[]): string {
  const hex = bytes.map((b) => b.toString(16).padStart(2, "0")).join("");
  return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
}

export function uuidBytes(text: string): number[] | null {
  if (!/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/.test(text)) return null;
  const hex = text.replace(/-/g, "");
  const out: number[] = [];
  for (let i = 0; i < 32; i += 2) out.push(Number.parseInt(hex.slice(i, i + 2), 16));
  return out;
}

export function compare(a: readonly number[], b: readonly number[]): number {
  for (let i = 0; i < 16; i++) {
    const d = (a[i] ?? 0) - (b[i] ?? 0);
    if (d !== 0) return d;
  }
  return 0;
}

/** Explicit numeric loopback HTTP with a port: what Central accepts in development only. */
function developmentOrigin(value: string): boolean {
  let url: URL;
  try {
    url = new URL(value);
  } catch {
    return false;
  }
  return (
    url.protocol === "http:" &&
    (url.hostname === "127.0.0.1" || url.hostname === "[::1]") &&
    url.port !== "" &&
    url.port !== "0" &&
    url.origin === value
  );
}

/**
 * central-api's `validate_origin`: canonical HTTPS, no credentials, path,
 * query or fragment. A page on loopback (a local try against an isolated
 * Central) also takes Central's development origins; unlike central-client,
 * which knows its endpoint, it cannot tell which Central answers, so it
 * takes either kind there and only HTTPS anywhere else.
 */
export function validOrigin(value: string, development = false): boolean {
  if (value.length > 256) return false;
  if (development && developmentOrigin(value)) return true;
  let url: URL;
  try {
    url = new URL(value);
  } catch {
    return false;
  }
  return (
    url.protocol === "https:" &&
    url.hostname !== "" &&
    url.username === "" &&
    url.password === "" &&
    url.origin === value &&
    !value.endsWith("/")
  );
}

const utf8 = new TextEncoder();

/** central-api's `GatewayMetadata::validate`, on the listing's own origin. */
export function validMetadata(m: GatewayMetadata, origin: string): boolean {
  let website: URL;
  try {
    website = new URL(m.website);
  } catch {
    return false;
  }
  const nameBytes = utf8.encode(m.name).length;
  return (
    nameBytes > 0 &&
    nameBytes <= 80 &&
    m.name.trim() === m.name &&
    !/\p{Cc}/u.test(m.name) &&
    m.version.length > 0 &&
    m.version.length <= 64 &&
    /^[A-Za-z0-9.\-_+]+$/.test(m.version) &&
    m.website.length <= 512 &&
    website.origin === origin &&
    website.username === "" &&
    website.password === "" &&
    !m.website.includes("?") &&
    !m.website.includes("#") &&
    website.href === m.website
  );
}

function validShownText(value: unknown, maxBytes: number): value is string {
  if (typeof value !== "string") return false;
  const bytes = utf8.encode(value).length;
  return (
    bytes > 0 &&
    bytes <= maxBytes &&
    value.trim() === value &&
    !/\p{Cc}/u.test(value) &&
    !/[\u061C\u200B\u200E\u200F\u2028-\u202E\u2060-\u2064\u2066-\u2069\uFEFF]/u.test(value)
  );
}

function isUInt(v: unknown, max = 0xffffffff): v is number {
  return typeof v === "number" && Number.isInteger(v) && v >= 0 && v <= max;
}

function isPrivacy(v: unknown): v is GatewayPrivacy {
  return v === "full" || v === "counts_only" || v === "private";
}

function isPresenceState(v: unknown): v is PlayerPresenceState {
  return v === "lobby" || v === "table" || v === "in_game" || v === "spectating";
}

function validTableSetting(v: unknown): v is GatewayTableSetting {
  if (typeof v !== "object" || v === null || Array.isArray(v)) return false;
  const o = v as Record<string, unknown>;
  const keys = Object.keys(o);
  const allowed = new Set([
    "format",
    "seats_occupied",
    "seats_total",
    "ai_seats",
    "clock",
    "turn",
    "spectators",
    "private_table",
  ]);
  if (keys.some((k) => !allowed.has(k))) return false;
  if (
    !validShownText(o["format"], 40) ||
    !validShownText(o["clock"], 40) ||
    !isUInt(o["seats_total"], 8) ||
    o["seats_total"] < 2 ||
    !isUInt(o["seats_occupied"], o["seats_total"]) ||
    !isUInt(o["ai_seats"], o["seats_total"]) ||
    !isUInt(o["spectators"], 65535) ||
    typeof o["private_table"] !== "boolean"
  ) {
    return false;
  }
  if (Object.hasOwn(o, "turn") && o["turn"] !== undefined && !isUInt(o["turn"], 65535)) {
    return false;
  }
  return true;
}

function validPlayerStat(v: unknown): v is GatewayPlayerStat {
  if (typeof v !== "object" || v === null || Array.isArray(v)) return false;
  const o = v as Record<string, unknown>;
  const keys = Object.keys(o);
  const allowed = new Set(["handle", "state", "context", "games_today"]);
  if (keys.some((k) => !allowed.has(k))) return false;
  if (!validShownText(o["handle"], 48) || !isPresenceState(o["state"]) || !isUInt(o["games_today"], 65535)) {
    return false;
  }
  if (Object.hasOwn(o, "context") && o["context"] !== undefined && !validShownText(o["context"], 80)) {
    return false;
  }
  return true;
}

/** central-api's `GatewayStats::validate`: bounded counts, table settings and privacy rules. */
export function validGatewayStats(v: unknown): v is GatewayStats {
  if (typeof v !== "object" || v === null || Array.isArray(v)) return false;
  const o = v as Record<string, unknown>;
  const allowed = new Set([
    "privacy",
    "running_games",
    "open_tables",
    "players_online",
    "players_in_lobby",
    "players_at_table",
    "players_in_game",
    "spectators",
    "ai_opponents",
    "games_completed_today",
    "hidden_players",
    "tables",
    "players",
  ]);
  if (Object.keys(o).some((k) => !allowed.has(k))) return false;
  if (
    !isPrivacy(o["privacy"]) ||
    !isUInt(o["running_games"]) ||
    !isUInt(o["open_tables"]) ||
    !isUInt(o["players_online"]) ||
    !isUInt(o["players_in_lobby"]) ||
    !isUInt(o["players_at_table"]) ||
    !isUInt(o["players_in_game"]) ||
    !isUInt(o["spectators"]) ||
    !isUInt(o["ai_opponents"]) ||
    !isUInt(o["games_completed_today"]) ||
    !isUInt(o["hidden_players"])
  ) {
    return false;
  }
  const tables = o["tables"] ?? [];
  const players = o["players"] ?? [];
  if (!Array.isArray(tables) || tables.length > 32 || !tables.every(validTableSetting)) return false;
  if (!Array.isArray(players) || players.length > 64 || !players.every(validPlayerStat)) return false;
  if (o["privacy"] !== "full" && (tables.length > 0 || players.length > 0)) return false;
  return true;
}
