// One conversation of the messenger: its head (who, the safety number, a
// group's settings), the messages as bubbles by day, a send in flight, and
// the composer. Text is plain text; nothing a message says becomes markup.
import { Fragment, useEffect, useMemo, useRef, useState, type KeyboardEvent } from "react";

import { Confirm } from "../app/Confirm";
import { Icon } from "../components/icons";
import { T, localeOf, t, useLang } from "../i18n";
import type { Person } from "../social/api";
import type { Review, Sealed } from "./api";
import type { Conversation as Conv } from "./conversations";
import { GroupSettings } from "./GroupPanel";
import { Avatar, clock, dayLabel, sameDay } from "./pieces";
import type { ChatSession, Snapshot } from "./session";
import type { GroupState } from "./useGroups";

export type WindowSession = Pick<ChatSession, "subscribe" | "getSnapshot" | "keys" | "send" | "poll" | "retry" | "discard" | "confirm" | "markSeen">;
const coarse = () => typeof matchMedia === "function" && matchMedia("(pointer: coarse)").matches;

export function Conversation({ conv, session, data, ownTag, friends, groups, onBack }: {
  conv: Conv; session: WindowSession; data: Snapshot; ownTag: number; friends: Person[]; groups: GroupState; onBack(): void;
}) {
  const lang = useLang();
  const [text, setText] = useState("");
  const [review, setReview] = useState<Review | null>(null);
  const [discard, setDiscard] = useState<Sealed | null>(null);
  const [verifying, setVerifying] = useState(false);
  const [verifyError, setVerifyError] = useState(false);
  const [settings, setSettings] = useState(false);
  const [visible, setVisible] = useState(100);
  const scroll = useRef<HTMLDivElement>(null);
  const follow = useRef(true);
  const olderHeight = useRef<number | null>(null);
  const group = conv.group;
  const peerTag = conv.person?.tag ?? null;
  const members = useMemo(() => group !== null ? group.members.map((m) => m.person.tag) : peerTag !== null ? [ownTag, peerTag] : [], [group, ownTag, peerTag]);
  const reviews = data.reviews.filter((r) => members.includes(Number.parseInt(r.tag, 16)) && Number.parseInt(r.tag, 16) !== ownTag);
  const changed = reviews.some((r) => r.changed);
  const canSend = conv.open && (group !== null || friends.some((p) => p.tag === peerTag));
  const messages = useMemo(() => data.messages.filter((m) => m.conv === conv.id), [data.messages, conv.id]);
  const shown = messages.slice(-visible);
  const name = conv.person?.displayName ?? conv.title;
  useEffect(() => { if (members.length > 0) void session.keys(members).catch(() => {}); }, [members, session]);
  const pendingHere = data.pending?.conv === conv.id;
  useEffect(() => {
    const node = scroll.current;
    if (node === null || (messages.length === 0 && !pendingHere)) return;
    if (olderHeight.current !== null) { node.scrollTop += node.scrollHeight - olderHeight.current; olderHeight.current = null; }
    else if (follow.current && visible > 0) node.scrollTop = node.scrollHeight;
  }, [messages.length, visible, pendingHere]);
  async function send() {
    if (!canSend || text.trim().length === 0 || data.busy || data.pending !== null || changed) return;
    const body = text;
    follow.current = true;
    const ok = await session.send(conv.id, body, members);
    if (ok || session.getSnapshot().pending?.text === body) setText("");
  }
  const keys = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (event.key === "Enter" && !event.shiftKey && !event.nativeEvent.isComposing && !coarse()) { event.preventDefault(); void send(); }
  };
  const subtitle = group !== null ? t(lang, "chat.members", { count: group.members.length })
    : conv.kind === "former" ? t(lang, "chat.formerGroup") : t(lang, "chat.encrypted");
  return <>
    <header className="chat-head">
      <button type="button" className="chat-icon-button chat-back" onClick={onBack} aria-label={t(lang, "chat.back")} title={t(lang, "chat.back")}><Icon name="prev" /></button>
      <Avatar name={name} seed={conv.id} group={conv.kind !== "direct"} />
      <div className="chat-head-text">
        <h2><bdi translate="no">{name}</bdi>{conv.person !== null && <span className="chat-row-tag" translate="no">#{conv.person.tag.toString(16).padStart(4, "0")}</span>}</h2>
        <p><Icon name="lock" className="chat-head-lock" />{subtitle}</p>
      </div>
      <div className="chat-head-tools">
        {reviews.length > 0 && <details className="chat-safety">
          <summary className="chat-icon-button" data-warn={changed || undefined} title={t(lang, "chat.safety")}><Icon name={changed ? "warn" : "shield"} /><span className="vh"><T id="chat.safety" /></span></summary>
          <ul className="chat-menu-pop">{reviews.map((r) => <li key={r.tag}><button type="button" onClick={() => { setReview(r); setVerifyError(false); }}><bdi>{r.snapshot.handle}</bdi>{r.changed && " ⚠"}</button></li>)}</ul>
        </details>}
        {group !== null && <button type="button" className="chat-icon-button" aria-expanded={settings} aria-controls="chat-group-settings" onClick={() => setSettings((v) => !v)} title={t(lang, "chat.groupManage")}><Icon name="sliders" /><span className="vh"><T id="chat.groupManage" /></span></button>}
      </div>
    </header>
    {group !== null && settings && <GroupSettings key={group.id} group={group} ownTag={ownTag} friends={friends} state={groups} onClose={() => setSettings(false)} />}
    {changed && <p className="chat-banner bad" role="alert"><Icon name="warn" className="chat-banner-icon" /><T id="chat.keysChanged" /></p>}
    {data.refused > 0 && <output className="chat-banner"><T id="chat.refused" /></output>}
    <div className="chat-messages" ref={scroll} onScroll={() => { const node = scroll.current; if (node !== null) follow.current = node.scrollHeight - node.scrollTop - node.clientHeight < 80; }} role="log" aria-label={t(lang, "chat.messages")} aria-live="polite" aria-relevant="additions">
      {data.loading && <output className="chat-note"><T id="app.loading" /></output>}
      {messages.length > visible && <button type="button" className="button button-ghost button-sm chat-older" onClick={() => { olderHeight.current = scroll.current?.scrollHeight ?? null; setVisible((n) => n + 100); }}><T id="chat.older" /></button>}
      {!data.loading && messages.length === 0 && data.pending?.conv !== conv.id && <p className="chat-empty"><T id="chat.empty" /></p>}
      {shown.map((m, i) => {
        const prev = shown[i - 1];
        const newDay = prev === undefined || !sameDay(prev.at, m.at);
        const own = m.sender === ownTag;
        const first = newDay || prev.sender !== m.sender || m.at - prev.at > 5 * 60_000;
        return <Fragment key={m.id}>
          {newDay && <p className="chat-day"><span>{dayLabel(m.at, lang)}</span></p>}
          <article className="chat-bubble" data-own={own} data-first={first || undefined}>
            {group !== null && !own && first && <small className="chat-sender"><bdi>{data.people.find((p) => p.tag === m.sender)?.displayName ?? `#${m.sender.toString(16)}`}</bdi></small>}
            <p className={m.emote ? "chat-emote" : undefined}>{m.text}</p>
            <time dateTime={new Date(m.at).toISOString()} title={new Intl.DateTimeFormat(localeOf(lang), { dateStyle: "medium", timeStyle: "short" }).format(m.at)}>{clock(m.at, lang)}</time>
          </article>
        </Fragment>;
      })}
      {data.pending?.conv === conv.id && <article className="chat-bubble chat-pending" data-own="true" data-first="true">
        <p>{data.pending.text}</p>
        <span className="chat-pending-state"><T id={data.pending.failed ? "chat.notConfirmed" : "chat.sending"} /></span>
        {data.pending.failed && <div className="action-row">
          <button type="button" className="button button-ghost button-sm" disabled={data.busy || changed} onClick={() => { void session.retry(); }}><T id="chat.retry" /></button>
          <button type="button" className="button button-danger button-sm" disabled={data.busy} onClick={() => setDiscard(data.pending?.sealed ?? null)}><T id="chat.discard" /></button>
        </div>}
      </article>}
    </div>
    {canSend ? <form className="chat-compose" onSubmit={(event) => { event.preventDefault(); void send(); }}>
      <label className="vh" htmlFor="chat-message"><T id="chat.message" /></label>
      <textarea id="chat-message" rows={Math.min(6, Math.max(1, text.split("\n").length))} maxLength={1000} value={text} disabled={data.busy || data.pending !== null}
        onChange={(event) => setText(event.target.value)} onKeyDown={keys} placeholder={t(lang, "chat.message")} aria-describedby="chat-hint" />
      <button type="submit" className="chat-send" disabled={data.busy || data.pending !== null || changed || text.trim().length === 0} aria-label={t(lang, "chat.send")} title={t(lang, "chat.send")}><Icon name="send" /></button>
      <p id="chat-hint" className="chat-hint"><T id="chat.enterHint" /></p>
    </form> : <p className="chat-compose chat-compose-closed"><T id={conv.kind === "direct" ? "chat.friendRequired" : "chat.groupLeft"} /></p>}
    {discard !== null && <Confirm title={<T id="chat.discard" />} action="chat.discard" onCancel={() => setDiscard(null)} onConfirm={() => { session.discard(discard); setDiscard(null); }}><p><T id="chat.discardConfirm" /></p></Confirm>}
    {review !== null && <Confirm title={<T id="chat.safety" />} action="chat.verify" danger={false} busy={verifying} onCancel={() => { if (!verifying) setReview(null); }} onConfirm={() => {
      if (verifying) return; setVerifying(true); setVerifyError(false);
      void session.confirm(review.snapshot).then(() => setReview(null), () => setVerifyError(true)).finally(() => setVerifying(false));
    }}><p><bdi>{review.snapshot.handle}</bdi></p><p><T id="chat.compare" /></p><p className="chat-number" translate="no">{review.number.match(/.{1,5}/g)?.join(" ")}</p>{verifyError && <p role="alert"><T id="chat.failed" /></p>}</Confirm>}
  </>;
}
