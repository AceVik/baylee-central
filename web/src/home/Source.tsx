import { DISCORD, GITHUB, ISSUES } from "../config";
import { num, T, useLang } from "../i18n";
import { parseRepo, relTime, useGitHub, type CiRun, type Commit } from "../github";
import { Section } from "../components/Section";
import { CiLink } from "./CiLink";

export function Source({ ci, commit }: { ci: CiRun | null | undefined; commit: Commit | null | undefined }) {
  const lang = useLang();
  const repo = useGitHub("", "repo", parseRepo);
  return (
    <Section
      id="source"
      alt
      eyebrow={<T en="Source" de="Quellcode" />}
      title={<T en="Open source, and it stays that way" de="Quelloffen, und das bleibt so" />}
    >
      <div className="two-col reveal">
        <div>
          <p className="promise">
            <T
              en="Baylee will always be open source and free. No paywall, no paid features, no ads. Not now, not later. The code is under the GNU Affero General Public License, which means anyone who runs a modified gateway for other people must offer them its source too."
              de="Baylee bleibt immer quelloffen und kostenlos. Keine Bezahlschranke, keine kostenpflichtigen Funktionen, keine Werbung. Nicht jetzt, nicht später. Der Code steht unter der GNU Affero General Public License: Wer ein verändertes Gateway für andere betreibt, muss auch ihnen den Quellcode anbieten."
            />
          </p>
          <p>
            <T
              en={
                <>
                  It is written in Rust, one person&apos;s project with a lot of help from coding agents, and every
                  change goes through the same gate: formatting, lints, the test suite, and a validation of every card
                  against its printed text. Issues and pull requests are welcome;{" "}
                  <a href={`${GITHUB}/blob/main/AGENTS.md`} rel="noopener">
                    AGENTS.md
                  </a>{" "}
                  and{" "}
                  <a href={`${GITHUB}/blob/main/CLAUDE.md`} rel="noopener">
                    CLAUDE.md
                  </a>{" "}
                  are the working rules for people and agents alike.
                </>
              }
              de={
                <>
                  Geschrieben in Rust, das Projekt einer Person mit viel Hilfe von Coding-Agenten, und jede Änderung geht
                  durch dasselbe Gate: Formatierung, Lints, die Testsuite und eine Prüfung jeder Karte gegen ihren
                  gedruckten Text. Issues und Pull Requests sind willkommen;{" "}
                  <a href={`${GITHUB}/blob/main/AGENTS.md`} rel="noopener">
                    AGENTS.md
                  </a>{" "}
                  und{" "}
                  <a href={`${GITHUB}/blob/main/CLAUDE.md`} rel="noopener">
                    CLAUDE.md
                  </a>{" "}
                  sind die Arbeitsregeln für Menschen und Agenten.
                </>
              }
            />
          </p>
          <p className="links-row">
            <a className="button button-ghost" href={GITHUB} rel="noopener">
              github.com/AceVik/baylee
            </a>
            <a className="button button-ghost" href={DISCORD} rel="noopener">
              <T en="Community on Discord" de="Community auf Discord" />
            </a>
            <a className="button button-ghost" href={`${GITHUB}/blob/main/LICENSE`} rel="noopener">
              AGPL-3.0
            </a>
            <a className="button button-ghost" href={`${GITHUB}/blob/main/docs/architecture.md`} rel="noopener">
              <T en="Architecture" de="Architektur" />
            </a>
          </p>
        </div>
        <div className="repo-card glass">
          <h3>
            <T en="The repository right now" de="Das Repository gerade jetzt" />
          </h3>
          <dl className="kv">
            <div>
              <dt>
                <T en="CI on main" de="CI auf main" />
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
                <T en="Last commit" de="Letzter Commit" />
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
                <T en="Stars" de="Sterne" />
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
                <T en="Open issues and pull requests" de="Offene Issues und Pull Requests" />
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
            <T
              en="Read from GitHub's public API when the page loads, cached in this tab for a while. Nothing about you is sent anywhere but to GitHub."
              de="Beim Laden aus GitHubs öffentlicher API gelesen und in diesem Tab eine Weile gemerkt. Über dich geht nichts irgendwohin außer zu GitHub."
            />
          </p>
        </div>
      </div>
    </Section>
  );
}
