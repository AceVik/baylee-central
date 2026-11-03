// The chat's and the groups' answers for the e2e mock (`mockCentral` with
// `area`), in the shapes src/chat/api.ts and src/chat/groups.ts read, and a
// stand-in for the chat's Worker steps (`CHAT_STEPS`): its "ciphertext" is
// the message in base64url, so the mock and the page agree on what was
// said. A friend answers a message once. Synthetic names only.

import type { AreaState } from "./area.ts";

const b64 = (value: unknown) => Buffer.from(JSON.stringify(value)).toString("base64url");
const unb64 = (value: string): Record<string, unknown> => JSON.parse(Buffer.from(value, "base64url").toString("utf8")) as Record<string, unknown>;
const hex = (tag: number) => tag.toString(16).padStart(4, "0");
/** A device's fingerprint: its tag, then a fixed tail. */
export const deviceOf = (tag: number): string => `${hex(tag)}d0d0d0d0d0d0`.slice(0, 16);
const device = (tag: number) => ({ id: deviceOf(tag), ik: "A".repeat(43), dk: "B".repeat(43), dsig: "C".repeat(86), label: "Browser", added_at: 1_700_000_000_000 });
const uuid = (n: number) => `019a3d70-${(0x1000 + n).toString(16)}-7000-8000-${n.toString(16).padStart(12, "0")}`;

export interface Delivery { seq: number; from: number; conv: string; text: string; at: number; id: string }
export interface ChatState {
  seq: number;
  deliveries: Delivery[];
  groups: { id: string; name: string; owner: number; members: number[] }[];
  invites: { group: string; name: string; inviter: number; at: number }[];
  answered: Set<string>;
}

const dm = (a: number, b: number) => `dm:${[a, b].sort((x, y) => x - y).map(hex).join(":")}`;
const social = (kind: string, response: Record<string, unknown>) => ({ result: "social", response: { kind, response } });

export function chatState(s: AreaState): ChatState {
  const me = s.accounts[s.me]?.tag ?? 0;
  const [heron, tide] = [s.accounts[2]?.tag ?? 0, s.accounts[3]?.tag ?? 0];
  const now = Date.now();
  const hour = 3_600_000;
  const group = uuid(900);
  const said: [number, string, string, number][] = [
    [tide, dm(me, tide), "Good game yesterday! That last turn with the counterspell was brutal.", now - 74 * hour],
    [me, dm(me, tide), "Haha, I had it the whole time. Rematch on the weekend?", now - 73.5 * hour],
    [heron, dm(me, heron), "Hey! Are you joining the Friday table?", now - 26 * hour],
    [me, dm(me, heron), "Yes, I'll bring the new green deck.", now - 25.8 * hour],
    [heron, dm(me, heron), "Perfect. Moss_Keeper said they'd host on the Sanctuary gateway.", now - 25.7 * hour],
    [heron, dm(me, heron), "Starting at 8 pm, four players, Commander rules.", now - 40 * 60_000],
    [tide, `grp:${group}`, "Who brings snacks this time?", now - 3 * hour],
    [heron, `grp:${group}`, "I can do it. Any requests?", now - 2.5 * hour],
    [me, `grp:${group}`, "Something without crumbs, please — my sleeves thank you.", now - 2.4 * hour],
    [tide, `grp:${group}`, "Deal. See you all on Friday!", now - 12 * 60_000],
  ];
  return {
    seq: said.length,
    deliveries: said.map(([from, conv, text, at], i) => ({ seq: i + 1, from, conv, text, at: Math.round(at), id: uuid(i + 1) })),
    groups: [{ id: group, name: "Friday Night Table", owner: me, members: [me, heron, tide] }],
    invites: [{ group: uuid(901), name: "Draft Club", inviter: s.accounts[4]?.tag ?? 0, at: Math.floor(now / 1000) - 3600 }],
    answered: new Set(),
  };
}

function person(s: AreaState, tag: number) {
  const a = s.accounts.find((x) => x.tag === tag);
  return { tag, display_name: a?.display_name ?? "Someone" };
}

function deliver(s: AreaState, d: Delivery) {
  const sealed = { hdr: b64({ v: 1, id: d.id, conv: d.conv, from: d.from, dev: deviceOf(d.from), ts: d.at, kind: "text" }), n: "N".repeat(32), ct: b64({ text: d.text, emote: false }), epk: "E".repeat(43), sig: "S".repeat(86), wraps: [{ d: deviceOf(s.accounts[s.me]?.tag ?? 0), k: "K".repeat(64) }] };
  return { seq: d.seq, sender: person(s, d.from), device: device(d.from), sealed, at: d.at };
}

