// The last 30 days of reports (central-api `ReportStats`): a column a day,
// the crashes stacked on the other reports, a legend in words with the
// totals, each day's numbers on hover and in a table. Read only when it is
// unfolded, so the list never waits for it.

import { useId, useState } from "react";

import { Failed } from "../../app/AppFrame";
import { localeOf, num, t, useLang, type Lang } from "../../i18n";
import { BOUNDS, type DayCount, type ReportsApi } from "./api";

interface Day {
  day: string;
  crashes: number;
  others: number;
}

/** The last `n` UTC days up to today's, each with its counts (none for a day Central did not name). */
export function lastDays(counts: readonly DayCount[], n: number = BOUNDS.statsDays, today = new Date()): Day[] {
  const end = Date.UTC(today.getUTCFullYear(), today.getUTCMonth(), today.getUTCDate());
  return Array.from({ length: n }, (_, i) => {
    const day = new Date(end - (n - 1 - i) * 86_400_000).toISOString().slice(0, 10);
    const of = counts.filter((c) => c.day === day);
    const crashes = of.filter((c) => c.kind === "crash").reduce((s, c) => s + c.count, 0);
    return { day, crashes, others: of.reduce((s, c) => s + c.count, 0) - crashes };
  });
}

/** A UTC day (`YYYY-MM-DD`) as the language writes a date: the day Central counted, wherever the reader is. */
function utcDay(lang: Lang, day: string): string {
  const at = Date.parse(day);
  return Number.isNaN(at) ? "" : new Date(at).toLocaleDateString(localeOf(lang), { dateStyle: "medium", timeZone: "UTC" });
}

const W = 10;
const BAR = 7;
const H = 100;

function Columns({ days }: { days: Day[] }) {
  const lang = useLang();
  const max = Math.max(1, ...days.map((d) => d.crashes + d.others));
  const scale = (n: number) => (n / max) * (H - 2);
  return (
    <div className="fb-chart-plot">
      <span className="fb-chart-max small num" aria-hidden="true">
        {num(lang, max)}
      </span>
      <svg viewBox={`0 0 ${days.length * W} ${H}`} preserveAspectRatio="none" className="fb-chart-svg" aria-hidden="true" focusable="false">
        <line x1="0" x2={days.length * W} y1={H - 0.5} y2={H - 0.5} className="fb-chart-base" />
        {days.map((d, i) => {
          const others = scale(d.others);
          const crashes = scale(d.crashes);
          const x = i * W + (W - BAR) / 2;
          return (
            <g key={d.day} className="fb-chart-day">
              <title>{t(lang, "fb.chartDay", { day: utcDay(lang, d.day), n: d.others + d.crashes, crashes: d.crashes })}</title>
              <rect x={i * W} y="0" width={W} height={H} className="fb-chart-hit" />
              {d.others > 0 && <rect x={x} y={H - others} width={BAR} height={others} className="fb-chart-reports" />}
              {/* a gap of the surface between the two, so they part without colour too */}
              {d.crashes > 0 && <rect x={x} y={H - others - crashes} width={BAR} height={Math.max(crashes - (d.others > 0 ? 1 : 0), 0.5)} className="fb-chart-crashes" />}
            </g>
          );
        })}
      </svg>
      <span className="fb-chart-axis small" aria-hidden="true">
        <span>{utcDay(lang, days[0]?.day ?? "")}</span>
        <span>{utcDay(lang, days.at(-1)?.day ?? "")}</span>
      </span>
    </div>
  );
}

export function Chart({ api }: { api: ReportsApi }) {
  const lang = useLang();
  const table = useId();
  const [state, setState] = useState<{ at: "idle" } | { at: "loading" } | { at: "failed" } | { at: "ready"; days: Day[] }>({ at: "idle" });
  const read = () => {
    setState({ at: "loading" });
    api.stats().then(
      (counts) => setState({ at: "ready", days: lastDays(counts) }),
      () => setState({ at: "failed" }),
    );
  };
  const total = (k: "crashes" | "others") => (state.at === "ready" ? state.days.reduce((s, d) => s + d[k], 0) : 0);
  return (
    <details
      className="fb-more fb-chart"
      onToggle={(e) => {
        if (e.currentTarget.open && state.at === "idle") read();
      }}
    >
      <summary>{t(lang, "fb.chart")}</summary>
      {state.at === "loading" && <div className="skeleton fb-chart-skeleton" aria-busy="true" />}
      {state.at === "failed" && <Failed reason="fb.errChart" onRetry={read} />}
      {state.at === "ready" && (
        <figure className="fb-chart-figure" aria-describedby={table}>
          <figcaption className="fb-chart-legend">
            <span>
              <span className="fb-swatch fb-chart-reports" aria-hidden="true" /> {t(lang, "fb.chartOthers", { n: total("others") })}
            </span>
            <span>
              <span className="fb-swatch fb-chart-crashes" aria-hidden="true" /> {t(lang, "fb.chartCrashes", { n: total("crashes") })}
            </span>
          </figcaption>
          <Columns days={state.days} />
          <details className="fb-more" id={table}>
            <summary>{t(lang, "fb.chartTable")}</summary>
            <table className="table">
              <thead>
                <tr>
                  <th scope="col">{t(lang, "fb.chartDayHead")}</th>
                  <th scope="col">{t(lang, "fb.chartOthersHead")}</th>
                  <th scope="col">{t(lang, "fb.chartCrashesHead")}</th>
                </tr>
              </thead>
              <tbody>
                {state.days
                  .filter((d) => d.crashes + d.others > 0)
                  .map((d) => (
                    <tr key={d.day}>
                      <th scope="row" className="num">
                        {utcDay(lang, d.day)}
                      </th>
                      <td className="num">{num(lang, d.others)}</td>
                      <td className="num">{num(lang, d.crashes)}</td>
                    </tr>
                  ))}
              </tbody>
            </table>
          </details>
        </figure>
      )}
    </details>
  );
}
