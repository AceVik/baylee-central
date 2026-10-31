// `/admin/feedback/<id>`: one report on its own page, with the way back to
// the list it was opened from (its query rides along in the address) and to
// that list's previous and next report (`ReportNeighbours`), which `k` and
// `j` also reach.

import { useEffect, useMemo, useState } from "react";

import { idOf, idText } from "../../account/owner-api";
import { Icon } from "../../components/icons";
import { T, t, useLang, useTitle } from "../../i18n";
import { Link, navigate, reportAt, useLocation } from "../../router";
import type { AdminApi } from "../admin-api";
import { AdminFrame, useAdminApi } from "../shared";
import "../../styles/feedback.css";
import { reportsApi, type Neighbours, type ReportsApi } from "./api";
import { forUs } from "./keys";
import { NoticeProvider, useAdmins } from "./pieces";
import { ReportView } from "./ReportView";
import { listPath, readQuery, requestOf, reportPath, type ListQuery } from "./query";

function Steps({ api, hex, query }: { api: ReportsApi; hex: string; query: ListQuery }) {
  const lang = useLang();
  const [near, setNear] = useState<{ hex: string; at: Neighbours } | null>(null);
  const asked = JSON.stringify([hex, query]);
  useEffect(() => {
    let live = true;
    const [of, q] = JSON.parse(asked) as [string, ListQuery];
    const r = requestOf(q);
    const id = idOf(of);
    if (id === null) return;
    api.neighbours(id, r.filter, r.sort, r.descending).then(
      (at) => {
        if (live) setNear({ hex, at });
      },
      // the page reads without its neighbours
      () => undefined,
    );
    return () => {
      live = false;
    };
  }, [api, hex, asked]);
  const at = near?.hex === hex ? near.at : null;
  // k and j step to the list's previous and next report
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!forUs(e) || at === null) return;
      const to = e.key === "j" ? at.next : e.key === "k" ? at.previous : null;
      if (to === null) return;
      e.preventDefault();
      navigate(reportPath(to, query));
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [at, query]);
  return (
    <nav className="fb-steps" aria-label={t(lang, "fb.steps")}>
      <Link className="button button-ghost button-sm" to={listPath(query)}>
        <Icon name="prev" className="button-icon" />
        {t(lang, "fb.backToList")}
      </Link>
      <span className="fb-steps-near">
        {at?.previous ? (
          <Link className="round-btn" to={reportPath(at.previous, query)} aria-keyshortcuts="k">
            <Icon name="prev" />
            <span className="vh">{t(lang, "fb.previous")}</span>
          </Link>
        ) : (
          <span className="round-btn fb-off" aria-hidden="true">
            <Icon name="prev" />
          </span>
        )}
        <span className="small num">{at !== null && at.position !== null ? t(lang, "fb.position", { n: at.position, total: at.total }) : at !== null ? t(lang, "fb.notInList") : ""}</span>
        {at?.next ? (
          <Link className="round-btn" to={reportPath(at.next, query)} aria-keyshortcuts="j">
            <Icon name="next" />
            <span className="vh">{t(lang, "fb.next")}</span>
          </Link>
        ) : (
          <span className="round-btn fb-off" aria-hidden="true">
            <Icon name="next" />
          </span>
        )}
      </span>
    </nav>
  );
}

function Report({ api, hex, query, ...rest }: { api: ReportsApi; hex: string; query: ListQuery } & Omit<Parameters<typeof ReportView>[0], "api" | "hex" | "admins" | "query">) {
  const lang = useLang();
  const admins = useAdmins(api);
  if (idOf(hex) === null) {
    return (
      <p className="small" role="alert">
        {t(lang, "fb.errReport")}
      </p>
    );
  }
  return (
    <div className="fb-page">
      <Steps api={api} hex={hex} query={query} />
      <ReportView key={hex} api={api} hex={hex} admins={admins} query={query} {...rest} />
    </div>
  );
}

export default function ReportPage({ client, reports }: { client?: AdminApi; reports?: ReportsApi }) {
  useTitle("title.adminReport");
  const api = useAdminApi(client);
  const [rapi] = useState(() => reports ?? reportsApi());
  const { pathname, search } = useLocation();
  const hex = reportAt(pathname) ?? "";
  const query = useMemo(() => readQuery(search), [search]);
  return (
    <AdminFrame page="adminReport" api={api} title={<T id="fb.reportTitle" />}>
      {({ summary, replace, recountUnread }) => (
        <NoticeProvider>
          <Report api={rapi} hex={hex} query={query} me={idText(summary.identity.account)} summary={summary} replace={replace} recountUnread={recountUnread} />
        </NoticeProvider>
      )}
    </AdminFrame>
  );
}
