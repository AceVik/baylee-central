import { useEffect, useMemo, useRef, useState, useSyncExternalStore } from "react";
import { Confirm } from "../app/Confirm";
import { T, t, useLang } from "../i18n";
import { Link, PATHS, useLocation } from "../router";
import { handleOf, socialApi, tagOf, type Person } from "../social/api";
import { useContacts } from "../social/useContacts";
import type { Review, Sealed } from "./api";
import { GroupList, GroupSettings } from "./GroupPanel";
import { groupApi } from "./groups";
import { useGroups } from "./useGroups";
import type { ChatSession } from "./session";

const CONTACTS = socialApi();
const GROUPS = groupApi();
export function direct(a: number, b: number): string { return `dm:${[a, b].sort((x, y) => x - y).map((tag) => tag.toString(16).padStart(4, "0")).join(":")}`; }
function partner(conv: string | null, ownTag: number): number | null {
  if (!conv?.startsWith("dm:")) return null;
  const tags = conv.slice(3).split(":").map((tag) => tagOf(tag));
  return tags.length === 2 && tags.includes(ownTag) ? tags.find((tag) => tag !== ownTag) ?? null : null;
}

type WindowSession = Pick<ChatSession, "subscribe" | "getSnapshot" | "keys" | "send" | "poll" | "retry" | "discard" | "confirm">;
export function ChatWindow({ session, ownTag, onLock }: { session: WindowSession; ownTag: number; onLock(): void }) {
  const lang = useLang();
  const location = useLocation();
  const data = useSyncExternalStore(session.subscribe, session.getSnapshot);
  const contacts = useContacts(CONTACTS);
  const groups = useGroups(GROUPS);
  const [conv, setConv] = useState<string | null>(() => {
    const tag = tagOf(new URLSearchParams(location.search).get("with") ?? "");
    return tag === null || tag === ownTag ? null : direct(ownTag, tag);
  });
  const [text, setText] = useState("");
  const [review, setReview] = useState<Review | null>(null);
  const [discard, setDiscard] = useState<Sealed | null>(null);
  const [verifying, setVerifying] = useState(false);
  const [verifyError, setVerifyError] = useState(false);
  const [visible, setVisible] = useState(100);
  const scroll = useRef<HTMLDivElement>(null);
  const follow = useRef(true);
  const olderHeight = useRef<number | null>(null);
  const friends = useMemo(() => contacts.items?.filter((c) => c.relation === "friend").map((c) => c.person) ?? [], [contacts.items]);
  const people = useMemo(() => {
    const known = new Map<number, Person>(data.people.map((p) => [p.tag, p]));
    const map = new Map<number, Person>(friends.map((p) => [p.tag, p]));
    const peers = new Set(data.messages.map((m) => partner(m.conv, ownTag)));
    peers.add(partner(conv, ownTag));
    for (const tag of peers) if (tag !== null && !map.has(tag)) map.set(tag, known.get(tag) ?? { tag, displayName: "?" });
    return [...map.values()].sort((a, b) => handleOf(a).localeCompare(handleOf(b)));
  }, [data.people, data.messages, friends, ownTag, conv]);
  const peerTag = partner(conv, ownTag);
  const peer = people.find((p) => p.tag === peerTag);
  const groupId = conv?.startsWith("grp:") ? conv.slice(4) : null;
  const group = groups.groups.find((g) => g.id === groupId);
  const oldGroups = useMemo(() => [...new Set(data.messages.map((m) => m.conv).filter((id) => id.startsWith("grp:") && !groups.groups.some((g) => `grp:${g.id}` === id)))], [data.messages, groups.groups]);
  const members = useMemo(() => group !== undefined ? group.members.map((m) => m.person.tag) : peerTag !== null ? [ownTag, peerTag] : [], [group, ownTag, peerTag]);
  const reviews = data.reviews.filter((r) => members.includes(Number.parseInt(r.tag, 16)) && Number.parseInt(r.tag, 16) !== ownTag);
  const changed = reviews.some((r) => r.changed);
  const canSend = group !== undefined || friends.some((p) => p.tag === peerTag);
  const messages = useMemo(() => data.messages.filter((m) => m.conv === conv), [data.messages, conv]);
  const title = group?.name ?? (groupId === null ? (peer === undefined ? t(lang, "chat.choose") : handleOf(peer)) : `${t(lang, "chat.formerGroup")} · ${groupId}`);
  useEffect(() => { if (members.length > 0) void session.keys(members).catch(() => {}); }, [members, session]);
  useEffect(() => {
    if (conv === null || messages.length === 0) return;
    const node = scroll.current;
    if (node === null) return;
    if (olderHeight.current !== null) { node.scrollTop += node.scrollHeight - olderHeight.current; olderHeight.current = null; }
    else if (follow.current && visible > 0) node.scrollTop = node.scrollHeight;
  }, [conv, messages.length, visible]);
  const choose = (id: string) => { setConv(id); setText(""); setVisible(100); follow.current = true; olderHeight.current = null; };
  async function send() {
    if (conv === null || !canSend || text.trim().length === 0) return;
    const body = text;
    const ok = await session.send(conv, body, members);
    if (ok || session.getSnapshot().pending?.text === body) setText("");
  }
  return <>
    <div className="chat-tools"><p><T id="chat.private" /></p><button className="button button-ghost" onClick={onLock}><T id="chat.lock" /></button></div>
    {(data.error || contacts.error) && <p role="alert"><T id="chat.failed" /> <button className="text-link" onClick={() => { void session.poll(); void contacts.reload(); }}><T id="chat.retry" /></button></p>}
    {data.refused > 0 && <output><T id="chat.refused" /></output>}
    {data.pending !== null && data.pending.conv !== conv && <p><button className="text-link" onClick={() => { if (data.pending !== null) choose(data.pending.conv); }}><T id="chat.pendingOther" /></button></p>}
    <div className="chat-layout">
      <aside className="chat-peers" aria-label={t(lang, "chat.conversations")}>
        <h2><T id="chat.conversations" /></h2>
        <Link to={PATHS.accountContacts} className="text-link"><T id="chat.manageContacts" /></Link>
        {people.length === 0 && <p><T id="chat.noContacts" /></p>}
        <ul>{people.map((p) => <li key={p.tag}><button aria-current={p.tag === peerTag ? "true" : undefined} onClick={() => choose(direct(ownTag, p.tag))}><bdi translate="no">{handleOf(p)}</bdi></button></li>)}</ul>
        <GroupList state={groups} ownTag={ownTag} selected={groupId} onChoose={(id) => choose(`grp:${id}`)} />
        {oldGroups.length > 0 && <><h3><T id="chat.formerGroup" /></h3><ul>{oldGroups.map((id) => <li key={id}><button onClick={() => choose(id)} aria-current={id === conv ? "true" : undefined}>{id.slice(4)}</button></li>)}</ul></>}
      </aside>
      <section className="chat-conversation" aria-label={title}>
        {conv === null ? <p className="chat-placeholder"><T id="chat.choose" /></p> : <>
          <header><h2><bdi translate="no">{title}</bdi></h2></header>
          {group !== undefined && <GroupSettings key={group.id} group={group} ownTag={ownTag} friends={friends} state={groups} />}
          {reviews.length > 0 && <details className="chat-safety"><summary><T id="chat.safety" /></summary><ul>{reviews.map((r) => <li key={r.tag}><button className="text-link" onClick={() => { setReview(r); setVerifyError(false); }}><bdi>{r.snapshot.handle}</bdi>{r.changed && " ⚠"}</button></li>)}</ul></details>}
          {changed && <p role="alert"><T id="chat.keysChanged" /></p>}
          <div className="chat-messages" ref={scroll} onScroll={() => { const node = scroll.current; if (node !== null) follow.current = node.scrollHeight - node.scrollTop - node.clientHeight < 80; }} role="log" aria-label={t(lang, "chat.messages")} aria-live="polite" aria-relevant="additions">
            {data.loading && <output><T id="app.loading" /></output>}
            {messages.length > visible && <button className="text-link" onClick={() => { olderHeight.current = scroll.current?.scrollHeight ?? null; setVisible((n) => n + 100); }}><T id="chat.older" /></button>}
            {!data.loading && messages.length === 0 && <p className="app-empty"><T id="chat.empty" /></p>}
            {messages.slice(-visible).map((m) => <article key={m.id} className="chat-message" data-own={m.sender === ownTag}>
              {groupId !== null && <small><bdi>{data.people.find((p) => p.tag === m.sender)?.displayName ?? `#${m.sender.toString(16)}`}</bdi></small>}
              <p className={m.emote ? "chat-emote" : undefined}>{m.text}</p>
              <time dateTime={new Date(m.at).toISOString()}>{new Intl.DateTimeFormat(lang, { dateStyle: "short", timeStyle: "short" }).format(m.at)}</time>
            </article>)}
            {data.pending?.conv === conv && <article className="chat-message chat-pending" data-own="true"><p>{data.pending.text}</p><span><T id={data.pending.failed ? "chat.notConfirmed" : "chat.sending"} /></span>{data.pending.failed && <><button disabled={data.busy || changed} onClick={() => { void session.retry(); }}><T id="chat.retry" /></button><button disabled={data.busy} onClick={() => setDiscard(data.pending?.sealed ?? null)}><T id="chat.discard" /></button></>}</article>}
          </div>
          {canSend ? <form className="chat-compose" onSubmit={(event) => { event.preventDefault(); void send(); }}>
            <label className="vh" htmlFor="chat-message"><T id="chat.message" /></label>
            <textarea id="chat-message" rows={3} maxLength={1000} value={text} disabled={data.busy || data.pending !== null} onChange={(event) => setText(event.target.value)} placeholder={t(lang, "chat.message")} />
            <button className="button" disabled={data.busy || data.pending !== null || changed || text.trim().length === 0}><T id="chat.send" /></button>
          </form> : <p><T id={groupId === null ? "chat.friendRequired" : "chat.groupLeft"} /></p>}
        </>}
      </section>
    </div>
    <p className="chat-note"><T id="chat.storage" /></p>
    {discard !== null && <Confirm title={<T id="chat.discard" />} action="chat.discard" onCancel={() => setDiscard(null)} onConfirm={() => { session.discard(discard); setDiscard(null); }}><p><T id="chat.discardConfirm" /></p></Confirm>}
    {review !== null && <Confirm title={<T id="chat.safety" />} action="chat.verify" danger={false} busy={verifying} onCancel={() => { if (!verifying) setReview(null); }} onConfirm={() => {
      if (verifying) return; setVerifying(true); setVerifyError(false);
      void session.confirm(review.snapshot).then(() => setReview(null), () => setVerifyError(true)).finally(() => setVerifying(false));
    }}><p><bdi>{review.snapshot.handle}</bdi></p><p><T id="chat.compare" /></p><p className="chat-number" translate="no">{review.number.match(/.{1,5}/g)?.join(" ")}</p>{verifyError && <p role="alert"><T id="chat.failed" /></p>}</Confirm>}
  </>;
}
