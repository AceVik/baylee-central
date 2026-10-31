// What an administrator decides about one report (`ReportView`, docs/
// reports.md §"Triage"): its status, priority, who looks after it, its
// labels, the report it duplicates and its GitHub issue; its notes, and its
// history in words. Each change names the revision it was read at, so two
// administrators never overwrite each other unseen (`ReportView` reads it
// again on a conflict).

import { useId, useState, type FormEvent } from "react";

import { idOf, idText } from "../../account/owner-api";
import { when } from "../../app/format";
import { Icon } from "../../components/icons";
import { t, useLang, type Key } from "../../i18n";
import { Link } from "../../router";
import { BOUNDS, KINDS, PRIORITIES, STATUSES, validLabel, type Priority, type ReportChange, type ReportDetail, type ReportKind } from "./api";
import { CATEGORY, issueUrl, newIssueUrl, parseIssue } from "./github";
import { Ago, KIND_WORD, PRIORITY_WORD, STATUS_WORD, shortId, type Admins } from "./pieces";
import { listPath, reportPath } from "./query";

/** A change and the words that say it was made. */
export type Apply = (change: ReportChange, said: string) => void;

interface Props {
  report: ReportDetail;
  busy: boolean;
  apply: Apply;
}

/** The statuses a button sets; `duplicate` comes with the report it duplicates. */
export const SETTABLE = STATUSES.filter((s) => s !== "duplicate");

export function StatusPicker({ report, busy, apply }: Props) {
  const lang = useLang();
  const now = report.summary.status;
  return (
    <fieldset className="fb-chips">
      <legend>{t(lang, "fb.status")}</legend>
      <div className="chips">
        {SETTABLE.map((s) => (
          <button
            key={s}
            type="button"
            className="chip-btn"
            aria-pressed={now === s}
            aria-disabled={busy || now === s ? true : undefined}
            onClick={() => {
              if (!busy && now !== s) apply({ status: s }, t(lang, "fb.statusSet", { status: t(lang, STATUS_WORD[s]) }));
            }}
          >
            {t(lang, STATUS_WORD[s])}
          </button>
        ))}
        {now === "duplicate" && (
          <button type="button" className="chip-btn" aria-pressed="true" aria-disabled="true">
            {t(lang, STATUS_WORD.duplicate)}
          </button>
        )}
      </div>
    </fieldset>
  );
}

const LOW_FIRST: readonly (Priority | null)[] = [null, ...PRIORITIES.toReversed()];

export function PriorityPicker({ report, busy, apply }: Props) {
  const lang = useLang();
  const now = report.summary.priority;
  return (
    <fieldset className="fb-chips">
      <legend>{t(lang, "fb.priority")}</legend>
      <div className="chips">
        {LOW_FIRST.map((p) => (
          <button
            key={p ?? "none"}
            type="button"
            className="chip-btn"
            aria-pressed={now === p}
            aria-disabled={busy || now === p ? true : undefined}
            onClick={() => {
              if (!busy && now !== p) apply({ priority: p }, t(lang, "fb.prioritySet", { priority: t(lang, PRIORITY_WORD[p ?? "none"]) }));
            }}
          >
            {t(lang, PRIORITY_WORD[p ?? "none"])}
          </button>
        ))}
      </div>
    </fieldset>
  );
}

export function AssigneePicker({ report, busy, apply, admins, me }: Props & { admins: Admins; me: string }) {
  const lang = useLang();
  const id = useId();
  const now = report.summary.assignee === null ? "" : idText(report.summary.assignee);
  const people = admins.list.map((a) => idText(a.id));
  if (!people.includes(me)) people.unshift(me);
  if (now !== "" && !people.includes(now)) people.push(now);
  const nameOf = (hex: string) => (hex === me ? t(lang, "fb.me") : admins.name(idOf(hex)));
  const pick = (hex: string) => {
    if (busy || hex === now) return;
    const whom = idOf(hex);
    apply({ assignee: whom }, whom === null ? t(lang, "fb.unassignedSaid") : t(lang, "fb.assignedSaid", { name: nameOf(hex) }));
  };
  return (
    <div className="field fb-field">
      <label htmlFor={id}>{t(lang, "fb.assignee")}</label>
      <div className="fb-inline">
        <select id={id} value={now} aria-disabled={busy ? true : undefined} onChange={(e) => pick(e.target.value)}>
          <option value="">{t(lang, "fb.nobody")}</option>
          {people.map((hex) => (
            <option key={hex} value={hex}>
              {nameOf(hex)}
            </option>
          ))}
        </select>
        {now !== me && (
          <button type="button" className="button button-ghost button-sm" onClick={() => pick(me)}>
            {t(lang, "fb.assignMe")}
          </button>
        )}
      </div>
    </div>
  );
}

