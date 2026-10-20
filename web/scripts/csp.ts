// The site's Content-Security-Policy, the one Caddy sends in production
// (README.md §"Serving it") and scripts/serve.ts sends locally, so a test
// against the build sees what a visitor's browser enforces. No inline script
// or style; GitHub's API for the releases, Scryfall's image host for card
// pictures, and the feedback service for the contact form. Central's
// directory and the Impressum are this origin (`/api`, `/site/impressum`).

export const FEEDBACK_ORIGIN = "https://feedback.baylee.acevik.de";

export function csp(extraConnect: readonly string[] = []): string {
  const connect = ["'self'", "https://api.github.com", FEEDBACK_ORIGIN, ...extraConnect].join(" ");
  return [
    "default-src 'self'",
    "script-src 'self'",
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
