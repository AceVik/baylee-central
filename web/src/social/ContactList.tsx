// The messenger's Contacts tab: requests to answer, friends to write to,
// requests sent, and blocked accounts, as rows of people with their
// actions; a friend's row opens the conversation.
import type { ReactNode } from "react";

import { Avatar, Menu } from "../chat/pieces";
import { Icon } from "../components/icons";
import { T, num, t, useLang, type Key } from "../i18n";
import { handleOf, type Change, type Contact, type Person, type Relation } from "./api";

const SECTIONS: readonly Relation[] = ["incoming", "friend", "outgoing", "blocked"];
const WORDS: Record<Relation, Key> = { incoming: "social.incoming", friend: "social.friends", outgoing: "social.outgoing", blocked: "social.blocked" };

function Person({ person, children, onOpen }: { person: Person; children: ReactNode; onOpen?: () => void }) {
  const lang = useLang();
  const body = <>
    <Avatar name={person.displayName} seed={String(person.tag)} size="sm" />
    <span className="chat-person-name"><bdi translate="no">{person.displayName}</bdi><span className="chat-row-tag" translate="no">#{person.tag.toString(16).padStart(4, "0")}</span></span>
  </>;
  return <li className="chat-person">
    {onOpen === undefined ? <span className="chat-person-main">{body}</span>
      : <button type="button" className="chat-person-main" onClick={onOpen} title={t(lang, "chat.open")}>{body}</button>}
    <span className="chat-person-actions">{children}</span>
  </li>;
}

export function ContactList({ items, busy, onChange, onOpen }: { items: Contact[]; busy: boolean; onChange(person: Person, change: Change): void; onOpen(person: Person): void }) {
  const lang = useLang();
  return <>{SECTIONS.map((relation) => {
    const contacts = items.filter((c) => c.relation === relation);
    if (contacts.length === 0) return null;
    const rows = <ul className="chat-people">{contacts.map(({ person }) => <Person key={person.tag} person={person} {...(relation === "friend" ? { onOpen: () => onOpen(person) } : {})}>
      {relation === "incoming" && <>
        <button type="button" className="button button-sm" disabled={busy} onClick={() => onChange(person, "accept")}><T id="social.accept" /></button>
        <button type="button" className="button button-ghost button-sm" disabled={busy} onClick={() => onChange(person, "remove")}><T id="social.decline" /></button>
      </>}
      {relation === "friend" && <button type="button" className="chat-icon-button" disabled={busy} onClick={() => onOpen(person)} title={t(lang, "chat.open")} aria-label={`${t(lang, "chat.open")}: ${handleOf(person)}`}><Icon name="chat" /></button>}
      {relation === "outgoing" && <button type="button" className="button button-ghost button-sm" disabled={busy} onClick={() => onChange(person, "remove")}><T id="social.cancel" /></button>}
      {relation === "blocked"
        ? <button type="button" className="button button-ghost button-sm" disabled={busy} onClick={() => onChange(person, "unblock")}><T id="social.unblock" /></button>
        : <Menu label={`${t(lang, "social.more")}: ${handleOf(person)}`}>
          {relation === "friend" && <button type="button" disabled={busy} onClick={() => onChange(person, "remove")}><T id="social.remove" /></button>}
          <button type="button" className="bad" disabled={busy} onClick={() => onChange(person, "block")}><T id="social.block" /></button>
        </Menu>}
    </Person>)}</ul>;
    const head = <><T id={WORDS[relation]} /> <span className="chat-count num">{num(lang, contacts.length)}</span></>;
    return relation === "blocked"
      ? <details key={relation} className="chat-section"><summary className="chat-side-head">{head}</summary>{rows}</details>
      : <section key={relation} className="chat-section"><h3 className="chat-side-head">{head}</h3>{rows}</section>;
  })}</>;
}
