// `/admin/feedback`: bug and crash reports, triaged (docs/reports.md §"The
// website's section"). The views with their counts, the search and every
// filter in the address (`query.ts`), shown as pills; facets, sortable
// columns and pages. A report opens in a drawer beside the list on a wide
// screen and on its own page otherwise; `j`/`k` move through the list and
// the drawer follows. The list never waits for its counts or its facets.

import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { ApiError, type AccountSummary } from "../../account/api";
import { idText } from "../../account/owner-api";
import { Failed, Panel, TableBox } from "../../app/AppFrame";
import { Icon } from "../../components/icons";
import { T, t, useLang, useTitle } from "../../i18n";
import { Link, navigate, useLocation } from "../../router";
import type { AdminApi } from "../admin-api";
import { AdminFrame, failureKey, useAdminApi } from "../shared";
import "../../styles/feedback.css";
import { OPEN, DONE, reportsApi, type ReportFacets, type ReportSummary, type ReportsApi } from "./api";
import { firstLine, issueUrl } from "./github";
import { forUs } from "./keys";
import { FilterPanel, Pager, Pills, SearchBox, SortHead, Views, panelCount, pillsOf, type Change } from "./ListControls";
import { Ago, KindPill, Labels, NoticeProvider, PriorityPill, Reporter, StatusPill, shortId, useAdmins, type Admins } from "./pieces";
import { ReportView, size } from "./ReportView";
import { VIEWS, changed, limitOf, listPath, readQuery, reportPath, requestOf, viewOf, withoutPage, writeQuery, type ListQuery, type ViewId } from "./query";

type Load = { at: "loading" } | { at: "failed"; kind: string } | { at: "ready"; reports: ReportSummary[]; total: number };

/** Whether the drawer fits beside the list (the app frame's own breakpoint). */
const wide = (): boolean => typeof window.matchMedia !== "function" || window.matchMedia("(min-width: 1100px)").matches;

/** A report counts as unread while it is new and this administrator has not opened it. */
export const unread = (r: ReportSummary): boolean => r.status === "new" && !r.seen;

/** The views' counts in two reads: the open reports' facets, and the trash's total. */
function useViewCounts(api: ReportsApi, me: string): [Partial<Record<ViewId, number>>, () => void] {
  const [counts, setCounts] = useState<Partial<Record<ViewId, number>>>({});
  const recount = useCallback(() => {
    // the statuses are counted without the filter's own statuses, the assignees under them
    api.facets({ statuses: OPEN }).then(
      (f) => {
        const status = (s: string) => f.statuses.find((c) => c.value === s)?.count ?? 0;
        const assignee = (hex: string | null) => f.assignees.find((c) => (c.value === null ? hex === null : idText(c.value) === hex))?.count ?? 0;
        setCounts((now) => ({
          ...now,
          inbox: status("new"),
          open: f.total,
          mine: assignee(me),
          unassigned: assignee(null),
          done: DONE.reduce((n, s) => n + status(s), 0),
          all: f.statuses.reduce((n, c) => n + c.count, 0),
        }));
      },
      // the views work without their counts
      () => undefined,
    );
    api.reports({ filter: { trash: true }, sort: "received", descending: true, offset: 0, limit: 1 }).then(
      (p) => setCounts((now) => ({ ...now, trash: p.total })),
      () => undefined,
    );
  }, [api, me]);
  useEffect(recount, [recount]);
  return [counts, recount];
}

/** The facets of the list's filter: read once per filter, never per page or order. */
function useFacets(api: ReportsApi, query: ListQuery): ReportFacets | null {
  // what the facets count over: the list without its page, order and size
  const { sort: _sort, asc: _asc, limit: _limit, ...over } = withoutPage(query);
  const scope = writeQuery(over);
  const [facets, setFacets] = useState<{ scope: string; facets: ReportFacets } | null>(null);
  useEffect(() => {
    let live = true;
    api.facets(requestOf(readQuery(scope)).filter).then(
      (f) => {
        if (live) setFacets({ scope, facets: f });
      },
      // the filters work without their counts
      () => undefined,
    );
    return () => {
      live = false;
    };
  }, [api, scope]);
  return facets?.scope === scope ? facets.facets : null;
}

interface ListProps {
  api: ReportsApi;
  me: string;
  summary: AccountSummary;
  replace(s: AccountSummary): void;
  recountUnread(): void;
}