export function LabelEditor({ report, busy, apply, known }: Props & { known: readonly string[] }) {
  const lang = useLang();
  const id = useId();
  const [draft, setDraft] = useState("");
  const [bad, setBad] = useState(false);
  const labels = report.summary.labels;
  const add = (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    const label = draft.trim().toLowerCase();
    if (!validLabel(label)) {
      setBad(true);
      return;
    }
    setBad(false);
    setDraft("");
    if (!busy && !labels.includes(label)) apply({ labels: [...labels, label].sort() }, t(lang, "fb.labelsSet"));
  };
  const full = labels.length >= BOUNDS.labels;
  return (
    <div className="fb-field">
      <p className="fb-legend">{t(lang, "fb.labels")}</p>
      {labels.length > 0 && (
        <ul className="fb-label-list">
          {labels.map((label) => (
            <li key={label}>
              <Link className="fb-label" to={listPath({ label })} translate="no">
                {label}
              </Link>
              <button
                type="button"
                className="round-btn fb-x"
                aria-disabled={busy ? true : undefined}
                onClick={() => {
                  if (!busy) apply({ labels: labels.filter((l) => l !== label) }, t(lang, "fb.labelsSet"));
                }}
              >
                <Icon name="close" />
                <span className="vh">{t(lang, "fb.removeLabel", { label })}</span>
              </button>
            </li>
          ))}
        </ul>
      )}
      <form className="fb-inline" onSubmit={add}>
        <label className="vh" htmlFor={id}>
          {t(lang, "fb.addLabel")}
        </label>
        <input
          id={id}
          list={`${id}-known`}
          className="fb-input"
          placeholder={t(lang, "fb.labelHint")}
          value={draft}
          maxLength={BOUNDS.label}
          spellCheck={false}
          aria-invalid={bad ? true : undefined}
          aria-describedby={bad ? `${id}-bad` : undefined}
          disabled={full}
          onChange={(e) => {
            setDraft(e.target.value);
            setBad(false);
          }}
        />
        <datalist id={`${id}-known`}>
          {known
            .filter((l) => !labels.includes(l))
            .map((l) => (
              <option key={l} value={l}>
                {l}
              </option>
            ))}
        </datalist>
        <button type="submit" className="button button-ghost button-sm" disabled={full || draft.trim() === ""}>
          {t(lang, "fb.addLabel")}
        </button>
      </form>
      {bad && (
        <p id={`${id}-bad`} className="small fb-bad" role="alert">
          {t(lang, "fb.badLabel")}
        </p>
      )}
    </div>
  );
}

