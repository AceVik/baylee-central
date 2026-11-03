import { useState } from "react";
import { Confirm } from "../app/Confirm";
import { T } from "../i18n";
import { handleOf, type Person } from "../social/api";
import type { Group, GroupChange } from "./groups";
import type { GroupState } from "./useGroups";

export function GroupList({ state, selected, ownTag, onChoose }: { state: GroupState; selected: string | null; ownTag: number; onChoose(id: string): void }) {
  const [name, setName] = useState("");
  return <div className="chat-groups">
    <h2><T id="chat.groups" /></h2>
    {state.error && <p className="form-status bad" role="alert"><T id="chat.groupFailed" /> <button type="button" className="text-link" onClick={() => { void state.reload(); }}><T id="chat.retry" /></button></p>}
    <ul className="chat-peers-list">{state.groups.map((g) => <li key={g.id}><button type="button" aria-current={selected === g.id ? "true" : undefined} onClick={() => onChoose(g.id)}><bdi>{g.name}</bdi></button></li>)}</ul>
    {state.invites.length > 0 && <>
      <h3><T id="chat.groupInvites" /></h3>
      <ul className="chat-invite-list">{state.invites.map((i) => <li key={i.group}>
        <div className="chat-invite-head"><bdi>{i.name}</bdi><small><bdi>{handleOf(i.inviter)}</bdi></small></div>
        <div className="action-row">
          <button type="button" className="button button-sm" disabled={state.busy} onClick={() => { void state.change(i.group, { change: "join" }).then((ok) => { if (ok !== false) onChoose(i.group); }); }}><T id="social.accept" /></button>
          <button type="button" className="button button-ghost button-sm" disabled={state.busy} onClick={() => { void state.change(i.group, { change: "withdraw", tag: ownTag }); }}><T id="social.decline" /></button>
        </div>
      </li>)}</ul>
    </>}
    <form onSubmit={(event) => { event.preventDefault(); void state.create(name).then((id) => { if (typeof id === "string") { setName(""); onChoose(id); } }); }}>
      <label htmlFor="new-group-name"><T id="chat.groupName" /></label>
      <input id="new-group-name" value={name} maxLength={96} disabled={state.busy} onChange={(event) => setName(event.target.value)} required />
      <button type="submit" className="button button-ghost button-sm" disabled={state.busy || name.trim().length === 0 || [...name].length > 48 || state.groups.length + state.invites.length >= 20}><T id="chat.groupCreate" /></button>
    </form>
  </div>;
}

export function GroupSettings({ group, ownTag, friends, state }: { group: Group; ownTag: number; friends: Person[]; state: GroupState }) {
  const [name, setName] = useState(group.name);
  const [invite, setInvite] = useState("");
  const [confirm, setConfirm] = useState<{ change: GroupChange; person?: Person } | null>(null);
  const owner = group.members.some((m) => m.owner && m.person.tag === ownTag);
  const available = friends.filter((p) => !group.members.some((m) => m.person.tag === p.tag) && !group.invited.some((i) => i.tag === p.tag));
  return <details className="chat-group-settings"><summary><T id="chat.groupManage" /> · {group.members.length}/50</summary>
    <div className="chat-group-body">
      {owner && <form className="chat-inline-form" onSubmit={(event) => { event.preventDefault(); void state.change(group.id, { change: "rename", name }); }}>
        <label htmlFor="group-name"><T id="chat.groupName" /></label>
        <div className="chat-inline-row">
          <input id="group-name" value={name} maxLength={96} onChange={(event) => setName(event.target.value)} disabled={state.busy} />
          <button type="submit" className="button button-ghost button-sm" disabled={state.busy || name.trim().length === 0 || [...name].length > 48}><T id="chat.groupRename" /></button>
        </div>
      </form>}
      <ul>{group.members.map((m) => <li key={m.person.tag}><bdi>{handleOf(m.person)}</bdi>{m.owner && <span className="pill gold"><T id="chat.groupOwner" /></span>}
        {owner && m.person.tag !== ownTag && <div className="action-row"><button type="button" className="button button-ghost button-sm" disabled={state.busy} onClick={() => setConfirm({ change: { change: "transfer", tag: m.person.tag }, person: m.person })}><T id="chat.groupTransfer" /></button><button type="button" className="button button-danger button-sm" disabled={state.busy} onClick={() => setConfirm({ change: { change: "remove", tag: m.person.tag }, person: m.person })}><T id="chat.groupRemove" /></button></div>}
      </li>)}</ul>
      <form className="chat-inline-form" onSubmit={(event) => { event.preventDefault(); const tag = Number(invite); if (!available.some((p) => p.tag === tag)) return; void state.change(group.id, { change: "invite", tag }).then((ok) => { if (ok !== false) setInvite(""); }); }}>
        <label htmlFor="group-invite"><T id="chat.groupInvite" /></label>
        <div className="chat-inline-row">
          <select id="group-invite" value={invite} disabled={state.busy} onChange={(event) => setInvite(event.target.value)}><option value=""><T id="chat.groupChoose" /></option>{available.map((p) => <option key={p.tag} value={p.tag}>{handleOf(p)}</option>)}</select>
          <button type="submit" className="button button-ghost button-sm" disabled={state.busy || invite === "" || group.members.length + group.invited.length >= 50}><T id="chat.groupInvite" /></button>
        </div>
      </form>
      {group.invited.length > 0 && <><h3><T id="chat.groupInvites" /></h3><ul>{group.invited.map((p) => <li key={p.tag}><bdi>{handleOf(p)}</bdi>{owner && <button type="button" className="button button-ghost button-sm" disabled={state.busy} onClick={() => { void state.change(group.id, { change: "withdraw", tag: p.tag }); }}><T id="social.cancel" /></button>}</li>)}</ul></>}
      <div className="action-row">
        <button type="button" className="button button-danger button-sm" disabled={state.busy} onClick={() => setConfirm({ change: owner ? { change: "delete" } : { change: "remove", tag: ownTag } })}><T id={owner ? "chat.groupDelete" : "chat.groupLeave"} /></button>
      </div>
    </div>
    {confirm !== null && <Confirm title={<bdi>{group.name}</bdi>} action={confirm.change.change === "delete" ? "chat.groupDelete" : confirm.change.change === "transfer" ? "chat.groupTransfer" : confirm.person === undefined ? "chat.groupLeave" : "chat.groupRemove"} busy={state.busy} onCancel={() => { if (!state.busy) setConfirm(null); }} onConfirm={() => { void state.change(group.id, confirm.change).then((ok) => { if (ok !== false) setConfirm(null); }); }}>
      {confirm.person !== undefined && <p><bdi>{handleOf(confirm.person)}</bdi></p>}
      <p><T id={confirm.change.change === "delete" ? "chat.groupDeleteConfirm" : confirm.change.change === "transfer" ? "chat.groupTransferConfirm" : "chat.groupRemoveConfirm"} /></p>
      {state.error && <p role="alert"><T id="chat.groupFailed" /></p>}
    </Confirm>}
  </details>;
}
