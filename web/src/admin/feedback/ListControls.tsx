// What steers the list of reports (`FeedbackPage`): the views with their
// counts, the search, every filter with its count under the rest of the
// filter (central-api `ReportFacets`), the active ones as pills that take
// themselves out, the sortable column heads and the pages. Each change is a
// new address (`query.ts`), so the list is a link.

import { useEffect, useId, useRef, useState, type ReactNode } from "react";

import { idText } from "../../account/owner-api";
import { Icon } from "../../components/icons";
import { num, t, useLang, type Key } from "../../i18n";
import { KINDS, PRIORITIES, SORTS, STATUSES, type Count, type ReportFacets, type ReportSort } from "./api";
import { KIND_WORD, PRIORITY_WORD, STATUS_WORD, shortId, type Admins } from "./pieces";
import { PAGE_SIZE, PAGE_SIZES, VIEW_PARTS, VIEWS, changed, cleanQuery, limitOf, startsAscending, type ListQuery, type ViewId } from "./query";

export type Change = { [K in keyof ListQuery]?: ListQuery[K] | undefined };

const VIEW_WORD: Record<ViewId, Key> = {
  inbox: "fb.viewInbox",
  open: "fb.viewOpen",
  mine: "fb.viewMine",
  unassigned: "fb.viewUnassigned",
  done: "fb.viewDone",
  all: "fb.viewAll",
  trash: "fb.viewTrash",
};

export const SORT_WORD: Record<ReportSort, Key> = {
  received: "fb.sortReceived",
  changed: "fb.sortChanged",
  priority: "fb.sortPriority",
  status: "fb.sortStatus",
  kind: "fb.sortKind",
  reporter: "fb.sortReporter",
  gateway: "fb.sortGateway",
};

/** The views, each with its count when Central gave one. */
export function Views({ view, counts, me, onPick }: { view: ViewId | null; counts: Partial<Record<ViewId, number>>; me: string; onPick(q: ListQuery): void }) {
  const lang = useLang();
  return (
    <fieldset className="fb-views">
      <legend className="vh">{t(lang, "fb.views")}</legend>
      {VIEWS.map((v) => {
        const n = counts[v.id];
        return (
          <button key={v.id} type="button" className="chip-btn" aria-pressed={view === v.id} onClick={() => onPick(v.query(me))}>
            {t(lang, VIEW_WORD[v.id])}
            {n !== undefined && " "}
            {n !== undefined && <span className="chip-n num">{num(lang, n)}</span>}
          </button>
        );
      })}
    </fieldset>
  );
}

/** How long typing pauses before the list follows. */
const PAUSE_MS = 300;

/** The search: the list follows it once typing pauses; Escape empties it, Ctrl/Cmd+K reaches it. */
export function SearchBox({ asked, onAsk }: { asked: string | undefined; onAsk(q: string | undefined): void }) {
  const lang = useLang();
  const id = useId();
  const [text, setText] = useState(asked ?? "");
  // another list (a view, a pill taken out) brings its own search
  const [was, setWas] = useState(asked);
  if (was !== asked) {
    setWas(asked);
    if ((cleanQuery(text) ?? undefined) !== asked) setText(asked ?? "");
  }
  const latest = useRef(onAsk);
  useEffect(() => {
    latest.current = onAsk;
  });
  useEffect(() => {
    const q = cleanQuery(text);
    if (q === asked) return;
    const pause = window.setTimeout(() => latest.current(q), PAUSE_MS);
    return () => window.clearTimeout(pause);
  }, [text, asked]);
  return (
    <form
      className="search-row fb-search"
      onSubmit={(e) => {
        e.preventDefault();
        onAsk(cleanQuery(text));
      }}
    >
      <label className="vh" htmlFor={id}>
        {t(lang, "fb.search")}
      </label>
      <input
        id={id}
        type="search"
        data-find=""
        aria-keyshortcuts="Control+K Meta+K"
        placeholder={t(lang, "fb.searchHint")}
        value={text}
        maxLength={400}
        onChange={(e) => setText(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Escape" && text !== "") {
            e.preventDefault();
            e.stopPropagation();
            setText("");
          }
        }}
      />
    </form>
  );
}