/** A report's id as an administrator pastes it: 32 hex digits, a UUID, or its page's address. */
export function readReportId(text: string): string | null {
  const plain = text.trim().toLowerCase().replaceAll("-", "");
  const found = /(?:^|\/)([0-9a-f]{32})(?:[/?#]|$)/.exec(plain)?.[1] ?? null;
  return found !== null && idOf(found) !== null ? found : null;
}

export function DuplicateEditor({ report, busy, apply, query }: Props & { query: Parameters<typeof reportPath>[1] }) {
  const lang = useLang();
  const id = useId();
  const [draft, setDraft] = useState("");
  const [bad, setBad] = useState(false);
  const original = report.summary.duplicateOf;
  if (original !== null) {
    return (
      <div className="fb-field">
        <p className="fb-legend">{t(lang, "fb.duplicateOf")}</p>
        <div className="fb-inline">
          <Link to={reportPath(original, query)}>
            <code>{shortId(original)}</code>
          </Link>
          <button
            type="button"
            className="button button-ghost button-sm"
            onClick={() => {
              if (!busy) apply({ duplicateOf: null }, t(lang, "fb.duplicateCleared"));
            }}
          >
            {t(lang, "fb.notDuplicate")}
          </button>
        </div>
      </div>
    );
  }
  const mark = (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    const found = readReportId(draft);
    if (found === null || found === idText(report.summary.id)) {
      setBad(true);
      return;
    }
    setBad(false);
    setDraft("");
    const target = idOf(found);
    if (!busy && target !== null) apply({ duplicateOf: target }, t(lang, "fb.duplicateSet"));
  };
  return (
    <form className="fb-field" onSubmit={mark}>
      <label htmlFor={id}>{t(lang, "fb.markDuplicate")}</label>
      <div className="fb-inline">
        <input
          id={id}
          className="fb-input"
          spellCheck={false}
          placeholder={t(lang, "fb.duplicateHint")}
          value={draft}
          aria-invalid={bad ? true : undefined}
          aria-describedby={bad ? `${id}-bad` : undefined}
          onChange={(e) => {
            setDraft(e.target.value);
            setBad(false);
          }}
        />
        <button type="submit" className="button button-ghost button-sm" disabled={draft.trim() === ""}>
          {t(lang, "fb.mark")}
        </button>
      </div>
      {bad && (
        <p id={`${id}-bad`} className="small fb-bad" role="alert">
          {t(lang, "fb.badDuplicate")}
        </p>
      )}
    </form>
  );
}

/** The issue a report is linked to, linking one, and filing a new one in neutral words. */
export function IssueEditor({ report, busy, apply }: Props) {
  const lang = useLang();
  const id = useId();
  const [draft, setDraft] = useState("");
  const [bad, setBad] = useState(false);
  const issue = report.summary.issue;
  const href = issue === null ? null : issueUrl(issue);
  const link = (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    const n = parseIssue(draft);
    if (n === null) {
      setBad(true);
      return;
    }
    setBad(false);
    setDraft("");
    if (!busy) apply({ issue: n }, t(lang, "fb.issueLinked", { n: String(n) }));
  };
  return (
    <div className="fb-field">
      <p className="fb-legend">{t(lang, "fb.issue")}</p>
      {href !== null && issue !== null ? (
        <div className="fb-inline">
          <a href={href} target="_blank" rel="noopener noreferrer">
            #{issue}
            <Icon name="external" className="fb-ext" />
            <span className="vh">{t(lang, "fb.newTab")}</span>
          </a>
          <button
            type="button"
            className="button button-ghost button-sm"
            onClick={() => {
              if (!busy) apply({ issue: null }, t(lang, "fb.issueUnlinked"));
            }}
          >
            {t(lang, "fb.unlink")}
          </button>
        </div>
      ) : (
        <form className="fb-inline" onSubmit={link}>
          <label className="vh" htmlFor={id}>
            {t(lang, "fb.issueLabel")}
          </label>
          <input
            id={id}
            className="fb-input"
            inputMode="text"
            spellCheck={false}
            placeholder={t(lang, "fb.issueHint")}
            value={draft}
            aria-invalid={bad ? true : undefined}
            aria-describedby={bad ? `${id}-bad` : undefined}
            onChange={(e) => {
              setDraft(e.target.value);
              setBad(false);
            }}
          />
          <button type="submit" className="button button-ghost button-sm" disabled={draft.trim() === ""}>
            {t(lang, "fb.link")}
          </button>
        </form>
      )}
      {bad && (
        <p id={`${id}-bad`} className="small fb-bad" role="alert">
          {t(lang, "fb.badIssue")}
        </p>
      )}
      {issue === null && <NewIssue kind={report.summary.kind} build={report.summary.build ?? report.summary.gatewayVersion} />}
    </div>
  );
}

/** GitHub's form for a new issue, filled with the administrator's own neutral words; its own state, so typing redraws it alone. */
function NewIssue({ kind, build }: { kind: ReportKind; build: string | null }) {
  const lang = useLang();
  const id = useId();
  const [summary, setSummary] = useState("");
  const [category, setCategory] = useState<ReportKind>(kind);
  const url = newIssueUrl({ summary, category, build });
  return (
    <details className="fb-more">
      <summary>{t(lang, "fb.newIssue")}</summary>
      <div className="field">
        <label htmlFor={`${id}-summary`}>{t(lang, "fb.summaryLabel")}</label>
        <textarea id={`${id}-summary`} rows={3} aria-describedby={`${id}-hint`} value={summary} onChange={(e) => setSummary(e.target.value)} />
        <p id={`${id}-hint`} className="small">
          {t(lang, "fb.summaryHint")}
        </p>
      </div>
      <div className="field fb-field">
        <label htmlFor={`${id}-category`}>{t(lang, "fb.category")}</label>
        <select id={`${id}-category`} value={category} onChange={(e) => setCategory(KINDS.find((k) => k === e.target.value) ?? kind)}>
          {KINDS.map((k) => (
            <option key={k} value={k}>
              {CATEGORY[k]}
            </option>
          ))}
        </select>
      </div>
      {url === null ? (
        <p className="small">{t(lang, "fb.summaryFirst")}</p>
      ) : (
        <a className="button button-ghost" href={url} target="_blank" rel="noopener noreferrer">
          {t(lang, "fb.openIssue")}
          <Icon name="external" className="button-icon" />
          <span className="vh">{t(lang, "fb.newTab")}</span>
        </a>
      )}
    </details>
  );
}

/** The notes: internal, each with its author and time; only its author removes one. */
export function Notes({ report, me, admins, busy, onAdd, onRemove }: { report: ReportDetail; me: string; admins: Admins; busy: boolean; onAdd(text: string): Promise<boolean>; onRemove(note: Uint8Array): void }) {
  const lang = useLang();
  const id = useId();
  const [draft, setDraft] = useState("");
  const left = BOUNDS.note - [...draft].length;
  const add = () => {
    const text = draft.trim();
    if (text === "" || busy || left < 0) return;
    void onAdd(text).then((ok) => {
      if (ok) setDraft("");
    });
  };
  return (
    <>
      {report.notes.length === 0 ? (
        <p className="small">{t(lang, "fb.noNotes")}</p>
      ) : (
        <ol className="fb-notes">
          {report.notes.map((n) => {
            const mine = idText(n.author) === me;
            return (
              <li key={idText(n.id)} className="fb-note">
                <p className="fb-note-head small">
                  <bdi translate="no">{mine ? t(lang, "fb.me") : admins.name(n.author)}</bdi> · <Ago at={n.at} />
                  {mine && (
                    <button type="button" className="round-btn fb-x" aria-disabled={busy ? true : undefined} onClick={() => !busy && onRemove(n.id)}>
                      <Icon name="close" />
                      <span className="vh">{t(lang, "fb.removeNote", { time: when(lang, n.at) })}</span>
                    </button>
                  )}
                </p>
                <p className="fb-note-text" dir="auto">
                  {n.text}
                </p>
              </li>
            );
          })}
        </ol>
      )}
      <form
        className="field fb-note-form"
        onSubmit={(e) => {
          e.preventDefault();
          add();
        }}
      >
        <label htmlFor={id}>{t(lang, "fb.addNote")}</label>
        <textarea
          id={id}
          rows={3}
          value={draft}
          aria-describedby={`${id}-hint`}
          aria-invalid={left < 0 ? true : undefined}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
              e.preventDefault();
              add();
            }
          }}
        />
        <div className="fb-note-foot">
          <span id={`${id}-hint`} className="small">
            {t(lang, "fb.noteHint")} <span className={`count${left < 200 ? " near" : ""}`}>{t(lang, "fb.charsLeft", { n: left })}</span>
          </span>
          <button type="submit" className="button button-ghost button-sm" disabled={draft.trim() === "" || left < 0}>
            {t(lang, "fb.saveNote")}
          </button>
        </div>
      </form>
    </>
  );
}

