// The messenger's Chats tab: group invitations, a new group, and every
// conversation with its last line, its time and what arrived unread.
import { useState } from "react";

import { Icon } from "../components/icons";
import { T, num, t, useLang } from "../i18n";
import { handleOf } from "../social/api";
import type { Conversation } from "./conversations";
import { Avatar, shortTime } from "./pieces";
import type { GroupState } from "./useGroups";

function Row({ c, ownTag, selected, locked, onChoose }: { c: Conversation; ownTag: number; selected: boolean; locked: boolean; onChoose(): void }) {
  const lang = useLang();
  const name = c.person?.displayName ?? c.title;
  const preview = c.last !== null
    ? `${c.last.sender === ownTag ? `${t(lang, "chat.you")}: ` : ""}${c.last.text}`
    : c.kind === "group" && c.group !== null ? t(lang, "chat.members", { count: c.group.members.length })
    : c.kind === "former" ? t(lang, "chat.formerGroup")
    : locked ? t(lang, "chat.encrypted") : t(lang, "chat.sayHello");
  return <li>
    <button type="button" className="chat-row" aria-current={selected ? "true" : undefined} data-unread={c.unread > 0 || undefined} onClick={onChoose}>
      <Avatar name={name} seed={c.id} group={c.kind !== "direct"} />
      <span className="chat-row-main">
        <span className="chat-row-top">
          <bdi className="chat-row-name" translate="no">{name}</bdi>
          {c.person !== null && <span className="chat-row-tag" translate="no">#{c.person.tag.toString(16).padStart(4, "0")}</span>}
          {c.last !== null && <time className="chat-row-time" dateTime={new Date(c.last.at).toISOString()}>{shortTime(c.last.at, lang)}</time>}
        </span>
        <span className="chat-row-bottom">
          <span className="chat-row-preview">{preview}</span>
          {c.unread > 0 && <span className="chat-badge num">{num(lang, c.unread)}<span className="vh"> {t(lang, "chat.unread")}</span></span>}
        </span>
      </span>
    </button>
  </li>;
}

export function NewGroup({ groups, onCreated, onCancel }: { groups: GroupState; onCreated(id: string): void; onCancel(): void }) {
  const [name, setName] = useState("");
  const full = groups.groups.length + groups.invites.length >= 20;
  return <form className="chat-new-group" onSubmit={(event) => { event.preventDefault(); void groups.create(name).then((id) => { if (typeof id === "string") { setName(""); onCreated(id); } }); }}>
    <label htmlFor="new-group-name"><T id="chat.groupName" /></label>
    <div className="chat-inline-row">
      <input id="new-group-name" value={name} maxLength={96} disabled={groups.busy} autoComplete="off" onChange={(event) => setName(event.target.value)} required />
      <button type="submit" className="button button-sm" disabled={groups.busy || name.trim().length === 0 || [...name].length > 48 || full}><T id="chat.groupCreate" /></button>
      <button type="button" className="button button-ghost button-sm" onClick={onCancel}><T id="app.cancel" /></button>
    </div>
  </form>;
}

export function ChatList({ list, ownTag, selected, locked, groups, filtered, onChoose }: {
  list: Conversation[]; ownTag: number; selected: string | null; locked: boolean; groups: GroupState; filtered: boolean; onChoose(id: string): void;
}) {
  return <>
    {groups.error && <p className="chat-side-note bad" role="alert"><T id="chat.groupFailed" /> <button type="button" className="text-link" onClick={() => { void groups.reload(); }}><T id="chat.retry" /></button></p>}
    {groups.invites.length > 0 && <section className="chat-invites" aria-label={undefined}>
      <h3 className="chat-side-head"><T id="chat.groupInvites" /></h3>
      <ul className="chat-invite-list">{groups.invites.map((i) => <li key={i.group} className="chat-invite">
        <Avatar name={i.name} seed={`grp:${i.group}`} group size="sm" />
        <span className="chat-invite-head"><bdi>{i.name}</bdi><small><bdi translate="no">{handleOf(i.inviter)}</bdi></small></span>
        <span className="action-row">
          <button type="button" className="button button-sm" disabled={groups.busy} onClick={() => { void groups.change(i.group, { change: "join" }).then((ok) => { if (ok !== false) onChoose(`grp:${i.group}`); }); }}><T id="social.accept" /></button>
          <button type="button" className="button button-ghost button-sm" disabled={groups.busy} onClick={() => { void groups.change(i.group, { change: "withdraw", tag: ownTag }); }}><T id="social.decline" /></button>
        </span>
      </li>)}</ul>
    </section>}
    {list.length === 0
      ? <p className="chat-side-empty"><T id={filtered ? "chat.noMatch" : "chat.noConversations"} /></p>
      : <ul className="chat-rows">{list.map((c) => <Row key={c.id} c={c} ownTag={ownTag} selected={c.id === selected} locked={locked} onChoose={() => onChoose(c.id)} />)}</ul>}
  </>;
}

export function SideTools({ onNewGroup, onAddContact }: { onNewGroup(): void; onAddContact(): void }) {
  const lang = useLang();
  return <span className="chat-side-tools">
    <button type="button" className="chat-icon-button" onClick={onNewGroup} title={t(lang, "chat.newGroup")} aria-label={t(lang, "chat.newGroup")}><Icon name="people" /></button>
    <button type="button" className="chat-icon-button" onClick={onAddContact} title={t(lang, "social.add")} aria-label={t(lang, "social.add")}><Icon name="userAdd" /></button>
  </span>;
}
