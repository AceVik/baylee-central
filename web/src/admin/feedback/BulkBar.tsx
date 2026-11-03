// What the list does to the reports selected in it (central-api
// `ReportsBulk`): one request for all of them, so every one changes or none
// does. In the trash: restore, or delete for good after a second step and a
// fresh password. Besides: read or unread, their ids, a CSV of them.

import { useId, useState, type ReactNode } from "react";

import { idOf, idText } from "../../account/owner-api";
import { Icon } from "../../components/icons";
import { t, useLang } from "../../i18n";
import { PRIORITIES, STATUSES, validLabel, type BulkAction, type ReportChange, type ReportSummary } from "./api";
import { firstLine } from "./github";
import { PRIORITY_WORD, STATUS_WORD, shortId, type Admins } from "./pieces";

/** A menu that acts as soon as a choice is made, then shows its question again. */
function Select({ id, word, busy, children, pick }: { id: string; word: string; busy: boolean; children: ReactNode; pick(v: string): void }) {
  return (
    <span className="fb-bulk-field">
      <label className="vh" htmlFor={id}>
        {word}
      </label>
      <select id={id} className="fb-select" value="" aria-disabled={busy ? true : undefined} onChange={(e) => !busy && e.target.value !== "" && pick(e.target.value)}>
        <option value="">{word}…</option>
        {children}
      </select>
    </span>
  );
}

