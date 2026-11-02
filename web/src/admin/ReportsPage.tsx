import { useEffect, useState } from "react";
import { T, useLang, useTitle } from "../i18n";
import { when } from "../app/format";
import { AdminFrame, useAdminApi } from "./shared";
import { reportsApi, type Reports } from "./reports-api";
const read = reportsApi();
function Inbox() {
  const lang = useLang();
  const [offset, setOffset] = useState(0);
  const [revision, refresh] = useState(0);
  const [loaded, setLoaded] = useState<{ offset: number; revision: number; data: Reports | null } | null>(null);
  const current = loaded?.offset === offset && loaded.revision === revision ? loaded : null;
  useEffect(() => {
    let live = true;
    void read(offset).then((data) => { if (live) setLoaded({ offset, revision, data }); }, () => { if (live) setLoaded({ offset, revision, data: null }); });
    return () => { live = false; };
  }, [offset, revision]);
  return <div className="card">
    <button className="button button-ghost" disabled={current === null} onClick={() => refresh((r) => r + 1)}><T id="adm.reportsRefresh" /></button>
    {current === null ? <output><T id="app.loading" /></output> : current.data === null ? <p role="alert"><T id="acct.errUnreachable" /></p> : <>
      {current.data.reports.length === 0 && <p><T id="adm.reportsEmpty" /></p>}
      {current.data.reports.map((report) => <article key={report.id} className="notification-settings"><p className="small"><time dateTime={new Date(report.received * 1000).toISOString()}>{when(lang, report.received)}</time> · <bdi>{report.kind}</bdi> · <bdi>{report.status}</bdi></p><p className="report-excerpt">{report.excerpt}</p></article>)}
      <div className="form-row"><button className="button button-ghost" disabled={offset === 0} onClick={() => setOffset((o) => Math.max(0, o - 50))}><T id="adm.reportsPrevious" /></button><button className="button button-ghost" disabled={offset + current.data.reports.length >= current.data.total} onClick={() => setOffset((o) => o + 50)}><T id="adm.more" /></button></div>
    </>}
  </div>;
}
export default function ReportsPage() {
  useTitle("adm.reports");
  const api = useAdminApi();
  return <AdminFrame api={api} page="adminReports" title={<T id="adm.reports" />} lead={<T id="adm.reportsLead" />}>{({ summary }) => <Inbox key={summary.identity.account.join()} />}</AdminFrame>;
}