function ReportList({ api, me, summary, replace, recountUnread }: ListProps) {
  const lang = useLang();
  const { search } = useLocation();
  const query = useMemo(() => readQuery(search), [search]);
  const key = writeQuery(query);
  const view = viewOf(query, me);
  const admins = useAdmins(api);
  const facets = useFacets(api, query);
  const [counts, recount] = useViewCounts(api, me);
  const [load, setLoad] = useState<Load>({ at: "loading" });
  // the list whose answer is drawn; another one asked for is on its way
  const [answered, setAnswered] = useState<string | null>(null);
  const [refreshing, setRefreshing] = useState(false);
  const busy = answered !== key || refreshing;
  const [cursor, setCursor] = useState(0);
  const [drawer, setDrawer] = useState<string | null>(null);
  const [filtersOpen, setFiltersOpen] = useState(() => panelCount(query, view) > 0);
  const rows = useRef<HTMLTableSectionElement>(null);
  const drawerRef = useRef<HTMLElement>(null);

  // another list starts at its top with the drawer shut
  const [shownKey, setShownKey] = useState(key);
  if (shownKey !== key) {
    setShownKey(key);
    setCursor(0);
    setDrawer(null);
  }

  const go = useCallback((q: ListQuery) => navigate(listPath(q), true), []);
  const set = (c: Change) => go(changed(query, c));

  // only the newest request's answer is drawn
  const latest = useRef(0);
  const read = useCallback(
    (asked: string) => {
      const mine = ++latest.current;
      api.reports(requestOf(readQuery(asked))).then(
        (page) => {
          if (mine !== latest.current) return;
          setLoad({ at: "ready", reports: page.reports, total: page.total });
          setAnswered(asked);
          setRefreshing(false);
        },
        (e: unknown) => {
          if (mine !== latest.current) return;
          setLoad({ at: "failed", kind: e instanceof ApiError ? e.kind : "network" });
          setAnswered(asked);
          setRefreshing(false);
        },
      );
    },
    [api],
  );
  useEffect(() => read(key), [key, read]);
  const again = useCallback(() => {
    setRefreshing(true);
    read(key);
    recount();
  }, [key, read, recount]);

  const reports = load.at === "ready" ? load.reports : [];
  const total = load.at === "ready" ? load.total : 0;

  /** A report as it is now, in the row that shows it. */
  const update = useCallback((s: ReportSummary) => {
    setLoad((now) => (now.at === "ready" ? { ...now, reports: now.reports.map((r) => (idText(r.id) === idText(s.id) ? s : r)) } : now));
  }, []);

  const focusRow = (i: number) => requestAnimationFrame(() => rows.current?.rows[i]?.querySelector<HTMLElement>("a")?.focus());

  const open = (i: number, page = false) => {
    const r = reports[i];
    if (r === undefined) return;
    setCursor(i);
    if (page || !wide()) navigate(reportPath(r.id, query));
    else {
      setDrawer(idText(r.id));
      requestAnimationFrame(() => drawerRef.current?.focus());
    }
  };

  const move = (delta: number) => {
    if (reports.length === 0) return;
    const next = Math.max(0, Math.min(cursor + delta, reports.length - 1));
    setCursor(next);
    const r = reports[next];
    if (drawer !== null && r !== undefined) setDrawer(idText(r.id));
    focusRow(next);
  };

  // the list's single keys (keys.ts): j/k and the arrows move, Enter opens beside, o on its page, Escape shuts the drawer
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape" && drawer !== null && !e.defaultPrevented) {
        const at = reports.findIndex((r) => idText(r.id) === drawer);
        setDrawer(null);
        if (at >= 0) focusRow(at);
        return;
      }
      if (!forUs(e)) return;
      const stop = () => e.preventDefault();
      if (e.key === "j" || (e.key === "ArrowDown" && drawer !== null)) {
        stop();
        move(1);
      } else if (e.key === "k" || (e.key === "ArrowUp" && drawer !== null)) {
        stop();
        move(-1);
      } else if (e.key === "o" && reports[cursor] !== undefined) {
        stop();
        open(cursor, true);
      } else if (e.key === "Enter" && e.target === document.body && reports[cursor] !== undefined) {
        stop();
        open(cursor);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  const pills = pillsOf(lang, query, view, admins, me);
  const extra = panelCount(query, view);
  const perPage = limitOf(query);
  const first = ((query.page ?? 1) - 1) * perPage;
  const unreadHere = reports.filter(unread).length;

  return (
    <div className={`fb-split${drawer !== null ? " with-drawer" : ""}`}>
      <Panel
        title={<T id="fb.reports" />}
        actions={
          <button type="button" className="button button-ghost button-sm" aria-busy={busy} onClick={again}>
            <Icon name="refresh" className="button-icon" />
            {t(lang, "fb.refresh")}
          </button>
        }
      >
        <Views view={view} counts={counts} me={me} onPick={(q) => go({ ...q, ...(query.sort === undefined ? {} : { sort: query.sort }), ...(query.asc ? { asc: true } : {}), ...(query.limit === undefined ? {} : { limit: query.limit }) })} />
        <div className="fb-toolbar">
          <SearchBox asked={query.q} onAsk={(q) => set({ q })} />
          <button type="button" className="button button-ghost button-sm" aria-expanded={filtersOpen} aria-controls="fb-filters" onClick={() => setFiltersOpen((o) => !o)}>
            <Icon name="filter" className="button-icon" />
            {extra > 0 ? t(lang, "fb.filtersN", { n: extra }) : t(lang, "fb.filters")}
          </button>
        </div>
        {filtersOpen && <FilterPanel id="fb-filters" query={query} facets={facets} admins={admins} me={me} set={set} />}
        <Pills
          pills={pills}
          set={set}
          onClear={() => {
            const base = view === null ? {} : (VIEWS.find((v) => v.id === view)?.query(me) ?? {});
            go({ ...base, ...(query.sort === undefined ? {} : { sort: query.sort }), ...(query.asc ? { asc: true } : {}) });
          }}
        />
        {query.trash === true && <p className="small">{t(lang, "fb.trashHint", { days: 30 })}</p>}
        <div aria-busy={busy}>
          {load.at === "loading" && <div className="skeleton app-skeleton" aria-hidden="true" />}
          {load.at === "failed" && <Failed reason={load.kind === "rejected" ? "fb.errList" : failureKey(load.kind)} onRetry={again} />}
          <output className="small list-count">
            {load.at === "ready" &&
              (total === 0
                ? t(lang, "fb.none")
                : t(lang, "fb.range", { from: first + 1, to: first + reports.length, total }) + (unreadHere > 0 ? ` · ${t(lang, "fb.unreadHere", { n: unreadHere })}` : ""))}
          </output>
          {load.at === "ready" && total === 0 && pills.length > 0 && (
            <p className="fb-empty">
              <button type="button" className="button button-ghost" onClick={() => go(view === null ? {} : (VIEWS.find((v) => v.id === view)?.query(me) ?? {}))}>
                {t(lang, "fb.clearFilters")}
              </button>
            </p>
          )}
          {reports.length > 0 && (
            <TableBox label={t(lang, "fb.reports")}>
              <table className="table fb-table">
                <thead>
                  <tr>
                    <th scope="col">{t(lang, "fb.colReport")}</th>
                    <SortHead sort="received" label="fb.colReceived" query={query} set={set} />
                    <SortHead sort="kind" label="fb.colKind" query={query} set={set} className="fb-wide" />
                    <SortHead sort="status" label="fb.colStatus" query={query} set={set} />
                    <SortHead sort="priority" label="fb.colPriority" query={query} set={set} className="fb-wide" />
                    <th scope="col" className="fb-wide">
                      {t(lang, "fb.colAssignee")}
                    </th>
                    <SortHead sort="gateway" label="fb.colGateway" query={query} set={set} className="fb-wide" />
                    <SortHead sort="reporter" label="fb.colReporter" query={query} set={set} className="fb-wide" />
                  </tr>
                </thead>
                <tbody ref={rows}>
                  {reports.map((r, i) => (
                    <Row
                      key={idText(r.id)}
                      report={r}
                      query={query}
                      admins={admins}
                      me={me}
                      cursor={i === cursor}
                      opened={drawer === idText(r.id)}
                      onOpen={() => open(i)}
                      set={set}
                    />
                  ))}
                </tbody>
              </table>
            </TableBox>
          )}
          {load.at === "ready" && total > perPage && <Pager query={query} total={total} go={go} />}
        </div>
      </Panel>
      {drawer !== null && (
        <aside ref={drawerRef} className="card fb-drawer" aria-label={t(lang, "fb.drawer")} tabIndex={-1}>
          <div className="fb-drawer-head">
            <Link className="button button-ghost button-sm" to={reportPath(drawer, query)}>
              {t(lang, "fb.openPage")}
            </Link>
            <button type="button" className="round-btn" onClick={() => setDrawer(null)} title="Esc">
              <Icon name="close" />
              <span className="vh">{t(lang, "fb.closeDrawer")}</span>
            </button>
          </div>
          <ReportView
            key={drawer}
            api={api}
            hex={drawer}
            me={me}
            admins={admins}
            query={query}
            summary={summary}
            replace={replace}
            compact
            recountUnread={recountUnread}
            onChanged={(s) => {
              update(s);
              recount();
            }}
            onGone={() => {
              setDrawer(null);
              again();
            }}
            onBack={again}
          />
        </aside>
      )}
    </div>
  );
}

