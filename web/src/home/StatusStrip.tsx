// The strip under the bar: the latest build, CI on main and the last commit,
// read from GitHub's API, and the gateways Baylee Central lists, online and
// not, read from its public directory. Only what a source says is shown;
// while a source has not answered, the strip links to it instead.

import { GITHUB } from "../config";
import { dateText, relTime, type CiRun, type Commit } from "../github";
import { T, num, t, useLang } from "../i18n";
import { useRelease } from "../download/useRelease";
import { Link, PATHS } from "../router";
import type { GatewayListing } from "../central/directory";
import { CiLink } from "./CiLink";

export function StatusStrip({ ci, commit, gateways }: {
  ci: CiRun | null | undefined;
  commit: Commit | null | undefined;
  /** The directory's first page, or null while it is read or when it could not be. */
  gateways: readonly GatewayListing[] | null;
}) {
  const lang = useLang();
  const { release } = useRelease();
  const online = gateways?.filter((g) => g.online).length ?? 0;
  return (
    <section className="strip" aria-label={t(lang, "status.label")}>
      <div className="strip-inner">
        <dl className="strip-group strip-left">
          <div className="stat">
            <dt>
              <T id="status.build" />
            </dt>
            <dd>
              {release ? (
                <>
                  <a href={release.html_url} target="_blank" rel="noopener">
                    {release.tag_name}
                  </a>
                  <span className="small"> · {dateText(release.published_at, lang)}</span>
                </>
              ) : (
                <a href={`${GITHUB}/releases`} target="_blank" rel="noopener">
                  <T id="status.releases" />
                </a>
              )}
            </dd>
          </div>
          <div className="stat">
            <dt>
              <T id="status.ci" />
            </dt>
            <dd>
              {ci ? (
                <CiLink run={ci} />
              ) : (
                <a href={`${GITHUB}/actions/workflows/ci.yml`} target="_blank" rel="noopener">
                  <T id="status.onGitHub" />
                </a>
              )}
            </dd>
          </div>
          <div className="stat stat-commit">
            <dt>
              <T id="status.commit" />
            </dt>
            <dd>
              {commit ? (
                <>
                  <a href={commit.url} target="_blank" rel="noopener">
                    <code>{commit.sha.slice(0, 7)}</code>
                  </a>
                  <span className="small"> · {relTime(commit.when, lang)}</span>
                </>
              ) : (
                <a href={`${GITHUB}/commits/main`} target="_blank" rel="noopener">
                  <T id="status.onGitHub" />
                </a>
              )}
            </dd>
          </div>
        </dl>
        <dl className="strip-group strip-right">
          <div className="stat">
            <dt>
              <T id="status.gateways" />
            </dt>
            <dd className="num">
              {gateways === null ? (
                <Link to={PATHS.gateways}>
                  <T id="status.directory" />
                </Link>
              ) : (
                <Link to={PATHS.gateways} className="stat-gw-link stat-pill">
                  <span className="stat-part">
                    <span className={`dot ${online > 0 ? "ok" : "off"}`} aria-hidden="true" />
                    {num(lang, online)} <T id="status.online" />
                  </span>
                  <span className="small"> · </span>
                  <span className="stat-part">
                    <span className="dot off" aria-hidden="true" />
                    {num(lang, gateways.length - online)} <T id="status.offline" />
                  </span>
                </Link>
              )}
            </dd>
          </div>
        </dl>
      </div>
    </section>
  );
}
