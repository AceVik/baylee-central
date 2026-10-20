// One card per platform with every file of the release, its checksum and
// signature; the visitor's own platform is marked.

import { RELEASES_PAGE } from "../config";
import { mb } from "../github";
import { T, tt, useLang } from "../i18n";
import { PLATFORMS, type ReleaseFile } from "./release";
import type { ReleaseState } from "./useRelease";

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
