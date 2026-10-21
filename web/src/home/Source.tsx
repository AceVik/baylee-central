import { DISCORD, GITHUB, ISSUES } from "../config";
import { num, T, useLang, type Elems } from "../i18n";
import { parseRepo, relTime, useGitHub, type CiRun, type Commit } from "../github";
import { Section } from "../components/Section";
import { CiLink } from "./CiLink";

const RULES: Elems = {
  agents: (words) => (
    <a href={`${GITHUB}/blob/main/AGENTS.md`} rel="noopener">
      {words}
    </a>
  ),
  claude: (words) => (
    <a href={`${GITHUB}/blob/main/CLAUDE.md`} rel="noopener">
      {words}
    </a>
  ),
};

export function Source({ ci, commit }: { ci: CiRun | null | undefined; commit: Commit | null | undefined }) {
  const lang = useLang();
  const repo = useGitHub("", "repo", parseRepo);
  return (
    <Section
      id="source"
      alt
      eyebrow={<T id="source.eyebrow" />}
      title={<T id="source.title" />}
    >
      <div className="two-col reveal">
        <div>
          <p className="promise">
            <T id="source.promise" />
          </p>
          <p>
            <T id="source.rust" elems={RULES} />
          </p>
          <p className="links-row">
            <a className="button button-ghost" href={GITHUB} rel="noopener">
              github.com/AceVik/baylee
            </a>
            <a className="button button-ghost" href={DISCORD} rel="noopener">
              <T id="source.discord" />
            </a>
            <a className="button button-ghost" href={`${GITHUB}/blob/main/LICENSE`} rel="noopener">
              AGPL-3.0
            </a>
            <a className="button button-ghost" href={`${GITHUB}/blob/main/docs/architecture.md`} rel="noopener">
              <T id="source.architecture" />
            </a>
          </p>
        </div>
        <div className="repo-card glass">
          <h3>
            <T id="source.repoTitle" />
          </h3>
          <dl className="kv">
            <div>
              <dt>
                <T id="status.ci" />
              </dt>
              <dd>
                {ci ? (
                  <>
                    <CiLink run={ci} />
                    <span className="small">
                      {" "}
                      · {relTime(ci.updatedAt, lang)} · {ci.sha.slice(0, 7)}
                    </span>
                  </>
                ) : (
                  "–"
                )}
              </dd>
            </div>
            <div>
              <dt>
                <T id="status.commit" />
              </dt>
              <dd>
                {commit ? (
                  <>
                    <a href={commit.url} rel="noopener">
                      {commit.sha.slice(0, 7)}
                    </a>{" "}
                    · {relTime(commit.when, lang)} · <span className="small">{commit.subject}</span>
                  </>
                ) : (
                  "–"
                )}
              </dd>
            </div>
            <div>
              <dt>
                <T id="source.stars" />
              </dt>
              <dd>
                {repo ? (
                  <a href={`${GITHUB}/stargazers`} rel="noopener">
                    {num(lang, repo.stars)}
                  </a>
                ) : (
                  "–"
                )}
              </dd>
            </div>
            <div>
              <dt>
                <T id="source.issues" />
              </dt>
              <dd>
                {repo ? (
                  <a href={ISSUES} rel="noopener">
                    {num(lang, repo.openIssues)}
                  </a>
                ) : (
                  "–"
                )}
              </dd>
            </div>
          </dl>
          <p className="small">
            <T id="source.note" />
          </p>
        </div>
      </div>
    </Section>
  );
}