const ACTION_WORD: Readonly<Record<string, Key>> = {
  "report.filed": "fb.hFiled",
  "report.status": "fb.hStatus",
  "report.issue": "fb.hIssue",
  "report.assignee": "fb.hAssignee",
  "report.priority": "fb.hPriority",
  "report.labels": "fb.hLabels",
  "report.duplicate": "fb.hDuplicate",
  "report.trashed": "fb.hTrashed",
  "report.restored": "fb.hRestored",
  "report.purged": "fb.hPurged",
  "report.note": "fb.hNote",
  "report.swept": "fb.hSwept",
};

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;

/** A history line's detail in words: an id as a name or a short id, a status or priority in the page's language. */
export function detailText(lang: Parameters<typeof t>[0], action: string, detail: string, admins: Admins): string {
  if (detail === "") return "";
  if (detail === "none") return t(lang, "fb.hNone");
  const word = (w: string) => {
    const s = STATUSES.find((x) => x === w);
    if (s !== undefined) return t(lang, STATUS_WORD[s]);
    const p = PRIORITIES.find((x) => x === w);
    return p === undefined ? w : t(lang, PRIORITY_WORD[p]);
  };
  switch (action) {
    case "report.status":
      return detail.split(" → ").map(word).join(" → ");
    case "report.priority":
      return word(detail);
    case "report.assignee":
      return UUID.test(detail) ? admins.name(idOf(detail.replaceAll("-", ""))) : detail;
    case "report.duplicate":
      return UUID.test(detail) ? `…${detail.replaceAll("-", "").slice(-8)}` : detail.startsWith("none:") ? t(lang, "fb.hOriginalGone") : detail;
    case "report.note":
      return detail === "added" ? t(lang, "fb.hNoteAdded") : detail === "removed" ? t(lang, "fb.hNoteRemoved") : detail;
    case "report.filed":
      return detail.startsWith("by ") ? detail.slice(3) : detail;
    default:
      return detail;
  }
}

