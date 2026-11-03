// One report, read and decided (docs/reports.md §"The website's section"):
// what the player wrote, where it came from, its triage, notes, the lists it
// belongs with, its history, and the trash. The report's page and the list's
// drawer draw the same view, the drawer in one column. Opening a report
// marks it opened for this administrator; every change names the revision
// it was read at, and a conflict reads the report again and says so.

import { useCallback, useEffect, useId, useMemo, useRef, useState, type ReactNode } from "react";

import { ApiError, type AccountSummary } from "../../account/api";
import { idOf } from "../../account/owner-api";
import { Confirm } from "../../app/Confirm";
import { CopyButton } from "../../app/Copy";
import { Failed } from "../../app/AppFrame";
import { when } from "../../app/format";
import { Icon } from "../../components/icons";
import { T, localeOf, num, t, useLang } from "../../i18n";
import { navigate } from "../../router";
import { failureKey, useChange } from "../shared";
import { BOUNDS, type ReportChange, type ReportDetail, type ReportSummary, type ReportsApi } from "./api";
import { firstLine } from "./github";
import { Ago, KindPill, PriorityPill, Reporter, StatusPill, shortId, useNotices, type Admins } from "./pieces";
import { listPath, reportPath, type ListQuery } from "./query";
import { ClientDetails, RecordDownload, RefText, useClientPart } from "./ClientView";
import { readRefs } from "./client";
import { AssigneePicker, DuplicateEditor, History, IssueEditor, LabelEditor, Notes, PriorityPicker, Related, StatusPicker } from "./Triage";

