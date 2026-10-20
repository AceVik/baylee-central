// The strip under the hero: the latest build, CI on main, the last commit,
// and the way to the gateway directory, read from GitHub's API.

import { GITHUB } from "../config";
import { dateText, relTime, type CiRun, type Commit } from "../github";
import { T, tt, useLang } from "../i18n";
import { useRelease } from "../download/useRelease";
import { Link, PATHS } from "../router";
import { CiLink } from "./CiLink";

export function StatusStrip({ ci, commit }: { ci: CiRun | null | undefined; commit: Commit | null | undefined }) {
  const lang = useLang();
  const { release } = useRelease();
  return (
    <section className="strip" aria-label={tt(lang, "Project status", "Projektstand")}>
      <dl className="strip-inner">
        <div className="stat">
          <dt>
            <T en="Latest build" de="Neueste Version" />
          </dt>
          <dd>
            {release ? (
              <>
                <a href={release.html_url} rel="noopener">
                  {release.tag_name}
                </a>
                <span className="small"> · {dateText(release.published_at, lang)}</span>
              </>
            ) : (
              <a href={`${GITHUB}/releases`} rel="noopener">
                <T en="see releases" de="siehe Releases" />
              </a>
            )}
          </dd>
        </div>
        <div className="stat">
          <dt>
            <T en="CI on main" de="CI auf main" />
          </dt>
          <dd>
            {ci ? (
              <CiLink run={ci} />
            ) : (
              <a href={`${GITHUB}/actions/workflows/ci.yml`} rel="noopener">
                <T en="see on GitHub" de="auf GitHub ansehen" />
              </a>
            )}
          </dd>
        </div>
        <div className="stat">
          <dt>
            <T en="Last commit" de="Letzter Commit" />
          </dt>
          <dd>
            {commit ? (
              <>
                <a href={commit.url} rel="noopener">
                  {commit.sha.slice(0, 7)}
                </a>
                <span className="small"> · {relTime(commit.when, lang)}</span>
              </>
            ) : (
              <a href={`${GITHUB}/commits/main`} rel="noopener">
                <T en="see on GitHub" de="auf GitHub ansehen" />
              </a>
            )}
          </dd>
        </div>
        <div className="stat">
          <dt>
            <T en="Gateways" de="Gateways" />
          </dt>
          <dd>
            <Link to={PATHS.gateways}>
              <T en="the directory" de="das Verzeichnis" />
            </Link>
          </dd>
        </div>
      </dl>
    </section>
  );
}
