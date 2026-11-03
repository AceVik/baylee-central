// One unlocked account's lifecycle. Network work is coalesced; every await is
// followed by a lifetime check before its result can start another operation.
// A sign-in unlocks it (account/flows.ts, `prepared`) or a password typed on
// the chat page does (`unlock`); either way it lives until the sign-in ends:
// signing out, Central's refusal or another account close it (held.ts).
import { ApiError, sameIdentity, type AccountSummary, type Api } from "../account/api";
import type { Person } from "../social/api";
import type { ChatApi, Keys, Message, Registration, Review, Sealed } from "./api";
import { chatCrypto, type ChatCrypto } from "./crypto";
import { trustStore, type TrustStore } from "./storage";
import { groupApi, type GroupApi } from "./groups";

export interface Pending { conv: string; text: string; sealed: Sealed; members: number[]; failed: boolean }
export interface Snapshot {
  messages: Message[]; people: Person[]; reviews: Review[]; pending: Pending | null; loading: boolean; error: boolean; refused: number; denied: boolean; busy: boolean; started: boolean;
  /** The newest message time looked at, by conversation: what the first complete read brought, then `markSeen`. Null before that read. */
  seen: ReadonlyMap<string, number> | null;
}
const INITIAL: Snapshot = { messages: [], people: [], reviews: [], pending: null, loading: true, error: false, refused: 0, denied: false, busy: false, started: false, seen: null };
/** An unlocked chat identity not yet registered with Central. */
export interface Prepared { device: string; registration: Registration }

export class ChatSession {
  private closed = false;
  private crypto: ChatCrypto;
  private store: TrustStore;
  private device = "";
  private cursor = 0;
  private replay = 0;
  private keyWork: Promise<unknown> = Promise.resolve();
  private available = new Set<number>();
  private polling: Promise<boolean> | null = null;
  private starting: Promise<void> | null = null;
  private restored = false;
  private directory = new Map<string, string>();
  private listeners = new Set<() => void>();
  private snapshot: Snapshot = INITIAL;
  private mutation = false;
  private locked: () => void = () => {};
  private timer: ReturnType<typeof setTimeout> | null = null;
  private delay = 2000;

  constructor(private api: ChatApi, private accountApi: Api, private summary: AccountSummary,
    crypto: ChatCrypto = chatCrypto(), store: TrustStore = trustStore(summary.identity.account), private groups: GroupApi = groupApi(),
    private prepared: Prepared | null = null) {
    this.crypto = crypto; this.store = store;
  }
  getSnapshot = (): Snapshot => this.snapshot;
  subscribe = (listener: () => void): (() => void) => { this.listeners.add(listener); return () => { this.listeners.delete(listener); }; };
  isClosed = (): boolean => this.closed;
  private live() { if (this.closed) throw new Error("closed"); }
  private update(change: Partial<Snapshot>) {
    if (this.closed) return;
    this.snapshot = { ...this.snapshot, ...change };
    for (const listener of this.listeners) listener();
  }
  close = () => {
    if (this.closed) return;
    this.closed = true; this.crypto.close();
    if (this.timer !== null) clearTimeout(this.timer);
    document.removeEventListener("visibilitychange", this.visibility);
    this.directory.clear(); this.available.clear(); this.snapshot = INITIAL; this.listeners.clear();
  };
  private lock = () => { this.close(); this.locked(); };
  // a page shown again reads at once; a hidden one does not read at all
  private visibility = () => {
    if (document.visibilityState !== "visible") return;
    if (this.timer !== null) clearTimeout(this.timer);
    this.schedule(0);
  };
  private schedule(wait: number) {
    if (this.closed) return;
    this.timer = setTimeout(() => {
      this.timer = null;
      const work = document.visibilityState === "visible" ? this.poll() : Promise.resolve(false);
      void work.then((more) => { this.schedule(more ? 0 : this.delay); });
    }, wait);
  }

