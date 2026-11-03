// The crashes the list's filter holds, grouped (central-api `CrashGroups`):
// one row per crash with its numbers taken out, how often it came, how many
// are still open, the first and the last time; a row's button shows its
// reports.

import { useCallback, useEffect, useRef, useState } from "react";

import { ApiError } from "../../account/api";
import { Failed, TableBox } from "../../app/AppFrame";
import { num, t, useLang } from "../../i18n";
import { failureKey } from "../shared";
import type { CrashGroup, ReportsApi } from "./api";
import { Ago } from "./pieces";
import { filterOf, readQuery } from "./query";

type Load = { at: "loading" } | { at: "failed"; kind: string } | { at: "ready"; groups: CrashGroup[] };

/** `scope`: the list's filter as the address writes it, without page and order. */
export function CrashGroups({ api, scope, onPick }: { api: ReportsApi; scope: string; onPick(signature: string): void }) {
  const lang = useLang();
  const [load, setLoad] = useState<{ scope: string; load: Load } | null>(null);
  // one read per filter and per "Try again"; only the newest answer is drawn
  const latest = useRef(0);
  const read = useCallback(() => {
    const mine = ++latest.current;
    api.crashGroups(filterOf(readQuery(scope))).then(
      (groups) => {
        if (mine === latest.current) setLoad({ scope, load: { at: "ready", groups } });
      },
      (e: unknown) => {
        if (mine === latest.current) setLoad({ scope, load: { at: "failed", kind: e instanceof ApiError ? e.kind : "network" } });
      },
    );
  }, [api, scope]);
  useEffect(read, [read]);
  const shown: Load = load?.scope === scope ? load.load : { at: "loading" };
  if (shown.at === "loading") return <div className="skeleton app-skeleton" aria-busy="true" />;
  if (shown.at === "failed") return <Failed reason={failureKey(shown.kind)} onRetry={read} />;
  if (shown.groups.length === 0) return <p className="small fb-empty">{t(lang, "fb.noGroups")}</p>;
  return (
    <TableBox label={t(lang, "fb.groups")}>
      <table className="table fb-table">
        <thead>
          <tr>
            <th scope="col">{t(lang, "fb.colCrash")}</th>
            <th scope="col">{t(lang, "fb.colCount")}</th>
            <th scope="col">{t(lang, "fb.colOpen")}</th>
            <th scope="col">{t(lang, "fb.colFirst")}</th>
            <th scope="col">{t(lang, "fb.colLast")}</th>
            <th scope="col">
              <span className="vh">{t(lang, "fb.showGroup")}</span>
            </th>
          </tr>
        </thead>
        <tbody>
          {shown.groups.map((g) => (
            <tr key={g.signature}>
              <th scope="row" className="fb-cell-report">
                <code className="fb-code">{g.message}</code>
                <span className="fb-marks small">
                  <code>{g.location}</code>
                </span>
              </th>
              <td className="num">{num(lang, g.count)}</td>
              <td className="num">{num(lang, g.open)}</td>
              <td className="num">
                <Ago at={g.first} />
              </td>
              <td className="num">
                <Ago at={g.last} />
              </td>
              <td className="table-actions">
                <button type="button" className="button button-ghost button-sm" onClick={() => onPick(g.signature)}>
                  {t(lang, "fb.showGroup")}
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </TableBox>
  );
}
