// The small pieces the feedback section shares: a report's kind, status and
// priority as pills in words, a moment as "5 minutes ago" with the exact time
// behind it, a reporter as an alias, the administrators by their handles,
// and the notices that say what an action did (with "Undo" where it can be
// undone). Nothing a report says is ever drawn as HTML: text only.

import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";

import { idText } from "../../account/owner-api";
import { when } from "../../app/format";
import { Icon } from "../../components/icons";
import { localeOf, t, useLang, type Key, type Lang } from "../../i18n";
import { who } from "../shared";
import { alias } from "./alias";
import type { Priority, ReportAdmin, ReportKind, ReportStatus, ReportsApi } from "./api";

export const KIND_WORD: Record<ReportKind, Key> = {
  bug: "fb.kindBug",
  improvement: "fb.kindImprovement",
  feedback: "fb.kindFeedback",
  crash: "fb.kindCrash",
  other: "fb.kindOther",
};

export const STATUS_WORD: Record<ReportStatus, Key> = {
  new: "fb.statusNew",
  triaged: "fb.statusTriaged",
  in_progress: "fb.statusInProgress",
  resolved: "fb.statusResolved",
  wont_fix: "fb.statusWontFix",
  duplicate: "fb.statusDuplicate",
};

export const PRIORITY_WORD: Record<Priority | "none", Key> = {
  critical: "fb.prioCritical",
  high: "fb.prioHigh",
  normal: "fb.prioNormal",
  low: "fb.prioLow",
  none: "fb.prioNone",
};

const STATUS_TONE: Record<ReportStatus, string> = {
  new: "info",
  triaged: "",
  in_progress: "warn",
  resolved: "ok",
  wont_fix: "off",
  duplicate: "off",
};

const PRIORITY_TONE: Record<Priority, string> = { critical: "bad", high: "warn", normal: "", low: "off" };

export function KindPill({ kind }: { kind: ReportKind }) {
  const lang = useLang();
  return <span className={`pill fb-kind${kind === "crash" ? " bad" : ""}`}>{t(lang, KIND_WORD[kind])}</span>;
}

export function StatusPill({ status }: { status: ReportStatus }) {
  const lang = useLang();
  return <span className={`pill ${STATUS_TONE[status]}`}>{t(lang, STATUS_WORD[status])}</span>;
}

/** A priority as a pill; nothing without one. */
export function PriorityPill({ priority }: { priority: Priority | null }) {
  const lang = useLang();
  if (priority === null) return null;
  return (
    <span className={`pill ${PRIORITY_TONE[priority]}`}>
      <span className="vh">{t(lang, "fb.priority")}: </span>
      {t(lang, PRIORITY_WORD[priority])}
    </span>
  );
}

/** A report's id as the page shows it short: its last eight hex digits, the random part of a v7 id. */
export const shortId = (id: Uint8Array | string): string => (typeof id === "string" ? id : idText(id)).slice(-8);

const UNITS: [Intl.RelativeTimeFormatUnit, number][] = [
  ["year", 31_536_000],
  ["month", 2_592_000],
  ["week", 604_800],
  ["day", 86_400],
  ["hour", 3_600],
  ["minute", 60],
];

/** How long ago `seconds` was, as the language says it. */
export function ago(lang: Lang, seconds: number, now = Date.now() / 1000): string {
  const delta = Math.round(seconds - now);
  const format = new Intl.RelativeTimeFormat(localeOf(lang), { numeric: "auto" });
  for (const [unit, size] of UNITS) {
    if (Math.abs(delta) >= size) return format.format(Math.round(delta / size), unit);
  }
  return format.format(0, "minute");
}

/** A moment as how long ago, the exact time in its title and for a screen reader's `dateTime`. */
export function Ago({ at }: { at: number }) {
  const lang = useLang();
  return (
    <time dateTime={new Date(at * 1000).toISOString()} title={when(lang, at)}>
      {ago(lang, at)}
    </time>
  );
}

/** A reporter as two words and four hex digits; the pseudonym itself in the title, and a filter when `onPick` is given. */
export function Reporter({ pseudonym, onPick }: { pseudonym: string; onPick?: () => void }) {
  const lang = useLang();
  const a = alias(pseudonym);
  const body = (
    <>
      <span className="fb-alias">{a.name}</span> <code className="fb-short">{a.short}</code>
    </>
  );
  if (onPick === undefined) {
    return (
      <span className="fb-reporter" title={pseudonym} translate="no">
        {body}
      </span>
    );
  }
  return (
    <button type="button" className="fb-reporter fb-linkish" title={t(lang, "fb.onlyBy", { who: a.name })} translate="no" onClick={onPick}>
      {body}
    </button>
  );
}

