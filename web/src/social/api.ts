// Central's shared contact protocol. Every untrusted response is checked before use.
import { answer, exact, malformed, poster, safeInteger, type Endpoint, type Json } from "../account/api";
import { validDisplayName, validTag } from "../account/names";

export interface Person { tag: number; displayName: string }
export type Relation = "friend" | "incoming" | "outgoing" | "blocked";
export type Change = "request" | "accept" | "remove" | "block" | "unblock";
export interface Contact { person: Person; relation: Relation }
export interface SocialApi {
  contacts(): Promise<Contact[]>;
  lookup(tag: number): Promise<Person | null>;
  change(tag: number, change: Change): Promise<void>;
}

export function publicPerson(raw: unknown): Person {
  const p = exact(raw, ["tag", "display_name"]);
  const tag = safeInteger(p["tag"], 0);
  const name = p["display_name"];
  if (!validTag(tag) || typeof name !== "string" || !validDisplayName(name)) throw malformed();
  return { tag, displayName: name };
}

function response(raw: unknown, kind: string, fields: string[]): Json {
  const r = exact(answer(raw, "social", ["response"])["response"], ["kind", ...fields]);
  if (r["kind"] !== kind) throw malformed();
  return r;
}

export function contacts(raw: unknown): Contact[] {
  const list = response(raw, "contacts", ["contacts"])["contacts"];
  if (!Array.isArray(list) || list.length > 1650) throw malformed();
  const seen = new Set<number>();
  const counts = { friend: 0, incoming: 0, outgoing: 0, blocked: 0 };
  const caps = { friend: 300, incoming: 300, outgoing: 50, blocked: 1000 };
  return list.map((entry: unknown) => {
    const c = exact(entry, ["person", "relation"]);
    const person = publicPerson(c["person"]);
    const relation = c["relation"];
    if (relation !== "friend" && relation !== "incoming" && relation !== "outgoing" && relation !== "blocked") throw malformed();
    if (seen.has(person.tag) || ++counts[relation] > caps[relation]) throw malformed();
    seen.add(person.tag);
    return { person, relation };
  });
}

/** Accept a canonical tag or a complete public handle, never fuzzy matching a name. */
export function tagOf(input: string): number | null {
  const text = input.trim().split("#").at(-1) ?? "";
  if (!/^(?:[0-9a-f]{4}|[1-9a-f][0-9a-f]{4,7})$/.test(text)) return null;
  const tag = Number.parseInt(text, 16);
  return validTag(tag) ? tag : null;
}

export const handleOf = (p: Person): string => `${p.displayName}#${p.tag.toString(16).padStart(4, "0")}`;

export function socialApi(endpoint: Endpoint = {}): SocialApi {
  const post = poster(endpoint);
  const run = <T>(request: Json, read: (raw: unknown) => T) => post({ operation: "social", request }, read);
  return {
    contacts: () => run({ action: "contacts" }, contacts),
    lookup: (tag) => run({ action: "lookup", tag }, (raw) => {
      const p = response(raw, "person", ["person"])["person"];
      const found = p === null ? null : publicPerson(p);
      if (found !== null && found.tag !== tag) throw malformed();
      return found;
    }),
    change: (tag, change) => run({ action: "contact", tag, change }, (raw) => { response(raw, "done", []); }),
  };
}