  /** After a reload: the password's own sign-in, then `start`. Returns the new session's end. */
  async unlock(password: string, onLock: () => void): Promise<number> {
    this.live(); this.locked = onLock;
    if (this.summary.tag === null) throw new Error("profile");
    const start = await this.crypto.ask({ op: "start", password, identity: this.summary.identity }); this.live();
    const challenge = await this.accountApi.loginStart(this.summary.identity, start.message); this.live();
    const finish = await this.crypto.ask({ op: "finish", response: challenge.message }); this.live();
    const session = await this.accountApi.loginFinish(challenge.attempt, finish.message); this.live();
    if (!sameIdentity(session.identity, this.summary.identity)) throw new Error("identity");
    const summary = await this.accountApi.accountSummary(); this.live();
    if (!sameIdentity(summary.identity, this.summary.identity) || summary.tag !== this.summary.tag) throw new Error("identity");
    const ready = await this.crypto.ask({ op: "unlock", wrapped: session.wrappedMaster, tag: this.summary.tag }); this.live();
    this.prepared = { device: ready.device, registration: ready.registration };
    await this.start(onLock);
    return summary.expiresAt;
  }

  /**
   * Restores the peer trust, registers the device and reads the own keys,
   * then reads the inbox while the page is in view. Coalesced; after a
   * failure the next call goes on where it stopped. Central's refusal locks.
   */
  start(onLock: () => void): Promise<void> {
    if (this.closed) return Promise.reject(new Error("closed"));
    this.locked = onLock;
    if (this.snapshot.started) return Promise.resolve();
    this.starting ??= this.begin().finally(() => { this.starting = null; });
    return this.starting;
  }
  private async begin(): Promise<void> {
    const prepared = this.prepared;
    const tag = this.summary.tag;
    if (prepared === null || tag === null) throw new Error("locked");
    try {
      if (!this.restored) {
        const trust = this.store.read();
        if (trust !== null) { await this.crypto.ask({ op: "restore", trust }); this.live(); }
        this.restored = true;
      }
      const registered = await this.api.register(prepared.registration); this.live();
      if (registered !== prepared.device) throw new Error("identity");
      this.device = prepared.device;
      await this.keys([tag]); this.live();
    } catch (error) {
      if (!this.closed) {
        const denied = error instanceof ApiError && error.kind === "rejected";
        this.update({ error: true, loading: false, denied });
        if (denied) this.lock();
      }
      throw error;
    }
    this.update({ started: true, error: false });
    document.addEventListener("visibilitychange", this.visibility);
    this.schedule(0);
  }

  keys(tags: number[]): Promise<Review[]> {
    const work = this.keyWork.then(() => this.observe([...new Set(tags)]));
    this.keyWork = work.catch(() => {});
    return work;
  }
  private async observe(tags: number[]): Promise<Review[]> {
    this.live();
    const keys = await this.api.keys(tags); this.live();
    for (const tag of tags) this.available.delete(tag);
    for (const key of keys) if (key.devices.length > 0) this.available.add(Number.parseInt(key.tag, 16));
    const changed = keys.filter((k) => k.devices.length > 0 && this.directory.get(k.tag) !== JSON.stringify(k));
    if (changed.length > 0) {
      const result = await this.crypto.ask({ op: "observe", keys: changed }); this.live();
      this.store.write(result.trust);
      for (const key of changed) this.directory.set(key.tag, JSON.stringify(key));
      const reviews = new Map(this.snapshot.reviews.map((review) => [review.tag, review]));
      for (const review of result.reviews) reviews.set(review.tag, review);
      this.update({ reviews: [...reviews.values()] });
    }
    return this.snapshot.reviews.filter((r) => tags.includes(Number.parseInt(r.tag, 16)));
  }

  async confirm(snapshot: Keys): Promise<void> {
    this.live();
    const result = await this.crypto.ask({ op: "confirm", snapshot }); this.live();
    this.store.write(result.trust);
    this.update({ reviews: this.snapshot.reviews.map((r) => r.tag === snapshot.tag ? { ...r, changed: false } : r) });
    // Re-read unacknowledged ciphertext after confirming a previously held identity.
    ++this.replay; this.cursor = 0; this.update({ refused: 0 }); void this.poll();
  }

