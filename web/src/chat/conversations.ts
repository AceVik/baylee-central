// The messenger's list: every conversation the page knows of, from the
// accepted contacts, the groups and the history the inbox replayed, each
// with its last message and what arrived since it was last looked at.
import { handleOf, tagOf, type Person } from "../social/api";
import type { Message } from "./api";
import type { Group } from "./groups";

export type ConvKind = "direct" | "group" | "former";
export interface Conversation {
  id: string;
  kind: ConvKind;
  /** The handle, the group's name, or a former group's id. */
  title: string;
  person: Person | null;
  group: Group | null;
  last: Message | null;
  unread: number;
  /** A friend (direct) or a member (group): messages can be sent. */
  open: boolean;
}

export function direct(a: number, b: number): string { return `dm:${[a, b].sort((x, y) => x - y).map((tag) => tag.toString(16).padStart(4, "0")).join(":")}`; }

/** The other side of a direct conversation of `ownTag`'s, or null. */
export function partner(conv: string | null, ownTag: number): number | null {
  if (!conv?.startsWith("dm:")) return null;
  const tags = conv.slice(3).split(":").map((tag) => tagOf(tag));
  return tags.length === 2 && tags.includes(ownTag) ? tags.find((tag) => tag !== ownTag) ?? null : null;
}

export function conversations({ ownTag, friends, known, groups, messages, seen }: {
  ownTag: number; friends: Person[]; known: Person[]; groups: Group[]; messages: Message[];
  /** The newest message time looked at, by conversation; null before the inbox was first read through. */
  seen: ReadonlyMap<string, number> | null;
}): Conversation[] {
  const last = new Map<string, Message>();
  const unread = new Map<string, number>();
  for (const m of messages) {
    last.set(m.conv, m);
    if (seen !== null && m.sender !== ownTag && m.at > (seen.get(m.conv) ?? 0)) unread.set(m.conv, (unread.get(m.conv) ?? 0) + 1);
  }
  const names = new Map(known.map((p) => [p.tag, p]));
  const people = new Map(friends.map((p) => [p.tag, p]));
  const out: Conversation[] = [];
  const add = (c: Omit<Conversation, "last" | "unread">) => out.push({ ...c, last: last.get(c.id) ?? null, unread: unread.get(c.id) ?? 0 });
  for (const person of friends) add({ id: direct(ownTag, person.tag), kind: "direct", title: handleOf(person), person, group: null, open: true });
  for (const conv of last.keys()) {
    const tag = partner(conv, ownTag);
    if (tag === null || people.has(tag)) continue;
    const person = names.get(tag) ?? { tag, displayName: "?" };
    people.set(tag, person);
    add({ id: conv, kind: "direct", title: handleOf(person), person, group: null, open: false });
  }
  for (const group of groups) add({ id: `grp:${group.id}`, kind: "group", title: group.name, person: null, group, open: group.members.some((m) => m.person.tag === ownTag) });
  for (const conv of last.keys()) {
    if (conv.startsWith("grp:") && !groups.some((g) => `grp:${g.id}` === conv)) add({ id: conv, kind: "former", title: conv.slice(4), person: null, group: null, open: false });
  }
  // the newest talk first, then the others by name
  return out.sort((a, b) => (b.last?.at ?? 0) - (a.last?.at ?? 0) || a.title.localeCompare(b.title));
}

/** An avatar's letters: the first of a name, or of its first two words. */
export function initials(name: string): string {
  const words = name.replace(/#[0-9a-f]+$/, "").split(/[\s_\-.]+/).filter(Boolean);
  const letters = words.length > 1 ? [words[0]?.[0], words[1]?.[0]] : [...(words[0] ?? "?")].slice(0, 1);
  return letters.join("").toUpperCase() || "?";
}

/** One of six avatar tints, the same for the same person or group everywhere. */
export function hue(seed: string): number {
  let h = 0;
  for (const c of seed) h = (h * 31 + (c.codePointAt(0) ?? 0)) >>> 0;
  return h % 6;
}