/** A report's labels as small chips, each a filter when `onPick` is given. */
export function Labels({ labels, onPick }: { labels: readonly string[]; onPick?: (label: string) => void }) {
  const lang = useLang();
  if (labels.length === 0) return null;
  return (
    <span className="fb-labels">
      {labels.map((label) =>
        onPick === undefined ? (
          <span key={label} className="fb-label" translate="no">
            {label}
          </span>
        ) : (
          <button key={label} type="button" className="fb-label fb-linkish" translate="no" title={t(lang, "fb.onlyLabel", { label })} onClick={() => onPick(label)}>
            {label}
          </button>
        ),
      )}
    </span>
  );
}

// ---------------------------------------------------------------- administrators

export interface Admins {
  list: ReportAdmin[];
  /** An administrator by their handle; a former one by the end of their id. */
  name(id: Uint8Array | null): string;
}

/** The administrators a report may be assigned to, read once; without them, ids are shown short. */
export function useAdmins(api: ReportsApi): Admins {
  const [list, setList] = useState<ReportAdmin[]>([]);
  useEffect(() => {
    let live = true;
    api.admins().then(
      (admins) => {
        if (live) setList(admins);
      },
      // assigning offers the reading administrator alone, and ids are shown short
      () => undefined,
    );
    return () => {
      live = false;
    };
  }, [api]);
  return useMemo(() => {
    const byId = new Map(list.map((a) => [idText(a.id), a] as const));
    return {
      list,
      name(id) {
        if (id === null) return "—";
        const a = byId.get(idText(id));
        return a !== undefined && a.displayName !== null && a.tag !== null ? who(a) : `…${shortId(id)}`;
      },
    };
  }, [list]);
}

// ---------------------------------------------------------------- notices

export interface Notice {
  id: number;
  text: string;
  bad: boolean;
  undo?: () => void;
}

interface Notices {
  /** Says what happened; `undo` puts a button beside it for a while. */
  say(text: string, opts?: { bad?: boolean; undo?: () => void }): void;
}

const NoticeContext = createContext<Notices>({ say: () => undefined });

export const useNotices = (): Notices => useContext(NoticeContext);

/** How long a notice stays, and one with "Undo". */
const NOTICE_MS = 5_000;
const UNDO_MS = 10_000;

/** The section's notices: a live region at the foot of the screen, at most three at once. */
export function NoticeProvider({ children }: { children: ReactNode }) {
  const lang = useLang();
  const [notices, setNotices] = useState<Notice[]>([]);
  // the live words: the newest notice's text, alone, so its buttons are not read out with it
  const [said, setSaid] = useState("");
  const next = useRef(1);
  const timers = useRef(new Map<number, number>());
  const drop = useCallback((id: number) => {
    window.clearTimeout(timers.current.get(id));
    timers.current.delete(id);
    setNotices((now) => now.filter((n) => n.id !== id));
  }, []);
  useEffect(() => {
    const all = timers.current;
    return () => {
      for (const timer of all.values()) window.clearTimeout(timer);
    };
  }, []);
  const value = useMemo<Notices>(
    () => ({
      say(text, opts = {}) {
        const id = next.current++;
        const notice: Notice = { id, text, bad: opts.bad === true, ...(opts.undo === undefined ? {} : { undo: opts.undo }) };
        setNotices((now) => [...now, notice].slice(-3));
        // a different string each time, so the same words twice are announced twice
        setSaid((now) => (now === text ? `${text} ` : text));
        // a failure stays until it is dismissed
        if (!notice.bad) timers.current.set(id, window.setTimeout(() => drop(id), notice.undo === undefined ? NOTICE_MS : UNDO_MS));
      },
    }),
    [drop],
  );
  return (
    <NoticeContext value={value}>
      {children}
      <output className="vh">{said}</output>
      <div className="fb-notices">
        {notices.map((n) => (
          <div key={n.id} className={`fb-notice${n.bad ? " bad" : ""}`}>
            <span>{n.text}</span>
            {n.undo !== undefined && (
              <button
                type="button"
                className="button button-ghost button-sm"
                onClick={() => {
                  drop(n.id);
                  n.undo?.();
                }}
              >
                {t(lang, "fb.undo")}
              </button>
            )}
            <button type="button" className="round-btn fb-notice-close" onClick={() => drop(n.id)}>
              <Icon name="close" />
              <span className="vh">{t(lang, "fb.dismiss")}</span>
            </button>
          </div>
        ))}
      </div>
    </NoticeContext>
  );
}
