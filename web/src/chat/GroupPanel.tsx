import { useState } from "react";
import { Confirm } from "../app/Confirm";
import { Icon } from "../components/icons";
import { T, t, useLang } from "../i18n";
import { handleOf, type Person } from "../social/api";
import type { Group, GroupChange } from "./groups";
import { Avatar, Menu } from "./pieces";
import type { GroupState } from "./useGroups";

export function GroupSettings({ group, ownTag, friends, state, onClose }: { group: Group; ownTag: number; friends: Person[]; state: GroupState; onClose(): void }) {
  const lang = useLang();
  const [name, setName] = useState(group.name);
  const [invite, setInvite] = useState("");
  const [confirm, setConfirm] = useState<{ change: GroupChange; person?: Person } | null>(null);
  const owner = group.members.some((m) => m.owner && m.person.tag === ownTag);
  const available = friends.filter((p) => !group.members.some((m) => m.person.tag === p.tag) && !group.invited.some((i) => i.tag === p.tag));
  return <section id="chat-group-settings" className="chat-group-settings" aria-labelledby="chat-group-settings-head">
    <header className="chat-drawer-head">
      <h3 id="chat-group-settings-head" className="chat-side-head"><T id="chat.groupManage" /> <span className="chat-count num">{group.members.length}/50</span></h3>
      <button type="button" className="chat-icon-button" onClick={onClose} aria-label={t(lang, "app.close")} title={t(lang, "app.close")}><Icon name="close" /></button>
    </header>
    <div className="chat-group-body">
      {owner && <form className="chat-inline-form" onSubmit={(event) => { event.preventDefault(); void state.change(group.id, { change: "rename", name }); }}>
        <label htmlFor="group-name"><T id="chat.groupName" /></label>
        <div className="chat-inline-row">
          <input id="group-name" value={name} maxLength={96} onChange={(event) => setName(event.target.value)} disabled={state.busy} />
          <button type="submit" className="button button-ghost button-sm" disabled={state.busy || name.trim().length === 0 || [...name].length > 48}><T id="chat.groupRename" /></button>
        </div>
      </form>}
      <ul className="chat-people">{group.members.map((m) => <li key={m.person.tag} className="chat-person"><span className="chat-person-main"><Avatar name={m.person.displayName} seed={String(m.person.tag)} size="sm" /><span className="chat-person-name"><bdi translate="no">{handleOf(m.person)}</bdi></span>{m.owner && <span className="pill gold"><T id="chat.groupOwner" /></span>}</span>
        {owner && m.person.tag !== ownTag && <Menu label={`${t(lang, "social.more")}: ${handleOf(m.person)}`}>
          <button type="button" disabled={state.busy} onClick={() => setConfirm({ change: { change: "transfer", tag: m.person.tag }, person: m.person })}><T id="chat.groupTransfer" /></button>
          <button type="button" className="bad" disabled={state.busy} onClick={() => setConfirm({ change: { change: "remove", tag: m.person.tag }, person: m.person })}><T id="chat.groupRemove" /></button>
        </Menu>}
      </li>)}</ul>
      <form className="chat-inline-form" onSubmit={(event) => { event.preventDefault(); const tag = Number(invite); if (!available.some((p) => p.tag === tag)) return; void state.change(group.id, { change: "invite", tag }).then((ok) => { if (ok !== false) setInvite(""); }); }}>
        <label htmlFor="group-invite"><T id="chat.groupInvite" /></label>
        <div className="chat-inline-row">
          <select id="group-invite" value={invite} disabled={state.busy} onChange={(event) => setInvite(event.target.value)}><option value=""><T id="chat.groupChoose" /></option>{available.map((p) => <option key={p.tag} value={p.tag}>{handleOf(p)}</option>)}</select>
          <button type="submit" className="button button-ghost button-sm" disabled={state.busy || invite === "" || group.members.length + group.invited.length >= 50}><T id="chat.groupInvite" /></button>
        </div>
      </form>
      {group.invited.length > 0 && <><h3 className="chat-side-head"><T id="chat.groupInvites" /></h3><ul className="chat-people">{group.invited.map((p) => <li key={p.tag} className="chat-person"><span className="chat-person-main"><Avatar name={p.displayName} seed={String(p.tag)} size="sm" /><span className="chat-person-name"><bdi translate="no">{handleOf(p)}</bdi></span></span>{owner && <button type="button" className="button button-ghost button-sm" disabled={state.busy} onClick={() => { void state.change(group.id, { change: "withdraw", tag: p.tag }); }}><T id="social.cancel" /></button>}</li>)}</ul></>}
      <div className="action-row">
        <button type="button" className="button button-danger button-sm" disabled={state.busy} onClick={() => setConfirm({ change: owner ? { change: "delete" } : { change: "remove", tag: ownTag } })}><T id={owner ? "chat.groupDelete" : "chat.groupLeave"} /></button>
      </div>
    </div>
    {confirm !== null && <Confirm title={<bdi>{group.name}</bdi>} action={confirm.change.change === "delete" ? "chat.groupDelete" : confirm.change.change === "transfer" ? "chat.groupTransfer" : confirm.person === undefined ? "chat.groupLeave" : "chat.groupRemove"} busy={state.busy} onCancel={() => { if (!state.busy) setConfirm(null); }} onConfirm={() => { void state.change(group.id, confirm.change).then((ok) => { if (ok !== false) setConfirm(null); }); }}>
      {confirm.person !== undefined && <p><bdi>{handleOf(confirm.person)}</bdi></p>}
      <p><T id={confirm.change.change === "delete" ? "chat.groupDeleteConfirm" : confirm.change.change === "transfer" ? "chat.groupTransferConfirm" : "chat.groupRemoveConfirm"} /></p>
      {state.error && <p role="alert"><T id="chat.groupFailed" /></p>}
    </Confirm>}
  </section>;
}
