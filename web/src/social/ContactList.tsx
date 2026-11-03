import { Link, PATHS } from "../router";
import { T, type Key } from "../i18n";
import { handleOf, type Change, type Contact, type Person, type Relation } from "./api";

const SECTIONS: readonly Relation[] = ["incoming", "friend", "outgoing", "blocked"];
const WORDS: Record<Relation, Key> = { incoming: "social.incoming", friend: "social.friends", outgoing: "social.outgoing", blocked: "social.blocked" };

export function ContactList({ items, busy, onChange }: { items: Contact[]; busy: boolean; onChange(person: Person, change: Change): void }) {
  return <>{SECTIONS.map((relation) => {
    const contacts = items.filter((c) => c.relation === relation);
    if (contacts.length === 0) return null;
    return <section key={relation} className="card app-panel">
      <div className="app-panel-head">
        <h2 className="app-panel-title"><T id={WORDS[relation]} /></h2>
        <span className="pill num">{contacts.length}</span>
      </div>
      <ul className="social-contacts">{contacts.map(({ person }) => <li key={person.tag} className="social-row">
        <bdi translate="no">{handleOf(person)}</bdi>
        <div className="social-actions">
          {relation === "friend" && <Link className="button button-sm" to={`${PATHS.accountChat}?with=${person.tag.toString(16).padStart(4, "0")}`}><T id="chat.open" /></Link>}
          {relation === "incoming" && <button className="button button-sm" disabled={busy} onClick={() => onChange(person, "accept")}><T id="social.accept" /></button>}
          {relation === "blocked" ? <button className="button button-ghost button-sm" disabled={busy} onClick={() => onChange(person, "unblock")}><T id="social.unblock" /></button>
            : <>
              <button className="button button-ghost button-sm" disabled={busy} onClick={() => onChange(person, "remove")}><T id={relation === "friend" ? "social.remove" : relation === "incoming" ? "social.decline" : "social.cancel"} /></button>
              <button className="button button-danger button-sm" disabled={busy} onClick={() => onChange(person, "block")}><T id="social.block" /></button>
            </>}
        </div>
      </li>)}</ul>
    </section>;
  })}</>;
}
