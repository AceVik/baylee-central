// The download: the big button (on the start page and the download page),
// the release's head line, and one card per platform. Every link goes
// straight to the file on GitHub (`browser_download_url`), which GitHub
// serves as an attachment, so the page stays.

import { useEffect, useMemo, useState } from "react";

import { RELEASES_PAGE } from "../config";
import { RELEASES_PATH, dateText, mb, useGitHub } from "../github";
import { T, tt, useLang } from "../i18n";
import { Icon } from "../components/icons";
import { Link } from "../router";
import { archNote, detect, pickRelease, platformFor, PLATFORMS, sortFiles, type Files, type Machine, type Platform, type Release, type ReleaseFile } from "./release";

export interface ReleaseState {
  /** `undefined` while asking GitHub, `null` when it did not answer. */
  release: Release | null | undefined;
  files: Files;
  machine: Machine | null;
  mine: Platform | null;
}

export function useRelease(): ReleaseState {
  const release = useGitHub(RELEASES_PATH, "releases", pickRelease);
  const [machine, setMachine] = useState<Machine | null>(null);
  useEffect(() => {
    let live = true;
    void detect().then((m) => {
      if (live) setMachine(m);
    });
    return () => {
      live = false;
    };
  }, []);
  const files = useMemo(() => (release ? sortFiles(release) : {}), [release]);
  return { release, files, machine, mine: platformFor(machine) };
}

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

function FileRow({ file }: { file: ReleaseFile }) {
  const lang = useLang();
  return (
    <li>
      <a className="file" href={file.asset.browser_download_url} rel="noopener">
        {file.asset.name}
      </a>
      <span className="kind">{tt(lang, file.label[0], file.label[1])}</span>
      <span className="size">{mb(file.asset.size, lang)}</span>
      {(file.sha ?? file.sig) && (
        <span className="aux">
          {file.sha && (
            <a href={file.sha.browser_download_url} rel="noopener">
              .sha256
            </a>
          )}
          {file.sha && file.sig && " · "}
          {file.sig && (
            <a href={file.sig.browser_download_url} rel="noopener">
              .sig
            </a>
          )}
        </span>
      )}
    </li>
  );
}

/** One card per platform; the visitor's own is marked. */
export function Platforms({ state }: { state: ReleaseState }) {
  const lang = useLang();
  const { release, files, mine } = state;
  if (!release) {
    return (
      <div className="platforms-empty">
        <p>
          {release === null ? (
            <T
              en="An installer and an archive for Windows (x64 and ARM64), macOS (Apple silicon) and Linux (x86-64 and ARM64), each with a .sha256 beside it, on the "
              de="Ein Installer und ein Archiv für Windows (x64 und ARM64), macOS (Apple Silicon) und Linux (x86-64 und ARM64), jeweils mit einer .sha256 daneben, auf der "
            />
          ) : (
            <T en="Asking GitHub for the files … They are also on the " de="Frage GitHub nach den Dateien … Sie liegen auch auf der " />
          )}
          <a href={RELEASES_PAGE} rel="noopener">
            <T en="releases page" de="Releases-Seite" />
          </a>
          .
        </p>
      </div>
    );
  }
  // a platform the release lacks is said below the cards instead (Intel Macs get their own card)
  const shown = PLATFORMS.filter((p) => (files[p.id]?.length ?? 0) > 0 || p.id === "mac-x64");
  const missing = PLATFORMS.filter((p) => p.id !== "mac-x64" && (files[p.id]?.length ?? 0) === 0);
  return (
    <>
      <ul className="platforms">
        {shown.map((p) => {
          const list = files[p.id] ?? [];
          const isMine = mine?.id === p.id;
          return (
            <li key={p.id} className={`plat glass${isMine ? " mine" : ""}`}>
              <h3 className="plat-name">
                {tt(lang, p.name[0], p.name[1])}
                {isMine && (
                  <span className="mine-tag">
                    <T en="your system" de="dein System" />
                  </span>
                )}
              </h3>
              {p.sub[0] !== "" && <p className="plat-sub">{tt(lang, p.sub[0], p.sub[1])}</p>}
              {list.length === 0 ? (
                <p className="plat-none">
                  <T
                    en="No build for Intel Macs yet. Baylee needs a Mac with an Apple chip (M1 or later)."
                    de="Noch kein Build für Intel-Macs. Baylee braucht einen Mac mit Apple-Chip (M1 oder neuer)."
                  />
                </p>
              ) : (
                <ul className="plat-files">
                  {list.map((f) => (
                    <FileRow key={f.asset.name} file={f} />
                  ))}
                </ul>
              )}
            </li>
          );
        })}
      </ul>
      {missing.length > 0 && (
        <p className="dl-fallback">
          {tt(
            lang,
            `This release has no file for: ${missing.map((p) => p.name[0]).join(", ")}. Older releases are on the releases page.`,
            `Diese Version hat keine Datei für: ${missing.map((p) => p.name[1]).join(", ")}. Ältere Versionen liegen auf der Releases-Seite.`,
          )}
        </p>
      )}
    </>
  );
}
