// `/admin/feedback`: bug and crash reports, triaged (docs/reports.md §"The
// website's section"). The views with their counts, the search and every
// filter in the address (`query.ts`), shown as pills; facets, sortable
// columns and pages. A report opens in a drawer beside the list on a wide
// screen and on its own page otherwise; `j`/`k` move through the list and
// the drawer follows. The list never waits for its counts or its facets.

import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { ApiError, type AccountSummary } from "../../account/api";
import { BusyButton } from "../../account/Busy";
import { idOf, idText } from "../../account/owner-api";
import { Failed, Panel, TableBox } from "../../app/AppFrame";
import { Confirm } from "../../app/Confirm";
import { Icon } from "../../components/icons";
import { T, t, useLang, useTitle } from "../../i18n";
import { Link, navigate, useLocation } from "../../router";
import type { AdminApi } from "../admin-api";
import { AdminFrame, failureKey, useAdminApi, useChange } from "../shared";
import "../../styles/feedback.css";
import { BOUNDS, DONE, OPEN, reportsApi, type BulkAction, type ReportFacets, type ReportSummary, type ReportsApi } from "./api";
import { BulkBar } from "./BulkBar";
import { Chart } from "./Chart";
import { CrashGroups } from "./CrashGroups";
import { EXPORT_MAX, download, toCsv } from "./csv";
import { firstLine, issueUrl } from "./github";
import { forUs, typing } from "./keys";
import { SavedViews } from "./SavedViews";
import { LIST_KEYS, Shortcuts } from "./Shortcuts";
import { SETTABLE } from "./Triage";
import { FilterPanel, Pager, Pills, SearchBox, SortHead, Views, panelCount, pillsOf, type Change } from "./ListControls";
import { Ago, KindPill, Labels, NoticeProvider, PriorityPill, Reporter, STATUS_WORD, StatusPill, shortId, useAdmins, useNotices, type Admins } from "./pieces";
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

/** A CSV's file name, by today's date. */
const csvName = () => `baylee-reports-${new Date().toISOString().slice(0, 10)}.csv`;

/** How often the list is read again, quietly, while the page is in view. */
export const REFRESH_MS = 30_000;

/** Runs `tick` every `ms` while the page is in view, and at once when it comes back after one was missed. */
function useWhileVisible(tick: () => void, ms: number): void {
  const latest = useRef(tick);
  useEffect(() => {
    latest.current = tick;
  });
  useEffect(() => {
    let last = Date.now();
    const run = () => {
      last = Date.now();
      latest.current();
    };
    const timer = window.setInterval(() => {
      if (!document.hidden) run();
    }, ms);
    const shown = () => {
      if (!document.hidden && Date.now() - last >= ms) run();
    };
    document.addEventListener("visibilitychange", shown);
    return () => {
      window.clearInterval(timer);
      document.removeEventListener("visibilitychange", shown);
    };
  }, [ms]);
}

