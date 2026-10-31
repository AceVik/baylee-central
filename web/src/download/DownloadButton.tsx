// The big button, on the start page and the download page: the file for this
// machine when there is one, else the release; and beside it, plainly, what
// could not be told about the machine. Every link goes straight to GitHub.

import { RELEASES_PAGE } from "../config";
import { dateText, mb } from "../github";
import { T, t, useLang, type Key } from "../i18n";
import { Icon } from "../components/icons";
import { Link } from "../router";
import { archNote, type ArchNote } from "./machine";
import type { ReleaseFile } from "./release";
import type { ReleaseState } from "./useRelease";
import "../styles/download.css";

function Sep() {
  return (
    <span className="sep" aria-hidden="true">
      ·
    </span>
  );
}

const NOTES: Record<NonNullable<ArchNote>, Key> = {
  unknown: "dl.noteUnknown",
  mobile: "dl.noteMobile",
  "mac-unsure": "dl.noteMacUnsure",
  "mac-intel": "dl.noteMacIntel",
  "windows-unsure": "dl.noteWindowsUnsure",
  "linux-unsure": "dl.noteLinuxUnsure",
};

/** The big button: the file for this machine when there is one, else the release. */
export function DownloadButton({ state, onDownloadPage }: { state: ReleaseState; onDownloadPage?: boolean }) {
  const lang = useLang();
  const { release, files, machine, mine } = state;
  const others = onDownloadPage === true ? "#platforms" : "/download#platforms";
  if (!release) {
    // asking, or GitHub did not answer: the releases page has every file.
    // While asking, the lines the answer adds are held empty, so nothing
    // below the button moves when it comes (no layout shift).
    const asking = release === undefined;
    const waitingNote = asking ? archNote(machine) : null;
    return (
      <div className="dl" aria-live="polite" aria-busy={asking}>
        <a className="button button-big" href={RELEASES_PAGE} rel="noopener">
          <Icon name="download" className="button-icon" />
          <T id="dl.button" />
        </a>
        <p className="dl-meta">
          <T id="dl.fallbackMeta" />
        </p>
        {asking && (
          <p className="dl-sub dl-held" aria-hidden="true">
            {"\u00a0"}
          </p>
        )}
        {waitingNote !== null && waitingNote !== "mac-intel" && <p className="dl-arch">{t(lang, NOTES[waitingNote])}</p>}
      </div>
    );
  }
  const best: ReleaseFile | null = mine ? (files[mine.id]?.[0] ?? null) : null;
  // An Intel Mac's note says the release lacks its build; with the file at hand there is nothing to say.
  const note = archNote(machine) === "mac-intel" && best ? null : archNote(machine);
  return (
    <div className="dl" aria-live="polite">
      <a className="button button-big" href={best ? best.asset.browser_download_url : release.html_url} rel="noopener">
        <Icon name="download" className="button-icon" />
        {best && mine
          ? t(lang, "dl.for", { system: t(lang, mine.name) })
          : t(lang, "dl.button")}
      </a>
      <p className="dl-meta">
        {release.tag_name}
        {best && (
          <>
            <Sep />
            {t(lang, best.label)}
            <Sep />
            {mb(best.asset.size, lang)}
          </>
        )}
        <Sep />
        {dateText(release.published_at, lang)}
      </p>
      <p className="dl-sub">
        {best?.sha && (
          <>
            <a href={best.sha.browser_download_url} rel="noopener">
              SHA-256
            </a>
            <Sep />
          </>
        )}
        {best?.sig && (
          <>
            <a href={best.sig.browser_download_url} rel="noopener">
              {t(lang, "dl.signature")}
            </a>
            <Sep />
          </>
        )}
        <a href={release.html_url} rel="noopener">
          {t(lang, "dl.notes")}
        </a>
        <Sep />
        {onDownloadPage === true ? (
          <a href={others}>{t(lang, "dl.others")}</a>
        ) : (
          <Link to={others}>{t(lang, "dl.others")}</Link>
        )}
      </p>
      {note !== null && <p className="dl-arch">{t(lang, NOTES[note])}</p>}
      {best?.kind === "archive" && (
        <p className="dl-arch">
          <T id="dl.archive" />
        </p>
      )}
    </div>
  );
}
