// `/admin`: the accounts, newest first, 50 a page, or those whose name holds
// a search's words, or the one with a `#tag` (central-api `AccountQuery`).

import { useCallback, useEffect, useState, type FormEvent } from "react";

import { idText } from "../account/owner-api";
import { Panel } from "../app/AppFrame";
import { day, when } from "../app/format";
import { Icon } from "../components/icons";
import { T, t, useLang, useTitle } from "../i18n";
import { Link, userPath } from "../router";
import type { AdminAccount, AdminApi } from "./admin-api";
import { AdminFrame, useAdminApi, who } from "./shared";

type Load = { at: "loading" } | { at: "failed" } | { at: "ready"; accounts: AdminAccount[]; more: boolean };

function Accounts({ api }: { api: AdminApi }) {
  const lang = useLang();
  const [query, setQuery] = useState("");
  const [asked, setAsked] = useState<string | null>(null);
  const [load, setLoad] = useState<Load>({ at: "loading" });
  const read = useCallback(
    (q: string | null, before: AdminAccount | null) => {
      api.accounts(q, before?.id ?? null).then(
        (page) =>
          setLoad((now) => ({
            at: "ready",
            accounts: before !== null && now.at === "ready" ? [...now.accounts, ...page.items] : page.items,
            more: page.more,
          })),
        () => setLoad({ at: "failed" }),
      );
    },
    [api],
  );
  useEffect(() => read(null, null), [read]);
  const search = (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    const q = query.trim() === "" ? null : query.trim();
    setAsked(q);
    setLoad({ at: "loading" });
    read(q, null);
  };
  return (
    <Panel title={<T id="adm.accounts" />}>
      <form className="search-row" onSubmit={search}>
        <label className="vh" htmlFor="adm-q">
          <T id="adm.search" />
        </label>
        <input id="adm-q" type="search" placeholder={t(lang, "adm.searchHint")} value={query} onChange={(e) => setQuery(e.target.value)} maxLength={64} />
        <button type="submit" className="button button-ghost">
          <T id="adm.search" />
        </button>
      </form>
      {load.at === "loading" && <div className="skeleton app-skeleton" aria-hidden="true" />}
      {load.at === "failed" && (
        <p className="small" role="alert">
          <T id="adm.errSearch" />
        </p>
      )}
      {load.at === "ready" && load.accounts.length === 0 && (
        <p className="small">
          <T id="adm.noAccounts" />
        </p>
      )}
      {load.at === "ready" && load.accounts.length > 0 && (
        <div className="table-wrap">
          <table className="table">
            <thead>
              <tr>
                <th scope="col"><T id="adm.handle" /></th>
                <th scope="col"><T id="adm.username" /></th>
                <th scope="col"><T id="adm.source" /></th>
                <th scope="col"><T id="adm.created" /></th>
                <th scope="col"><T id="adm.lastSignIn" /></th>
              </tr>
            </thead>
            <tbody>
              {load.accounts.map((a) => (
                <tr key={idText(a.id)}>
                  <th scope="row">
                    <Link to={userPath(idText(a.id))}>
                      <bdi translate="no">{who(a)}</bdi>
                    </Link>
                    {a.admin && (
                      <>
                        {" "}
                        <span className="pill gold">
                          <Icon name="shield" className="pill-icon" />
                          <T id="adm.adminPill" />
                        </span>
                      </>
                    )}
                  </th>
                  <td><code translate="no">{a.username}</code></td>
                  <td>{t(lang, a.source === "legacy" ? "adm.legacy" : "adm.registered")}</td>
                  <td className="num">{day(lang, a.createdAt)}</td>
                  <td className="num">{a.lastSignInAt === null ? "—" : when(lang, a.lastSignInAt)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      {load.at === "ready" && load.more && (
        <p className="dir-more">
          <button type="button" className="button button-ghost" onClick={() => read(asked, load.accounts.at(-1) ?? null)}>
            <T id="adm.more" />
          </button>
        </p>
      )}
    </Panel>
  );
}

export default function UsersPage({ client }: { client?: AdminApi }) {
  useTitle("title.admin");
  const api = useAdminApi(client);
  return (
    <AdminFrame page="admin" title={<T id="adm.usersTitle" />} lead={<T id="adm.usersLead" />}>
      {() => <Accounts api={api} />}
    </AdminFrame>
  );
}
