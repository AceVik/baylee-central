import { useState } from "react";
import { AppFrame, Failed, SignedOut } from "../app/AppFrame";
import { Confirm } from "../app/Confirm";
import { useSummary } from "../app/useSummary";
import { useAccount } from "../account/session";
import { T, useTitle } from "../i18n";
import { Link, PATHS } from "../router";
import { handleOf, socialApi, type Change, type Person, type SocialApi } from "./api";
import { ContactList } from "./ContactList";
import { FindContact } from "./FindContact";
import { useContacts } from "./useContacts";
import "../styles/social.css";

function Contacts({ api, ownTag }: { api: SocialApi; ownTag: number }) {
  const { items, error, denied, busy, reload, change } = useContacts(api);
  const [confirm, setConfirm] = useState<{ person: Person; change: "block" | "remove" } | null>(null);
  const act = (person: Person, action: Change) => {
    if (action === "block" || action === "remove") setConfirm({ person, change: action });
    else void change(person.tag, action);
  };
  if (denied) return <SignedOut />;
  return <>
    <FindContact api={api} ownTag={ownTag} busy={busy} onAdd={(tag) => change(tag, "request")} />
    {error && <Failed reason="social.failed" onRetry={() => { void reload(); }} />}
    {items === null ? !error && <output className="card app-panel app-skeleton skeleton"><span className="vh"><T id="app.loading" /></span></output>
      : items.length === 0 ? <div className="card app-panel app-empty"><p><T id="social.empty" /></p></div>
      : <ContactList items={items} busy={busy} onChange={act} />}
    {confirm !== null && <Confirm title={<bdi translate="no">{handleOf(confirm.person)}</bdi>} action={confirm.change === "block" ? "social.block" : "social.remove"}
      busy={busy} onCancel={() => { if (!busy) setConfirm(null); }} onConfirm={() => { if (!busy) void change(confirm.person.tag, confirm.change).then((ok) => { if (ok) setConfirm(null); }); }}>
      <p><T id={confirm.change === "block" ? "social.confirmBlock" : "social.confirmRemove"} /></p>
    </Confirm>}
  </>;
}

export default function ContactsPage({ client }: { client?: SocialApi }) {
  useTitle("title.accountContacts");
  const { state, reload } = useSummary();
  const { generation } = useAccount();
  const [api] = useState(() => client ?? socialApi());
  return <AppFrame page="accountContacts" admin={state.at === "ready" && state.summary.admin} eyebrow="app.account" title={<T id="social.title" />} lead={<T id="social.lead" />}>
    {state.at === "signedOut" ? <SignedOut /> : state.at === "failed" ? <Failed onRetry={reload} />
      : state.at === "loading" ? <output className="card app-panel app-skeleton skeleton"><span className="vh"><T id="app.loading" /></span></output>
      : state.summary.tag === null || state.summary.displayName === null ? <div className="card app-panel app-empty"><p><Link to={PATHS.account}><T id="social.profileNeeded" /></Link></p></div>
      : <Contacts key={`${state.summary.identity.account.join()}:${generation}`} api={api} ownTag={state.summary.tag} />}
  </AppFrame>;
}
