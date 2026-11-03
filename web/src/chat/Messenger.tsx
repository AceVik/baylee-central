// The messenger (docs/website-design.md §"Components", the messenger): the
// conversations and contacts beside the open conversation, as one window,
// from 760 px; below that one of the two at a time. Contacts and groups are
// Central's and need no keys; the messages need the unlocked chat, which a
// sign-in brings and a reload takes away (held.ts).
import { useEffect, useMemo, useState, useSyncExternalStore, type FormEvent } from "react";

import { Icon } from "../components/icons";
import { T, num, t, useLang } from "../i18n";
import { useLocation } from "../router";
import { handleOf, socialApi, tagOf, type Change, type Person } from "../social/api";
import { ContactList } from "../social/ContactList";
import { FindContact } from "../social/FindContact";
import { useContacts } from "../social/useContacts";
import { Confirm } from "../app/Confirm";
import { ChatList, NewGroup, SideTools } from "./ChatList";
import { Conversation, type WindowSession } from "./Conversation";
import { conversations, direct } from "./conversations";
import { groupApi } from "./groups";
import { Avatar } from "./pieces";
import type { Snapshot } from "./session";
import { useGroups } from "./useGroups";

const CONTACTS = socialApi();
const GROUPS = groupApi();
const LOCKED: Snapshot = { messages: [], people: [], reviews: [], pending: null, loading: false, error: false, refused: 0, denied: false, busy: false, started: false, seen: null };
const none = () => () => {};
export type Tab = "chats" | "contacts";

