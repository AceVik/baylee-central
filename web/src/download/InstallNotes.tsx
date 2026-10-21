// First start on an unsigned build, checking a download, and what each
// system needs.

import { GITHUB, RELEASES_PAGE } from "../config";
import { Section } from "../components/Section";
import { T, t, useLang, type Elems, type Lang } from "../i18n";

/** The install notes in the README of `lang`'s language. */
const notes = (lang: Lang): Elems => ({
  notes: (words) => (
    <a href={`${GITHUB}${t(lang, "link.readmeInstall")}`} rel="noopener">
      {words}
    </a>
  ),
});

export function InstallNotes() {
  const lang = useLang();
  return (
    <Section id="install" alt eyebrow={<T id="install.eyebrow" />} title={<T id="install.title" />}>
      <div className="two-col">
        <div>
          <h3>
            <T id="install.firstTitle" />
          </h3>
          <p>
            <T id="install.first" elems={notes(lang)} />
          </p>
          <h3>
            <T id="install.checkTitle" />
          </h3>
          <p>
            <T id="install.check" />
          </p>
        </div>
        <div>
          <h3>
            <T id="install.needsTitle" />
          </h3>
          <ul className="plain">
            <li>
              <T id="install.windows" />
            </li>
            <li>
              <T id="install.mac" />
            </li>
            <li>
              <T id="install.linux" />
            </li>
            <li>
              <T id="install.browser" />
            </li>
            <li>
              <T id="install.phones" />
            </li>
          </ul>
          <p className="small">
            <a href={RELEASES_PAGE} rel="noopener">
              <T id="install.releases" />
            </a>
          </p>
        </div>
      </div>
    </Section>
  );
}