const countOf = <T,>(counts: readonly Count<T>[] | undefined, match: (v: T) => boolean): number | undefined => counts?.find((c) => match(c.value))?.count;

/** `values` with `value` in or out, in `order`; none left is no filter. */
export function toggled<T extends string>(values: readonly T[] | undefined, value: T, order: readonly T[]): T[] | undefined {
  const now = new Set(values ?? []);
  if (now.has(value)) now.delete(value);
  else now.add(value);
  const next = order.filter((v) => now.has(v));
  return next.length > 0 ? next : undefined;
}

function Chips<T extends string>({
  legend,
  values,
  on,
  word,
  count,
  onToggle,
}: {
  legend: string;
  values: readonly T[];
  on: readonly T[] | undefined;
  word(v: T): string;
  count(v: T): number | undefined;
  onToggle(v: T): void;
}) {
  const lang = useLang();
  return (
    <fieldset className="fb-chips">
      <legend>{legend}</legend>
      <div className="chips">
        {values.map((v) => {
          const n = count(v);
          return (
            <button key={v} type="button" className="chip-btn" aria-pressed={on?.includes(v) === true} onClick={() => onToggle(v)}>
              {word(v)}
              {n !== undefined && " "}
              {n !== undefined && <span className="chip-n num">{num(lang, n)}</span>}
            </button>
          );
        })}
      </div>
    </fieldset>
  );
}

function Field({ label, children, id }: { label: string; children: ReactNode; id: string }) {
  return (
    <div className="field fb-field">
      <label htmlFor={id}>{label}</label>
      {children}
    </div>
  );
}

const ALL_PRIORITIES = [...PRIORITIES, "none"] as const;

