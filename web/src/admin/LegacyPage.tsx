// `/admin/legacy`: what is left of the legacy store (docs/native-login.md
// §"Legacy login"): the deadline after which the sweep deletes every row,
// and each account of the official gateway that has not moved to Central
// yet, with how far its move got and what holds it back (a lockout, a ban,
// a password the gateway wanted changed). Names, times and counters only:
// Central never answers a hash or a snapshot. A Central without a legacy
// store refuses the read, and the page says there is none.

import { useCallback, useEffect, useState } from "react";

import { ApiError } from "../account/api";
import { Panel } from "../app/AppFrame";
import { day, when } from "../app/format";
import { T, t, useLang, useTitle, type Key } from "../i18n";
import type { AdminApi, LegacyAccount, LegacyProgress, LegacyStore } from "./admin-api";
import { AdminFrame, useAdminApi } from "./shared";

const PROGRESS: Readonly<Record<LegacyProgress, { word: Key; tone: string }>> = {
  unstarted: { word: "leg.unstarted", tone: "off" },
  reserved: { word: "leg.reserved", tone: "warn" },
  enrolled: { word: "leg.enrolled", tone: "warn" },
  restored: { word: "leg.restored", tone: "warn" },
};

/** What holds an account's move back now, in the store's own order: a ban, a forced password, a lockout. */
function held(a: LegacyAccount, now: number): { word: Key; until?: number }[] {
  const out: { word: Key; until?: number }[] = [];
  if (a.banned) out.push({ word: "leg.banned" });
  if (a.mustChangePassword) out.push({ word: "leg.mustChange" });
  if (a.lockedUntil !== null && a.lockedUntil > now) out.push({ word: "leg.locked", until: a.lockedUntil });
  return out;
}

function Accounts({ store, now }: { store: LegacyStore; now: number }) {
  const lang = useLang();
  if (store.accounts.length === 0) return <p className="small"><T id="leg.none" /></p>;
  return (
    <div className="table-wrap">
      <table className="table">
        <thead>
          <tr>
            <th scope="col"><T id="leg.account" /></th>
            <th scope="col"><T id="leg.progress" /></th>
            <th scope="col"><T id="leg.held" /></th>
            <th scope="col"><T id="leg.failures" /></th>
            <th scope="col"><T id="leg.imported" /></th>
          </tr>
        </thead>
        <tbody>
          {store.accounts.map((a) => {
            const progress = PROGRESS[a.progress];
            const holds = held(a, now);
            return (
              <tr key={a.legacyId}>
                <th scope="row">
                  <code translate="no">{a.username}</code>
                  {a.displayName !== null && (
                    <>
                      {" "}
                      <bdi translate="no">{a.tag === null ? a.displayName : `${a.displayName}#${a.tag.toString(16)}`}</bdi>
                    </>
                  )}
                </th>
                <td><span className={`pill ${progress.tone}`}>{t(lang, progress.word)}</span></td>
                <td>
                  {holds.length === 0
                    ? "—"
                    : holds.map((h) => (
                        <span key={h.word} className="pill bad">
                          {h.until === undefined ? t(lang, h.word) : t(lang, h.word, { until: when(lang, h.until) })}
                        </span>
                      ))}
                </td>
                <td className="num">{a.failures}</td>
                <td className="num">{day(lang, a.importedAt)}</td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}

/** A read's outcome; `now` is when it came, against which a lockout is shown. */
type Load = { at: "loading" } | { at: "ready"; store: LegacyStore; now: number } | { at: "none" } | { at: "failed" };

function Store({ api }: { api: AdminApi }) {
  const lang = useLang();
  const [load, setLoad] = useState<Load>({ at: "loading" });
  const read = useCallback(() => {
    api.legacyAccounts().then(
      (store) => setLoad({ at: "ready", store, now: Date.now() / 1000 }),
      // a Central without a legacy store refuses the read
      (e: unknown) => setLoad(e instanceof ApiError && e.kind === "rejected" ? { at: "none" } : { at: "failed" }),
    );
  }, [api]);
  useEffect(read, [read]);
  return (
    <Panel title={<T id="leg.title" />}>
      {load.at === "loading" && <div className="skeleton app-skeleton" aria-hidden="true" />}
      {load.at === "failed" && (
        <p className="small" role="alert">
          <T id="acct.errUnreachable" />{" "}
          <button
            type="button"
            className="button button-ghost button-sm"
            onClick={() => {
              setLoad({ at: "loading" });
              read();
            }}
          >
            <T id="leg.retry" />
          </button>
        </p>
      )}
      {load.at === "none" && <p className="small"><T id="leg.noStore" /></p>}
      {load.at === "ready" && (
        <>
          <p className="small">
            {load.store.deadline === null
              ? t(lang, "leg.noDeadline")
              : t(lang, "leg.deadline", { day: day(lang, load.store.deadline), count: load.store.accounts.length })}
          </p>
          <Accounts store={load.store} now={load.now} />
        </>
      )}
    </Panel>
  );
}

export default function LegacyPage({ client }: { client?: AdminApi }) {
  useTitle("title.adminLegacy");
  const api = useAdminApi(client);
  return (
    <AdminFrame page="adminLegacy" title={<T id="leg.pageTitle" />} lead={<T id="leg.lead" />}>
      {() => <Store api={api} />}
    </AdminFrame>
  );
}
