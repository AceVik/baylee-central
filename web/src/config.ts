// Where the site's data comes from. Nothing here is a secret: every value
// ends up in the built files a browser downloads.

export const REPO = "AceVik/baylee";
export const GITHUB = `https://github.com/${REPO}`;
export const GITHUB_API = `https://api.github.com/repos/${REPO}`;
export const RELEASES_PAGE = `${GITHUB}/releases`;
export const ISSUES = `${GITHUB}/issues`;
export const DISCORD = "https://discord.gg/4pP3uAwgY";
/** This web app's own source (AGPL-3.0, §13). */
export const SITE_SOURCE = "https://github.com/AceVik/baylee-central/tree/main/web";
/** Baylee Central's repository: public so anyone can check how it handles their data. */
export const CENTRAL_SOURCE = "https://github.com/AceVik/baylee-central";

/** A page served from this machine for a try talks to services on this machine. */
export function onLoopback(hostname: string = window.location.hostname): boolean {
  return /^(localhost|127\.0\.0\.1|\[::1\])$/.test(hostname);
}

/** The feedback service of Baylee's repository (`docs/feedback.md`), for the contact form. */
export const FEEDBACK_ORIGIN = import.meta.env.VITE_FEEDBACK_ORIGIN ?? "https://feedback.baylee.acevik.de";
/** Where a page on this machine finds a local feedback service. */
export const FEEDBACK_LOCAL = "http://127.0.0.1:28780";

export function contactEndpoint(hostname?: string): string {
  return `${onLoopback(hostname) ? FEEDBACK_LOCAL : FEEDBACK_ORIGIN}/site/contact`;
}

/**
 * The Impressum's proof of work and blob: the site's own origin, which Caddy
 * passes on to the feedback service (README.md §"The Impressum").
 */
export function impressumEndpoint(hostname?: string): string {
  return onLoopback(hostname) ? `${FEEDBACK_LOCAL}/site/impressum` : "/site/impressum";
}

/** Central's API, same origin (README.md §"The gateway directory"). */
export const CENTRAL_API = import.meta.env.VITE_CENTRAL_API ?? "/api";

/** The card status (`npm run content`), fetched lazily. */
export const DATA_BASE = "/data/";

/**
 * The card status of a release: `web/site/data/` at its tag in Baylee's
 * repository, which every release carries for its own pool (Baylee's
 * `docs/releasing.md`). Read from raw.githubusercontent.com, which answers
 * any origin; a release's assets do not.
 */
export const RELEASE_DATA = (tag: string): string =>
  `https://raw.githubusercontent.com/${REPO}/${encodeURIComponent(tag)}/web/site/data/`;
/** The screenshots (`npm run content`). */
export const MEDIA_BASE = "/media/";