/** Every filter Central has, with its counts; a list of options grows past a value the facets no longer name, so a chosen one stays visible. */
export function FilterPanel({ query, facets, admins, me, set, id }: { query: ListQuery; facets: ReportFacets | null; admins: Admins; me: string; set(c: Change): void; id: string }) {
  const lang = useLang();
  const f = useId();
  const labels = facets?.labels ?? [];
  const gateways = facets?.gateways ?? [];
  const assigneeCount = (hex: string | null) => countOf(facets?.assignees, (v) => (v === null ? hex === null : idText(v) === hex));
  const counted = (n: number | undefined) => (n === undefined ? "" : ` (${num(lang, n)})`);
  const people = admins.list.map((a) => idText(a.id));
  if (!people.includes(me)) people.unshift(me);
  return (
    <div id={id} className="fb-filters">
      <div className="fb-chip-groups">
        <Chips
          legend={t(lang, "fb.status")}
          values={STATUSES}
          on={query.statuses}
          word={(s) => t(lang, STATUS_WORD[s])}
          count={(s) => countOf(facets?.statuses, (v) => v === s)}
          onToggle={(s) => set({ statuses: toggled(query.statuses, s, STATUSES) })}
        />
        <Chips
          legend={t(lang, "fb.kind")}
          values={KINDS}
          on={query.kinds}
          word={(k) => t(lang, KIND_WORD[k])}
          count={(k) => countOf(facets?.kinds, (v) => v === k)}
          onToggle={(k) => set({ kinds: toggled(query.kinds, k, KINDS) })}
        />
        <Chips
          legend={t(lang, "fb.priority")}
          values={ALL_PRIORITIES}
          on={query.priorities}
          word={(p) => t(lang, PRIORITY_WORD[p])}
          count={(p) => countOf(facets?.priorities, (v) => (v ?? "none") === p)}
          onToggle={(p) => set({ priorities: toggled(query.priorities, p, ALL_PRIORITIES) })}
        />
      </div>
      <div className="fb-fields">
        <Field label={t(lang, "fb.assignee")} id={`${f}-assignee`}>
          <select id={`${f}-assignee`} value={query.assignee ?? ""} onChange={(e) => set({ assignee: e.target.value === "" ? undefined : e.target.value })}>
            <option value="">{t(lang, "fb.anyone")}</option>
            <option value="none">
              {t(lang, "fb.nobody")}
              {counted(assigneeCount(null))}
            </option>
            {people.map((hex) => (
              <option key={hex} value={hex}>
                {hex === me ? t(lang, "fb.me") : admins.name(admins.list.find((a) => idText(a.id) === hex)?.id ?? null)}
                {counted(assigneeCount(hex))}
              </option>
            ))}
            {query.assignee !== undefined && query.assignee !== "none" && !people.includes(query.assignee) && <option value={query.assignee}>…{shortId(query.assignee)}</option>}
          </select>
        </Field>
        <Field label={t(lang, "fb.label")} id={`${f}-label`}>
          <select id={`${f}-label`} value={query.label ?? ""} onChange={(e) => set({ label: e.target.value === "" ? undefined : e.target.value })}>
            <option value="">{t(lang, "fb.any")}</option>
            {labels.map((c) => (
              <option key={c.value} value={c.value}>
                {c.value}
                {counted(c.count)}
              </option>
            ))}
            {query.label !== undefined && !labels.some((c) => c.value === query.label) && <option value={query.label}>{query.label}</option>}
          </select>
        </Field>
        <Field label={t(lang, "fb.gateway")} id={`${f}-gateway`}>
          <select id={`${f}-gateway`} value={query.gateway ?? ""} onChange={(e) => set({ gateway: e.target.value === "" ? undefined : e.target.value })}>
            <option value="">{t(lang, "fb.any")}</option>
            {gateways.map((c) => (
              <option key={c.value} value={c.value}>
                {c.value}
                {counted(c.count)}
              </option>
            ))}
            {query.gateway !== undefined && !gateways.some((c) => c.value === query.gateway) && <option value={query.gateway}>{query.gateway}</option>}
          </select>
        </Field>
        <Field label={t(lang, "fb.channel")} id={`${f}-channel`}>
          <select
            id={`${f}-channel`}
            value={query.channel ?? ""}
            onChange={(e) => set({ channel: e.target.value === "gateway" || e.target.value === "direct" ? e.target.value : undefined })}
          >
            <option value="">{t(lang, "fb.any")}</option>
            <option value="gateway">
              {t(lang, "fb.channelGateway")}
              {counted(countOf(facets?.channels, (v) => v === "gateway"))}
            </option>
            <option value="direct">
              {t(lang, "fb.channelDirect")}
              {counted(countOf(facets?.channels, (v) => v === "direct"))}
            </option>
          </select>
        </Field>
        <Field label={t(lang, "fb.record")} id={`${f}-record`}>
          <select id={`${f}-record`} value={query.record === undefined ? "" : query.record ? "yes" : "no"} onChange={(e) => set({ record: e.target.value === "" ? undefined : e.target.value === "yes" })}>
            <option value="">{t(lang, "fb.any")}</option>
            <option value="yes">{t(lang, "fb.withRecord")}</option>
            <option value="no">{t(lang, "fb.withoutRecord")}</option>
          </select>
        </Field>
        <Field label={t(lang, "fb.reporter")} id={`${f}-reporter`}>
          <select id={`${f}-reporter`} value={query.reporter ?? ""} onChange={(e) => set({ reporter: e.target.value === "" ? undefined : e.target.value })}>
            <option value="">{t(lang, "fb.anyone")}</option>
            {(facets?.reporters ?? []).map((c) => (
              <option key={c.value} value={c.value}>
                {c.value.slice(0, 12)}
                {counted(c.count)}
              </option>
            ))}
            {query.reporter !== undefined && !(facets?.reporters ?? []).some((c) => c.value === query.reporter) && <option value={query.reporter}>{query.reporter.slice(0, 12)}</option>}
          </select>
        </Field>
        <Field label={t(lang, "fb.from")} id={`${f}-from`}>
          <input id={`${f}-from`} type="date" value={query.from ?? ""} max={query.to} onChange={(e) => set({ from: e.target.value === "" ? undefined : e.target.value })} />
        </Field>
        <Field label={t(lang, "fb.to")} id={`${f}-to`}>
          <input id={`${f}-to`} type="date" value={query.to ?? ""} min={query.from} onChange={(e) => set({ to: e.target.value === "" ? undefined : e.target.value })} />
        </Field>
        <Field label={t(lang, "fb.sort")} id={`${f}-sort`}>
          <select
            id={`${f}-sort`}
            value={`${query.sort ?? "received"}:${query.asc === true ? "asc" : "desc"}`}
            onChange={(e) => {
              const [sort, dir] = e.target.value.split(":");
              const picked = SORTS.find((s) => s === sort) ?? "received";
              set({ sort: picked, asc: dir === "asc" ? true : undefined });
            }}
          >
            {SORTS.flatMap((s) =>
              (startsAscending(s) ? (["asc", "desc"] as const) : (["desc", "asc"] as const)).map((dir) => (
                <option key={`${s}:${dir}`} value={`${s}:${dir}`}>
                  {t(lang, SORT_WORD[s])} · {t(lang, dir === "asc" ? "fb.ascending" : "fb.descending")}
                </option>
              )),
            )}
          </select>
        </Field>
      </div>
      <label className="check-label">
        <input type="checkbox" checked={query.unseen === true} onChange={(e) => set({ unseen: e.target.checked ? true : undefined })} />
        {t(lang, "fb.onlyUnread")}
      </label>
    </div>
  );
}

