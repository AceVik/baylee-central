// How to report a bug or talk to the project: F8 in the client, Discord,
// GitHub issues, and what the client, the gateway and the feedback service
// keep.

import { DISCORD, GITHUB, ISSUES } from "../config";
import { Section } from "../components/Section";
import { T, type Elems } from "../i18n";
import { Link, PATHS } from "../router";

const LINKS: Elems = {
  discord: (words) => (
    <a href={DISCORD} rel="noopener">
      {words}
    </a>
  ),
  issues: (words) => (
    <a href={ISSUES} rel="noopener">
      {words}
    </a>
  ),
  docs: (words) => (
    <a href={`${GITHUB}/blob/main/docs/privacy.md`} rel="noopener">
      {words}
    </a>
  ),
  notice: (words) => <Link to={PATHS.privacy}>{words}</Link>,
};

export function Reports() {
  return (
    <Section id="reports" alt eyebrow={<T id="reports.eyebrow" />} title={<T id="reports.title" />}>
      <div className="two-col">
        <div>
          <p>
            <T id="reports.f8" />
          </p>
          <p>
            <T id="reports.usage" />
          </p>
        </div>
        <div>
          <p>
            <T id="reports.discord" elems={LINKS} />
          </p>
          <p>
            <T id="reports.issues" elems={LINKS} />
          </p>
          <p>
            <T id="reports.privacy" elems={LINKS} />
          </p>
        </div>
      </div>
    </Section>
  );
}