function ReportList({ api, me, summary, replace, recountUnread }: ListProps) {
  const lang = useLang();
  const notices = useNotices();
  const { search } = useLocation();
  const query = useMemo(() => readQuery(search), [search]);
  const key = writeQuery(query);
  const view = viewOf(query, me);
  const inTrash = query.trash === true;
  const admins = useAdmins(api);
  const facets = useFacets(api, query);
  const [counts, recount] = useViewCounts(api, me);
  const [load, setLoad] = useState<Load>({ at: "loading" });
  // the list whose answer is drawn; another one asked for is on its way
  const [answered, setAnswered] = useState<string | null>(null);
  const [refreshing, setRefreshing] = useState(false);
  const [working, setWorking] = useState(false);
  const busy = answered !== key || refreshing;
  // reports that came while the list was read quietly: announced, not shown, until asked for
  const [arrived, setArrived] = useState<{ reports: ReportSummary[]; total: number } | null>(null);
  const [cursor, setCursor] = useState(0);
  const [checked, setChecked] = useState<ReadonlySet<string>>(new Set());
  const [drawer, setDrawer] = useState<string | null>(null);
  const [groups, setGroups] = useState(false);
  const [help, setHelp] = useState(false);
  const [purging, setPurging] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [filtersOpen, setFiltersOpen] = useState(() => panelCount(query, view) > 0);
  const { run, pending } = useChange(summary, replace);
  const rows = useRef<HTMLTableSectionElement>(null);
  const drawerRef = useRef<HTMLElement>(null);
  const shownLoad = useRef(load);
  useEffect(() => {
    shownLoad.current = load;
  });

  // another list starts at its top, nothing selected, the drawer shut
  const [shownKey, setShownKey] = useState(key);
  if (shownKey !== key) {
    setShownKey(key);
    setCursor(0);
    setChecked(new Set());
    setDrawer(null);
    setArrived(null);
  }

  const go = useCallback((q: ListQuery) => navigate(listPath(q), true), []);
  const set = (c: Change) => go(changed(query, c));

  // only the newest request's answer is drawn
  const latest = useRef(0);
  const read = useCallback(
    (asked: string, quiet = false) => {
      const mine = ++latest.current;
      api.reports(requestOf(readQuery(asked))).then(
        (page) => {
          if (mine !== latest.current) return;
          const now = shownLoad.current;
          const known = new Set(now.at === "ready" ? now.reports.map((r) => idText(r.id)) : []);
          if (quiet && now.at === "ready" && page.reports.some((r) => !known.has(idText(r.id)))) {
            // rows already shown take their news in place; the new ones wait, so nothing moves under the reader
            const fresh = new Map(page.reports.map((r) => [idText(r.id), r] as const));
            setLoad({ ...now, reports: now.reports.map((r) => fresh.get(idText(r.id)) ?? r) });
            setArrived(page);
          } else {
            setLoad({ at: "ready", reports: page.reports, total: page.total });
            if (!quiet) setArrived(null);
          }
          setAnswered(asked);
          setRefreshing(false);
        },
        (e: unknown) => {
          if (mine !== latest.current) return;
          setAnswered(asked);
          setRefreshing(false);
          // a quiet read that fails leaves the list as it is
          if (!quiet) setLoad({ at: "failed", kind: e instanceof ApiError ? e.kind : "network" });
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
  useWhileVisible(() => {
    if (shownLoad.current.at !== "ready" || groups) return;
    read(key, true);
    recount();
    recountUnread();
  }, REFRESH_MS);

  const reports = load.at === "ready" ? load.reports : [];
  const total = load.at === "ready" ? load.total : 0;
  const newOnes = arrived === null ? 0 : Math.max(arrived.reports.filter((r) => !reports.some((x) => idText(x.id) === idText(r.id))).length, arrived.total - total);

  /** A report as it is now, in the row that shows it. */
  const update = useCallback((s: ReportSummary) => {
    setLoad((now) => (now.at === "ready" ? { ...now, reports: now.reports.map((r) => (idText(r.id) === idText(s.id) ? s : r)) } : now));
  }, []);

  const focusRow = (i: number) => requestAnimationFrame(() => rows.current?.rows[i]?.querySelector<HTMLElement>("th a")?.focus());

  const open = (i: number, page = false) => {
    const r = reports[i];
    if (r === undefined) return;
    setCursor(i);
    if (page || !wide()) navigate(reportPath(r.id, query));
    else {
      setDrawer(idText(r.id));
      // the keyboard goes on in the drawer, which starts at its top; the page stays where it was
      requestAnimationFrame(() => {
        drawerRef.current?.focus({ preventScroll: true });
        drawerRef.current?.scrollTo?.({ top: 0 });
      });
    }
  };

  const move = (delta: number) => {
    if (reports.length === 0) return;
    const next = Math.max(0, Math.min(cursor + delta, reports.length - 1));
    setCursor(next);
    const r = reports[next];
    if (drawer !== null && r !== undefined) {
      setDrawer(idText(r.id));
      drawerRef.current?.scrollTo?.({ top: 0 });
    }
    focusRow(next);
  };

  const toggle = (hex: string) =>
    setChecked((now) => {
      const next = new Set(now);
      if (!next.delete(hex)) next.add(hex);
      return next;
    });

  /** What an action works on: the selection, else the report under the cursor. */
  const targets = (): ReportSummary[] => (checked.size > 0 ? reports.filter((r) => checked.has(idText(r.id))) : reports[cursor] === undefined ? [] : [reports[cursor]]);
  const fail = (e: unknown) => notices.say(t(lang, failureKey(e instanceof ApiError ? e.kind : "network")), { bad: true });

  /** One request for all of `ids` (the targets): all of them change or none. */
  const bulk = (action: BulkAction, said: string, ids: readonly Uint8Array[] = targets().map((r) => r.id)) => {
    if (ids.length === 0 || working) return;
    setWorking(true);
    api
      .bulk(ids, action)
      .then(() => {
        const undo =
          action.do === "trash"
            ? () =>
                api.bulk(ids, { do: "restore" }).then(() => {
                  notices.say(t(lang, "fb.bulkRestored", { n: ids.length }));
                  again();
                  recountUnread();
                }, fail)
            : undefined;
        notices.say(said, undo === undefined ? {} : { undo: () => void undo() });
        if (action.do !== "change") {
          setChecked(new Set());
          if (drawer !== null && ids.some((id) => idText(id) === drawer)) setDrawer(null);
        }
        again();
        recountUnread();
      }, fail)
      .finally(() => setWorking(false));
  };

  const purge = () =>
    run(() => {
      const ids = targets().map((r) => r.id);
      setWorking(true);
      api
        .bulk(ids, { do: "purge" })
        .then(() => {
          notices.say(t(lang, "fb.bulkPurged", { n: ids.length }));
          setChecked(new Set());
          again();
        }, fail)
        .finally(() => {
          setWorking(false);
          setPurging(false);
        });
    });

  const seen = (to: boolean, of: readonly ReportSummary[] = targets()) => {
    if (of.length === 0) return;
    for (const r of of) update({ ...r, seen: to });
    api.setSeen(
      of.map((r) => r.id),
      to,
    ).then(() => {
      notices.say(t(lang, to ? "fb.markedRead" : "fb.markedUnread", { n: of.length }));
      recountUnread();
    }, fail);
  };

  const copy = (text: string, said: string) => {
    if (!navigator.clipboard?.writeText) {
      notices.say(t(lang, "copy.impossible"), { bad: true });
      return;
    }
    navigator.clipboard.writeText(text).then(
      () => notices.say(said),
      () => notices.say(t(lang, "copy.failed"), { bad: true }),
    );
  };

  const exportRows = (of: readonly ReportSummary[]) => {
    if (of.length === 0) return;
    download(csvName(), toCsv(of, admins.name));
    notices.say(t(lang, "fb.exported", { n: of.length }));
  };
  /** Every report the filter matches, page by page, up to `EXPORT_MAX`. */
  const exportAll = async () => {
    if (exporting) return;
    setExporting(true);
    const all: ReportSummary[] = [];
    const base = requestOf(query);
    try {
      for (let offset = 0; offset < Math.min(total, EXPORT_MAX); offset += BOUNDS.maxPage) {
        const page = await api.reports({ ...base, offset, limit: Math.min(BOUNDS.maxPage, EXPORT_MAX - offset) });
        all.push(...page.reports);
        if (page.reports.length === 0) break;
      }
      exportRows(all);
    } catch (e: unknown) {
      fail(e);
    } finally {
      setExporting(false);
    }
  };

  const perPage = limitOf(query);
  const page = query.page ?? 1;

  // the list's single keys (keys.ts, the `?` dialog)
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape" && !e.defaultPrevented && !typing(e.target)) {
        if (drawer !== null) {
          const at = reports.findIndex((r) => idText(r.id) === drawer);
          setDrawer(null);
          if (at >= 0) focusRow(at);
        } else if (checked.size > 0) setChecked(new Set());
        return;
      }
      if (!forUs(e) || help) return;
      const k = e.key;
      const under = reports[cursor];
      const act = (f: () => void) => {
        e.preventDefault();
        f();
      };
      if (k === "j" || (k === "ArrowDown" && drawer !== null)) act(() => move(1));
      else if (k === "k" || (k === "ArrowUp" && drawer !== null)) act(() => move(-1));
      else if (k === "?") act(() => setHelp(true));
      else if (k === "/") act(() => document.querySelector<HTMLInputElement>("[data-find]")?.focus());
      else if (k === "r") act(again);
      else if (k === "[" && page > 1) act(() => go({ ...query, page: page - 1 }));
      else if (k === "]" && page * perPage < total) act(() => go({ ...query, page: page + 1 }));
      else if (k === "e") act(() => exportRows(checked.size > 1 ? targets() : reports));
      else if (k === "a" && reports.length > 0) act(() => setChecked(checked.size === reports.length ? new Set() : new Set(reports.map((r) => idText(r.id)))));
      else if (under === undefined) return;
      else if (k === "o") act(() => open(cursor, true));
      else if (k === "Enter" && e.target === document.body) act(() => open(cursor));
      else if (k === "x") act(() => toggle(idText(under.id)));
      else if (k === "c") act(() => copy(`${window.location.origin}${reportPath(under.id)}`, t(lang, "fb.linkCopied")));
      else if (k === "u") act(() => seen(!targets().every((r) => r.seen)));
      else if (inTrash) return;
      else if (k === "#" || k === "Delete") act(() => bulk({ do: "trash" }, t(lang, "fb.bulkTrashed", { n: targets().length })));
      else if (k === "m") {
        const mine = targets().every((r) => r.assignee !== null && idText(r.assignee) === me);
        act(() => bulk({ do: "change", change: { assignee: mine ? null : idOf(me) } }, t(lang, mine ? "fb.bulkUnassigned" : "fb.bulkMine", { n: targets().length })));
      } else if (/^[1-5]$/.test(k)) {
        const status = SETTABLE[Number(k) - 1];
        if (status !== undefined) act(() => bulk({ do: "change", change: { status } }, t(lang, "fb.bulkStatus", { status: t(lang, STATUS_WORD[status]), n: targets().length })));
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  const pills = pillsOf(lang, query, view, admins, me);
  const extra = panelCount(query, view);
  const first = (page - 1) * perPage;
  const unreadHere = reports.filter(unread).length;
  const selected = reports.filter((r) => checked.has(idText(r.id)));
  const allChecked = reports.length > 0 && selected.length === reports.length;
  const { sort: _sort, asc: _asc, limit: _limit, ...scope } = withoutPage(query);

  return (
    <div className={`fb-split${drawer !== null ? " with-drawer" : ""}`}>
      <Panel
        title={<T id="fb.reports" />}
        actions={
          <>
            <button type="button" className="round-btn" aria-keyshortcuts="?" onClick={() => setHelp(true)}>
              <Icon name="keyboard" />
              <span className="vh">{t(lang, "fb.keysTitle")}</span>
            </button>
            <button type="button" className="button button-ghost button-sm" aria-busy={busy} onClick={again}>
              <Icon name="refresh" className="button-icon" />
              {t(lang, "fb.refresh")}
            </button>
          </>
        }
      >
        <Views view={view} counts={counts} me={me} onPick={(q) => go({ ...q, ...(query.sort === undefined ? {} : { sort: query.sort }), ...(query.asc ? { asc: true } : {}), ...(query.limit === undefined ? {} : { limit: query.limit }) })} />
        <div className="fb-toolbar">
          <SearchBox asked={query.q} onAsk={(q) => set({ q })} />
          <button type="button" className="button button-ghost button-sm" aria-expanded={filtersOpen} aria-controls="fb-filters" onClick={() => setFiltersOpen((o) => !o)}>
            <Icon name="filter" className="button-icon" />
            {extra > 0 ? t(lang, "fb.filtersN", { n: extra }) : t(lang, "fb.filters")}
          </button>
          <SavedViews api={api} query={writeQuery(withoutPage(query))} />
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
        <Chart api={api} />
        {inTrash && <p className="small">{t(lang, "fb.trashHint", { days: BOUNDS.trashDays })}</p>}
        <output className="fb-arrived">
          {newOnes > 0 && (
            <>
              <span>{t(lang, "fb.arrived", { n: newOnes })}</span>{" "}
              <button
                type="button"
                className="button button-sm"
                onClick={() => {
                  if (arrived !== null) setLoad({ at: "ready", ...arrived });
                  setArrived(null);
                  setCursor(0);
                }}
              >
                {t(lang, "fb.showArrived")}
              </button>
            </>
          )}
        </output>
        <div className="fb-countline">
          <output className="small list-count">
            {load.at === "ready" &&
              (total === 0
                ? t(lang, "fb.none")
                : t(lang, "fb.range", { from: first + 1, to: first + reports.length, total }) + (unreadHere > 0 ? ` · ${t(lang, "fb.unreadHere", { n: unreadHere })}` : ""))}
          </output>
          {load.at === "ready" && total > 0 && !groups && (
            <BusyButton busy={exporting} className="button button-ghost button-sm" onClick={() => void exportAll()}>
              {t(lang, "fb.exportAll", { n: Math.min(total, EXPORT_MAX) })}
            </BusyButton>
          )}
          {!inTrash && (
            <fieldset className="fb-mode">
              <legend className="vh">{t(lang, "fb.shownAs")}</legend>
              <button type="button" className="chip-btn" aria-pressed={!groups} onClick={() => setGroups(false)}>
                {t(lang, "fb.asList")}
              </button>
              <button type="button" className="chip-btn" aria-pressed={groups} onClick={() => setGroups(true)}>
                {t(lang, "fb.asGroups")}
              </button>
            </fieldset>
          )}
        </div>
        {selected.length > 0 && !groups && (
          <BulkBar
            selected={selected}
            admins={admins}
            me={me}
            labels={(facets?.labels ?? []).map((c) => c.value)}
            inTrash={inTrash}
            busy={working}
            onBulk={(action, said, ids) => bulk(action, said, ids ?? selected.map((r) => r.id))}
            onPurge={() => setPurging(true)}
            onSeen={(to) => seen(to, selected)}
            onCopy={() => copy(selected.map((r) => idText(r.id)).join("\n"), t(lang, "fb.idsCopied", { n: selected.length }))}
            onExport={() => exportRows(selected)}
            onClear={() => setChecked(new Set())}
          />
        )}
        {groups && !inTrash ? (
          <CrashGroups
            api={api}
            scope={writeQuery(scope)}
            onPick={(signature) => {
              setGroups(false);
              set({ signature });
            }}
          />
        ) : (
          <div aria-busy={busy}>
            {load.at === "loading" && <div className="skeleton app-skeleton" aria-hidden="true" />}
            {load.at === "failed" && <Failed reason={load.kind === "rejected" ? "fb.errList" : failureKey(load.kind)} onRetry={again} />}
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
                      <td className="fb-check">
                        <label className="fb-hit">
                          <input
                            type="checkbox"
                            aria-label={t(lang, "fb.selectAll")}
                            checked={allChecked}
                            ref={(el) => {
                              if (el) el.indeterminate = selected.length > 0 && !allChecked;
                            }}
                            onChange={() => setChecked(allChecked ? new Set() : new Set(reports.map((r) => idText(r.id))))}
                          />
                        </label>
                      </td>
                      <th scope="col">{t(lang, "fb.colReport")}</th>
                      <SortHead sort="received" label="fb.colReceived" query={query} set={set} />
                      <SortHead sort="kind" label="fb.colKind" query={query} set={set} className="fb-wide" />
                      <SortHead sort="status" label="fb.colStatus" query={query} set={set} className="fb-status" />
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
                        checked={checked.has(idText(r.id))}
                        opened={drawer === idText(r.id)}
                        onCheck={() => toggle(idText(r.id))}
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
        )}
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
            onChanged={update}
            onDecided={recount}
            onGone={() => {
              setDrawer(null);
              again();
            }}
            onBack={again}
          />
        </aside>
      )}
      {help && <Shortcuts keys={LIST_KEYS} onClose={() => setHelp(false)} />}
      {purging && (
        <Confirm title={t(lang, "fb.purgeManyTitle", { n: targets().length })} action="fb.purgeGo" busy={working} onCancel={() => setPurging(false)} onConfirm={purge}>
          <p>{t(lang, "fb.purgeText")}</p>
        </Confirm>
      )}
      {pending}
    </div>
  );
}

function Row({
  report: r,
  query,
  admins,
  me,
  cursor,
  checked,
  opened,
  onCheck,
  onOpen,
  set,
}: {
  report: ReportSummary;
  query: ListQuery;
  admins: Admins;
  me: string;
  cursor: boolean;
  checked: boolean;
  opened: boolean;
  onCheck(): void;
  onOpen(): void;
  set(c: Change): void;
}) {
  const lang = useLang();
  const isUnread = unread(r);
  const line = firstLine(r.excerpt, 120);
  return (
    <tr className={[cursor ? "fb-cursor" : "", opened ? "fb-opened" : "", isUnread ? "fb-unread" : "", checked ? "fb-checked" : ""].filter(Boolean).join(" ") || undefined}>
      <td className="fb-check">
        <label className="fb-hit">
          <input type="checkbox" checked={checked} onChange={onCheck} aria-label={t(lang, "fb.selectOne", { what: line === "" ? shortId(r.id) : firstLine(line, 40) })} />
        </label>
      </td>
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
          {/* beside the drawer the status column gives way, and the status shows here */}
          <span className="fb-status-inline">
            <StatusPill status={r.status} />
          </span>
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
      <td className="num fb-received">
        <Ago at={r.createdAt} />
      </td>
      <td className="fb-wide">
        <KindPill kind={r.kind} />
      </td>
      <td className="fb-status">
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
