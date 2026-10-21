import { RELEASES_PAGE } from "../config";
import { dateText } from "../github";
import { T, useLang, type Elems } from "../i18n";
import type { Release } from "./release";

const RELEASES: Elems = {
  releases: (words) => (
    <a href={RELEASES_PAGE} rel="noopener">
      {words}
    </a>
  ),
};

export function ReleaseHead({ release }: { release: Release | null | undefined }) {
  const lang = useLang();
  if (release === undefined) {
    return (
      <p className="release-head" aria-live="polite">
        <T id="release.loading" />
      </p>
    );
  }
  if (release === null) {
    return (
      <p className="release-head" aria-live="polite">
        <T id="release.noAnswer" elems={RELEASES} />
      </p>
    );
  }
  return (
    <p className="release-head" aria-live="polite">
      <span className="ver">{release.tag_name}</span>
      {release.prerelease && (
        <span className="tag">
          <T id="release.pre" />
        </span>
      )}
      <span>{dateText(release.published_at, lang)}</span>
      <a href={release.html_url} rel="noopener">
        <T id="release.notes" />
      </a>
      <a href={RELEASES_PAGE} rel="noopener">
        <T id="release.all" />
      </a>
    </p>
  );
}