/** `bytes` as a person reads it: 912 B, 21.1 KiB, 3.4 MiB. */
export function size(lang: Parameters<typeof num>[0], bytes: number): string {
  if (bytes < 1024) return `${num(lang, bytes)} B`;
  const units = ["KiB", "MiB", "GiB"] as const;
  let value = bytes / 1024;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${value.toLocaleString(localeOf(lang), { maximumFractionDigits: value < 10 ? 1 : 0 })} ${units[unit] ?? "GiB"}`;
}

function Section({ title, children, folded = false, className = "" }: { title: string; children: ReactNode; folded?: boolean; className?: string }) {
  const id = useId();
  if (folded) {
    return (
      <details className={`card fb-section fb-fold ${className}`}>
        <summary>
          <h3 className="fb-section-title">{title}</h3>
        </summary>
        {children}
      </details>
    );
  }
  return (
    <section className={`card fb-section ${className}`} aria-labelledby={id}>
      <h3 id={id} className="fb-section-title">
        {title}
      </h3>
      {children}
    </section>
  );
}

type Load = { at: "loading" } | { at: "failed"; kind: string } | { at: "ready"; report: ReportDetail };

export interface ReportViewProps {
  api: ReportsApi;
  /** The report's id in hex. */
  hex: string;
  /** The reading administrator's id in hex. */
  me: string;
  admins: Admins;
  /** The list it was opened from, for the ways back and on. */
  query: ListQuery;
  summary: AccountSummary;
  replace(s: AccountSummary): void;
  /** One column, the long parts folded: the list's drawer. */
  compact?: boolean;
  /** It was read or changed (its triage, its unread mark): the row that shows it follows. */
  onChanged?(s: ReportSummary): void;
  /** An administrator decided something (a change, a note, the trash): the counts may have moved. */
  onDecided?(): void;
  /** It left the list it was opened from: into the trash, or gone for good. */
  onGone?(hex: string): void;
  /** It came back from the trash, by the notice's "Undo". */
  onBack?(hex: string): void;
  /** The navigation's unread count may have moved. */
  recountUnread(): void;
}

export function ReportView({ api, hex, me, admins, query, summary, replace, compact = false, onChanged, onDecided, onGone, onBack, recountUnread }: ReportViewProps) {
  const lang = useLang();
  const notices = useNotices();
  // one id per report, not one per render: the reads depend on it
  const id = useMemo(() => idOf(hex) ?? new Uint8Array(16), [hex]);
  const [load, setLoad] = useState<Load>({ at: "loading" });
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [asking, setAsking] = useState(false);
  const [known, setKnown] = useState<string[] | null>(null);
  const { run, pending } = useChange(summary, replace);
  // the client's details, once the report says they arrived; in the drawer a moment later, so stepping through reads only where it stops
  const [client, retryClient] = useClientPart(api, id, hex, load.at === "ready" && load.report.client?.received === true, compact ? 400 : 0);
  // the callbacks of the newest render, for answers that come after it
  const hooks = useRef({ onChanged, onDecided, onGone, onBack, recountUnread });
  useEffect(() => {
    hooks.current = { onChanged, onDecided, onGone, onBack, recountUnread };
  });

  const show = useCallback((report: ReportDetail) => {
    setLoad({ at: "ready", report });
    hooks.current.onChanged?.(report.summary);
  }, []);

  const read = useCallback(
    (first: boolean) => {
      let live = true;
      api.report(id).then(
        (report) => {
          if (!live) return;
          // opening it marks it opened for this administrator; the mark is a convenience, the next read says how it is
          if (first && !report.summary.seen) {
            const opened = { ...report, summary: { ...report.summary, seen: true } };
            show(opened);
            api.setSeen([id], true).then(
              () => hooks.current.recountUnread(),
              () => undefined,
            );
          } else show(report);
        },
        (e: unknown) => {
          if (live) setLoad({ at: "failed", kind: e instanceof ApiError ? e.kind : "network" });
        },
      );
      return () => {
        live = false;
      };
    },
    [api, id, show],
  );
  useEffect(() => read(true), [read]);

  const change = (c: ReportChange, said: string) => {
    if (load.at !== "ready" || busy) return;
    setBusy(true);
    setError(null);
    api
      .setReport(id, c, load.report.summary.revision)
      .then(
        (report) => {
          show(report);
          notices.say(said);
          hooks.current.onDecided?.();
          if (c.status !== undefined || Object.hasOwn(c, "duplicateOf")) hooks.current.recountUnread();
        },
        (e: unknown) => {
          const kind = e instanceof ApiError ? e.kind : "network";
          if (kind === "conflict") {
            // another administrator changed it meanwhile: shown as it is now, decided again
            read(false);
            setError(t(lang, "fb.conflict"));
          } else setError(t(lang, kind === "rejected" ? "fb.errRefused" : failureKey(kind)));
        },
      )
      .finally(() => setBusy(false));
  };

  const addNote = (text: string): Promise<boolean> => {
    setBusy(true);
    setError(null);
    return api
      .addNote(id, text)
      .then(
        (report) => {
          show(report);
          notices.say(t(lang, "fb.noteSaid"));
          return true;
        },
        (e: unknown) => {
          setError(t(lang, failureKey(e instanceof ApiError ? e.kind : "network")));
          return false;
        },
      )
      .finally(() => setBusy(false));
  };

  const removeNote = (note: Uint8Array) => {
    setBusy(true);
    setError(null);
    api
      .removeNote(id, note)
      .then(
        (report) => {
          show(report);
          notices.say(t(lang, "fb.noteRemovedSaid"));
        },
        (e: unknown) => setError(t(lang, failureKey(e instanceof ApiError ? e.kind : "network"))),
      )
      .finally(() => setBusy(false));
  };

  /** Out of the trash: the banner's button, or the notice's "Undo" after the view may have gone. */
  const restore = () => {
    api.restore(id).then(
      () => {
        notices.say(t(lang, "fb.restoredOne"));
        hooks.current.recountUnread();
        const back = hooks.current.onBack;
        if (back) back(hex);
        else read(false);
      },
      (e: unknown) => notices.say(t(lang, failureKey(e instanceof ApiError ? e.kind : "network")), { bad: true }),
    );
  };

  const trash = () => {
    if (busy) return;
    setBusy(true);
    setError(null);
    api
      .trash(id)
      .then(
        () => {
          hooks.current.recountUnread();
          hooks.current.onDecided?.();
          const gone = hooks.current.onGone;
          notices.say(t(lang, "fb.trashedOne"), { undo: restore });
          if (gone) gone(hex);
          else read(false);
        },
        (e: unknown) => setError(t(lang, failureKey(e instanceof ApiError ? e.kind : "network"))),
      )
      .finally(() => setBusy(false));
  };

  const purge = () => {
    setBusy(true);
    api
      .purge(id)
      .then(
        () => {
          setAsking(false);
          notices.say(t(lang, "fb.purgedOne"));
          const gone = hooks.current.onGone;
          if (gone) gone(hex);
          else navigate(listPath({ ...query, trash: true }), true);
        },
        (e: unknown) => {
          setAsking(false);
          setError(t(lang, failureKey(e instanceof ApiError ? e.kind : "network")));
        },
      )
      .finally(() => setBusy(false));
  };

  if (load.at === "loading") return <div className="card skeleton app-skeleton" aria-busy="true" />;
  if (load.at === "failed") return <Failed reason={load.kind === "rejected" ? "fb.errReport" : failureKey(load.kind)} onRetry={() => read(true)} />;

  const report = load.report;
  const s = report.summary;
  const heading = firstLine(report.text, 120) || t(lang, "fb.noText");
  const page = `${window.location.origin}${reportPath(id)}`;
  const triage = (
    <Section title={t(lang, "fb.triage")}>
      {s.deletedAt === null ? (
        <>
          <StatusPicker report={report} busy={busy} apply={change} />
          <PriorityPicker report={report} busy={busy} apply={change} />
          <AssigneePicker report={report} busy={busy} apply={change} admins={admins} me={me} />
          <div
            onFocusCapture={() => {
              if (known !== null) return;
              setKnown([]);
              api.facets({}).then(
                (f) => setKnown(f.labels.map((c) => c.value)),
                () => undefined,
              );
            }}
          >
            <LabelEditor report={report} busy={busy} apply={change} known={known ?? []} />
          </div>
          <DuplicateEditor report={report} busy={busy} apply={change} query={query} />
          <IssueEditor report={report} busy={busy} apply={change} />
        </>
      ) : (
        <p className="small">{t(lang, "fb.trashNoTriage")}</p>
      )}
    </Section>
  );
  const facts = (
    <Section title={t(lang, "fb.facts")}>
      <dl className="kv fb-kv">
        <div>
          <dt>{t(lang, "fb.received")}</dt>
          <dd>
            {when(lang, s.createdAt)} · <Ago at={s.createdAt} />
          </dd>
        </div>
        <div>
          <dt>{t(lang, "fb.changed")}</dt>
          <dd>
            <Ago at={s.updatedAt} />
          </dd>
        </div>
        <div>
          <dt>{t(lang, "fb.gateway")}</dt>
          <dd>
            {s.channel === "direct" ? (
              t(lang, "fb.channelDirect")
            ) : (
              <>
                <bdi translate="no">{s.gateway}</bdi>
                {s.gatewaySays !== null && s.gatewaySays !== s.gateway && (
                  <span className="small">
                    {" "}
                    · {t(lang, "fb.callsItself")} <bdi translate="no">{s.gatewaySays}</bdi>
                  </span>
                )}
              </>
            )}
            {report.gatewayUrl !== null && (
              <>
                <br />
                <code className="fb-code" translate="no">
                  {report.gatewayUrl}
                </code>
              </>
            )}
          </dd>
        </div>
        {s.gatewayVersion !== null && (
          <div>
            <dt>{t(lang, "fb.gatewayBuild")}</dt>
            <dd>
              <code className="fb-code">{s.gatewayVersion}</code>
            </dd>
          </div>
        )}
        <div>
          <dt>{t(lang, "fb.clientBuild")}</dt>
          <dd>
            {s.build === null ? "—" : <code className="fb-code">{s.build}</code>}
            {report.commit !== null && (
              <>
                {" "}
                <code className="fb-code">{report.commit.slice(0, 12)}</code>
              </>
            )}
          </dd>
        </div>
        <div>
          <dt>{t(lang, "fb.reporter")}</dt>
          <dd>
            <Reporter pseudonym={s.reporter} />
          </dd>
        </div>
        {report.gameId !== null && (
          <div>
            <dt>{t(lang, "fb.game")}</dt>
            <dd>
              <code className="fb-code" translate="no">
                {report.gameId}
              </code>
            </dd>
          </div>
        )}
        <div>
          <dt>{t(lang, "fb.recordPart")}</dt>
          <dd>
            {report.record === null
              ? t(lang, "fb.noRecord")
              : t(lang, report.record.received ? "fb.recordHere" : "fb.recordAwaited", {
                  size: size(lang, report.record.bytes),
                  who: t(lang, s.recordOrigin === "client" ? "fb.byClient" : "fb.byGateway"),
                  end: t(lang, s.recordComplete ? "fb.recordComplete" : "fb.recordPartial"),
                })}
            {report.record?.received === true && (
              <>
                <br />
                <RecordDownload api={api} id={id} hex={hex} origin={s.recordOrigin} />
              </>
            )}
          </dd>
        </div>
        <div>
          <dt>{t(lang, "fb.clientPart")}</dt>
          <dd>{report.client === null ? t(lang, "fb.noClient") : t(lang, report.client.received ? "fb.clientHere" : "fb.clientAwaited", { size: size(lang, report.client.bytes) })}</dd>
        </div>
        <div>
          <dt>{t(lang, "fb.id")}</dt>
          <dd className="copyable">
            <code className="fb-code" translate="no">
              {hex}
            </code>
            <CopyButton text={hex} label={t(lang, "fb.copyId")} />
          </dd>
        </div>
      </dl>
    </Section>
  );
  const text = (
    <Section title={t(lang, "fb.wrote")}>
      {report.text.trim() === "" ? (
        <p className="small">{t(lang, "fb.noText")}</p>
      ) : (
        <p className="fb-text" dir="auto">
          <RefText text={report.text} refs={client.at === "ready" ? readRefs(client.client) : null} />
        </p>
      )}
      {s.crash !== null && (
        <div className="fb-crash">
          <p className="fb-legend">{t(lang, "fb.crash")}</p>
          <pre className="fb-pre">{s.crash}</pre>
          {report.crashLocation !== null && <code className="fb-code">{report.crashLocation}</code>}
        </div>
      )}
    </Section>
  );
  const sent = (
    <Section title={t(lang, "fb.clientSent")}>
      <ClientDetails load={client} onRetry={retryClient} compact={compact} />
    </Section>
  );
  const notes = (
    <Section title={report.notes.length > 0 ? t(lang, "fb.notesN", { n: report.notes.length }) : t(lang, "fb.notes")}>
      <Notes report={report} me={me} admins={admins} busy={busy} onAdd={addNote} onRemove={removeNote} />
    </Section>
  );
  const related = (
    <Section title={t(lang, "fb.related")}>
      <Related report={report} />
    </Section>
  );
  const history = (
    <Section title={t(lang, "fb.history")} folded={compact}>
      <History report={report} admins={admins} />
    </Section>
  );
  const bin =
    s.deletedAt === null ? (
      <Section title={t(lang, "fb.trash")}>
        <p className="small">{t(lang, "fb.trashHint", { days: BOUNDS.trashDays })}</p>
        <button type="button" className="button button-danger" aria-disabled={busy ? true : undefined} onClick={trash}>
          <Icon name="trash" className="button-icon" />
          {t(lang, "fb.toTrash")}
        </button>
      </Section>
    ) : null;

  return (
    <article className={`fb-report${compact ? " fb-compact" : ""}`} aria-labelledby={`r-${hex}`}>
      <header className="fb-report-head">
        <p className="fb-report-pills">
          <KindPill kind={s.kind} /> <StatusPill status={s.status} /> <PriorityPill priority={s.priority} />
          {s.notes > 0 && <span className="pill">{t(lang, "fb.notesN", { n: s.notes })}</span>}
        </p>
        <h2 id={`r-${hex}`} className="fb-report-title" dir="auto">
          {heading}
        </h2>
        <p className="small copyable">
          <span>
            <code className="fb-code">…{shortId(hex)}</code> · <Ago at={s.createdAt} />
          </span>
          <CopyButton text={page} label={t(lang, "fb.copyLink")} />
        </p>
      </header>
      {s.deletedAt !== null && (
        <div className="card fb-trashed" role="note">
          <p>
            <T id="fb.inTrashSince" args={{ time: when(lang, s.deletedAt), days: BOUNDS.trashDays }} />
          </p>
          <div className="action-row">
            <button type="button" className="button" onClick={restore}>
              {t(lang, "fb.restore")}
            </button>
            <button type="button" className="button button-danger" onClick={() => setAsking(true)}>
              {t(lang, "fb.purge")}
            </button>
          </div>
        </div>
      )}
      {error !== null && (
        <p className="fb-error" role="alert">
          <Icon name="warn" className="pill-icon" /> {error}
        </p>
      )}
      {compact ? (
        <div className="fb-report-col">
          {triage}
          {text}
          {sent}
          {facts}
          {notes}
          {related}
          {history}
          {bin}
        </div>
      ) : (
        <div className="fb-report-grid">
          <div className="fb-report-col">
            {text}
            {sent}
            {facts}
            {history}
          </div>
          <div className="fb-report-col">
            {triage}
            {notes}
            {related}
            {bin}
          </div>
        </div>
      )}
      {asking && (
        <Confirm title={t(lang, "fb.purgeTitle")} action="fb.purgeGo" busy={busy} onCancel={() => setAsking(false)} onConfirm={() => run(purge)}>
          <p>{t(lang, "fb.purgeText")}</p>
        </Confirm>
      )}
      {pending}
    </article>
  );
}