export function History({ report, admins }: { report: ReportDetail; admins: Admins }) {
  const lang = useLang();
  if (report.audit.length === 0) return <p className="small">{t(lang, "fb.noHistory")}</p>;
  return (
    <ol className="fb-history">
      {report.audit.map((row, i) => {
        const what = ACTION_WORD[row.action];
        const detail = detailText(lang, row.action, row.detail, admins);
        return (
          <li key={i}>
            <span className="small">
              <Ago at={row.at} />
            </span>{" "}
            <bdi translate="no">{row.actor === null ? t(lang, "fb.hCentral") : admins.name(row.actor)}</bdi>
            {": "}
            {what === undefined ? <code>{row.action}</code> : t(lang, what)}
            {detail !== "" && (
              <>
                {" "}
                <bdi className="fb-detail">{detail}</bdi>
              </>
            )}
          </li>
        );
      })}
    </ol>
  );
}

/** The ways to the lists this report belongs with: its crash group, its duplicates. */
export function Related({ report }: { report: ReportDetail }) {
  const lang = useLang();
  const s = report.summary;
  const hex = idText(s.id);
  return (
    <ul className="fb-related">
      {s.signature !== null && (
        <li>
          <Link to={listPath({ signature: s.signature })}>{t(lang, "fb.sameCrash")}</Link>
        </li>
      )}
      {s.duplicateOf === null && (
        <li>
          <Link to={listPath({ duplicateOf: hex })}>{t(lang, "fb.itsDuplicates")}</Link>
        </li>
      )}
      <li>
        <Link to={listPath({ reporter: s.reporter })}>{t(lang, "fb.sameReporter")}</Link>
      </li>
      <li>
        <Link to={listPath({ kinds: [s.kind] })}>{t(lang, "fb.sameKind", { kind: t(lang, KIND_WORD[s.kind]) })}</Link>
      </li>
    </ul>
  );
}
