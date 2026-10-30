// `/admin/audit`: what administrators and the operator did, newest first, 50
// a page. An event names ids, never words (docs/administration.md); a tag's
// change keeps both tags, which the page reads from the account's page.

import { useCallback, useEffect, useState } from "react";

import { idText } from "../account/owner-api";
import { Panel } from "../app/AppFrame";
import { when } from "../app/format";
import { T, t, useLang, useTitle, type Key } from "../i18n";
import { Link, userPath } from "../router";
import type { AdminApi, AuditEvent } from "./admin-api";
import { AdminFrame, useAdminApi } from "./shared";

/** The words for an audited action; an action this page does not know is shown as Central names it. */
const ACTIONS: Readonly<Record<string, Key>> = {
  "account.admin_granted": "aud.adminGranted",
  "account.admin_removed": "aud.adminRemoved",
  "account.reset_issued": "aud.resetIssued",
  "account.tag_changed": "aud.tagChanged",
  "gateway.approved": "aud.gatewayApproved",
  "gateway.disabled": "aud.gatewayDisabled",
  "gateway.token_revoked": "aud.tokenRevoked",
};

const short = (id: Uint8Array) => idText(id).slice(-8);

export function AuditTable({ events }: { events: AuditEvent[] }) {
  const lang = useLang();
  if (events.length === 0) return <p className="small"><T id="aud.none" /></p>;
  return (
    <div className="table-wrap">
      <table className="table">
        <thead>
          <tr>
            <th scope="col"><T id="aud.when" /></th>
            <th scope="col"><T id="aud.what" /></th>
            <th scope="col"><T id="aud.by" /></th>
            <th scope="col"><T id="aud.subject" /></th>
          </tr>
        </thead>
        <tbody>
          {events.map((e) => {
            const word = ACTIONS[e.action];
            return (
              <tr key={idText(e.id)}>
                <td className="num">{when(lang, e.at)}</td>
                <th scope="row">{word === undefined ? <code>{e.action}</code> : t(lang, word)}</th>
                <td>{e.actor === null ? t(lang, "aud.operator") : <Link to={userPath(idText(e.actor))}><code>…{short(e.actor)}</code></Link>}</td>
                <td>
                  {e.account !== null && <Link to={userPath(idText(e.account))}><code>…{short(e.account)}</code></Link>}
                  {e.gateway !== null && <code>gw …{short(e.gateway)}</code>}
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}

function Events({ api }: { api: AdminApi }) {
  const [events, setEvents] = useState<AuditEvent[] | null>(null);
  const [more, setMore] = useState(false);
  const [failed, setFailed] = useState(false);
  const read = useCallback(
    (before: AuditEvent | null) => {
      api.audit(null, before?.id ?? null).then(
        (page) => {
          setEvents((now) => (before === null || now === null ? page.items : [...now, ...page.items]));
          setMore(page.more);
        },
        () => setFailed(true),
      );
    },
    [api],
  );
  useEffect(() => read(null), [read]);
  return (
    <Panel title={<T id="aud.title" />}>
      {failed && <p className="small" role="alert"><T id="acct.errUnreachable" /></p>}
      {events === null && !failed && <div className="skeleton app-skeleton" aria-hidden="true" />}
      {events !== null && <AuditTable events={events} />}
      {events !== null && more && (
        <p className="dir-more">
          <button type="button" className="button button-ghost" onClick={() => read(events.at(-1) ?? null)}>
            <T id="adm.more" />
          </button>
        </p>
      )}
    </Panel>
  );
}

export default function AuditPage({ client }: { client?: AdminApi }) {
  useTitle("title.adminAudit");
  const api = useAdminApi(client);
  return (
    <AdminFrame page="adminAudit" title={<T id="aud.pageTitle" />} lead={<T id="aud.lead" />}>
      {() => <Events api={api} />}
    </AdminFrame>
  );
}
