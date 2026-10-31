// The site's Content-Security-Policy, the one Caddy sends in production
// (README.md §"Serving it") and scripts/serve.ts sends locally, so a test
// against the build sees what a visitor's browser enforces. No inline script
// or style; GitHub's API for the releases and raw.githubusercontent.com for
// the newest release's card status, Scryfall's image host for card
// pictures, and the feedback service for the contact form. Central's
// directory and the Impressum are this origin (`/api`, `/site/impressum`).
// `'wasm-unsafe-eval'` lets the account pages' Worker compile the browser
// crypto's WebAssembly (owner, 21.10.2026); it allows no `eval` of
// JavaScript and no inline script. GitHub is reachable only at Baylee's own
// repository's paths, so a script that got in could not post to another
// repository or gist through it; no picture is a `data:` URL. `blob:` is a
// picture the page made itself: the window's screenshot in a report's client
// part, shown only after its PNG signature was checked
// (src/admin/feedback/client.ts), its address given back when it goes.

export const FEEDBACK_ORIGIN = "https://feedback.baylee.app";
/** src/config.ts's REPO (that module reads Vite's settings, which Node has not); test/csp.test.ts holds the two together. */
export const REPO = "AceVik/baylee";

/** The browser features the site never uses, switched off for it and anything it might embed. */
export const PERMISSIONS = "camera=(), microphone=(), geolocation=(), payment=(), usb=(), serial=(), bluetooth=(), hid=()";

export function csp(extraConnect: readonly string[] = []): string {
  const connect = [
    "'self'",
    // the repository itself, and below it: releases, CI runs, commits (src/github.ts)
    `https://api.github.com/repos/${REPO}`,
    `https://api.github.com/repos/${REPO}/`,
    // the card status at the newest release's tag (src/config.ts RELEASE_DATA)
    `https://raw.githubusercontent.com/${REPO}/`,
    FEEDBACK_ORIGIN,
    ...extraConnect,
  ].join(" ");
  return [
    "default-src 'self'",
    "script-src 'self' 'wasm-unsafe-eval'",
    "style-src 'self'",
    "img-src 'self' blob: https://cards.scryfall.io",
    "font-src 'self'",
    `connect-src ${connect}`,
    "worker-src 'self'",
    "object-src 'none'",
    "base-uri 'none'",
    "form-action 'none'",
    "frame-ancestors 'none'",
  ].join("; ");
}