function Unlock({ onUnlock, onBack }: { onUnlock(password: string): Promise<boolean>; onBack(): void }) {
  const lang = useLang();
  const [password, setPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(false);
  const submit = (event: FormEvent) => {
    event.preventDefault();
    if (busy || password.length === 0) return;
    const secret = password; setPassword(""); setBusy(true); setError(false);
    void onUnlock(secret).then((ok) => { setBusy(false); setError(!ok); });
  };
  return <div className="chat-lock">
    <button type="button" className="chat-icon-button chat-back" onClick={onBack} aria-label={t(lang, "chat.back")} title={t(lang, "chat.back")}><Icon name="prev" /></button>
    <span className="chat-lock-icon" aria-hidden="true"><Icon name="lock" /></span>
    <h2><T id="chat.unlockTitle" /></h2>
    <p><T id="chat.lockedLead" /></p>
    <form className="chat-inline-row" onSubmit={submit}>
      <label className="vh" htmlFor="chat-password"><T id="chat.password" /></label>
      <input id="chat-password" type="password" autoComplete="current-password" value={password} maxLength={1024} disabled={busy} placeholder={t(lang, "chat.password")} onChange={(event) => setPassword(event.target.value)} required />
      <button className="button" disabled={busy || password.length === 0}>{busy ? t(lang, "app.loading") : t(lang, "chat.unlock")}</button>
    </form>
    {error && <p className="field-error" role="alert"><T id="chat.unlockFailed" /></p>}
  </div>;
}

export function Messenger({ ownTag, ownName, session, opening = false, initialTab = "chats", onUnlock }: {
  ownTag: number; ownName: string; session: WindowSession | null;
  /** The kept chat is being opened: neither the conversation nor the lock yet. */
  opening?: boolean;
  initialTab?: Tab; onUnlock(password: string): Promise<boolean>;
}) {
  const lang = useLang();
  const location = useLocation();
  const data = useSyncExternalStore(session?.subscribe ?? none, session?.getSnapshot ?? (() => LOCKED));
  const contacts = useContacts(CONTACTS);
  const groups = useGroups(GROUPS);
  const [tab, setTab] = useState<Tab>(initialTab);
  const [query, setQuery] = useState("");
  const [making, setMaking] = useState(false);
  const [unlocking, setUnlocking] = useState(false);
  const [confirm, setConfirm] = useState<{ person: Person; change: "block" | "remove" } | null>(null);
  const [selected, setSelected] = useState<string | null>(() => {
    const tag = tagOf(new URLSearchParams(location.search).get("with") ?? "");
    return tag === null || tag === ownTag ? null : direct(ownTag, tag);
  });
  const friends = useMemo(() => contacts.items?.filter((c) => c.relation === "friend").map((c) => c.person) ?? [], [contacts.items]);
  const all = useMemo(() => conversations({ ownTag, friends, known: data.people, groups: groups.groups, messages: data.messages, seen: data.seen }), [ownTag, friends, data.people, data.messages, groups.groups, data.seen]);
  const current = all.find((c) => c.id === selected) ?? null;
  const lastAt = current?.last?.at ?? 0;
  // the open conversation is read, also what arrives in it while it is open
  useEffect(() => { if (selected !== null && lastAt > 0) session?.markSeen(selected); }, [session, selected, lastAt]);
  const needle = query.trim().toLowerCase();
  const list = needle === "" ? all : all.filter((c) => c.title.toLowerCase().includes(needle));
  const incoming = contacts.items?.filter((c) => c.relation === "incoming").length ?? 0;
  const unread = all.reduce((n, c) => n + c.unread, 0);
  const choose = (id: string) => { setSelected(id); setMaking(false); };
  const act = (person: Person, change: Change) => {
    if (change === "block" || change === "remove") setConfirm({ person, change });
    else void contacts.change(person.tag, change);
  };
  const view = current !== null || (session === null && unlocking) ? "conversation" : "list";
  return <div className="chat-app" data-view={view}>
    <aside className="chat-side" aria-label={t(lang, "chat.conversations")}>
      <header className="chat-side-top">
        <Avatar name={ownName} seed={String(ownTag)} />
        <span className="chat-side-me"><bdi translate="no">{ownName}</bdi><span className="chat-row-tag" translate="no">#{ownTag.toString(16).padStart(4, "0")}</span></span>
        <SideTools onNewGroup={() => { setTab("chats"); setMaking((v) => !v); }} onAddContact={() => { setTab("contacts"); document.getElementById("contact-tag")?.focus(); }} />
      </header>
      {session === null && !opening && <p className="chat-side-locked"><Icon name="lock" className="chat-head-lock" /><T id="chat.unlockTitle" /> <button type="button" className="button button-sm" onClick={() => setUnlocking(true)}><T id="chat.unlock" /></button></p>}
      <div className="chat-tabs" role="tablist" aria-label={t(lang, "chat.conversations")}>
        <button type="button" role="tab" id="chat-tab-chats" aria-selected={tab === "chats"} aria-controls="chat-panel" onClick={() => setTab("chats")}>
          <T id="chat.tabChats" />{unread > 0 && <span className="chat-badge num">{num(lang, unread)}</span>}
        </button>
        <button type="button" role="tab" id="chat-tab-contacts" aria-selected={tab === "contacts"} aria-controls="chat-panel" onClick={() => setTab("contacts")}>
          <T id="chat.tabContacts" />{incoming > 0 && <span className="chat-badge num">{num(lang, incoming)}<span className="vh"> {t(lang, "social.incoming")}</span></span>}
        </button>
      </div>
      <div id="chat-panel" role="tabpanel" aria-labelledby={tab === "chats" ? "chat-tab-chats" : "chat-tab-contacts"} className="chat-side-body">
        {tab === "chats" ? <>
          <label className="chat-search">
            <Icon name="search" className="chat-search-icon" />
            <span className="vh"><T id="chat.search" /></span>
            <input type="search" value={query} onChange={(event) => setQuery(event.target.value)} placeholder={t(lang, "chat.search")} autoComplete="off" spellCheck={false} />
          </label>
          {making && <NewGroup groups={groups} onCreated={(id) => choose(`grp:${id}`)} onCancel={() => setMaking(false)} />}
          <ChatList list={list} ownTag={ownTag} selected={selected} locked={session === null} groups={groups} filtered={needle !== ""} onChoose={choose} />
        </> : <>
          <FindContact api={CONTACTS} ownTag={ownTag} busy={contacts.busy} onAdd={(tag) => contacts.change(tag, "request")} />
          {contacts.error && <p className="chat-side-note bad" role="alert"><T id="social.failed" /> <button type="button" className="text-link" onClick={() => { void contacts.reload(); }}><T id="chat.retry" /></button></p>}
          {contacts.items === null ? !contacts.error && <output className="chat-side-empty"><T id="app.loading" /></output>
            : contacts.items.length === 0 ? <p className="chat-side-empty"><T id="social.empty" /></p>
            : <ContactList items={contacts.items} busy={contacts.busy} onChange={act} onOpen={(p) => { choose(direct(ownTag, p.tag)); setTab("chats"); }} />}
        </>}
      </div>
    </aside>
    <section className="chat-main" aria-label={current?.title ?? t(lang, "chat.choose")}>
      {(data.error || contacts.error) && session !== null && <p className="chat-banner bad" role="alert"><T id="chat.failed" /> <button type="button" className="text-link" onClick={() => { void session.poll(); void contacts.reload(); }}><T id="chat.retry" /></button></p>}
      {data.pending !== null && data.pending.conv !== selected && <p className="chat-banner"><button type="button" className="text-link" onClick={() => { if (data.pending !== null) choose(data.pending.conv); }}><T id="chat.pendingOther" /></button></p>}
      {session === null && opening ? <div className="chat-welcome"><output className="chat-note"><T id="app.loading" /></output></div>
        : session === null ? <Unlock onUnlock={onUnlock} onBack={() => { setUnlocking(false); setSelected(null); }} />
        : current === null ? <div className="chat-welcome">
          <span className="chat-lock-icon" aria-hidden="true"><Icon name="chat" /></span>
          <h2><T id="chat.choose" /></h2>
          <p><T id="chat.chooseLead" /></p>
          <p className="chat-note"><Icon name="lock" className="chat-head-lock" /><T id="chat.storage" /></p>
        </div>
        : <Conversation key={current.id} conv={current} session={session} data={data} ownTag={ownTag} friends={friends} groups={groups} onBack={() => setSelected(null)} />}
    </section>
    {confirm !== null && <Confirm title={<bdi translate="no">{handleOf(confirm.person)}</bdi>} action={confirm.change === "block" ? "social.block" : "social.remove"}
      busy={contacts.busy} onCancel={() => { if (!contacts.busy) setConfirm(null); }} onConfirm={() => { if (!contacts.busy) void contacts.change(confirm.person.tag, confirm.change).then((ok) => { if (ok) setConfirm(null); }); }}>
      <p><T id={confirm.change === "block" ? "social.confirmBlock" : "social.confirmRemove"} /></p>
    </Confirm>}
  </div>;
}
