// The big button, on the start page and the download page: the file for this
// machine when there is one, else the release; and beside it, plainly, what
// could not be told about the machine. Every link goes straight to GitHub.

import { RELEASES_PAGE } from "../config";
import { dateText, mb } from "../github";
import { T, tt, useLang } from "../i18n";
import { Icon } from "../components/icons";
import { Link } from "../router";
import { archNote } from "./machine";
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

const NOTES = {
  unknown: [
    "We could not tell your system from this browser; every file is listed under Download.",
    "Dein System ließ sich aus diesem Browser nicht erkennen; unter Download sind alle Dateien aufgeführt.",
  ],
  mobile: [
    "There are no builds for phones and tablets yet. The button takes you to the release for your computer.",
    "Für Handy und Tablet gibt es noch keine Builds. Der Knopf führt zum Release für deinen Rechner.",
  ],
  "mac-unsure": [
    "This browser does not say whether your Mac has an Apple chip or an Intel one. The file above is for Apple silicon (M1 and later); there is no Intel build yet.",
    "Dieser Browser verrät nicht, ob dein Mac einen Apple- oder einen Intel-Chip hat. Die Datei oben ist für Apple Silicon (M1 und neuer); einen Intel-Build gibt es noch nicht.",
  ],
  "mac-intel": [
    "Your Mac reports an Intel chip, and there is no Intel build yet. The button takes you to the release page.",
    "Dein Mac meldet einen Intel-Chip, und einen Intel-Build gibt es noch nicht. Der Knopf führt zur Release-Seite.",
  ],
  "windows-unsure": [
    "If this is a Windows on ARM PC (a Snapdragon laptop, say), take the ARM file under Download instead.",
    "Wenn das ein Windows-auf-ARM-PC ist (etwa ein Snapdragon-Laptop), nimm stattdessen die ARM-Datei unter Download.",
  ],
  "linux-unsure": [
    "Your browser does not say which CPU this is; x86-64 is assumed. The ARM file is under Download.",
    "Dein Browser sagt nicht, welche CPU das ist; x86-64 ist angenommen. Die ARM-Datei steht unter Download.",
  ],
} as const;

/** The big button: the file for this machine when there is one, else the release. */
export function DownloadButton({ state, onDownloadPage }: { state: ReleaseState; onDownloadPage?: boolean }) {
  const lang = useLang();
  const { release, files, machine, mine } = state;
  const others = onDownloadPage === true ? "#platforms" : "/download#platforms";
  if (!release) {
    // asking, or GitHub did not answer: the releases page has every file
    return (
      <div className="dl" aria-live="polite" aria-busy={release === undefined}>
        <a className="button button-big" href={RELEASES_PAGE} rel="noopener">
          <Icon name="download" className="button-icon" />
          <T en="Download Baylee" de="Baylee herunterladen" />
        </a>
        <p className="dl-meta">
          <T
            en="Windows · macOS (Apple silicon) · Linux. The latest release is on GitHub."
            de="Windows · macOS (Apple Silicon) · Linux. Die neueste Version liegt auf GitHub."
          />
        </p>
      </div>
    );
  }
  const best: ReleaseFile | null = mine ? (files[mine.id]?.[0] ?? null) : null;
  const note = archNote(machine);
  return (
    <div className="dl" aria-live="polite">
      <a className="button button-big" href={best ? best.asset.browser_download_url : release.html_url} rel="noopener">
        <Icon name="download" className="button-icon" />
        {best && mine
          ? tt(lang, `Download for ${mine.name[0]}`, `Download für ${mine.name[1]}`)
          : tt(lang, "Download Baylee", "Baylee herunterladen")}
      </a>
      <p className="dl-meta">
        {release.tag_name}
        {best && (
          <>
            <Sep />
            {tt(lang, best.label[0], best.label[1])}
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
              {tt(lang, "signature", "Signatur")}
            </a>
            <Sep />
          </>
        )}
        <a href={release.html_url} rel="noopener">
          {tt(lang, "release notes", "Release-Notizen")}
        </a>
        <Sep />
        {onDownloadPage === true ? (
          <a href={others}>{tt(lang, "other systems", "andere Systeme")}</a>
        ) : (
          <Link to={others}>{tt(lang, "other systems", "andere Systeme")}</Link>
        )}
      </p>
      {note !== null && <p className="dl-arch">{tt(lang, NOTES[note][0], NOTES[note][1])}</p>}
      {best?.kind === "archive" && (
        <p className="dl-arch">
          <T
            en="This release ships as an archive: unpack it and start Baylee. Installers come with the next release."
            de="Diese Version kommt als Archiv: entpacken und Baylee starten. Installer kommen mit der nächsten Version."
          />
        </p>
      )}
    </div>
  );
}
