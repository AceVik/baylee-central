import { answer, exact, malformed, poster, safeInteger, type Endpoint, type Json } from "../account/api";
import { publicPerson, type Person } from "../social/api";

export interface Member { person: Person; owner: boolean }
export interface Group { id: string; name: string; members: Member[]; invited: Person[] }
export interface Invite { group: string; name: string; inviter: Person; at: number }
export interface Groups { groups: Group[]; invites: Invite[] }
export type GroupChange = { change: "rename"; name: string } | { change: "invite" | "withdraw" | "remove" | "transfer"; tag: number } | { change: "join" | "delete" };
export interface GroupApi {
  list(): Promise<Groups>;
  create(name: string): Promise<string>;
  change(id: string, change: GroupChange): Promise<void>;
}
export function groupId(raw: unknown): string {
  if (typeof raw !== "string" || !/^[\da-f]{8}-[\da-f]{4}-7[\da-f]{3}-[89ab][\da-f]{3}-[\da-f]{12}$/.test(raw)) throw malformed();
  return raw;
}
function groupName(raw: unknown): string {
  if (typeof raw !== "string" || raw.trim().length === 0 || [...raw].length > 48 || /[\p{Cc}\u00ad\u061c\u180e\u200b-\u200f\u2028-\u202e\u2060-\u206f\ufeff\ufff9-\ufffb]/u.test(raw)) throw malformed();
  return raw;
}
function list(raw: unknown, max: number): unknown[] {
  if (!Array.isArray(raw) || raw.length > max) throw malformed();
  return raw;
}
function group(raw: unknown): Group {
  const g = exact(raw, ["id", "name", "members", "invited"]);
  const members = list(g["members"], 50).map((entry) => {
    const member = exact(entry, ["person", "owner"]);
    if (typeof member["owner"] !== "boolean") throw malformed();
    return { person: publicPerson(member["person"]), owner: member["owner"] };
  });
  const invited = list(g["invited"], 50).map(publicPerson);
  const people = [...members.map((m) => m.person), ...invited];
  if (people.length > 50 || members.filter((m) => m.owner).length !== 1 || new Set(people.map((p) => p.tag)).size !== people.length) throw malformed();
  return { id: groupId(g["id"]), name: groupName(g["name"]), members, invited };
}
export function groupSnapshot(raw: unknown): Groups {
  const s = exact(raw, ["groups", "invites"]);
  const groups = list(s["groups"], 20).map(group);
  const invites = list(s["invites"], 20).map((entry) => {
    const i = exact(entry, ["group", "name", "inviter", "at"]);
    return { group: groupId(i["group"]), name: groupName(i["name"]), inviter: publicPerson(i["inviter"]), at: safeInteger(i["at"], 0) };
  });
  const ids = [...groups.map((g) => g.id), ...invites.map((i) => i.group)];
  if (ids.length > 20 || new Set(ids).size !== ids.length) throw malformed();
  return { groups, invites };
}
function response(raw: unknown, kind: string, fields: string[]): Json {
  const s = exact(answer(raw, "social", ["response"])["response"], ["kind", "response"]);
  if (s["kind"] !== "group") throw malformed();
  const r = exact(s["response"], ["kind", ...fields]);
  if (r["kind"] !== kind) throw malformed();
  return r;
}
export function groupApi(endpoint: Endpoint = {}): GroupApi {
  const post = poster(endpoint);
  const run = <T>(request: Json, read: (raw: unknown) => T) => post({ operation: "social", request: { action: "group", request } }, read);
  return {
    list: () => run({ command: "list" }, (raw) => groupSnapshot(response(raw, "groups", ["snapshot"])["snapshot"])),
    create: (name) => run({ command: "create", name }, (raw) => groupId(response(raw, "created", ["id"])["id"])),
    change: (id, change) => run({ command: "change", id, change }, (raw) => { response(raw, "done", []); }),
  };
}
