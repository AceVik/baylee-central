import { RELEASES_PAGE } from "../config";
import { dateText } from "../github";
import { T, useLang } from "../i18n";
import type { Release } from "./release";

export function ReleaseHead({ release }: { release: Release | null | undefined }) {
  const lang = useLang();
  if (release === undefined) {
    return (
      <p className="release-head" aria-live="polite">
        <T en="Loading the latest release from GitHub …" de="Lade die neueste Version von GitHub …" />
      </p>
    );
  }
  if (release === null) {
    return (
      <p className="release-head" aria-live="polite">
        <T
          en="GitHub did not answer just now; it lets a page like this ask 60 times an hour. "
          de="GitHub hat gerade nicht geantwortet; einer Seite wie dieser erlaubt es 60 Anfragen pro Stunde. "
        />
        <a href={RELEASES_PAGE} rel="noopener">
          <T en="The releases page has every file." de="Die Releases-Seite hat jede Datei." />
        </a>
      </p>
    );
  }
  return (
    <p className="release-head" aria-live="polite">
      <span className="ver">{release.tag_name}</span>
      {release.prerelease && (
        <span className="tag">
          <T en="pre-release" de="Vorabversion" />
        </span>
      )}
      <span>{dateText(release.published_at, lang)}</span>
      <a href={release.html_url} rel="noopener">
        <T en="Release notes and checksums" de="Release-Notizen und Prüfsummen" />
      </a>
      <a href={RELEASES_PAGE} rel="noopener">
        <T en="All releases" de="Alle Versionen" />
      </a>
    </p>
  );
}
