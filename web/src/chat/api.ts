// Browser cookie transport with bounded, strict decoding before worker/UI use.
import { answer, exact, malformed, poster, safeInteger, type Endpoint, type Json } from "../account/api";
import { publicPerson, type Person } from "../social/api";

export interface Device { id: string; ik: string; dk: string; dsig: string; label: string; added_at: number }
export interface Keys { tag: string; handle: string; devices: Device[] }
export interface Registration { ik: string; dk: string; dsig: string; label: string }
export interface Sealed { hdr: string; n: string; ct: string; epk: string; sig: string; wraps: { d: string; k: string }[] }
export interface Delivery { seq: number; sender: { tag: number; display_name: string }; device: Device; sealed: Sealed; at: number }
export interface Message { id: string; conv: string; sender: number; text: string; emote: boolean; at: number }
export interface Review { tag: string; number: string; changed: boolean; snapshot: Keys }
export interface ChatApi {
  register(device: Registration): Promise<string>;
  keys(tags: number[]): Promise<Keys[]>;
  send(conv: string, sealed: Sealed): Promise<{ id: string; at: number }>;
  inbox(device: string, after: number): Promise<{ messages: Delivery[]; more: boolean }>;
}

function text(value: unknown, max: number): string {
  if (typeof value !== "string" || value.length > max) throw malformed();
  return value;
}
function b64(value: unknown, max: number, length?: number): string {
  const s = text(value, max);
  if (!/^[\w-]+$/.test(s) || (length !== undefined && s.length !== length)) throw malformed();
  return s;
}
function fingerprint(value: unknown): string {
  const id = text(value, 16);
  if (!/^[0-9a-f]{16}$/.test(id)) throw malformed();
  return id;
}
function device(raw: unknown): Device {
  const d = exact(raw, ["id", "ik", "dk", "dsig", "label", "added_at"]);
  const label = text(d["label"], 24);
  if (!/^[\x20-\x7e]{1,24}$/.test(label) || label.trim() !== label) throw malformed();
  return { id: fingerprint(d["id"]), ik: b64(d["ik"], 43, 43), dk: b64(d["dk"], 43, 43), dsig: b64(d["dsig"], 86, 86), label, added_at: safeInteger(d["added_at"], 0) };
}
function keys(raw: unknown): Keys {
  const k = exact(raw, ["tag", "handle", "devices"]);
  const tag = text(k["tag"], 8);
  if (!/^(?:[0-9a-f]{4}|[1-9a-f][0-9a-f]{4,7})$/.test(tag) || Number.parseInt(tag, 16) > 0x7fff_ffff || Number.parseInt(tag, 16) === 0) throw malformed();
  const handle = text(k["handle"], 25);
  const list = k["devices"];
  if (!handle.endsWith(`#${tag}`) || !Array.isArray(list) || list.length > 5) throw malformed();
  const devices = list.map(device);
  if (new Set(devices.map((d) => d.id)).size !== devices.length) throw malformed();
  return { tag, handle, devices };
}
export function sealed(raw: unknown, maxWraps = 250): Sealed {
  const s = exact(raw, ["hdr", "n", "ct", "epk", "sig", "wraps"]);
  const wraps = s["wraps"];
  if (!Array.isArray(wraps) || wraps.length > maxWraps) throw malformed();
  return { hdr: b64(s["hdr"], 684), n: b64(s["n"], 32, 32), ct: b64(s["ct"], 5526), epk: b64(s["epk"], 43, 43), sig: b64(s["sig"], 86, 86), wraps: wraps.map((entry: unknown) => {
    const w = exact(entry, ["d", "k"]);
    return { d: fingerprint(w["d"]), k: b64(w["k"], 64, 64) };
  }) };
}
function delivery(raw: unknown): Delivery {
  const d = exact(raw, ["seq", "sender", "device", "sealed", "at"]);
  const p: Person = publicPerson(d["sender"]);
  return { seq: safeInteger(d["seq"], 1), sender: { tag: p.tag, display_name: p.displayName }, device: device(d["device"]), sealed: sealed(d["sealed"], 1), at: safeInteger(d["at"], 0) };
}
function response(raw: unknown, kind: string, fields: string[]): Json {
  const social = exact(answer(raw, "social", ["response"])["response"], ["kind", "response"]);
  if (social["kind"] !== "chat") throw malformed();
  const r = exact(social["response"], ["kind", ...fields]);
  if (r["kind"] !== kind) throw malformed();
  return r;
}
function signedId(envelope: Sealed, conv: string): string {
  try {
    const bytes = Uint8Array.from(atob(envelope.hdr.replace(/-/g, "+").replace(/_/g, "/")), (c) => c.charCodeAt(0));
    if (bytes.length > 512) throw malformed();
    const h = exact(JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(bytes)) as unknown, ["v", "id", "conv", "from", "dev", "ts", "kind"]);
    if (h["conv"] !== conv) throw malformed();
    return text(h["id"], 36);
  } catch { throw malformed(); }
}

export function chatApi(endpoint: Endpoint = {}): ChatApi {
  const post = poster(endpoint);
  const run = <T>(request: Json, read: (raw: unknown) => T) => post({ operation: "social", request: { action: "chat", request } }, read);
  return {
    register: (registration) => run({ command: "register", device: registration }, (raw) => fingerprint(response(raw, "registered", ["id"])["id"])),
    keys: (tags) => run({ command: "keys", tags }, (raw) => {
      const list = response(raw, "keys", ["keys"])["keys"];
      if (!Array.isArray(list) || list.length > tags.length) throw malformed();
      const result = list.map(keys);
      if (new Set(result.map((k) => k.tag)).size !== result.length || result.some((k) => !tags.includes(Number.parseInt(k.tag, 16)))) throw malformed();
      return result;
    }),
    send: (conv, envelope) => run({ command: "send", conv, sealed: envelope }, (raw) => {
      const r = response(raw, "sent", ["id", "at"]);
      const id = text(r["id"], 36);
      if (!/^[\da-f]{8}-[\da-f]{4}-7[\da-f]{3}-[89ab][\da-f]{3}-[\da-f]{12}$/.test(id) || id !== signedId(envelope, conv)) throw malformed();
      return { id, at: safeInteger(r["at"], 0) };
    }),
    inbox: (deviceId, after) => run({ command: "inbox", device: deviceId, after }, (raw) => {
      const r = response(raw, "inbox", ["messages", "more"]);
      const list = r["messages"];
      if (!Array.isArray(list) || list.length > 50 || typeof r["more"] !== "boolean") throw malformed();
      const messages = list.map(delivery);
      let previous = after;
      for (const m of messages) { if (m.seq <= previous) throw malformed(); previous = m.seq; }
      if (r["more"] && messages.length !== 50) throw malformed();
      return { messages, more: r["more"] };
    }),
  };
}
