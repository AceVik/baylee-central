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
    {state.error && <p role="alert"><T id="chat.groupFailed" /><button className="text-link" onClick={() => { void state.reload(); }}><T id="chat.retry" /></button></p>}
    <ul>{state.groups.map((g) => <li key={g.id}><button aria-current={selected === g.id ? "true" : undefined} onClick={() => onChoose(g.id)}><bdi>{g.name}</bdi></button></li>)}</ul>
    {state.invites.length > 0 && <h3><T id="chat.groupInvites" /></h3>}
    <ul>{state.invites.map((i) => <li key={i.group}><bdi>{i.name}</bdi><small><bdi>{handleOf(i.inviter)}</bdi></small>
      <button disabled={state.busy} onClick={() => { void state.change(i.group, { change: "join" }).then((ok) => { if (ok !== false) onChoose(i.group); }); }}><T id="social.accept" /></button>
      <button disabled={state.busy} onClick={() => { void state.change(i.group, { change: "withdraw", tag: ownTag }); }}><T id="social.decline" /></button>
    </li>)}</ul>
    <form onSubmit={(event) => { event.preventDefault(); void state.create(name).then((id) => { if (typeof id === "string") { setName(""); onChoose(id); } }); }}>
      <label htmlFor="new-group-name"><T id="chat.groupName" /></label>
      <input id="new-group-name" value={name} maxLength={96} disabled={state.busy} onChange={(event) => setName(event.target.value)} required />
      <button className="button button-ghost" disabled={state.busy || name.trim().length === 0 || [...name].length > 48 || state.groups.length + state.invites.length >= 20}><T id="chat.groupCreate" /></button>
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
    {owner && <form onSubmit={(event) => { event.preventDefault(); void state.change(group.id, { change: "rename", name }); }}>
      <label htmlFor="group-name"><T id="chat.groupName" /></label><input id="group-name" value={name} maxLength={96} onChange={(event) => setName(event.target.value)} disabled={state.busy} />
      <button disabled={state.busy || name.trim().length === 0 || [...name].length > 48}><T id="chat.groupRename" /></button>
    </form>}
    <ul>{group.members.map((m) => <li key={m.person.tag}><bdi>{handleOf(m.person)}</bdi>{m.owner && <small><T id="chat.groupOwner" /></small>}
      {owner && m.person.tag !== ownTag && <><button disabled={state.busy} onClick={() => setConfirm({ change: { change: "transfer", tag: m.person.tag }, person: m.person })}><T id="chat.groupTransfer" /></button><button disabled={state.busy} onClick={() => setConfirm({ change: { change: "remove", tag: m.person.tag }, person: m.person })}><T id="chat.groupRemove" /></button></>}
    </li>)}</ul>
    <form onSubmit={(event) => { event.preventDefault(); const tag = Number(invite); if (!available.some((p) => p.tag === tag)) return; void state.change(group.id, { change: "invite", tag }).then((ok) => { if (ok !== false) setInvite(""); }); }}>
      <label htmlFor="group-invite"><T id="chat.groupInvite" /></label>
      <select id="group-invite" value={invite} disabled={state.busy} onChange={(event) => setInvite(event.target.value)}><option value=""><T id="chat.groupChoose" /></option>{available.map((p) => <option key={p.tag} value={p.tag}>{handleOf(p)}</option>)}</select>
      <button disabled={state.busy || invite === "" || group.members.length + group.invited.length >= 50}><T id="chat.groupInvite" /></button>
    </form>
    {group.invited.length > 0 && <><h3><T id="chat.groupInvites" /></h3><ul>{group.invited.map((p) => <li key={p.tag}><bdi>{handleOf(p)}</bdi>{owner && <button disabled={state.busy} onClick={() => { void state.change(group.id, { change: "withdraw", tag: p.tag }); }}><T id="social.cancel" /></button>}</li>)}</ul></>}
    <button className="button button-danger" disabled={state.busy} onClick={() => setConfirm({ change: owner ? { change: "delete" } : { change: "remove", tag: ownTag } })}><T id={owner ? "chat.groupDelete" : "chat.groupLeave"} /></button>
    {confirm !== null && <Confirm title={<bdi>{group.name}</bdi>} action={confirm.change.change === "delete" ? "chat.groupDelete" : confirm.change.change === "transfer" ? "chat.groupTransfer" : confirm.person === undefined ? "chat.groupLeave" : "chat.groupRemove"} busy={state.busy} onCancel={() => { if (!state.busy) setConfirm(null); }} onConfirm={() => { void state.change(group.id, confirm.change).then((ok) => { if (ok !== false) setConfirm(null); }); }}>
      {confirm.person !== undefined && <p><bdi>{handleOf(confirm.person)}</bdi></p>}
      <p><T id={confirm.change.change === "delete" ? "chat.groupDeleteConfirm" : confirm.change.change === "transfer" ? "chat.groupTransferConfirm" : "chat.groupRemoveConfirm"} /></p>
      {state.error && <p role="alert"><T id="chat.groupFailed" /></p>}
    </Confirm>}
  </details>;
}