/** How many filters are set in the panel beyond what the view decides. */
export function panelCount(query: ListQuery, view: ViewId | null): number {
  const parts: (keyof ListQuery)[] = ["kinds", "priorities", "label", "gateway", "channel", "record", "reporter", "from", "to", "unseen", "signature", "duplicateOf"];
  if (view === null) parts.push(...VIEW_PARTS);
  return parts.filter((p) => query[p] !== undefined).length;
}

interface Pill {
  key: string;
  text: string;
  change: Change;
}

/** The active filters beyond the view's own, each as a pill that takes it out. */
export function pillsOf(lang: Parameters<typeof t>[0], query: ListQuery, view: ViewId | null, admins: Admins, me: string): Pill[] {
  const pills: Pill[] = [];
  const owned = new Set<keyof ListQuery>(view === null ? [] : VIEW_PARTS);
  const add = (part: keyof ListQuery, text: string, change: Change) => {
    if (!owned.has(part)) pills.push({ key: `${part}:${text}`, text, change });
  };
  for (const s of query.statuses ?? []) add("statuses", `${t(lang, "fb.status")}: ${t(lang, STATUS_WORD[s])}`, { statuses: toggled(query.statuses, s, STATUSES) });
  for (const k of query.kinds ?? []) add("kinds", `${t(lang, "fb.kind")}: ${t(lang, KIND_WORD[k])}`, { kinds: toggled(query.kinds, k, KINDS) });
  for (const p of query.priorities ?? []) add("priorities", `${t(lang, "fb.priority")}: ${t(lang, PRIORITY_WORD[p])}`, { priorities: toggled(query.priorities, p, ALL_PRIORITIES) });
  if (query.assignee !== undefined) {
    const whom = query.assignee === "none" ? t(lang, "fb.nobody") : query.assignee === me ? t(lang, "fb.me") : admins.name(admins.list.find((a) => idText(a.id) === query.assignee)?.id ?? null);
    add("assignee", `${t(lang, "fb.assignee")}: ${whom}`, { assignee: undefined });
  }
  if (query.label !== undefined) add("label", `${t(lang, "fb.label")}: ${query.label}`, { label: undefined });
  if (query.gateway !== undefined) add("gateway", `${t(lang, "fb.gateway")}: ${query.gateway}`, { gateway: undefined });
  if (query.channel !== undefined) add("channel", t(lang, query.channel === "direct" ? "fb.channelDirect" : "fb.channelGateway"), { channel: undefined });
  if (query.reporter !== undefined) add("reporter", `${t(lang, "fb.reporter")}: ${query.reporter.slice(0, 12)}`, { reporter: undefined });
  if (query.signature !== undefined) add("signature", `${t(lang, "fb.crashGroup")}: ${query.signature.slice(0, 8)}`, { signature: undefined });
  if (query.duplicateOf !== undefined) add("duplicateOf", t(lang, "fb.duplicatesOf", { id: shortId(query.duplicateOf) }), { duplicateOf: undefined });
  if (query.q !== undefined) add("q", `“${query.q}”`, { q: undefined });
  if (query.from !== undefined) add("from", `${t(lang, "fb.from")}: ${query.from}`, { from: undefined });
  if (query.to !== undefined) add("to", `${t(lang, "fb.to")}: ${query.to}`, { to: undefined });
  if (query.record !== undefined) add("record", t(lang, query.record ? "fb.withRecord" : "fb.withoutRecord"), { record: undefined });
  if (query.unseen === true) add("unseen", t(lang, "fb.onlyUnread"), { unseen: undefined });
  if (query.trash === true) add("trash", t(lang, "fb.viewTrash"), { trash: undefined });
  return pills;
}

