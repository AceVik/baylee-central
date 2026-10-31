// `/admin/users`: the accounts, newest first, 50 a page, or those whose name
// holds a search's words, or the one with a `#tag` (central-api
// `AccountQuery`). The list follows the field as it is typed (after a short
// pause, the newest answer winning), the search stands in the address
// (`?q=`) so going back finds it again, Escape empties the field and
// Ctrl/Cmd+K reaches it from anywhere on the page.

import { useCallback, useEffect, useRef, useState, type FormEvent } from "react";

import { BusyButton } from "../account/Busy";
import { idText } from "../account/owner-api";
import { Failed, Panel, TableBox } from "../app/AppFrame";
import { day, when } from "../app/format";
import { Icon } from "../components/icons";
import { T, t, useLang, useTitle } from "../i18n";
import { Link, userPath } from "../router";
import type { AdminAccount, AdminApi } from "./admin-api";
import { AdminFrame, useAdminApi, who } from "./shared";

type Load = { at: "loading" } | { at: "failed" } | { at: "ready"; accounts: AdminAccount[]; more: boolean; adding: boolean };

/** The search in the address, as the page was opened. */
function askedAt(): string | null {
  const q = new URLSearchParams(window.location.search).get("q")?.trim() ?? "";
  return q === "" ? null : q.slice(0, 64);
}

/** How long typing pauses before the list follows. */
const PAUSE_MS = 300;

function Accounts({ api }: { api: AdminApi }) {
  const lang = useLang();
  const [query, setQuery] = useState(() => askedAt() ?? "");
  const [asked, setAsked] = useState<string | null>(askedAt);
  const [load, setLoad] = useState<Load>({ at: "loading" });
  const rows = useRef<HTMLTableSectionElement>(null);
  // only the newest request's answer is drawn
  const latest = useRef(0);
  const read = useCallback(
    (q: string | null, before: AdminAccount | null) => {
      const mine = ++latest.current;
      api.accounts(q, before?.id ?? null).then(
        (page) => {
          if (mine !== latest.current) return;
          let first = 0;
          setLoad((now) => {
            const kept = before !== null && now.at === "ready" ? now.accounts : [];
            first = kept.length;
            return { at: "ready", accounts: [...kept, ...page.items], more: page.more, adding: false };
          });
          // after "More", the keyboard goes on at the first new account
          if (before !== null) requestAnimationFrame(() => rows.current?.rows[first]?.querySelector("a")?.focus());
        },
        () => {
          if (mine === latest.current) setLoad({ at: "failed" });
        },
      );
    },
    [api],
  );
  const more = () => {
    if (load.at !== "ready" || load.adding) return;
    setLoad({ ...load, adding: true });
    read(asked, load.accounts.at(-1) ?? null);
  };
  const ask = useCallback(
    (text: string) => {
      const q = text.trim() === "" ? null : text.trim();
      setAsked(q);
      setLoad({ at: "loading" });
      read(q, null);
      const url = new URL(window.location.href);
      if (q === null) url.searchParams.delete("q");
      else url.searchParams.set("q", q);
      window.history.replaceState(null, "", url);
    },
    [read],
  );
  // the first list: the one the address asks for
  const first = useRef(asked);
  useEffect(() => read(first.current, null), [read]);
  // the list follows the field once typing pauses
  useEffect(() => {
    const q = query.trim() === "" ? null : query.trim();
    if (q === asked) return;
    const pause = window.setTimeout(() => ask(query), PAUSE_MS);
    return () => window.clearTimeout(pause);
  }, [query, asked, ask]);
  const search = (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    ask(query);
  };
  return (
    <Panel title={<T id="adm.accounts" />}>
      <form className="search-row" onSubmit={search}>
        <label className="vh" htmlFor="adm-q">
          <T id="adm.search" />
        </label>
        <input
          id="adm-q"
          type="search"
          data-find=""
          aria-keyshortcuts="Control+K Meta+K"
          placeholder={t(lang, "adm.searchHint")}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Escape" && query !== "") {
              e.preventDefault();
              setQuery("");
            }
          }}
          maxLength={64}
        />
        <button type="submit" className="button button-ghost">
          <T id="adm.search" />
        </button>
      </form>
      <div aria-busy={load.at === "loading" || (load.at === "ready" && load.adding)}>
      {load.at === "loading" && <div className="skeleton app-skeleton" aria-hidden="true" />}
      {load.at === "failed" && (
        <Failed
          reason="adm.errSearch"
          onRetry={() => {
            setLoad({ at: "loading" });
            read(asked, null);
          }}
        />
      )}
      <output className="small list-count">
        {load.at === "ready" && (load.accounts.length === 0 ? t(lang, "adm.noAccounts") : t(lang, "adm.shown", { n: load.accounts.length }))}
      </output>
      {load.at === "ready" && load.accounts.length > 0 && (
        <TableBox label={t(lang, "adm.accounts")}>
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
            <tbody ref={rows}>
              {load.accounts.map((a) => (
                <tr key={idText(a.id)}>
                  <th scope="row">
                    <Link to={userPath(idText(a.id))}>
                      <bdi translate="no">{who(a)}</bdi>
                    </Link>
                    {a.admin && (
                      <>
                        {" "}
                        <span className="pill info">
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
        </TableBox>
      )}
      {load.at === "ready" && load.more && (
        <p className="dir-more">
          <BusyButton busy={load.adding} className="button button-ghost" onClick={more}>
            <T id="adm.more" />
          </BusyButton>
        </p>
      )}
      </div>
    </Panel>
  );
}

export default function UsersPage({ client }: { client?: AdminApi }) {
  useTitle("title.adminUsers");
  const api = useAdminApi(client);
  return (
    <AdminFrame page="adminUsers" api={api} title={<T id="adm.usersTitle" />} lead={<T id="adm.usersLead" />}>
      {() => <Accounts api={api} />}
    </AdminFrame>
  );
}
