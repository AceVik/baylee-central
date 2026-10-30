// `/admin/users/<id>`: one account as an administrator sees it, with its
// changes: another tag (unique at Central, so `Name#tag` stays unique; the
// audit keeps both), the administrator role, and a reset code shown once.
// Each change needs a password sign-in less than 30 minutes old and writes
// an audit event; the latest twenty are listed below.

import { useCallback, useEffect, useState, type FormEvent } from "react";

import type { AccountSummary } from "../account/api";
import { MAX_TAG, tagText } from "../account/names";
import { idOf, idText } from "../account/owner-api";
import { Panel } from "../app/AppFrame";
import { Confirm } from "../app/Confirm";
import { SecretBox } from "../app/SecretBox";
import { day, when } from "../app/format";
import { Modal } from "../components/Modal";
import { T, t, useLang, useTitle, type Key } from "../i18n";
import { Link, PATHS, useLocation, userAt } from "../router";
import { ApiError } from "../account/api";
import type { AccountDetail, AdminApi } from "./admin-api";
import { AuditTable } from "./AuditPage";
import { AdminFrame, failureKey, useAdminApi, useChange, who } from "./shared";

/** A tag as an administrator types it: one to eight hex digits, `#` allowed. */
export function readTag(text: string): number | null {
  const hex = text.trim().replace(/^#/, "");
  if (!/^[0-9a-fA-F]{1,8}$/.test(hex)) return null;
  const tag = Number.parseInt(hex, 16);
  return tag >= 1 && tag <= MAX_TAG ? tag : null;
}

type Asking = { at: "tag" } | { at: "admin"; admin: boolean } | { at: "reset" } | { at: "code"; code: string; expiresAt: number } | null;

function Detail({ api, id, summary, replace }: { api: AdminApi; id: Uint8Array; summary: AccountSummary; replace(s: AccountSummary): void }) {
  const lang = useLang();
  const [detail, setDetail] = useState<AccountDetail | null>(null);
  const [failed, setFailed] = useState(false);
  const [asking, setAsking] = useState<Asking>(null);
  const [tag, setTag] = useState("");
  const [said, setSaid] = useState<{ bad: boolean; key: Key } | null>(null);
  const { run, pending } = useChange(summary, replace);
  const read = useCallback(() => {
    api.account(id).then(setDetail, () => setFailed(true));
  }, [api, id]);
  useEffect(read, [read]);

  if (failed) return <p className="small" role="alert"><T id="adm.errAccount" /></p>;
  if (detail === null) return <div className="card skeleton app-skeleton" aria-hidden="true" />;
  const a = detail.account;
  const self = idText(a.id) === idText(summary.identity.account);
  const fail = (error: unknown) => setSaid({ bad: true, key: failureKey(error instanceof ApiError ? error.kind : "network") });

  const saveTag = (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    if (readTag(tag) === null) {
      setSaid({ bad: true, key: "adm.errTag" });
      return;
    }
    setAsking({ at: "tag" });
  };
  const changeTag = () => {
    const next = readTag(tag);
    setAsking(null);
    if (next === null) return;
    run(() =>
      api.setTag(a.id, next).then(() => {
        setSaid({ bad: false, key: "adm.tagSaved" });
        setTag("");
        read();
      }, fail),
    );
  };
  const changeAdmin = (admin: boolean) => {
    setAsking(null);
    run(() =>
      api.setAdmin(a.id, admin).then(() => {
        setSaid({ bad: false, key: admin ? "adm.adminGranted" : "adm.adminRemoved" });
        read();
      }, fail),
    );
  };
  const reset = () => {
    setAsking(null);
    run(() =>
      api.resetLogin(a.id).then((code) => {
        setAsking({ at: "code", ...code });
        read();
      }, fail),
    );
  };

  // nothing to do to a pending account, nor to one's own
  const acts = a.state !== "pending" && !self;

  return (
    <>
      <Panel title={<bdi translate="no">{who(a)}</bdi>}>
        <dl className="kv">
          <div><dt><T id="adm.username" /></dt><dd><code translate="no">{a.username}</code></dd></div>
          <div><dt><T id="adm.tag" /></dt><dd><code translate="no">{a.tag === null ? "—" : `#${tagText(a.tag)}`}</code></dd></div>
          <div><dt><T id="adm.source" /></dt><dd>{t(lang, a.source === "legacy" ? "adm.legacy" : "adm.registered")}</dd></div>
          <div><dt><T id="adm.state" /></dt><dd>{t(lang, `adm.state_${a.state}` as Key)}</dd></div>
          <div><dt><T id="adm.created" /></dt><dd className="num">{day(lang, a.createdAt)}</dd></div>
          <div><dt><T id="adm.lastSignIn" /></dt><dd className="num">{a.lastSignInAt === null ? "—" : when(lang, a.lastSignInAt)}</dd></div>
          <div><dt><T id="adm.recovery" /></dt><dd>{t(lang, detail.recovery ? "adm.yes" : "adm.no")}</dd></div>
          <div><dt><T id="adm.role" /></dt><dd>{t(lang, a.admin ? "adm.adminPill" : "adm.member")}</dd></div>
        </dl>
        <output className={`form-status${said?.bad === true ? " bad" : ""}`}>{said === null ? "" : t(lang, said.key)}</output>
      </Panel>

      {a.state !== "pending" && (
        <Panel title={<T id="adm.tagTitle" />}>
          <p className="small"><T id="adm.tagText" /></p>
          <form className="search-row" noValidate onSubmit={saveTag}>
            <label className="vh" htmlFor="adm-tag"><T id="adm.tag" /></label>
            <input id="adm-tag" inputMode="text" autoComplete="off" spellCheck={false} placeholder={a.tag === null ? "#1a2b" : `#${tagText(a.tag)}`} value={tag} onChange={(e) => setTag(e.target.value)} maxLength={9} />
            <button type="submit" className="button button-ghost"><T id="adm.tagSave" /></button>
          </form>
        </Panel>
      )}

      {(acts || a.resetUntil !== null) && (
        <Panel title={<T id="adm.actions" />}>
          {acts && (
            <div className="action-row">
              <button type="button" className={a.admin ? "button button-danger" : "button button-ghost"} onClick={() => setAsking({ at: "admin", admin: !a.admin })}>
                <T id={a.admin ? "adm.removeAdmin" : "adm.makeAdmin"} />
              </button>
              {!a.admin && (
                <button type="button" className="button button-ghost" onClick={() => setAsking({ at: "reset" })}>
                  <T id="adm.reset" />
                </button>
              )}
            </div>
          )}
          {a.resetUntil !== null && <p className="small"><T id="adm.resetOpen" args={{ time: when(lang, a.resetUntil) }} /></p>}
        </Panel>
      )}

      {detail.gateways.length > 0 && (
        <Panel title={<T id="adm.ownedGateways" />}>
          <ul className="plain">
            {detail.gateways.map((g) => (
              <li key={g.origin}>
                <code translate="no">{g.origin}</code> · {t(lang, g.approved ? "gwo.approved" : "gwo.pending")}
              </li>
            ))}
          </ul>
        </Panel>
      )}

      <Panel title={<T id="adm.auditTitle" />}>
        <AuditTable events={detail.audit} />
      </Panel>

      {asking?.at === "tag" && (
        <Confirm title={<T id="adm.tagConfirm" args={{ name: who(a), tag: tagText(readTag(tag) ?? 1) }} />} action="adm.tagSave" danger={false} onConfirm={changeTag} onCancel={() => setAsking(null)}>
          <p><T id="adm.tagWarn" /></p>
        </Confirm>
      )}
      {asking?.at === "admin" && (
        <Confirm title={<T id={asking.admin ? "adm.makeAdminTitle" : "adm.removeAdminTitle"} args={{ name: who(a) }} />} action={asking.admin ? "adm.makeAdmin" : "adm.removeAdmin"} danger={!asking.admin} onConfirm={() => changeAdmin(asking.admin)} onCancel={() => setAsking(null)}>
          <p><T id={asking.admin ? "adm.makeAdminText" : "adm.removeAdminText"} /></p>
        </Confirm>
      )}
      {asking?.at === "reset" && (
        <Confirm title={<T id="adm.resetTitle" args={{ name: who(a) }} />} action="adm.reset" onConfirm={reset} onCancel={() => setAsking(null)}>
          <p><T id="adm.resetText" /></p>
        </Confirm>
      )}
      {asking?.at === "code" && (
        <Modal className="app-dialog app-dialog-wide" label={t(lang, "adm.codeTitle")} onClose={() => {}}>
          <h2 className="app-dialog-title"><T id="adm.codeTitle" /></h2>
          <p className="app-dialog-body"><T id="adm.codeText" args={{ time: when(lang, asking.expiresAt) }} /></p>
          <SecretBox secret={asking.code} label={t(lang, "adm.codeTitle")} />
          <div className="app-dialog-actions">
            <button type="button" className="button" onClick={() => setAsking(null)}><T id="tok.kept" /></button>
          </div>
        </Modal>
      )}
      {pending}
    </>
  );
}

export default function UserPage({ client }: { client?: AdminApi }) {
  useTitle("title.adminUser");
  const api = useAdminApi(client);
  const { pathname } = useLocation();
  const hex = userAt(pathname);
  const id = hex === null ? null : idOf(hex);
  return (
    <AdminFrame page="adminUser" title={<T id="adm.userTitle" />}>
      {({ summary, replace }) =>
        id === null ? (
          <p className="small"><T id="adm.errAccount" /> <Link to={PATHS.admin}><T id="adm.back" /></Link></p>
        ) : (
          <Detail key={hex} api={api} id={id} summary={summary} replace={replace} />
        )
      }
    </AdminFrame>
  );
}