export function Pills({ pills, set, onClear }: { pills: readonly Pill[]; set(c: Change): void; onClear(): void }) {
  const lang = useLang();
  if (pills.length === 0) return null;
  return (
    <ul className="fb-pills" aria-label={t(lang, "fb.activeFilters")}>
      {pills.map((p) => (
        <li key={p.key}>
          <button type="button" className="chip-btn fb-pill" onClick={() => set(p.change)}>
            <bdi>{p.text}</bdi>
            <Icon name="close" className="fb-pill-x" />
            <span className="vh">{t(lang, "fb.removeFilter")}</span>
          </button>
        </li>
      ))}
      {pills.length > 1 && (
        <li>
          <button type="button" className="button button-ghost button-sm" onClick={onClear}>
            {t(lang, "fb.clearFilters")}
          </button>
        </li>
      )}
    </ul>
  );
}

/** A column head that sorts the list by it, `aria-sort` on the one that does. */
export function SortHead({ sort, query, set, label, className }: { sort: ReportSort; query: ListQuery; set(c: Change): void; label: Key; className?: string }) {
  const lang = useLang();
  const current = (query.sort ?? "received") === sort;
  const asc = query.asc === true;
  return (
    <th scope="col" className={className} aria-sort={current ? (asc ? "ascending" : "descending") : undefined}>
      <button
        type="button"
        className="fb-sort"
        onClick={() => set(current ? { sort, asc: asc ? undefined : true } : { sort, asc: startsAscending(sort) ? true : undefined })}
      >
        {t(lang, label)}
        <span className="fb-sort-mark" aria-hidden="true">
          {current ? (asc ? "↑" : "↓") : ""}
        </span>
        <span className="vh">{t(lang, "fb.sortBy")}</span>
      </button>
    </th>
  );
}

/** The pages of the list and its page size. */
export function Pager({ query, total, go }: { query: ListQuery; total: number; go(q: ListQuery): void }) {
  const lang = useLang();
  const id = useId();
  const size = limitOf(query);
  const page = query.page ?? 1;
  const pages = Math.max(1, Math.ceil(total / size));
  const to = (p: number) => go({ ...query, page: Math.min(Math.max(p, 1), pages) });
  return (
    <nav className="fb-pager" aria-label={t(lang, "fb.pages")}>
      <button type="button" className="round-btn" disabled={page <= 1} onClick={() => to(page - 1)}>
        <Icon name="prev" />
        <span className="vh">{t(lang, "fb.prevPage")}</span>
      </button>
      <span className="small num">{t(lang, "fb.pageOf", { n: page, total: pages })}</span>
      <button type="button" className="round-btn" disabled={page >= pages} onClick={() => to(page + 1)}>
        <Icon name="next" />
        <span className="vh">{t(lang, "fb.nextPage")}</span>
      </button>
      <label className="small" htmlFor={id}>
        {t(lang, "fb.pageSize")}
      </label>
      <select id={id} className="fb-select" value={size} onChange={(e) => go(changed(query, { limit: Number(e.target.value) === PAGE_SIZE ? undefined : Number(e.target.value) }))}>
        {PAGE_SIZES.map((s) => (
          <option key={s} value={s}>
            {num(lang, s)}
          </option>
        ))}
      </select>
    </nav>
  );
}