function Row({ report: r, query, admins, me, cursor, opened, onOpen, set }: { report: ReportSummary; query: ListQuery; admins: Admins; me: string; cursor: boolean; opened: boolean; onOpen(): void; set(c: Change): void }) {
  const lang = useLang();
  const isUnread = unread(r);
  const line = firstLine(r.excerpt, 120);
  return (
    <tr className={[cursor ? "fb-cursor" : "", opened ? "fb-opened" : "", isUnread ? "fb-unread" : ""].filter(Boolean).join(" ") || undefined}>
      <th scope="row" className="fb-cell-report">
        {isUnread && (
          <span className="fb-dot" title={t(lang, "fb.unread")}>
            <span className="vh">{t(lang, "fb.unread")}: </span>
          </span>
        )}
        <Link
          to={reportPath(r.id, query)}
          dir="auto"
          onClick={(e) => {
            if (e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey || e.altKey || !wide()) return;
            e.preventDefault();
            onOpen();
          }}
        >
          {line === "" ? t(lang, "fb.noText") : line}
        </Link>
        <span className="fb-marks">
          <Labels labels={r.labels} onPick={(label) => set({ label })} />
          {r.notes > 0 && <span className="small">{t(lang, "fb.notesN", { n: r.notes })}</span>}
          {r.duplicateOf !== null && <span className="small">{t(lang, "fb.dupOf", { id: shortId(r.duplicateOf) })}</span>}
          {r.recordBytes !== null && (
            <span className="small">
              {t(lang, "fb.recordMark", { size: size(lang, r.recordBytes) })}
              {r.recordOrigin === "client" && ` · ${t(lang, "fb.byClientShort")}`}
            </span>
          )}
          {r.issue !== null && issueUrl(r.issue) !== null && (
            <a className="small" href={issueUrl(r.issue) ?? undefined} target="_blank" rel="noopener noreferrer">
              #{r.issue}
              <span className="vh"> {t(lang, "fb.newTab")}</span>
            </a>
          )}
        </span>
      </th>
      <td className="num">
        <Ago at={r.createdAt} />
      </td>
      <td className="fb-wide">
        <KindPill kind={r.kind} />
      </td>
      <td>
        <StatusPill status={r.status} />
      </td>
      <td className="fb-wide">{r.priority === null ? <span className="small">—</span> : <PriorityPill priority={r.priority} />}</td>
      <td className="fb-wide">
        {r.assignee === null ? (
          <span className="small">—</span>
        ) : (
          <button type="button" className="fb-linkish" onClick={() => set({ assignee: idText(r.assignee ?? new Uint8Array()) })}>
            <bdi translate="no">{idText(r.assignee) === me ? t(lang, "fb.me") : admins.name(r.assignee)}</bdi>
          </button>
        )}
      </td>
      <td className="fb-wide">
        {r.channel === "direct" ? (
          <span className="small">{t(lang, "fb.channelDirect")}</span>
        ) : (
          <button type="button" className="fb-linkish" onClick={() => set({ gateway: r.gateway })}>
            <bdi translate="no">{r.gateway}</bdi>
          </button>
        )}
      </td>
      <td className="fb-wide">
        <Reporter pseudonym={r.reporter} onPick={() => set({ reporter: r.reporter })} />
      </td>
    </tr>
  );
}

export default function FeedbackPage({ client, reports }: { client?: AdminApi; reports?: ReportsApi }) {
  useTitle("title.adminFeedback");
  const api = useAdminApi(client);
  const [rapi] = useState(() => reports ?? reportsApi());
  return (
    <AdminFrame page="adminFeedback" api={api} title={<T id="fb.title" />} lead={<T id="fb.lead" />}>
      {({ summary, replace, recountUnread }) => (
        <NoticeProvider>
          <ReportList api={rapi} me={idText(summary.identity.account)} summary={summary} replace={replace} recountUnread={recountUnread} />
        </NoticeProvider>
      )}
    </AdminFrame>
  );
}
