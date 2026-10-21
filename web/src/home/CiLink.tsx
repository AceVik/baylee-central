import { T } from "../i18n";
import type { CiRun } from "../github";

/** The latest CI run on main, as a dot and a word, linked to the run. */
export function CiLink({ run }: { run: CiRun }) {
  const ok = run.conclusion === "success";
  const cls = ok ? "ok" : run.conclusion === "failure" ? "bad" : "warn";
  return (
    <a href={run.url} rel="noopener">
      <span className={`dot ${cls}`} aria-hidden="true" />
      {ok ? (
        <T id="ci.passing" />
      ) : run.conclusion === "failure" ? (
        <T id="ci.failing" />
      ) : (
        run.conclusion
      )}
    </a>
  );
}
