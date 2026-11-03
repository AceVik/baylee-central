// The big button, on the start page and the download page: the file for this
// machine when there is one, else the release; subtly showing the version and
// architecture inside the button, with a quick-access dropdown on the right to
// reach every other system/architecture installer directly. Every link goes
// straight to GitHub.

import { useEffect, useId, useRef, useState } from "react";

import { RELEASES_PAGE } from "../config";
import { dateText, mb } from "../github";
import { T, t, useLang, type Key } from "../i18n";
import { Icon } from "../components/icons";
import { Link } from "../router";
import { archNote, type ArchNote } from "./machine";
import { PLATFORMS, type Platform, type ReleaseFile } from "./release";
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

function archText(platform: Platform | null): string | null {
  if (!platform) return null;
  if (platform.id === "mac-arm64") return "Apple Silicon (ARM64)";
  if (platform.id === "mac-x64") return "Intel (x64)";
  return platform.arch === "arm64" ? "ARM64" : "x64";
}

/** The big button: the file for this machine when there is one, else the release, plus a quick-access dropdown. */
export function DownloadButton({ state, onDownloadPage }: { state: ReleaseState; onDownloadPage?: boolean }) {
  const lang = useLang();
  const { release, files, machine, mine } = state;
  const others = onDownloadPage === true ? "#platforms" : "/download#platforms";
  const [menuOpen, setMenuOpen] = useState(false);
  const wrapRef = useRef<HTMLDivElement>(null);
  const toggleRef = useRef<HTMLButtonElement>(null);
  const menuId = useId();

  // the list closes on a press outside, on Escape (the keyboard back on its
  // toggle) and when the keyboard leaves it
  useEffect(() => {
    const wrap = wrapRef.current;
    if (!menuOpen || wrap === null) return;
    const onPointerDown = (e: PointerEvent) => {
      if (e.target instanceof Node && !wrap.contains(e.target)) setMenuOpen(false);
    };
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      setMenuOpen(false);
      toggleRef.current?.focus();
    };
    const onFocusOut = (e: FocusEvent) => {
      if (e.relatedTarget instanceof Node && !wrap.contains(e.relatedTarget)) setMenuOpen(false);
    };
    document.addEventListener("pointerdown", onPointerDown);
    document.addEventListener("keydown", onKeyDown);
    wrap.addEventListener("focusout", onFocusOut);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown);
      document.removeEventListener("keydown", onKeyDown);
      wrap.removeEventListener("focusout", onFocusOut);
    };
  }, [menuOpen]);

  if (!release) {
    // asking, or GitHub did not answer: the releases page has every file.
    // While asking, the lines the answer adds are held empty, so nothing
    // below the button moves when it comes (no layout shift).
    const asking = release === undefined;
    const waitingNote = asking ? archNote(machine) : null;
    const fallbackLabel = t(lang, "dl.button");
    return (
      <div className="dl" aria-live="polite" aria-busy={asking}>
        <div className="dl-split">
          <a className="button button-big dl-main" href={RELEASES_PAGE} rel="noopener" aria-label={fallbackLabel}>
            <Icon name="download" className="button-icon" />
            <span className="dl-btn-text">
              <span className="dl-btn-title">{fallbackLabel}</span>
              {mine && <span className="dl-btn-badge">{archText(mine)}</span>}
            </span>
          </a>
        </div>
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
  const mainLabel = best && mine ? t(lang, "dl.for", { system: t(lang, mine.name) }) : t(lang, "dl.button");
  const archLabel = archText(mine);
  const badgeText = archLabel ? `${release.tag_name} · ${archLabel}` : release.tag_name;
  const availablePlatforms = PLATFORMS.filter((p) => (files[p.id]?.length ?? 0) > 0);

  return (
    <div className="dl">
      <div className="dl-split-wrap" ref={wrapRef}>
        <div className="dl-split">
          <a
            className="button button-big dl-main"
            href={best ? best.asset.browser_download_url : release.html_url}
            rel="noopener"
            aria-label={mainLabel}
          >
            <Icon name="download" className="button-icon" />
            <span className="dl-btn-text">
              <span className="dl-btn-title">{mainLabel}</span>
              <span className="dl-btn-badge">{badgeText}</span>
            </span>
          </a>
          {availablePlatforms.length > 0 && (
            <button
              ref={toggleRef}
              type="button"
              className="button dl-split-toggle"
              aria-expanded={menuOpen}
              aria-controls={menuId}
              aria-label={t(lang, "dl.quickAccess")}
              title={t(lang, "dl.quickAccess")}
              onClick={() => setMenuOpen((o) => !o)}
            >
              <Icon name="down" className={`dl-chevron${menuOpen ? " open" : ""}`} />
            </button>
          )}
        </div>
        {menuOpen && availablePlatforms.length > 0 && (
          <section id={menuId} className="dl-menu" aria-label={t(lang, "dl.quickAccess")}>
            <ul className="dl-menu-list">
              {availablePlatforms.map((p) => {
                const pFiles = files[p.id] ?? [];
                const primary = pFiles[0];
                if (!primary) return null;
                const isMine = mine?.id === p.id;
                return (
                  <li key={p.id} className={`dl-menu-item${isMine ? " is-mine" : ""}`}>
                    <a
                      className="dl-menu-link"
                      href={primary.asset.browser_download_url}
                      rel="noopener"
                      onClick={() => setMenuOpen(false)}
                    >
                      <span className="dl-menu-plat">
                        <span className="dl-menu-name">{t(lang, p.name)}</span>
                        <span className="dl-menu-arch">{archText(p)}</span>
                      </span>
                      <span className="dl-menu-meta">
                        <span>{t(lang, primary.label)}</span>
                        <Sep />
                        <span>{mb(primary.asset.size, lang)}</span>
                      </span>
                    </a>
                  </li>
                );
              })}
            </ul>
            <div className="dl-menu-foot">
              {onDownloadPage === true ? (
                <a href={others} onClick={() => setMenuOpen(false)}>
                  {t(lang, "dl.others")}
                </a>
              ) : (
                <Link to={others} onClick={() => setMenuOpen(false)}>
                  {t(lang, "dl.others")}
                </Link>
              )}
              <Sep />
              <a href={release.html_url} rel="noopener" onClick={() => setMenuOpen(false)}>
                {t(lang, "dl.notes")}
              </a>
            </div>
          </section>
        )}
      </div>
      <p className="dl-meta" aria-live="polite">
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
