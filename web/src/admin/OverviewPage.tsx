// `/admin`: the administration's first view. A few calm tiles with the
// counts Central reads in one statement (`AdminRequest::Overview`), each
// leading to the page where the work is; the one that asks for attention,
// gateways awaiting approval, says so in words and in the state colour.
// What is left of the legacy store comes from its own read.

import { useEffect, useState, type ReactNode } from "react";

import { Failed } from "../app/AppFrame";
import { day } from "../app/format";
import { T, num, t, useLang, useTitle, type Key } from "../i18n";
import { useCountUp } from "../motion";
import { Link, PATHS } from "../router";
import type { AdminApi, LegacyStore, Overview, ReportsAtGlance } from "./admin-api";
import { AdminFrame, useAdminApi } from "./shared";

function Tile({ label, n, tone, children, to, go }: { label: Key; n: number; tone?: "warn"; children: ReactNode; to: string; go: Key }) {
  const lang = useLang();
  const shown = useCountUp(n) ?? n;
  return (
    <section className={`card stat reveal${tone === "warn" ? " stat-warn" : ""}`}>
      <h2 className="stat-label">
        <T id={label} />
      </h2>
      <p className="stat-n num">{num(lang, shown)}</p>
      <p className="small stat-sub">{children}</p>
      <Link className="stat-go" to={to}>
        <T id={go} />
      </Link>
    </section>
  );
}

type Legacy = { at: "loading" } | { at: "none" } | { at: "ready"; store: LegacyStore };

/** The open reports and the new crashes: `undefined` while they are read, `null` when Central did not count them (the tile is left out). */
function useReports(api: AdminApi): ReportsAtGlance | null | undefined {
  const [reports, setReports] = useState<ReportsAtGlance | null | undefined>(undefined);
  useEffect(() => {
    api.reportsAtGlance().then(setReports, () => setReports(null));
  }, [api]);
  return reports;
}

function Tiles({ api, overview, uncounted, recount }: { api: AdminApi; overview: Overview | null; uncounted: boolean; recount(): void }) {
  const lang = useLang();
  const [legacy, setLegacy] = useState<Legacy>({ at: "loading" });
  const reports = useReports(api);
  useEffect(() => {
    api.legacyAccounts().then(
      (store) => setLegacy({ at: "ready", store }),
      // no store on this Central, or none reachable: the tile is left out
      () => setLegacy({ at: "none" }),
    );
  }, [api]);
  if (overview === null && uncounted) return <Failed reason="adm.errOverview" onRetry={recount} />;
  if (overview === null) {
    return (
      <div className="stats" aria-busy="true">
        {[0, 1, 2, 3].map((i) => (
          <div key={i} className="card skeleton stat-skeleton" aria-hidden="true" />
        ))}
      </div>
    );
  }
  const left = legacy.at === "ready" ? legacy.store : null;
  return (
    <div className="stats stagger">
      <Tile label="ov.accounts" n={overview.accounts} to={PATHS.adminUsers} go="ov.accountsGo">
        {t(lang, "ov.accountsSub", { week: overview.accountsWeek, moved: overview.moved })}
      </Tile>
      <Tile label="ov.pending" n={overview.pending} {...(overview.pending > 0 ? { tone: "warn" as const } : {})} to={PATHS.adminGateways} go="ov.pendingGo">
        {t(lang, overview.pending > 0 ? "ov.pendingSub" : "ov.pendingNone")}
      </Tile>
      {reports === undefined && <div className="card skeleton stat-skeleton" aria-hidden="true" />}
      {reports !== undefined && reports !== null && (
        <Tile
          label="ov.reports"
          n={reports.open}
          {...(reports.newCrashes > 0 ? { tone: "warn" as const } : {})}
          to={reports.newCrashes > 0 ? `${PATHS.adminFeedback}?status=new&kind=crash` : `${PATHS.adminFeedback}?status=new,triaged,in_progress`}
          go={reports.newCrashes > 0 ? "ov.crashesGo" : "ov.reportsGo"}
        >
          {reports.newCrashes > 0 ? t(lang, "ov.reportsCrashes", { n: reports.newCrashes }) : t(lang, "ov.reportsCalm")}
        </Tile>
      )}
      <Tile label="ov.gateways" n={overview.approved} to={`${PATHS.adminGateways}?state=approved`} go="ov.gatewaysGo">
        {t(lang, "ov.gatewaysSub", { online: overview.online, disabled: overview.disabled, tokens: overview.liveTokens })}
      </Tile>
      <Tile label="ov.admins" n={overview.admins} to={PATHS.adminAudit} go="ov.adminsGo">
        {t(lang, "ov.adminsSub")}
      </Tile>
      {left !== null && (
        <Tile label="ov.legacy" n={left.accounts.length} to={PATHS.adminLegacy} go="ov.legacyGo">
          {left.deadline === null ? t(lang, "leg.noDeadline") : t(lang, "ov.legacySub", { day: day(lang, left.deadline) })}
        </Tile>
      )}
    </div>
  );
}

export default function OverviewPage({ client }: { client?: AdminApi }) {
  useTitle("title.admin");
  const api = useAdminApi(client);
  return (
    <AdminFrame page="admin" api={api} title={<T id="ov.title" />} lead={<T id="ov.lead" />}>
      {({ overview, uncounted, recount }) => <Tiles api={api} overview={overview} uncounted={uncounted} recount={recount} />}
    </AdminFrame>
  );
}