  poll(): Promise<boolean> {
    if (this.closed || this.snapshot.denied || !this.snapshot.started) return Promise.resolve(false);
    if (this.polling !== null) return this.polling;
    this.polling = this.read().finally(() => { this.polling = null; });
    return this.polling;
  }
  private async read(): Promise<boolean> {
    const replay = this.replay;
    try {
      const page = await this.api.inbox(this.device, this.cursor); this.live();
      const senders = [...new Set(page.messages.map((m) => m.sender.tag))];
      if (senders.length > 0) { await this.keys(senders); this.live(); }
      const opened = await this.crypto.ask({ op: "open", messages: page.messages }); this.live();
      if (this.replay !== replay) return true;
      const messages = new Map(this.snapshot.messages.map((m) => [m.id, m]));
      const people = new Map(this.snapshot.people.map((p) => [p.tag, p]));
      let refused = this.snapshot.refused;
      for (const item of opened.messages) {
        if (item.message === null) { refused += 1; continue; }
        const old = messages.get(item.message.id);
        if (old !== undefined && JSON.stringify(old) !== JSON.stringify(item.message)) { refused += 1; continue; }
        messages.set(item.message.id, item.message);
      }
      for (const item of page.messages) people.set(item.sender.tag, { tag: item.sender.tag, displayName: item.sender.display_name });
      this.cursor = page.messages.at(-1)?.seq ?? this.cursor;
      this.delay = 2000;
      const sorted = [...messages.values()].sort((a, b) => a.at - b.at || a.id.localeCompare(b.id)).slice(-5000);
      let seen = this.snapshot.seen;
      if (seen === null && !page.more) {
        const first = new Map<string, number>();
        for (const m of sorted) first.set(m.conv, m.at);
        seen = first;
      }
      this.update({ messages: sorted, people: [...people.values()], refused, loading: false, error: false, seen });
      return page.more;
    } catch (error) {
      if (!this.closed) {
        const denied = error instanceof ApiError && error.kind === "rejected";
        this.delay = Math.min(this.delay * 2, 30_000);
        this.update({ error: true, loading: false, denied });
        if (denied) this.lock();
      }
      return false;
    }
  }

  /** The conversation's messages up to now count as read. */
  markSeen(conv: string): void {
    const seen = this.snapshot.seen;
    if (this.closed || seen === null) return;
    const last = this.snapshot.messages.findLast((m) => m.conv === conv)?.at ?? 0;
    if (last > (seen.get(conv) ?? 0)) this.update({ seen: new Map(seen).set(conv, last) });
  }

  private async recipients(conv: string, members: number[]): Promise<number[]> {
    if (!conv.startsWith("grp:")) return members;
    const snapshot = await this.groups.list(); this.live();
    const group = snapshot.groups.find((g) => `grp:${g.id}` === conv);
    if (group === undefined || !group.members.some((m) => m.person.tag === this.summary.tag)) throw new Error("membership ended");
    return group.members.map((m) => m.person.tag);
  }

  async send(conv: string, text: string, members: number[]): Promise<boolean> {
    this.live();
    if (this.mutation || this.snapshot.pending !== null || !this.snapshot.started) return false;
    this.mutation = true; this.update({ busy: true });
    try {
      members = await this.recipients(conv, members); this.live();
      const reviews = await this.keys(members); this.live();
      if (reviews.some((review) => review.changed) || members.some((tag) => !this.available.has(tag))) throw new Error("recipient unavailable");
      const result = await this.crypto.ask({ op: "seal", conv, text, emote: false, members, now: Date.now() }); this.live();
      this.update({ pending: { conv, text, members: [...members], sealed: result.sealed, failed: false } });
      return await this.deliver();
    } catch { this.update({ error: true }); return false; }
    finally { this.mutation = false; this.update({ busy: false }); }
  }
  discard(sealed: Sealed): void {
    this.live();
    if (!this.mutation && this.snapshot.pending?.sealed === sealed) this.update({ pending: null });
  }
  async retry(): Promise<boolean> {
    this.live();
    if (this.mutation || this.snapshot.pending === null) return false;
    this.mutation = true; this.update({ busy: true });
    try {
      const pending = this.snapshot.pending;
      const members = await this.recipients(pending.conv, pending.members); this.live();
      if (members.length !== pending.members.length || members.some((tag) => !pending.members.includes(tag))) return false;
      const reviews = await this.keys(pending.members); this.live();
      if (reviews.some((review) => review.changed) || pending.members.some((tag) => !this.available.has(tag))) return false;
      return await this.deliver();
    } catch { this.update({ error: true }); return false; }
    finally { this.mutation = false; this.update({ busy: false }); }
  }
  private async deliver(): Promise<boolean> {
    const pending = this.snapshot.pending;
    if (pending === null) return false;
    try {
      await this.api.send(pending.conv, pending.sealed); this.live();
      this.update({ pending: null, error: false }); void this.poll(); return true;
    } catch (error) {
      // A Conflict proves that this exact envelope was not accepted. Keep the
      // composer's text so a fresh membership/key snapshot can seal it again.
      if (error instanceof ApiError && error.kind === "conflict") this.update({ pending: null, error: true });
      else this.update({ pending: { ...pending, failed: true } });
      return false;
    }
  }
}
