// One card per platform with every file of the release, its checksum and
// signature; the visitor's own platform is marked.

import { RELEASES_PAGE } from "../config";
import { mb } from "../github";
import { T, t, useLang, type Elems } from "../i18n";
import { PLATFORMS, type ReleaseFile } from "./release";
import type { ReleaseState } from "./useRelease";

const RELEASES: Elems = {
  releases: (words) => (
    <a href={RELEASES_PAGE} rel="noopener">
      {words}
    </a>
  ),
};

function FileRow({ file }: { file: ReleaseFile }) {
  const lang = useLang();
  return (
    <li>
      <a className="file" href={file.asset.browser_download_url} rel="noopener">
        {file.asset.name}
      </a>
      <span className="kind">{t(lang, file.label)}</span>
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
          <T id={release === null ? "plat.empty" : "plat.asking"} elems={RELEASES} />
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
                {t(lang, p.name)}
                {isMine && (
                  <span className="mine-tag">
                    <T id="plat.yours" />
                  </span>
                )}
              </h3>
              {p.sub !== null && <p className="plat-sub">{t(lang, p.sub)}</p>}
              {list.length === 0 ? (
                <p className="plat-none">
                  <T id="plat.noIntel" />
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
          {t(lang, "plat.missing", { systems: missing.map((p) => t(lang, p.name)).join(", ") })}
        </p>
      )}
    </>
  );
}