/** The answer to a chat command, or a status to refuse it with. */
export function chatAnswer(s: AreaState, c: ChatState, request: Record<string, unknown>): unknown {
  const me = s.accounts[s.me]?.tag ?? 0;
  switch (request["command"]) {
    case "register": return social("chat", { kind: "registered", id: deviceOf(me) });
    case "keys": {
      const tags = (request["tags"] as number[]).filter((tag) => s.accounts.some((a) => a.tag === tag && a.display_name !== null));
      return social("chat", { kind: "keys", keys: tags.map((tag) => ({ tag: hex(tag), handle: `${person(s, tag).display_name}#${hex(tag)}`, devices: [device(tag)] })) });
    }
    case "inbox": {
      const after = request["after"] as number;
      const page = c.deliveries.filter((d) => d.seq > after).slice(0, 50);
      return social("chat", { kind: "inbox", messages: page.map((d) => deliver(s, d)), more: c.deliveries.filter((d) => d.seq > after).length > 50 });
    }
    case "send": {
      const conv = request["conv"] as string;
      const sealed = request["sealed"] as { hdr: string; ct: string };
      const header = unb64(sealed.hdr);
      const body = unb64(sealed.ct);
      const at = Date.now();
      c.deliveries.push({ seq: ++c.seq, from: me, conv, text: String(body["text"]), at, id: String(header["id"]) });
      // a friend answers a direct message once, a moment later
      const other = conv.startsWith("dm:") ? conv.slice(3).split(":").map((t) => Number.parseInt(t, 16)).find((t) => t !== me) : undefined;
      if (other !== undefined && !c.answered.has(conv)) {
        c.answered.add(conv);
        setTimeout(() => { c.deliveries.push({ seq: ++c.seq, from: other, conv, text: "Sounds good — see you at the table!", at: Date.now(), id: uuid(500 + c.seq) }); }, 1200);
      }
      return social("chat", { kind: "sent", id: header["id"], at });
    }
  }
  return 400;
}

/** The answer to a group command, or a status to refuse it with. */
export function groupAnswer(s: AreaState, c: ChatState, request: Record<string, unknown>): unknown {
  switch (request["command"]) {
    case "list":
      return social("group", { kind: "groups", snapshot: {
        groups: c.groups.map((g) => ({ id: g.id, name: g.name, members: g.members.map((tag) => ({ person: person(s, tag), owner: tag === g.owner })), invited: [] })),
        invites: c.invites.map((i) => ({ group: i.group, name: i.name, inviter: person(s, i.inviter), at: i.at })),
      } });
    case "create": {
      const id = uuid(902 + c.groups.length);
      c.groups.push({ id, name: String(request["name"]), owner: s.accounts[s.me]?.tag ?? 0, members: [s.accounts[s.me]?.tag ?? 0] });
      return social("group", { kind: "created", id });
    }
    case "change": {
      const change = request["change"] as { change: string };
      const invite = c.invites.find((i) => i.group === request["id"]);
      if (invite !== undefined && (change.change === "join" || change.change === "withdraw")) {
        c.invites = c.invites.filter((i) => i !== invite);
        if (change.change === "join") c.groups.push({ id: invite.group, name: invite.name, owner: invite.inviter, members: [invite.inviter, s.accounts[s.me]?.tag ?? 0] });
      }
      return social("group", { kind: "done" });
    }
  }
  return 400;
}

/** The chat's Worker steps (src/chat/steps.ts) as the stand-in Workers speak them, for `STUB` and `CHAT_STUB`. */
export const CHAT_STEPS = `
let chat = null;
const b64 = (v) => btoa(String.fromCharCode(...new TextEncoder().encode(JSON.stringify(v)))).replace(/\\+/g, "-").replace(/\\//g, "_").replace(/=+$/, "");
const unb64 = (s) => JSON.parse(new TextDecoder().decode(Uint8Array.from(atob(s.replace(/-/g, "+").replace(/_/g, "/")), (c) => c.charCodeAt(0))));
const deviceOf = (tag) => (tag.toString(16).padStart(4, "0") + "d0d0d0d0d0d0").slice(0, 16);
const registration = { ik: "A".repeat(43), dk: "B".repeat(43), dsig: "C".repeat(86), label: "Browser" };
const uuid7 = () => { const h = [...crypto.getRandomValues(new Uint8Array(16))].map((b) => b.toString(16).padStart(2, "0")).join(""); return h.slice(0, 8) + "-" + h.slice(8, 12) + "-7" + h.slice(13, 16) + "-8" + h.slice(17, 20) + "-" + h.slice(20, 32); };
function chatStep(r) {
  if (chat === null) throw new Error("failed");
  switch (r.op) {
    case "restore": return { op: r.op };
    case "observe": return { op: r.op, reviews: r.keys.filter((k) => k.devices.length > 0).map((k) => ({ tag: k.tag, number: "1".repeat(60), changed: false, snapshot: k })), trust: new Uint8Array([1]) };
    case "confirm": return { op: r.op, trust: new Uint8Array([1]) };
    case "seal": {
      const hdr = b64({ v: 1, id: uuid7(), conv: r.conv, from: chat.tag, dev: deviceOf(chat.tag), ts: r.now, kind: "text" });
      return { op: r.op, sealed: { hdr, n: "N".repeat(32), ct: b64({ text: r.text, emote: r.emote }), epk: "E".repeat(43), sig: "S".repeat(86), wraps: r.members.map((t) => ({ d: deviceOf(t), k: "K".repeat(64) })) } };
    }
    case "open": return { op: r.op, messages: r.messages.map((m) => {
      const h = unb64(m.sealed.hdr); const c = unb64(m.sealed.ct);
      return { seq: m.seq, message: { id: h.id, conv: h.conv, sender: m.sender.tag, text: c.text, emote: c.emote, at: m.at } };
    }) };
  }
  throw new Error("failed");
}
`;
