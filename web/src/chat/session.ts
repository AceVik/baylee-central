// One unlocked account's lifecycle. Network work is coalesced; every await is
// followed by a lifetime check before its result can start another operation.
import { ApiError, sameIdentity, type AccountSummary, type Api } from "../account/api";
import type { Person } from "../social/api";
import type { ChatApi, Keys, Message, Review, Sealed } from "./api";
import { chatCrypto, type ChatCrypto } from "./crypto";
import { trustStore, type TrustStore } from "./storage";
import { groupApi, type GroupApi } from "./groups";

export interface Pending { conv: string; text: string; sealed: Sealed; members: number[]; failed: boolean }
export interface Snapshot { messages: Message[]; people: Person[]; reviews: Review[]; pending: Pending | null; loading: boolean; error: boolean; refused: number; denied: boolean; busy: boolean }
const INITIAL: Snapshot = { messages: [], people: [], reviews: [], pending: null, loading: true, error: false, refused: 0, denied: false, busy: false };

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
  private directory = new Map<string, string>();
  private listeners = new Set<() => void>();
  private snapshot: Snapshot = INITIAL;
  private mutation = false;
  private hiddenAt: number | null = null;
  private locked: () => void = () => {};
  private timer: ReturnType<typeof setTimeout> | null = null;
  private delay = 2000;

  constructor(private api: ChatApi, private accountApi: Api, private summary: AccountSummary,
    crypto: ChatCrypto = chatCrypto(), store: TrustStore = trustStore(summary.identity.account), private groups: GroupApi = groupApi()) {
    this.crypto = crypto; this.store = store;
  }
  getSnapshot = (): Snapshot => this.snapshot;
  subscribe = (listener: () => void): (() => void) => { this.listeners.add(listener); return () => { this.listeners.delete(listener); }; };
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
    window.removeEventListener("pagehide", this.lock);
    this.directory.clear(); this.available.clear(); this.snapshot = INITIAL; this.listeners.clear();
  };
  private lock = () => { this.close(); this.locked(); };
  private visibility = () => {
    if (document.visibilityState !== "visible") { this.hiddenAt = Date.now(); return; }
    if (this.hiddenAt !== null && Date.now() - this.hiddenAt >= 5 * 60_000) { this.lock(); return; }
    this.hiddenAt = null;
    if (this.timer !== null) clearTimeout(this.timer);
    this.schedule(0);
  };
  private schedule(wait: number) {
    if (this.closed) return;
    this.timer = setTimeout(() => {
      this.timer = null;
      if (this.hiddenAt !== null && Date.now() - this.hiddenAt >= 5 * 60_000) { this.lock(); return; }
      const work = document.visibilityState === "visible" ? this.poll() : Promise.resolve(false);
      void work.then((more) => { this.schedule(more ? 0 : this.delay); });
    }, wait);
  }

  async unlock(password: string, onLock: () => void): Promise<number> {
    this.live(); this.locked = onLock;
    if (this.summary.tag === null) throw new Error("profile");
    const trust = this.store.read();
    const start = await this.crypto.ask({ op: "start", password, identity: this.summary.identity }); this.live();
    const challenge = await this.accountApi.loginStart(this.summary.identity, start.message); this.live();
    const finish = await this.crypto.ask({ op: "finish", response: challenge.message }); this.live();
    const session = await this.accountApi.loginFinish(challenge.attempt, finish.message); this.live();
    if (!sameIdentity(session.identity, this.summary.identity)) throw new Error("identity");
    const summary = await this.accountApi.accountSummary(); this.live();
    if (!sameIdentity(summary.identity, this.summary.identity) || summary.tag !== this.summary.tag) throw new Error("identity");
    const ready = await this.crypto.ask({ op: "unlock", wrapped: session.wrappedMaster, tag: this.summary.tag, trust }); this.live();
    const registered = await this.api.register(ready.registration); this.live();
    if (registered !== ready.device) throw new Error("identity");
    this.device = ready.device;
    await this.keys([this.summary.tag]); this.live();
    document.addEventListener("visibilitychange", this.visibility);
    window.addEventListener("pagehide", this.lock);
    this.schedule(0);
    return summary.expiresAt;
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
    if (this.closed || this.snapshot.denied) return Promise.resolve(false);
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
      this.update({ messages: [...messages.values()].sort((a, b) => a.at - b.at || a.id.localeCompare(b.id)).slice(-5000), people: [...people.values()], refused, loading: false, error: false });
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

  private async recipients(conv: string, members: number[]): Promise<number[]> {
    if (!conv.startsWith("grp:")) return members;
    const snapshot = await this.groups.list(); this.live();
    const group = snapshot.groups.find((g) => `grp:${g.id}` === conv);
    if (group === undefined || !group.members.some((m) => m.person.tag === this.summary.tag)) throw new Error("membership ended");
    return group.members.map((m) => m.person.tag);
  }

  async send(conv: string, text: string, members: number[]): Promise<boolean> {
    this.live();
    if (this.mutation || this.snapshot.pending !== null) return false;
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