export function BulkBar({
  selected,
  admins,
  me,
  labels,
  inTrash,
  busy,
  onBulk,
  onPurge,
  onSeen,
  onCopy,
  onExport,
  onClear,
}: {
  selected: readonly ReportSummary[];
  admins: Admins;
  me: string;
  /** The labels in use, offered. */
  labels: readonly string[];
  inTrash: boolean;
  busy: boolean;
  /** `ids`: when not the whole selection (a merge leaves its original out). */
  onBulk(action: BulkAction, said: string, ids?: readonly Uint8Array[]): void;
  onPurge(): void;
  onSeen(seen: boolean): void;
  onCopy(): void;
  onExport(): void;
  onClear(): void;
}) {
  const lang = useLang();
  const f = useId();
  const [label, setLabel] = useState("");
  const n = selected.length;
  const change = (c: ReportChange, said: string, ids?: readonly Uint8Array[]) => onBulk({ do: "change", change: c }, said, ids);
  const theirs = [...new Set(selected.flatMap((r) => r.labels))].sort();
  const people = admins.list.map((a) => idText(a.id));
  if (!people.includes(me)) people.unshift(me);
  const addLabel = () => {
    const l = label.trim().toLowerCase();
    if (!validLabel(l)) return;
    setLabel("");
    change({ labelsAdd: [l] }, t(lang, "fb.bulkLabelled", { label: l, n }));
  };
  return (
    <section className="fb-bulk" aria-label={t(lang, "fb.selection")}>
      <p className="fb-bulk-count">
        <strong>{t(lang, "fb.selected", { n })}</strong>
        <button type="button" className="button button-ghost button-sm" onClick={onClear}>
          {t(lang, "fb.deselect")}
        </button>
      </p>
      <div className="fb-bulk-actions">
        {inTrash ? (
          <>
            <button type="button" className="button button-ghost button-sm" aria-disabled={busy ? true : undefined} onClick={() => !busy && onBulk({ do: "restore" }, t(lang, "fb.bulkRestored", { n }))}>
              {t(lang, "fb.restore")}
            </button>
            <button type="button" className="button button-danger button-sm" aria-disabled={busy ? true : undefined} onClick={() => !busy && onPurge()}>
              {t(lang, "fb.purge")}
            </button>
          </>
        ) : (
          <>
            <Select busy={busy} id={`${f}-status`} word={t(lang, "fb.setStatus")} pick={(v) => {
              const s = STATUSES.find((x) => x === v);
              if (s) change({ status: s }, t(lang, "fb.bulkStatus", { status: t(lang, STATUS_WORD[s]), n }));
            }}>
              {STATUSES.filter((s) => s !== "duplicate").map((s) => (
                <option key={s} value={s}>
                  {t(lang, STATUS_WORD[s])}
                </option>
              ))}
            </Select>
            <Select busy={busy} id={`${f}-priority`} word={t(lang, "fb.setPriority")} pick={(v) => {
              const p = v === "none" ? null : (PRIORITIES.find((x) => x === v) ?? undefined);
              if (p !== undefined) change({ priority: p }, t(lang, "fb.bulkPriority", { priority: t(lang, PRIORITY_WORD[p ?? "none"]), n }));
            }}>
              {[...PRIORITIES, "none" as const].map((p) => (
                <option key={p} value={p}>
                  {t(lang, PRIORITY_WORD[p])}
                </option>
              ))}
            </Select>
            <Select busy={busy} id={`${f}-assign`} word={t(lang, "fb.assignTo")} pick={(v) => {
              const whom = v === "none" ? null : idOf(v);
              if (v === "none" || whom !== null) change({ assignee: whom }, whom === null ? t(lang, "fb.bulkUnassigned", { n }) : t(lang, "fb.bulkAssigned", { name: v === me ? t(lang, "fb.me") : admins.name(whom), n }));
            }}>
              {people.map((hex) => (
                <option key={hex} value={hex}>
                  {hex === me ? t(lang, "fb.me") : admins.name(admins.list.find((a) => idText(a.id) === hex)?.id ?? null)}
                </option>
              ))}
              <option value="none">{t(lang, "fb.nobody")}</option>
            </Select>
            <span className="fb-bulk-field fb-inline">
              <label className="vh" htmlFor={`${f}-label`}>
                {t(lang, "fb.addLabel")}
              </label>
              <input
                id={`${f}-label`}
                list={`${f}-labels`}
                className="fb-input"
                placeholder={t(lang, "fb.addLabel")}
                value={label}
                maxLength={32}
                spellCheck={false}
                onChange={(e) => setLabel(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") {
                    e.preventDefault();
                    addLabel();
                  }
                }}
              />
              <datalist id={`${f}-labels`}>
                {labels.map((l) => (
                  <option key={l} value={l}>
                    {l}
                  </option>
                ))}
              </datalist>
              <button type="button" className="round-btn" disabled={busy || !validLabel(label.trim().toLowerCase())} onClick={addLabel}>
                <Icon name="plus" />
                <span className="vh">{t(lang, "fb.addLabel")}</span>
              </button>
            </span>
            {theirs.length > 0 && (
              <Select busy={busy} id={`${f}-unlabel`} word={t(lang, "fb.removeLabelAll")} pick={(l) => change({ labelsRemove: [l] }, t(lang, "fb.bulkUnlabelled", { label: l, n }))}>
                {theirs.map((l) => (
                  <option key={l} value={l}>
                    {l}
                  </option>
                ))}
              </Select>
            )}
            {n > 1 && (
              <Select busy={busy} id={`${f}-merge`} word={t(lang, "fb.mergeInto")} pick={(hex) => {
                const original = selected.find((r) => idText(r.id) === hex);
                if (original === undefined) return;
                const rest = selected.filter((r) => r !== original).map((r) => r.id);
                change({ duplicateOf: original.id }, t(lang, "fb.bulkMerged", { n: rest.length, id: shortId(original.id) }), rest);
              }}>
                {selected
                  .filter((r) => r.duplicateOf === null)
                  .map((r) => (
                    <option key={idText(r.id)} value={idText(r.id)}>
                      …{shortId(r.id)} · {firstLine(r.excerpt, 40) || t(lang, "fb.noText")}
                    </option>
                  ))}
              </Select>
            )}
            <button type="button" className="button button-danger button-sm" aria-disabled={busy ? true : undefined} onClick={() => !busy && onBulk({ do: "trash" }, t(lang, "fb.bulkTrashed", { n }))}>
              <Icon name="trash" className="button-icon" />
              {t(lang, "fb.toTrash")}
            </button>
          </>
        )}
        <button type="button" className="button button-ghost button-sm" onClick={() => onSeen(true)}>
          {t(lang, "fb.markRead")}
        </button>
        <button type="button" className="button button-ghost button-sm" onClick={() => onSeen(false)}>
          {t(lang, "fb.markUnread")}
        </button>
        <button type="button" className="button button-ghost button-sm" onClick={onCopy}>
          {t(lang, "fb.copyIds")}
        </button>
        <button type="button" className="button button-ghost button-sm" onClick={onExport}>
          {t(lang, "fb.exportSelected", { n })}
        </button>
      </div>
    </section>
  );
}

