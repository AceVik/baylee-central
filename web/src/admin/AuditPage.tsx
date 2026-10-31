// `/admin/audit`: what administrators and the operator did, newest first, 50
// a page. An event records ids, never words (docs/administration.md); the
// read brings today's names of the accounts and gateways a page names, which
// the table shows (the id's end where a name is gone); a tag's change keeps
// both tags, which the page reads from the account's page.

import { useCallback, useEffect, useRef, useState, type RefObject } from "react";

import { BusyButton } from "../account/Busy";
import { idText } from "../account/owner-api";
import { Failed, Panel, TableBox } from "../app/AppFrame";
import { when } from "../app/format";
import { T, t, useLang, useTitle, type Key } from "../i18n";
import { Link, userPath } from "../router";
import type { AdminApi, AuditEvent, AuditNames } from "./admin-api";
import { AdminFrame, useAdminApi, who } from "./shared";

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

/** An account as the log names it: its handle today, else the end of its id. */
function Account({ id, names }: { id: Uint8Array; names: AuditNames | undefined }) {
  const named = names?.accounts.get(idText(id));
  const handle = named === undefined ? "—" : who({ displayName: named.displayName, tag: named.tag });
  return (
    <Link to={userPath(idText(id))} title={idText(id)}>
      {handle === "—" ? <code>…{short(id)}</code> : <bdi translate="no">{handle}</bdi>}
    </Link>
  );
}

/** A gateway as the log names it: its name today (its address beside), else the end of its id. */
function Gateway({ id, names }: { id: Uint8Array; names: AuditNames | undefined }) {
  const named = names?.gateways.get(idText(id));
  if (named === undefined) return <code title={idText(id)}>gw …{short(id)}</code>;
  return (
    <span title={named.origin}>
      <bdi>{named.name}</bdi>
    </span>
  );
}

export function AuditTable({ events, names, rows }: { events: AuditEvent[]; names?: AuditNames; rows?: RefObject<HTMLTableSectionElement | null> }) {
  const lang = useLang();
  if (events.length === 0) return <p className="small"><T id="aud.none" /></p>;
  return (
    <TableBox label={t(lang, "aud.title")}>
      <table className="table">
        <thead>
          <tr>
            <th scope="col"><T id="aud.when" /></th>
            <th scope="col"><T id="aud.what" /></th>
            <th scope="col"><T id="aud.by" /></th>
            <th scope="col"><T id="aud.subject" /></th>
          </tr>
        </thead>
        <tbody ref={rows}>
          {events.map((e) => {
            const word = ACTIONS[e.action];
            return (
              <tr key={idText(e.id)}>
                <td className="num">{when(lang, e.at)}</td>
                <th scope="row" tabIndex={-1}>{word === undefined ? <code>{e.action}</code> : t(lang, word)}</th>
                <td>{e.actor === null ? t(lang, "aud.operator") : <Account id={e.actor} names={names} />}</td>
                <td>
                  {e.account !== null && <Account id={e.account} names={names} />}
                  {e.account !== null && e.gateway !== null && " · "}
                  {e.gateway !== null && <Gateway id={e.gateway} names={names} />}
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </TableBox>
  );
}

function Events({ api }: { api: AdminApi }) {
  const [events, setEvents] = useState<AuditEvent[] | null>(null);
  const [names, setNames] = useState<AuditNames>({ accounts: new Map(), gateways: new Map() });
  const [more, setMore] = useState(false);
  const [adding, setAdding] = useState(false);
  const [failed, setFailed] = useState(false);
  const rows = useRef<HTMLTableSectionElement>(null);
  const read = useCallback(
    (before: AuditEvent | null) => {
      api.audit(null, before?.id ?? null).then(
        (page) => {
          let first = 0;
          setEvents((now) => {
            const kept = before === null || now === null ? [] : now;
            first = kept.length;
            return [...kept, ...page.items];
          });
          setMore(page.more);
          // the names of every page read so far
          setNames((now) => ({
            accounts: new Map([...now.accounts, ...page.names.accounts]),
            gateways: new Map([...now.gateways, ...page.names.gateways]),
          }));
          setAdding(false);
          // after "More", the keyboard goes on at the first new event
          if (before !== null) requestAnimationFrame(() => rows.current?.rows[first]?.querySelector<HTMLElement>("a, th")?.focus());
        },
        () => {
          setAdding(false);
          setFailed(true);
        },
      );
    },
    [api],
  );
  useEffect(() => read(null), [read]);
  return (
    <Panel title={<T id="aud.title" />}>
      <div aria-busy={(events === null && !failed) || adding}>
        {failed && (
          <Failed
            onRetry={() => {
              setFailed(false);
              read(events?.at(-1) ?? null);
            }}
          />
        )}
        {events === null && !failed && <div className="skeleton app-skeleton" aria-hidden="true" />}
        {events !== null && <AuditTable events={events} names={names} rows={rows} />}
        {events !== null && more && !failed && (
          <p className="dir-more">
            <BusyButton
              busy={adding}
              className="button button-ghost"
              onClick={() => {
                setAdding(true);
                read(events.at(-1) ?? null);
              }}
            >
              <T id="adm.more" />
            </BusyButton>
          </p>
        )}
      </div>
    </Panel>
  );
}

export default function AuditPage({ client }: { client?: AdminApi }) {
  useTitle("title.adminAudit");
  const api = useAdminApi(client);
  return (
    <AdminFrame page="adminAudit" api={api} title={<T id="aud.pageTitle" />} lead={<T id="aud.lead" />}>
      {() => <Events api={api} />}
    </AdminFrame>
  );
}
