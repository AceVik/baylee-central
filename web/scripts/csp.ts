// The site's Content-Security-Policy, the one Caddy sends in production
// (README.md §"Serving it") and scripts/serve.ts sends locally, so a test
// against the build sees what a visitor's browser enforces. No inline script
// or style; GitHub's API for the releases and raw.githubusercontent.com for
// the newest release's card status, Scryfall's image host for card
// pictures, and the feedback service for the contact form. Central's
// directory and the Impressum are this origin (`/api`, `/site/impressum`).
// `'wasm-unsafe-eval'` lets the account pages' Worker compile the browser
// crypto's WebAssembly (owner, 21.10.2026); it allows no `eval` of
// JavaScript and no inline script.

export const FEEDBACK_ORIGIN = "https://feedback.baylee.app";

export function csp(extraConnect: readonly string[] = []): string {
  const connect = [
    "'self'",
    "https://api.github.com",
    // the card status at the newest release's tag (src/config.ts RELEASE_DATA)
    "https://raw.githubusercontent.com",
    FEEDBACK_ORIGIN,
    ...extraConnect,
  ].join(" ");
  return [
    "default-src 'self'",
    "script-src 'self' 'wasm-unsafe-eval'",
    "style-src 'self'",
    "img-src 'self' data: https://cards.scryfall.io",
    "font-src 'self'",
    `connect-src ${connect}`,
    "worker-src 'self'",
    "object-src 'none'",
    "base-uri 'none'",
    "form-action 'none'",
    "frame-ancestors 'none'",
  ].join("; ");
}
