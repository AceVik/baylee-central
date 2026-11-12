# Proposal: the website rebuilt as server-rendered Rust

*10.11.2026.* The owner asked on 10.11.2026 for the whole frontend of
`baylee.app` to be rebuilt as server-side rendered Rust (Leptos or similar):
search-engine optimised, accessible by keyboard alone, the legal texts
reworked, a fully built-out signed-in area and a redesigned chat that stays an
installable PWA. This document is the inventory of what exists, the plan, and
the decisions the owner has to take before code that needs them lands. It is
the contract for the branch `claude/frontend-leptos-af71a6`; a change of plan
changes this file first.

Status: **accepted** by the owner on 10.11.2026 with the decisions of §10;
the dependency graph of [dependency-proposal/leptos.md](../dependency-proposal/leptos.md)
and its three gate exceptions are approved.

## 1. What exists today (inventory at `4f5e471`)

The site is a React 19 + Vite single-page app under `web/` (about 15,000
lines of TypeScript, 1,300 of CSS, 1,319 Fluent messages in English and
German), built at deploy time by `ops/deploy-hook` (`npm ci && npm run content
&& npm run build`) and served by Caddy as static files from
`/opt/baylee-app/web`, with `/api/*` passed to Central's browser listener
(`127.0.0.1:28791`) and `/site/impressum*` to Baylee's feedback service. The
Caddy site block exists only as documentation (the old `web/README.md` §"Serving it", removed 12.11.2026).

### 1.1 Pages

| Path | What it shows | Data |
| --- | --- | --- |
| `/` | status strip (release, CI, commit, gateways online), hero with the painting and a WebGL 2 cloud simulation, download button, screenshots, gateways, about and the AI opponents, contact form, source | GitHub API, Central directory |
| `/download` | the newest release's files per platform, install notes, checksums and signatures | GitHub API |
| `/cards` | card status: totals, eras of four years, finished and next sets, every set, a set's cards with Scryfall images | `raw.githubusercontent.com` at the release tag, `cards.scryfall.io` |
| `/gateways` | Central's public directory, official first, search, copy address, how to host one | Central `gateway_directory` |
| `/contact` | contact form, how to report a bug | feedback service, cross-origin |
| `/impressum` | operator details behind a proof of work, drawn scrambled and `aria-hidden` | feedback service |
| `/privacy`, `/terms` | privacy notice (draft for the owner's lawyer); Central's terms from `legal/terms.*.md` | build time |
| `/register`, `/signin` | registration with beta key and recovery code; sign-in (OPAQUE in a Worker) | Central |
| `/account` | handle, session, recovery code, display name, notification settings | Central |
| `/account/security` | sessions (website, client with its gateways), end the client session, sign out | Central `owner` |
| `/account/gateways` | registered gateways, tokens (create once-shown, revoke) | Central `owner` |
| `/account/chat`, `/account/contacts` | messenger: conversations, groups, contacts | Central `social` |
| `/admin…` | overview, users, user, gateways, audit, legacy migration, feedback triage (about 5,000 lines alone) | Central `admin` |

No canonical URLs, hreflang, sitemap, JSON-LD or per-page descriptions; one
static OpenGraph image; every extensionless path answers 200 with the shell
(soft 404); page titles are set by script only. Without JavaScript only a
stale `<noscript>` text is shown.

### 1.2 Requests to third parties today

The visitor's browser talks to `api.github.com` (releases, CI, commit, repo)
on every home and download visit, to `raw.githubusercontent.com` for the card
status, to `cards.scryfall.io` when a set is opened, and to
`feedback.baylee.app` for the contact form. The owner's new rule (no request
to any third party) removes all four from the browser (§4.3).

### 1.3 Central API used by the browser

All calls are `POST /api/v1` on the same origin with `Content-Type:
application/json`, `X-Baylee-Central: 1`, `credentials: same-origin` (the
directory `omit`), `redirect: error`, no `Authorization`; byte strings are
JSON number arrays; every answer is checked against its exact shape; a
refusal is read by status only (401 rejected, 403 forbidden or terms, 409
conflict, 429 busy, 5xx unavailable). Shapes are central-api's
(`wire.rs`, `owner.rs`, `social.rs`, `chat.rs`, `groups.rs`,
`notifications.rs`, `admin.rs`).

| Area | Operations with a UI today |
| --- | --- |
| Accounts | `register`, `enrollment_start`, `enrollment_finish`, `discover`, `login_start`, `login_finish`, `renew_session`, `account_summary`, `set_recovery`, `set_display_name`, `logout`, `terms`, `terms_status`, `accept_terms` |
| Owner | `sessions`, `end_client_session`, `gateways`, `add_gateway`, `create_token`, `revoke_token` |
| Social | `contacts`, `lookup`, `contact` (request, accept, remove, block, unblock) |
| Chat | `register`, `keys`, `send`, `inbox` |
| Groups | `list`, `create`, `change` (rename, invite, join, withdraw, remove, transfer, delete) |
| Notifications | `read`, `set`, `subscribe`, `unsubscribe` |
| Admin | every `AdminRequest`, and the raw report-part route |
| Public | `gateway_directory` |

**Offered by the browser listener (`central-identity` `browser_operation`)
but without any UI or browser crypto:** `change_password_start/finish`,
`reset_start/finish`, `account_export`, `delete_account_start/finish`,
`key_check_status`, `set_key_check`, `pair_start/status/approve/cancel`,
`session_status` (client exists, no caller). Their native implementations
are in `central-client` (`password.rs`, `reset.rs`, `export.rs`,
`deletion.rs`, `pairing.rs`). (This was the state before the rewrite; the
new site has the password change and the reset since 12.11.2026, §16.)

### 1.4 Browser crypto and the chat

- `central-web-crypto` (Rust, `#![forbid(unsafe_code)]`) is committed as
  WebAssembly with its wasm-bindgen glue in `crates/central-web/crypto/`, checked
  byte for byte by CI (`cargo xtask wasm --check`, canonical x86-64 Linux
  build). Exports: `enrollment_start/finish`, `login_start/finish`,
  `login_confirm`, `recovery_create`, `chat_unlock`, `chat_resume`, and the
  `Chat` object (`device`, `registration`, `observe`, `confirm`, `seal`,
  `open`, `save_trust`, `restore_trust`, `keep`). The live issuer is
  compiled in.
- One module Worker per sign-in or registration runs Argon2 (64 MiB) and
  OPAQUE; the page never holds the master key. After a sign-in of an account
  with a tag, the same Worker keeps the chat identity (`chat_unlock`) and
  seals it into IndexedDB `baylee-chat`/`kept` under a fresh non-extractable
  AES-GCM key (owner decision 03.11.2026: the chat never needs unlocking
  while signed in); a reload resumes it (`chat_resume`). A password is only a
  fallback when the kept identity cannot be opened.
- Peer trust (TOFU) is stored encrypted under the vault key in localStorage
  `baylee-chat-trust:v1:<account hex>`; a changed key set blocks sending
  until the exact snapshot is confirmed (safety number in groups of five).
- The inbox is polled every 2 s while visible (0 s while `more`, doubling to
  30 s on errors), 50 deliveries a page, cursor in memory only (each session
  replays retained history), newest 5,000 kept; groups every 5 s, contacts
  every 10 s. Unread counts are in memory, not persisted. Conversation ids
  `dm:<lo>:<hi>` and `grp:<uuid>`. No attachments, presence, typing or
  receipts exist in the protocol. Messages are rendered as text only.
- Compatibility with the game client (Baylee `docs/central-chat-sync.md`):
  same identity derivation, device registration (`label`), header and body
  JSON, wraps, conversation ids, tag formatting (`{:04x}`), inbox cursor
  semantics, and the account document's exact bytes at registration (the
  client's structs are `deny_unknown_fields`).

### 1.5 PWA and notifications

Manifest `id "/"`, `start_url "/account"`, scope `/`; `/sw.js` is generated
with a content-hashed cache name `baylee-public-<hash>`, caches only the
public build, keeps one previous build, never caches `/api/` or `/site/`;
navigation is network first with the cached shell as fallback. Push payloads
are `{v:1, kind, nonce, lang}` with fixed texts; category bits chat 1,
contacts 2, updates 4, feedback 8, gateways 16; the installation nonce is in
localStorage `baylee-push-installation-v1` and IndexedDB `baylee-push`. The
install offer (`InstallNotice`) and the update prompt (`ACTIVATE_UPDATE`) exist
in code but are not wired.

### 1.6 Words, theme, storage

Fluent (`@fluent/bundle`), English and German, a subset of Fluent only
(messages, `$variables`, 41 plural selectors, markup tags, no terms or
attributes). Language: `?lang`, then localStorage `baylee-site-lang`, then the
browser. Theme: device, light or dark (`baylee-site-theme`), applied before
the first paint by `public/boot.js`. Storage keys in use:
`baylee-site-lang`, `baylee-site-theme`, `baylee-central-session` (session
end only), `baylee-push-installation-v1`, `baylee-chat-trust:v1:<acct>`,
`baylee-feedback-keys`, `baylee-clouds`; sessionStorage
`baylee-site:{releases,ci,commit,repo}`; IndexedDB `baylee-chat`,
`baylee-push`.

### 1.7 Drift and gaps found

- The privacy notice does not name the theme key, the push nonce, the chat
  trust store, both IndexedDB stores, the Web Push providers, the card
  status host, or that the home page reads the directory on every visit.
- `web/package.json` says `AGPL-3.0-only`, contrary to `LICENSE.md` § 6.
- Hard-coded English on the home page (`About.tsx`), a stale `<noscript>`,
  dead components (`Online.tsx`, `Opponents.tsx`, `InstallNotice`), an
  unreachable branch in `LegalText.tsx`.
- The download page has no way to the `-v2` fallback archives for CPUs
  without AVX2, which Baylee's `docs/releasing.md` leaves to the site.
- The Impressum's scrambled, `aria-hidden` drawing is not WCAG AA.

## 2. Stack

### 2.1 Decision (proposed)

**Leptos 0.8.23** (the newest stable; 0.9 is a release candidate; since
11.11.2026 0.9.0-rc1 on the owner's word, §10 item 9) in
**islands** mode, rendered on the server with `to_html()` inside handlers of
the already approved **Axum 0.8.9** — without `leptos_axum`,
`leptos_router`, `leptos_meta`, server functions or `cargo-leptos`. Routing,
`<head>`, caching and headers are first-party code; the islands' WebAssembly
is built by a new `cargo xtask site` with the pinned wasm-bindgen CLI 0.2.130,
as `cargo xtask wasm` already builds the crypto.

- Content pages are complete HTML in the first response and load **no**
  WebAssembly at all. Only pages with interactive parts (sign-in, register,
  the account area, the chat, the contact form, the theme switch) load the
  islands bundle.
- Islands are hydrated by a small first-party module script (an external
  file, 25 lines) instead of Leptos' inline hydration script, so the CSP
  stays `script-src 'self' 'wasm-unsafe-eval'` with no nonce.

**Spike (10.11.2026, recorded here, not committed):** a Leptos 0.8.23 island
in a private module, under `#![forbid(unsafe_code)]`, `missing_docs` and
clippy `pedantic` at deny, built natively (SSR) and for `wasm32` (hydrate),
hydrated in Chromium under `default-src 'self'; script-src 'self'
'wasm-unsafe-eval'; style-src 'self'; require-trusted-types-for 'script';
trusted-types 'none'`: the button updated on click, no console error, no
Trusted Types violation. Size of the runtime with one island after
wasm-bindgen: 199 KB WebAssembly (65 KB Brotli, 75 KB gzip) and 30 KB glue.
Findings: an island must live in a private module (its generated props
struct has no docs), the hydrate build warns about props of server-only
components (fixed with `cfg_attr` in the crate), and updates are applied in a
microtask after the event.

### 2.2 Why Leptos, and what it costs

- **For:** one language and one component model for the server and the
  interactive parts; the account and chat islands can use `central-api`'s own
  serde types (`deny_unknown_fields`, exact byte lengths), so the browser
  speaks exactly what the native client speaks; text is escaped by
  construction; islands keep public pages free of WebAssembly.
- **Cost:** 82 new packages (inventory, unsafe markers, licences and build
  scripts in [dependency-proposal/leptos.md](../dependency-proposal/leptos.md)),
  new features on six approved ones, and `web-sys` features. **Three gate
  exceptions** that only the owner can grant:
  1. `paste` 1.0.15 is unmaintained (RUSTSEC-2024-0436); `cargo audit --deny
     warnings` and `cargo deny` (`unmaintained = "all"`) fail without an
     ignore. It is a compile-time proc macro used by `leptos`, `tachys`,
     `reactive_graph` and `either_of`; Leptos 0.9.0-rc1 still uses it.
  2. `xxhash-rust` 0.8.19 is BSL-1.0 (OSI-approved, permissive), not on
     `deny.toml`'s allow list; used by `server_fn`.
  3. `base16` 0.2.1 is CC0-1.0, not on the allow list; used by
     `wasm_split_macros`.
- **Alternatives considered:**
  - *Axum with a first-party HTML builder, interactive parts in Rust with
    `wasm-bindgen` and `web-sys` but no framework.* Fewest packages (about
    five), no exceptions, but a hand-written DOM layer for the chat and the
    account area: more first-party code to review and keep correct, which
    the priorities (security, QA, then performance, then clean code) do not
    favour over a reviewed framework.
  - *Server-rendered Rust with TypeScript islands* (reusing today's code):
    against the owner's preference for Rust; keeps Node in the build.
  - *Dioxus fullstack:* a larger graph and the same exceptions class.
  - *cargo-leptos:* downloads tool binaries (wasm-opt, sass, tailwind) at
    build time; not needed.

**If the owner refuses the exceptions,** the first alternative is the
fallback: everything below except §2.1 stays as written.

### 2.3 What stays JavaScript, and why

Only where the platform requires a script file:

- `boot.js` in `<head>`, blocking: applies the stored theme before the first
  paint (WebAssembly cannot run before it). About 15 lines.
- The islands bootstrap (above).
- The two **Worker entry points** (account crypto, chat): a Worker must start
  from a script; they load the committed `central-web-crypto` module
  unchanged and keep today's message protocol. The page side of that
  protocol moves to Rust.
- The **service worker** `/sw.js`: must be a script at the origin root.

All plain ES modules, no bundler and no Node at build time; Node stays a
development tool for the browser tests (Playwright, axe-core, Lighthouse).

### 2.4 The browser crypto

`central-web-crypto` and its committed bytes stay as they are for the pages
that exist today. Five flows have no browser crypto yet:

| Flow | Needs new WebAssembly exports? |
| --- | --- |
| Account deletion | **No**: `delete_account_start` takes KE1 from `login_start`, `login_finish` makes KE3 — the existing exports. |
| Data export (Central's record) | **No**: `account_export` needs a sign-in younger than 30 minutes, no key. Decrypting the private objects for the export would need one. |
| Password change | **Yes**: login with the old password and enrolment of the new one in one state, rewrapping the *same* master key (`central-client` `change_password`). |
| Reset with a code | **Yes** (`central-client` `reset`): new enrolment, keep or new vault. |
| Pairing the game client | **Yes**: key check, HPKE seal of the master key (`MasterKey::seal_for_pairing`); also, Baylee's client does not parse `baylee://pair` yet. |

New exports change the committed module: they get the three passes of
[security assurance](../security-assurance.md), known-answer tests against
`central-client`, and the canonical rebuild (Docker on Rosetta, as on
03.11.2026). They come in their own commit after the pages that need no new
crypto, and the PR states which flows are in it.

## 3. Architecture

```
browser ──https──▶ Caddy ──┬── /api/*                  ──▶ Central browser listener (unchanged)
                           ├── /site/contact, /site/impressum* ──▶ feedback service (unchanged)
                           └── everything else         ──▶ central-web (new, 127.0.0.1)
```

- **`central-web`**, a new unprivileged process: no database, no secrets, no
  session. It renders pages, serves the hashed assets, `sitemap.xml`,
  `robots.txt`, the manifests and `sw.js`. It never reads or forwards the
  `__Host-central` cookie; Caddy strips `Cookie` before it. Signed-in data is
  read by the islands in the browser from `/api/v1` exactly as today, so the
  session never passes through it and a compromise of `central-web` is no
  worse than a compromise of today's static files.
- **Public data, fetched by the server:** GitHub's releases, CI run, newest
  commit and repository (10-minute cache, GitHub's unauthenticated budget is
  60 an hour per address), the card status at the newest release's tag
  (cached per tag, which never changes), Central's directory (60-second
  cache, through the browser listener on loopback with the site's own
  `Origin` and `X-Forwarded-For: 127.0.0.1`). Outbound HTTPS uses the
  approved `ureq` + Rustls, as `central-push` does; only
  `api.github.com/repos/AceVik/baylee…` and
  `raw.githubusercontent.com/AceVik/baylee/…` are allowed hosts. When GitHub
  does not answer, the last good answer, else the deploy-time copy
  (`npm run content` today, `cargo xtask site --content` then) is used, and
  the page says how old it is.
- **Crates:** `central-web` (the components, pages and islands; one crate for
  both targets, Leptos' `ssr` or `hydrate` chosen by `cfg(target_arch)`, so
  the gate's `--all-features` never enables both), `central-web-server` (the
  binary: Axum, configuration, fetchers, static files),
  `central-web-islands` (the `cdylib` entry). All source-available;
  `central-web` uses `central-api` for the wire types (as `central-service`
  does today). Islands live in private modules.
- **Words:** the Fluent files stay the source (translators keep their
  format); a first-party build script turns the subset in use into typed Rust
  functions, so a missing message or argument does not compile, nothing is
  parsed at run time and the islands carry only the messages they use.
- **Assets:** CSS, fonts, images, scripts and the islands bundle get content
  hashes in their names (`/assets/<name>.<hash>.<ext>`, immutable for a year,
  SRI on every script and stylesheet tag), precompressed with Brotli and gzip
  where a compressor is available; HTML is compressed by Caddy (zstd, gzip).
- **Headers:** `Content-Security-Policy: default-src 'self'; script-src 'self'
  'wasm-unsafe-eval'; style-src 'self'; img-src 'self'; font-src 'self';
  connect-src 'self'; worker-src 'self'; manifest-src 'self'; object-src
  'none'; base-uri 'none'; form-action 'self'; frame-ancestors 'none';
  require-trusted-types-for 'script'; trusted-types 'none'` (Trusted Types
  verified in the spike; kept only while every island passes the browser
  tests), `Cross-Origin-Opener-Policy: same-origin`,
  `Cross-Origin-Resource-Policy: same-origin`, `Referrer-Policy: no-referrer`,
  `X-Content-Type-Options: nosniff`, `Permissions-Policy` as today, HSTS from
  Caddy. No inline style or script anywhere (a test parses every page).
- **Status codes:** real 404 with a helpful page, 410 for removed paths, 301
  for moved ones, 302 by `Accept-Language` for unprefixed paths (§5).

## 4. Requirements and how each is met

### 4.1 SEO

One `h1` and an ordered outline per page; per-page `<title>` and
description; canonical URL; `hreflang` en, de and `x-default`; breadcrumbs
with `BreadcrumbList`; `SoftwareApplication` (the game, with the release's
version and operating systems), `Organization`/`WebSite` on the home page,
`FAQPage` on the FAQ; OpenGraph and Twitter cards with a real preview image
per section (1200 × 630, made from the painting and the page's title at build
time with `resvg`, committed); `sitemap.xml` with `lastmod` (the content's
own date: release date, card status `generated_at`, a legal text's `updated`);
`robots.txt` keeping today's AI-crawler refusals and TDM reservation; fast
TTFB (no database, cached data); images as AVIF and WebP with `width` and
`height`; the first-paint fonts preloaded. Search-console verification only by
deploy configuration (a file or meta value from `central-web`'s config, never
committed). Targets: Lighthouse 100 for SEO, Accessibility and Best
Practices, at least 95 for Performance; LCP < 2.5 s, INP < 200 ms, CLS < 0.1.
The Rich Results Test is Google's online tool and needs a public URL: the
JSON-LD is checked against schema.org in our tests, and the owner runs the
Rich Results Test on staging.

### 4.2 Accessibility and keyboard

WCAG 2.2 AA: semantic HTML first; skip link; landmarks; visible two-part
focus ring (gap and ring, 3:1 on every surface); no traps (native `<dialog>`
with focus return); roving tabindex in the conversation list, the card grid
and tab lists; Escape closes what it opened; a polite live region for new
messages and an assertive one for failures; form errors tied with
`aria-describedby` and an error summary that takes focus; 44 × 44 px targets;
nothing on hover alone; 200 % zoom and 320 px without sideways scrolling;
`prefers-reduced-motion`, `prefers-color-scheme` and `forced-colors`.
Chat shortcuts and their help overlay: §6.3. Checked by axe-core in the
Playwright suite on every page in both themes and both widths, Lighthouse,
and a written keyboard-only walkthrough in the PR.

### 4.3 Third parties, DSGVO, licences

The browser requests nothing from any other origin: GitHub data comes from
the server (§3), downloads are plain links to GitHub (a navigation the
visitor chooses), the contact form posts to `/site/contact` on this origin
(Caddy passes it to the feedback service; needs `https://baylee.app` in
`FEEDBACK_CONTACT_ORIGINS`), fonts and icons are served from here. **Card
images:** Scryfall's terms forbid proxying their images ("You may not simply
repackage, republish, or proxy Scryfall data", quoted in Baylee's
`docs/legal.md`), and the owner's rule forbids loading them from Scryfall in
the visitor's browser, so the card lists show names, statuses and a link to
each card's Scryfall page, without pictures (owner decision 1). Every
dependency, font, icon and image is recorded with its licence in a new
`NOTICE.md`; icons are Lucide (ISC), drawn inline as SVG.

### 4.4 Legal texts

German and English, rewritten for clarity and checked against what the code
does (§1.7):

- **Impressum** (§ 5 DDG): rendered by the server from a deploy-time file
  named in `central-web`'s configuration (never committed, never logged), as
  plain text, `noindex`. This replaces the proof of work and the scrambled
  drawing, which are not WCAG AA (owner decision 2).
- **Privacy notice** (Art. 13 GDPR): one section per processing — hosting
  and logs, the session cookie and the browser storage keys (each named), the
  account data, the chat (what Central can see: contacts, groups, times,
  sizes, device keys; not the messages), Web Push (the three providers by
  name, what a push carries), the contact form, the per-address counters,
  the server's requests to GitHub, rights and the authority. Purpose, legal
  basis and retention for each. A draft for the owner's lawyer, as before.
- **Terms of use:** `legal/terms.{de,en}.md` are compiled into Central;
  their wording may be clarified but the version stays `draft` (publishing a
  version is the owner's step).
- **Fan Content notice:** Wizards' own wording on every page, quoted from the
  policy page (last updated 15.11.2017):
  „Baylee is unofficial Fan Content permitted under the Fan Content Policy.
  Not approved/endorsed by Wizards. Portions of the materials used are
  property of Wizards of the Coast. ©Wizards of the Coast LLC." and its
  official German form „Baylee ist ein inoffizieller Fan-Inhalt im Rahmen der
  Richtlinie für Fan-Inhalte. Nicht von Wizards gefördert/gesponsert.
  Bestandteile des enthaltenen Materials sind Eigentum von Wizards of the
  Coast. ©Wizards of the Coast LLC.", plus a page explaining it.

### 4.5 Responsive and mobile

Mobile first, `min-width: 320px` on the page, container queries where a
component decides, safe-area insets, the composer above the on-screen
keyboard (`visualViewport` and the `interactive-widget=resizes-content`
viewport), a bottom navigation within thumb reach on phones. Checked at 320,
375, 768, 1024, 1440 and 2560 px, portrait and landscape, in the browser
tests.

### 4.6 Styling and fonts

Design tokens as custom properties (§6.1), `rem` for layout and type, `em`
for what scales with its own text, `px` only for hairlines, fluid type with
`clamp()`, one spacing scale, logical properties, container queries, light
and dark from the same tokens, no inline styles. Fonts: the game's pair,
Alegreya Sans (interface) and Faustina (prose), SIL OFL 1.1, the existing
subset `woff2` files, served from this origin, `font-display: swap`, the two
first-paint faces preloaded, the licences beside them and in the notices.

## 5. URLs

- **Every page carries its language:** `/en/…` and `/de/…`, with English
  slugs in both (eleven languages are planned; translated slugs would
  multiply redirects and break the client's links). Without cookies and
  before any script, only the URL can carry a visitor's language.
- **Unprefixed paths** (`/`, `/download`, `/account/chat`, `/register`, …):
  302 to the language the browser prefers (`Accept-Language`, `Vary`), so
  every existing link, the client's `/register`, the PWA's `/account` and the
  notification targets keep working. `?lang=` on an old link is honoured
  once by that redirect.
- **hreflang** names both languages and `x-default` (the English page);
  canonical is the page itself.
- **Map:** `/{lang}/` home (with the card numbers and the album behind the
  big logo, §10); `download`; `gateways` (with how to host one); `faq`;
  `security` (four diagrams in steps, [its claims](../security-page.md));
  `contact`;
  `impressum`, `privacy`, `terms`, `fan-content`; `signin`, `register`,
  `reset`; `account` and its sections (§6.2); `pair`; `admin…` as today.
- **Old fragments:** a set used to be `/cards#set-<code>`. A fragment never
  reaches the server, so the cards page keeps an element with
  `id="set-<code>"` on each set's row, linking to `/{lang}/cards/<code>`: an
  old link lands on the right row without script.

## 6. Design

Weighed from a design consultation with Fable 5.2 (10.11.2026); the decisions
below are this proposal's, recorded with their reasons.

### 6.1 Visual language: "one garden, two hours"

*Superseded on 11.11.2026.* The owner, having seen the result: "Mir hat das
Design der alten React Webseite deutlich besser gefallen." The site now
takes the old site's look and its functions again, and the code is
rewritten, not copied:
- the logo, the header with its dropdowns, the hero with the split
  download button, the glass panels, the pill buttons and the footer;
- the chat in the old look, with the new and better UX;
- Font Awesome allowed;
- animations welcome, but no hover changes an element's size or place (a
  test over the stylesheets checks this).

What follows was the redesign this replaced, kept as the record of the
change.

Kept: sunlight by day and the blue hour by night, the owner-approved
painting, the game's two typefaces. Changed: from glass panels, deep shadows,
pill buttons and centred symmetry to calm paper-like surfaces separated by
hairlines, a shadow only on what floats, start-aligned text wherever one
works (centred only in the home hero and the remembrance), and one ornament,
the *horizon*: a thin rule under each page head that fades from sun gold.

- **Gold is the one thing to do in both themes** (today the light theme's
  "gold" is teal). Teal (day) and cornflower (night) are the text accent for
  links, selection and the unread dot; they never fill a button.
- **Tokens by meaning** (light / dark), contrast measured with WCAG relative
  luminance: `--ground` `#f7f3ea` / `#0f1b2d`, `--ground-alt` `#ecefe6` /
  `#13223a`, `--surface` `#fffdf7` / `#182943`, `--surface-raised` `#f0eee3`
  / `#213556`, `--bubble-own` `#dcebe6` / `#24406a`, `--ink` `#1f3a40` /
  `#eef1f8` (10.9 / 15.3 on ground), `--ink-soft` `#3f5a5d` / `#cbd5ea`,
  `--ink-faint` `#52696a` / `#a5b4d1` (≥ 4.8 everywhere it is drawn),
  `--title` `#143840` / `#fff6e2`, `--action` `#e9c46a` / `#e6c27f` with
  `--on-action` `#2a1d04` / `#1a1306` (9.9 / 10.9) and, in light, an
  `--action-edge` `#8a5a05` border (3.6 against the fill, 5.4 against the
  ground), `--accent` `#1a5f6e` / `#a9c4ff`, `--focus` `#1f5c68` / `#ffe7a6`
  drawn as a 2 px gap in `--ground` and a 3 px ring, `--edge` `#6f8583` /
  `#7488ad` (≥ 3.4), states `--ok` `#1c7245` / `#7fd3a1`, `--warn` `#9a4700`
  / `#f5a35c` (orange, so it is never mistaken for the gold), `--bad`
  `#a3302a` / `#f5a097`, each with a tint for notices. The contrast test
  measures every pair in both schemes.
- **Type:** Faustina for public page and section titles and long prose,
  Alegreya Sans for every control, the whole signed-in area and messages;
  body from 17 px (Alegreya Sans has a small x-height); scale `--text-xs`
  0.8125rem to `--text-3xl` `clamp(2.25rem, 1.5rem + 3.4vw, 3.75rem)`, each
  clamp mixing rem and vw so zoom still scales; prose measure 34–38rem;
  tabular figures for numbers in tables and times.
- **Space** 0.25–6rem in nine steps, `--gutter` `clamp(1rem, 0.5rem + 2.5vw,
  2rem)`; **radii** 0.5 / 0.875 / 1.25rem and full; buttons become rounded
  rectangles; four elevation steps; motion 120 / 180 / 240 / 320 ms, opacity
  and transform only, all still under reduced motion. The scroll-reveal and
  stagger are retired (decoration that needs script).
- **Imagery:** the world painting full-bleed in the home hero only (a still
  picture: the WebGL cloud simulation is not ported, owner decision 4), the
  cloud portrait beside sign-in and register and as the header mark, the
  remembrance's photographs only on its page, the client screenshots as
  captioned figures. No picture behind forms, lists or the chat. Icons:
  Lucide only, always with a label.

### 6.2 Information architecture

- **Public bar** (from 60rem): mark, Download, Cards, Gateways, Baylee,
  Contact; then language, theme (script only) and Sign in or the handle.
  **Phones:** a slim top bar and a bottom bar with Home, Download, Cards,
  More; "More" is a link to the footer's menu, a sheet with script.
- **Footer:** Play, About, Legal; the languages; the Fan Content notice;
  the dedication.
- **Signed-in frame** (one for account and administration): sidebar from
  64rem (Home, Chat, Contacts; Settings: Profile, Security, Notifications,
  Gateways & tokens, Privacy & data; Downloads; Administration for
  administrators; Sign out), a labelled rail from 40rem, a bottom bar on
  phones (Home, Chat, Contacts, More). Paths `/{lang}/account`,
  `…/account/chat`, `…/account/chat/<conversation>`, `…/account/contacts`,
  `…/account/profile`, `…/account/security`, `…/account/notifications`,
  `…/account/gateways`, `…/account/privacy`.
- **Account home:** the handle with copy; "needs you" (no recovery code, new
  terms, contact requests, invitations, changed keys); unread chats; the
  game client's sessions; the newest version; a getting-started checklist.
- **Terms acceptance** is an interstitial inside the frame; withdrawing
  notifications stays reachable.

### 6.3 The chat

- **Layout:** one pane at a time below 48rem (the open conversation takes
  the whole dynamic viewport, the bottom bar hides, Back in the head);
  list beside conversation from 48rem; details (group, safety number) as a
  sheet, docked from 90rem; the frame capped at 100rem. The pane decides by
  container query.
- **Conversation list:** a `nav` with a list of links (real URLs), one Tab
  stop with roving focus (Up/Down, Home/End, Enter), Right to a row's menu;
  invitations first; unread in weight, a dot and the number in words; search
  reached by Ctrl/Cmd+K.
- **Messages:** an ordered list with a heading per day; each message's hidden
  prefix says who and when; "Load older" keeps the reading position; an
  unread divider drawn once; a "3 new messages" pill when scrolled up. Not a
  live region; announcements go through one polite region (incoming in the
  open conversation, at most one every 3 s, bursts summarised; others by name
  only) and one assertive region (not sent, keys changed).
- **Composer:** grows to six lines; Enter sends, Shift+Enter breaks a line,
  Ctrl/Cmd+Enter always sends; on touch keyboards Enter breaks and the button
  sends; IME composition never sends; the counter appears from 900 of 1000
  characters; a failed send keeps the text and offers "Try again".
- **Shortcuts** (screened against browsers, AltGr, macOS Option and screen
  readers): Ctrl/Cmd+K find; Alt+Up/Down previous/next conversation;
  Alt+Shift+Up/Down previous/next unread; Escape closes, else composer → list;
  outside fields `j`/`k`, `u`, `r` (composer), `n` (new), `i` (details), `?`
  (help), with one switch that turns single keys off (WCAG 2.1.4). The Alt
  pairs are verified with NVDA and VoiceOver before they ship, else dropped;
  every shortcut has a visible control.
- **Changed keys:** the composer is replaced by a notice ("compare the safety
  number before you write again"); the dialog shows the number in groups of
  five with "I compared it and it matches" and "Accept new keys"; history
  stays readable.
- **Groups:** a details panel (rename, members, invite from contacts,
  pending invitations, leave or delete, owner transfer) where every
  irreversible step names the group and uses a verb.
- **Unread** stays per device and in memory (the protocol has no read
  state); the title carries the count.

### 6.4 Onboarding and empty states

Registration in one column with the 1–2 s of Argon2 announced ("Securing
your password…"), the recovery code as a full-page step with copy, download
and "I have saved my recovery code"; the account home's checklist; empty
states that say the next step (no contacts: your own handle to share; no
gateways: most players never need one); push unavailable named by its real
cause; an offline strip. Without script, sign-in and register explain that
the password is processed in the browser.

## 7. PWA

Kept compatible with what is in users' browsers: `/sw.js` at scope `/`,
manifest `id "/"`, the push payload, category bits, IndexedDB and storage
names (§1.6), and deletion of old `baylee-public-*` caches. New: one manifest
per language (`start_url` `/en/account` or `/de/account`), an offline page in
the cache, the install offer after `beforeinstallprompt`, and an update prompt
that warns about unsent input before reloading. The cache still holds only
public assets.

## 8. Phases on this branch

1. **This proposal** and the dependency proposal. *Stop:* the owner accepts
   or refuses the graph and the three exceptions (§2.2) and takes the
   decisions in §10.
2. **Server and content pages:** the crates, `cargo xtask site`, the design
   system, all public pages with SEO, the legal texts, the browser tests with
   axe-core, Lighthouse.
3. **Sign-in and the signed-in area:** register, sign-in, terms, recovery
   code, display name, profile, security, sessions, gateways and tokens,
   notifications, privacy (export, deletion); then the new crypto exports
   (password change, reset) in their own reviewed commit.
4. **Chat and PWA.**
5. **Administration** at parity, including feedback triage. Until then
   `web/` stays and Caddy serves `/admin*` from its build; `web/` (and its CI
   job, the deploy hook's npm step and `cargo xtask wasm`'s output path) is
   removed only when nothing needs it.

Each phase keeps `cargo xtask gate`, the policy, `crypto-scope`, audit and
deny green and is one or more commits. A wasm32 clippy and test step joins the
gate (it builds only natively today). Merging and deploying stay with the
owner; the deploy hook, a systemd unit for `central-web` and the Caddy block
are changed in the branch and documented in [operations](../operations.md).

## 9. APIs a useful feature lacks

Not changed here; listed for the owner:

- **Presence** ("online") for contacts: no API (the gateway owns game
  presence).
- **Read state across devices:** unread counts exist only per device.
- **Renaming the display name** after it is set (`set_display_name` sets it
  once).
- **E-mail and two-factor authentication:** none in Central; OPAQUE plus the
  recovery code are the only factors. A second factor needs a design first.
- **A list of the account's devices** (Central keeps no device registry;
  sessions only) and signing out every session at once from the website (the
  password change ends them all; there is no separate operation).
- **The language preference** of an account (the account document is
  end-to-end encrypted, so the server cannot read it).
- **Gateway terms** on the website (`gateway_terms` is refused by the browser
  listener).
- **Attachments** in the chat.

## 10. Owner decisions (10.11.2026)

1. **Leptos with the three exceptions: yes** ("1. Ja wählen.").
2. **Impressum:** the proof of work stays (it is not what conflicts with
   WCAG); what changed is the drawing: once solved, the details are ordinary,
   accessible text. The data stays where it was, in the feedback service's
   deploy-time blob.
   Repaired on 11.11.2026 (the owner asked for the protection to be mended
   and improved): the rewrite had lost two of the service's three aids, and
   no test held them. Back, and held by `pages.rs` and
   `web/e2e-site/impressum.spec.ts`: the bait link to
   `/site/impressum/kontakt` (hidden from people by `hidden`, a class,
   `aria-hidden` and `tabindex="-1"`; whatever follows it gets invented
   details for a day) and the third yes-or-no answer about the browser (a
   software WebGL renderer, asked only after the click). New: no `mailto:`
   or `tel:` address stands in the page (two buttons build them when
   pressed), and the e-mail address and telephone number are each written
   in two halves around a few characters under `hidden`, so that the page
   read as markup or `textContent` yields an address under `.example` and a
   number with two digits too many, while sight, screen reader, selection
   and copy get the exact text. That last part stops collectors that read
   any page, nothing written for this one: the blob's key is published in
   the script. Tried once end to end against Baylee's real service on
   loopback with made-up details (17 bits for a person's browser, 20 to 21
   for a driven one, the decoy after the bait and for a headless browser).
3. **Cards:** the card lists and pictures go entirely. Only the numbers
   remain: fully implemented, generated, partly implemented and all cards,
   each with an explanation (a toggletip, no script). `/cards` and its
   sets send to the start page's numbers.
4. **The WebGL cloud simulation goes**: a still painting (why it could have
   stayed: it needs script or its own WebAssembly, runs on the main thread
   all the time, and needs a pause control under WCAG 2.2.2).
5. **Contact form:** "do what is right": through `/site/contact` on this
   origin, as `/site/impressum` already is; no request to another origin.
6. **Pairing:** build the page now and teach Baylee's client the link (a
   separate session in the Baylee repository).
7. **No separate remembrance page:** the album opens when the big logo is
   pressed (a dialog, as before).
8. **Fan Content notice in an open conversation on a phone:** in the
   conversation's details, "nicely and not bulky".

Relayed later the same day by the parent session:

9. **Leptos 0.9 if viable, else 0.8.23.** 0.9.0 exists only as
   `0.9.0-rc1` (31.10.2026), a pre-release whose notes announce "a wide
   variety of semver-breaking changes" and ask for reports of breakage,
   with changes to hydration; `0.8.23` of the same day is the release
   marked latest and still receives backports. So **0.8.23 stayed**, with
   the three gate exceptions as approved. *Superseded on 11.11.2026:* the
   owner said "upgrade auf leptos 0.9 (auch wenn es noch ein rc ist)", and
   the site now runs on `0.9.0-rc1`. The new graph, its inventory and the
   changes it needed are in `../dependency-proposal/leptos.md`
   §"0.9.0-rc1". The owner's condition on server functions is met
   trivially: the site has none (no `leptos_axum`, no server functions);
   `central-web` answers `GET` and `HEAD` for pages, hashed assets and
   generated files only, and every form posts to Central or, through Caddy,
   to the feedback service.
10. **Open beta.** The site speaks of the open beta: register freely,
    download, play; no keys, invitation codes or closed beta in it. The
    owner allowed removing the closed beta's keys from Central too, on
    conditions: the API stays compatible for the client and the gateway;
    what goes is listed here and in the PR; data changes only by a
    migration that deletes nothing else; and the replacement protection
    is in place before the key stops being one. The replacement is the
    owner's design of 28.10.2026 (`docs/registration-abuse.md`): Central's
    own proof of work, harder as registrations flood in, the per-address
    budgets with a daily one, a honeypot and a minimum fill time. The
    game repository's part (the gateway's `invite` mode, the feedback
    console, `baylee-ops invite`) is another session's.
11. **See it running.** After each phase the site runs locally (a local
    database and test accounts, never production) in the browser pane
    until the owner has finished looking.

## 11. As built: phase 2 (server and content pages)

- `crates/central-web` (pages, islands, the words), `central-web-server`
  (the binary), `central-web-islands` (the module); `cargo xtask site`.
- The words: `locales/<lang>/site.ftl` turned into typed functions by
  `build.rs` (332 messages each; a missing message, an argument English does
  not have, or other tags than English fail the build).
- Phones: below 40rem only the page scrolls and the bottom bar sits below
  it, so it never covers a control (axe's target-size rule found that a
  sticky bar does); 40–60rem: a menu button in the bar opens the same sheet.
- Fonts subset further to Latin-1 and the punctuation used (−35 % for
  Alegreya Sans), declared with `unicode-range`.
- Trusted Types are enforced (`trusted-types baylee`): the islands start
  Workers only through the `baylee` policy, which vouches for the site's own
  hashed scripts alone.
- The theme switch, the invoker-command fallback and the account link's
  name are `boot.js`; dialogs, menus and explanations are HTML
  (`<dialog>`, `popover`, invoker commands) and work without script.
- Tests: `central-web` (words, markdown, data, contrast of every token pair
  in both schemes), `central-web-server` (every page in both languages:
  one `h1`, canonical, hreflang, OpenGraph, JSON-LD that parses, no inline
  style or script, sized pictures, the Fan Content notice, nothing from
  elsewhere; addresses and status codes; the download by machine), and
  `web/e2e-site` (Playwright, CI job `site`).

**Measured (10.11.2026, local, Lighthouse 13.5.0, behind gzip as Caddy
encodes):**

| Page | Mobile perf. | LCP (mobile) | Desktop perf. | A11y | Best pr. | SEO |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `/en/` | 96 | 2.8 s | 100 | 100 | 100 | 100 |
| `/en/download` | 99 | 1.8 s | 100 | 100 | 100 | 100 |
| `/de/contact` | 97 | 2.4 s | 100 | 100 | 100 | 100 |
| `/en/gateways` | 99 | 1.8 s | 100 | 100 | 100 | 100 |
| `/de/faq` | 99 | 1.9 s | 100 | 100 | 100 | 100 |
| `/de/impressum` | 97 | 2.6 s | 100 | 100 | 100 | 69 (`noindex` on purpose) |

Mobile is Lighthouse's simulated slow 4G; the start page's LCP is the
painting. CLS ≤ 0.014 everywhere. The Rich Results Test needs a public
address (staging).

## 12. As built: phase 3 (sign-in and the signed-in area)

- **Islands** `SignIn`, `Register`, `Account` (one per section page:
  overview, profile, security, privacy and data, gateways and tokens). The
  server renders the frame, the headings and the navigation and never sees
  the session; the islands talk to `/api/v1` with `central-api`'s own
  request and response types (strict parsing, refusals by status only).
- **Crypto**: `crates/central-web/js/crypto-worker.js`, a module Worker
  (started only through the Trusted Types policy) that loads the committed
  `central-web-crypto` module unchanged; `cargo xtask site` checks its bytes
  against `SHA256SUMS` before shipping them. The page sends the password
  and bytes and receives bytes and words; the master key stays in the
  Worker. The kept chat identity (IndexedDB `baylee-chat`/`kept`) and its
  format are unchanged.
- **Flows**: sign-in (discovery, OPAQUE, the master key opened in the
  Worker: checked, or a recovery code made for a moved account, or the chat
  identity kept), registration (beta key, names checked with central-api's
  rules, enrolment, the recovery code shown once with a download and "I have
  saved it", then a sign-in), Central's terms (texts must hash to the
  version's digest before "Accept" is offered), choosing the display name,
  sessions and signing the client out, signing out (forgetting the kept
  identity), the data export (JSON download, re-sign-in when the last one is
  older than 30 minutes), account deletion (the password proven in the
  Worker with the existing `login_start`/`login_finish`, then a last
  confirmation within Central's 60 seconds), gateways (register, tokens
  made after a fresh sign-in and shown once, revoked).
- **Not yet** (each needs new exports in `central-web-crypto`, i.e. a
  change to the committed, byte-checked module with its own review and the
  canonical rebuild): approving a pairing. The password change and the
  reset with a reset code came on 12.11.2026 (§16). Notification settings
  come with the PWA (phase 4).
- **Tests**: unit tests of the form rules and term selection; server tests
  that every form and account page is an island page, `noindex`, one `h1`;
  `web/e2e-site/account.spec.ts` drives a sign-in through the real
  WebAssembly in its Worker against a stand-in Central that cannot answer
  OPAQUE truthfully (the Worker must refuse, the page must say "wrong
  sign-in", the password must appear in no request), the registration's
  early refusals and the signed-out account area; axe-core and the 44 px
  check now cover the forms and the account pages too.
- **Not verified here**: a full sign-in and registration against a real
  Central (`web/e2e-real` describes that setup: PostgreSQL 18 with TLS,
  Central in its deployed mode, a TLS stand-in for baylee.app). It is the
  first thing to run on staging before this replaces the old pages.
  *Done in phase 4*: `web/e2e-real/site-chat.spec.ts` (§13).

## 13. As built: phase 4 (the messenger and the app)

**The messenger** is one island for `/{lang}/account/chat` and
`/account/contacts` (`crates/central-web/src/islands/chat/`), so moving
between conversations and contacts never restarts the Worker or replays
the inbox.

- *Opening.* The Worker resumes the identity the sign-in kept
  (`resume`); where nothing is kept, a sign-in inside the chat's own Worker
  opens it (`sign_in_with`, then `unlocked`), which also works without
  IndexedDB. `boot.js` deletes `baylee-chat` whenever no session is live.
- *Work.* One job at a time (start, inbox, keys, send, try again, confirm),
  as the previous site's serial Worker; the inbox every 2 s in view, at once
  while `more`, backing off to 30 s; a quiet read changes nothing on the
  page. Central's 401 ends the chat and forgets the kept identity.
- *Checks* beyond `central-api`'s strict types, ported from `chat/api.ts`,
  `social/api.ts` and `chat/groups.ts`: base64 lengths and alphabets, at
  most a page, a cursor that only moves forward, keys only for the tags
  asked, an acceptance naming the sent id and conversation (the signed
  header), contacts and groups bounded and unique. A message that differs
  from one already shown under its id is refused, never replaced.
- *Sending* keeps the sealed envelope for "Try again" after re-checking
  members and keys; Conflict gives the text back to the composer; changed
  keys replace the composer until the safety number (groups of five) is
  compared and confirmed.
- *Keyboard.* The list is one tab stop (arrows, Home, End, Enter, real
  links), Ctrl/Cmd+K finds, Escape closes the details or returns from the
  composer to the list, Back returns to the row it came from, Enter sends
  (Ctrl/Cmd+Enter always; on touch keyboards Enter breaks the line). Days
  are headings, each message has a hidden "who, when", and one polite
  region announces arrivals at most every 3 s.
- *Not built from §6.3*: the single-key shortcuts and the Alt pairs (they
  were to ship only after a check with NVDA and VoiceOver, which this branch
  could not make; WCAG 2.1.4 favours shipping none), the "3 new messages"
  pill, the unread divider, and hiding the bottom bar while a conversation
  is open on a phone (the bar stays; the conversation fills the rest).
- *The Fan Content notice* (owner, 10.11.2026: "nicely and not bulky"): one
  quiet line with a link at the foot of a conversation's details.
- Icons in islands are drawn element by element: Trusted Types allow no
  `innerHTML` in the browser, and an island's view runs there. For the same
  reason an island holds no "inert" element: Leptos builds an element whose
  attributes and children are all literal (`<label for="x">Website</label>`)
  through `innerHTML` on a `<template>`, which works when the server
  rendered it and throws once the browser renders it again. Literal text
  goes in braces (`{"Website"}`), and `crates/central-web/tests/inert.rs`
  scans the islands and the views they share for any such element.
- Where an island replaces the part that held the focus, the new part takes
  the focus where it begins (`islands/focus.rs`). Before, the next Tab went
  on at the page's end and skipped, for one, the recovery code. This covers:
  - a form replaced by its next step;
  - the Impressum's button replaced by the details;
  - the contact form replaced by its thanks;
  - in the account area: re-auth, the deletion steps, the export, a new
    token and its gateway, the registration switch and the chosen display
    name;
  - push switches, which are marked busy instead of disabled while they
    save.

  Found by the keyboard walk of 11.11.2026 and kept by the browser tests.
- The gateway cards' field ids came from the origin's length, so two
  equally long origins shared their labels' targets. They now come from
  the gateway's id, and a test checks the page's ids are unique.

**The app.**

- `/sw.js` keeps one cache per build (`baylee-site-<stamp>`; `cargo xtask
  site` stamps it from every asset's address): the offline page in both
  languages and the files it needs, then hashed assets as fetched. Pages
  come from the network, and only without one the offline page answers in
  the address's language. `/api` and `/site` are untouched. A new build
  waits for the visitor; `baylee-public-*` caches go.
- `pwa.js` (a module with SRI on every page, the fifth script beside §2.3's
  four) registers it through a Trusted Types policy of its own,
  `baylee-sw`, that vouches for `/sw.js` alone (the CSP now names
  `trusted-types baylee baylee-sw`). It offers a waiting version, warns once
  about unsent text before reloading, and shows "Install as an app" in the
  account area after `beforeinstallprompt`. It registers the worker only
  for someone signed in, for the installed app (`display-mode`) and for a
  device whose notifications are on (the nonce in localStorage); for anyone
  else it removes a worker left from earlier and its caches, so a reader
  gets nothing stored (owner's decision, 11 November 2026). The removed
  worker goes on answering for the page that removed it, so the page
  tells it to retire (a `RETIRE` message): the worker leaves every
  request to the network from then on, waits for what it was writing and
  takes its caches away itself; the page removes the caches too, after
  the worker's word or three seconds (12 November 2026: a fetch under way
  while the cache went had made it again until the next page). The
  removal itself waits for nothing: the previous site's worker never
  answers. While such a page stays open, a browser hands the same worker
  to a page that registers it again (signed in in another tab), retired
  and with no cache, and nothing would install it anew before the next
  build; so a page that wants the worker says so (`KEEP`), and the worker
  caches again and fills its cache if it has none.
- `/{lang}/offline` (200, not indexed) and
  `/{lang}/account/notifications`: Web Push on this device and the
  account's categories, the protocol, bits, nonce and storage names
  unchanged. The browser's push interfaces are reached through
  `js_sys::Reflect`, so no `web-sys` feature was added. Sign-in,
  registration and sign-out turn this device's push off first, as before,
  and that never waits for a service worker the browser refused.
- The islands start after the page's `load` event, so their WebAssembly
  (2.75 MB, about 600 KB with Brotli, with the messenger) never competes
  with the first paint of the text.

**Found and fixed on the way.** A new account's recovery code is shown
before Central's terms again (phase 3 had them first, so declining lost the
code). Signed in, the bar's "Your account" pushed every page sideways
between 60 and 72rem. The contacts' section headings skipped a level.
Rendering the whole messenger on every inbox read (it lost composer text
and focus, and trapped in WebAssembly) became memos that pass only
changes.

**Measured (11.11.2026, the final build, local, Lighthouse 13.5.0, behind
gzip):**

| Page | Mobile perf. | LCP (mobile) | Desktop perf. | SEO |
| --- | ---: | ---: | ---: | ---: |
| `/en/` | 95 | 2.9 s | 100 | 100 |
| `/en/download`, `/en/gateways`, `/en/faq` | 99 | 1.7–1.8 s | 100 | 100 |
| `/en/contact` | 99 | 2.0 s | 100 | 100 |
| `/de/impressum` | 99 | 2.0 s | 100 | 69 (`noindex`) |
| `/en/signin`, `/en/register` | 99 | 1.8 s | 98, 100 | 66 (`noindex`) |
| `/en/account`, `/de/account` | 98–99 | 1.8–2.0 s | 100 | 66 (`noindex`) |
| `/en/account/chat`, `/de/account/chat`, `/en/account/contacts` | 98–99 | 1.7–1.8 s | 100 | 66 (`noindex`) |

Accessibility and Best Practices are 100 everywhere. CLS is at most 0.052
and TBT 0 ms. The account pages are measured signed out.

Three changes got there from 81 for register, chat and contacts and 94 for
the overview (CLS 0.134):
- The islands start after the first frame as well as after load. The
  WebAssembly fetch then never precedes the first paint, which had put it
  on the simulated critical path.
- The account column holds the window's height from the start, so the
  footer no longer jumps when the island's content arrives.
- The messenger's page renders a lead on the server. It stays one element
  until the messenger opens, so the largest paint never waits for the
  WebAssembly.

**The old site's look (11.11.2026, owner decision of that day; §6.1).**
Every page was ported to the old React site's design, rewritten rather
than copied: the start page (status strip, hero with the memorial logo,
screenshots with their dialog, gateways, what Baylee is, the opponents,
the card numbers, the contact form, the open-source promise), Baylee's
album as the old book, the download page with its platform cards, the
gateway directory with finding and "Run your own", the contact page with
"Found something wrong?", the reading pages with their eyebrows and the
privacy notice's contents, sign-in and register beside Baylee's cloud,
and the account frame. The old texts came along word for word where the
old pages had them. Motion was added where it does not move a box on
hover (tests/hover.rs): cards rise into view on a scroll timeline, a
sheen slides across a card under the pointer, the cloud floats.

What that cost and how it was won back:
- The islands now also wait for the page's largest paint (a
  `PerformanceObserver`, a second at most), and islands far down a page
  (the start page's form) for the reader to come within a screen of one,
  or to reach into one. The start page fetches no WebAssembly at all
  until then.
- The start page's screenshots are drawn by `home.js` as their cards come
  near (a `<noscript>` picture without script): six stills inside the
  browser's lazy-loading reach delayed a phone's painting by about 250 ms.
  Their recordings (animated AVIF) load the same way.
- The stylesheet is built without comments, blank lines and indentation
  (24.5 to 19.1 kB gzip).

**Measured (11.11.2026, the old site's look, same setup):**

| Page | Mobile perf. | LCP (mobile) | Desktop perf. | SEO |
| --- | ---: | ---: | ---: | ---: |
| `/en/` | 96–98 | 2.3–2.6 s | 100 | 100 |
| `/en/download`, `/en/gateways`, `/en/privacy`, `/en/faq` | 98–100 | 1.6–2.1 s | 100 | 100 |
| `/en/contact` | 98–99 | 2.1–2.3 s | 100 | 100 |
| `/de/impressum` | 99 | 2.0 s | 100 | 69 (`noindex`) |
| `/en/signin`, `/en/register` | 98–99 | 1.9–2.3 s | 100 | 66 (`noindex`) |
| `/en/account`, `/en/account/chat`, `/en/account/contacts` | 98 | 2.1 s | 100 | 66 (`noindex`) |

Accessibility and Best Practices are 100 on every page measured; CLS at
most 0.046 (the start page on a desktop, as the download button's faces
arrive), TBT at most 85 ms. The start page's mobile LCP sits around
Lighthouse's 2.5 s line from run to run (the painting itself, 51 kB of
AVIF, behind the HTML, the stylesheet and three faces on simulated slow
4G).

**Tests.**

- Native units: model, wire checks, base64, times, push.
- `web/e2e-site/chat.spec.ts`: a stand-in Central and a canned Worker.
  Covers hostile markup kept as text, axe and 44 px with a conversation and
  its details open, the keyboard, changed keys and 320 px.
- `web/e2e-site/pwa.spec.ts`: offline in German, the cache's contents, and
  an update served by a server of its own. The update waits, warns, then
  reloads into the new cache.
- `web/e2e-real/site-chat.spec.ts` against a real Central, PostgreSQL 18
  and the committed crypto module, desktop and phone, the service worker
  registered. Two accounts register, become contacts by handle, write both
  ways, reload without a password, compare equal safety numbers, share,
  write to and leave a group, and keep a notification switch across a
  reload; localStorage holds no message and no password. Device Web Push
  itself needs a push service and was not run.

**Open.**

- The pairing page (owner decision 6, "build it now"). The client's half is
  done on Baylee's `c41/pair-link`. The page needs three new exports in the
  committed `central-web-crypto` module (the key check, the HPKE seal to the
  client's key, the approval proof), hence that module's own three-pass
  review and the canonical Linux rebuild.
- Password change and reset on the website (new exports too): built on
  12.11.2026, §16.
- The administration still comes from the old build (§11, Caddy `/admin*`).

## 14. As built: the owner's round of 11.11.2026

What the owner asked for after seeing the old look back, and what came of
it. Each point names where it is held.

**Look.**

- Type: Literata for titles and prose, Inter for the interface, both
  variable, with Cyrillic; Japanese, Korean and Chinese in the reader's
  system fonts ([design](../website-design.md#typography)).
- Links are underlined once the pointer or the keyboard is on them, and
  where one leads is shown in the window's lower left corner (`hint.js`).
- The hero: what Baylee is as badges, Wizards' Fan Content notice at its
  foot, the old site's clouds (`clouds.js`, started when the page is idle,
  not sent to phones). Sections stand closer; panels have layered shadows.
- The bar folds into the menu button up to 1080 px and unfolds from 1081
  px: six sections do not fit beside each other below that in most of the
  eleven languages. From 1081 to 1279 px the language button shows its code
  without the name. `languages.spec.ts` holds one row at 1080, 1081, 1140,
  1279 and 1280 px in every language.
- Titles: the start page's title is one sentence a line (`<nb>` in the
  words), leads balance, and compounds that begin with a figure do not
  break ([design](../website-design.md#typography)).

**Eleven languages.** English, German, French, Italian, Spanish, Brazilian
Portuguese, Russian, Japanese, Korean, Simplified and Traditional Chinese:
the languages Magic is printed in. English and German are written by hand,
the other nine by a machine, which every such page says. The build refuses
a language that lacks anything the English has; a test in `xtask` refuses
a language whose text is still English in 12 % of its messages or more. The legal
texts apply in German and English; Central's terms exist in those two only.

**The security page** (`/{lang}/security`, in the bar, the footer and as
four cards on the start page): signing in, keys in memory, chat encryption
and pairing as diagrams in steps, drawn by the server and moved by the
stylesheet alone. Every sentence has a row in
[its evidence table](../security-page.md); a second reader's ten
corrections are in. A test refuses words that promise ("secure", "safe",
"guarantee") in any language.

**The figures.** Gateways may tell Central how many players, open tables
and games they have (`GatewayPulse`), and Central publishes those counts
and its number of accounts without a credential (`PublicFigures`;
[gateway registry](../gateway-registry.md)). The site's server reads them
with the directory once a minute; the strip under the bar adds them up and
a gateway's card shows its own. A number is drawn only while a source
reports it, and nothing is kept past the minute it was read in. The
gateway's half is Baylee's (a brief is with the project's notes); until a
gateway reports, the strip shows the accounts, the cards, the stars and
the gateways online (the stars left it on 12.11.2026, §19).

**The Impressum** is in §10, point 2.

**The bar** no longer links Discord (owner, 11.11.2026); the start page,
the contact section and the footer do.

**Registration** was reviewed as an attacker would use it and hardened
([registration abuse](../registration-abuse.md), "Hardened"): a challenge
costs more for a request that looks automated (never a refusal), a flood
raises the price to 64 times the base, an address's block is counted, and
the page counts its three seconds from the challenge's arrival, which had
refused people on slow connections. The page shows the proof's progress
once it takes long.

**Signing in** counts guesses at a password
([native login](../native-login.md), "Guessing a password online"): per
account, per address, per account at an address and per IPv6 /48, by the
hour and the day, on both listeners, with every address's own share of
what all callers share. A sign-in with the right password spends nothing,
and an address an account signed in from stays open to it for 30 days
while strangers are refused. The owner accepted on 11.11.2026 what that
tells someone at the same address; the privacy page says it, the security
page names what guessing still finds and who can keep whom out. Nothing
on the wire changed, so the released client and gateway are served as
before.

**The proof of work costs memory** (owner, 11.11.2026;
[registration abuse](../registration-abuse.md), "A proof that costs
memory"): Argon2id at 16 MiB in SHA-256's place, because a script solved
the SHA-256 proof about thirteen times as fast as the page and now solves
this one 1.3 times as fast, one core against one. One Rust function finds
the answer in the page's Worker and checks it in Central, through the
`argon2` crate the password stretch already used; no dependency changed.
Central answers the page's request with a new answer, `RegisterWork`,
takes no SHA-256 solution, and now computes for each registration: one of
its four workers, 16 MiB, a few milliseconds. `Register` is unchanged on
the wire, and the released client and gateway send none of this. The page
solves the proof in a Worker of its own that ends before the password's
starts, and asks a Central from before the older way, so the site is
deployed first and Central after it
([operations](../operations.md#registration)); the other order leaves an
open register page unable to register until it is reloaded. The privacy
page says what the puzzle costs, what its difficulty depends on and that
Central computes part of it again.

## 15. As built: the owner's second round of 11.11.2026

What the owner asked for after the first round, and what came of it. Each
point names where it is held.

**The strip under the bar.** Beside what it had, it shows how many players
are connected, how many of them are in a game, the open tables and the
running games (added up from the gateways that report; the pulse got its
fourth count for this, [gateway registry](../gateway-registry.md)), and how
often the site's pages were opened. That last number is the site's own: a
deployment that names a file (`views`) counts browser navigations into it,
one number for all pages and visitors and nothing about a request
([operations](../operations.md), [security
assurance](../security-assurance.md)); without the setting nothing is
counted or shown, and the privacy notice has its section only where it is
counted. On a phone four counts stand two and two (laid out anew on
12.11.2026, §19).

**Menus.** The download's menu and the language menu lie on a layered
shade, come in with their rows one after the other and leave as they came
(`boot.js` holds a menu open for its leaving animation; without motion it
closes at once). The download's rows name their files, and the menu's head
names the release.

**Signing in and registering** open as dialogs over the page that asked,
where script runs; the pages stay, and are where a link leads without
script or with a modifier key. The dialogs hold the pages' own islands,
taken over only when a dialog first opens, so a visitor who never opens
one loads no WebAssembly. A form in an island that is not yet taken over
does not submit (`data-hydrated`). Fields carry a glyph, and a password's
field an eye inside it (`islands/field.rs`). `dialogs.spec.ts` holds it.

**Light.** The light scheme was set anew in its two blocks: a ground the
cards rise from, bands that differ, edges that show, and every shade in
the scheme's own ink. `tests/contrast.rs` holds AA for it as for the dark.

**Baylee's album.** A page turns as paper does: the leaf is a copy of the
chapter cut into ten strips, each hinged on the one before, so it bends
while its edge lifts, throws a shade on the page below and lies flat where
it lands; going back, the earlier chapter is laid down over the one being
left. A photograph is laid on the one before it. A finger's sideways
stroke turns the chapter or the photograph. With less motion asked for,
page and photograph change at once (`home.js`, `album.css`).

**The gateway directory** has its numbers as tiles (listed, online,
players, of them in a game, open tables, running games), a search over
name and address (Ctrl or ⌘ + K), chips that say how many each would show,
a sort (as listed, name, players, open tables, version), cards or a list,
and an address that says what is shown (`?q=&show=&sort=&view=`), so a
view can be passed on. All of it is `directory.js` over the cards the
server wrote; nothing is stored.

**Movement.** Where the browser can show one state giving way to another
(view transitions): pages fade over as before, a new colour scheme spreads
from its switch, and the directory's cards glide to their places on a
chip, a sort or a change of layout. All of it only where nobody asked for
less motion, and no hover moves or resizes an element as a whole.

**Something for those who look.** Typing "baylee" outside a field lets a
cat walk across the foot of the window (`hint.js`): paw prints, hidden
from a screen reader, in nobody's way, gone after a few seconds.

## 16. As built: a reset code and a password change (12.11.2026)

Central and `central-client` could do both since 28.10.2026; the site could
do neither. Nothing on the wire and nothing in Central changed.

- **The module.** `central-web-crypto` has five more exports, each a
  wrapper around what `central-client` does with `central-crypto`:
  `reset_start`, `reset_keep` and `reset_new` for a reset
  ([account reset](../account-reset.md)), `password_start` and
  `password_finish` for a change ([native login](../native-login.md),
  "Password change"). The committed module was rebuilt in the pinned
  container ([xtask](../xtask.md), "The website's WebAssembly").
- **The reset page**, `/{lang}/reset` (`Page::Reset`, `islands/reset.rs`):
  not indexed, in no menu and not in the sitemap, without the sign-in
  dialogs, and `/reset` leads to it. The sign-in form, on its page and in
  its dialog, links to it with a plain link, and the FAQ's answer about a
  forgotten password does. What it does and does not tell is in
  [account reset](../account-reset.md), "The website's page".
- **The password's change** is a card in the account area's security
  section (`islands/password.rs`): the current password, the new one twice.
  The form gives way to what became of it.
- **Shared.** `field::labelled` draws a labelled field with its glyph, its
  hint and, for a password, its eye; `clip::copier` is a recovery code's
  copy button with the line that says whether the browser took it (the
  reset's new code and the registration's both have it). A step's card is
  as wide as what holds it, not as its longest word, and its heading
  hyphenates: in the registration dialog the German code step had been
  cut off at a phone's width.
- **Words.** English and German by hand; the nine other languages by
  machine (French, Italian and Brazilian Portuguese with the change, the
  six others the same day), with each language's own vocabulary and a new
  word for "vault". No speaker has read them.
- **Tests.** Unit tests of every decision the two forms make, server tests
  of the page and its links, `web/e2e-site/reset.spec.ts` and
  `password.spec.ts` with a stand-in Central and the real Worker, and
  `web/e2e-real/site-reset.spec.ts` and `site-password.spec.ts` against a
  real Central ([security assurance](../security-assurance.md), "The website
  redeems a reset code and changes a password").

**Open.** Issuing a reset code in the new site's administration (built
the same day, §18); a reading
of the nine machine translations by speakers; a reset that keeps the data
through a device still signed in is the client's (M6).

## 17. As built: an account's optional e-mail address (12.11.2026)

The website's half of `central_api::email` (the wire of `4d0528f`; what
Central does with an address is `docs/email.md`, which comes with the
server's half). It was built against the types and a stand-in's answers:
when this was written, the service refused both new requests.

- **The section.** "E-mail address" is a card under the password's change
  in the account area's security section (`islands/email.rs`). It asks
  `EmailRequest::Status` when it is shown and draws what Central holds,
  never what it guessed: every answer is the state after it. Its faces
  are the explanation with the form (no address), the code's field (an
  address waits), the confirmed address with "Change" and "Remove", the
  question before a removal, and, where `mail` is false, one sentence and
  what is stored with the ways to drop it, without a form.
- **Naming an address proves the password.** The form checks the address
  with `central_api::email::address`, Central's own rule, signs in once
  more as the password's change does (`password::sign_in_again`, in a
  Worker that is ended right after), and only then sends `Set`. The
  sessions listed above it are read again. A wrong password is the
  Worker's refusal: nothing about the address is sent.
- **The code** is read by `MailCode::parse` before it is sent, so a text
  that is no code never counts as a wrong one. The field shows capitals
  by style and leaves what was typed alone.
- **What a refusal means.** `Rejected` says little here (a wrong code, no
  address that waits, no mail, no session). Only a refused `Confirm` is
  called a wrong code. After it, and after an answer that did not arrive
  (which the page calls neither done nor refused), the state is read
  again and drawn. A refused `Status` signs nobody out by itself: the page
  asks `AccountSummary`, and only its refusal is a lost session. A Central
  that knows the session and refuses the question gets one quiet line and
  "Try again"; the rest of the page stays.
- **The reset page** has a block before its form, "No reset code?": a
  sign-in name and `ResetMail` ([account reset](../account-reset.md), "The
  website's page").
- **The export.** The file the privacy section saves is Central's export
  with one more key, `email` (`address`, `confirmed_at`, `pending`,
  `pending_until`), read with `Status` once the export answered; the
  export's own message did not change. Whether this Central sends mail is
  not the account's and is not in the file. Where the state cannot be
  read, the file is the export alone and the page says that the address
  is not in it.
- **Words.** 49 new messages and three changed ones (`faq-password-a`,
  `reset-lead`, `reset-code-hint`), English and German by hand. The nine
  other languages carried the English text for these 52; the same day
  they, the new `email-code-dropped` and the changed privacy paragraphs
  were translated by machine (Sonnet 5.5, by the owner's instruction),
  unread by a speaker, as were the mails' words. The privacy notice has a
  paragraph "E-mail address" after "Your account", and its date is the
  12th.
- **Tests.** Unit tests of every decision the section and the block make
  and of the requests' JSON, `web/e2e-site/email.spec.ts` (every state and
  transition against a stand-in that keeps an address as the contract
  describes it; the sign-in once more in a canned Worker, a wrong password
  in the real one) and the block's tests in `reset.spec.ts`: at both sizes,
  at 320 px, in German, with the 44 px targets, and with axe in the dark
  and in the light scheme. `check.ts`'s `schemes` switches the scheme under
  the page as it stands, since these states have no address of their own;
  it asks for reduced motion while it measures, so that no colour is read
  in the middle of its fade, and says so if anything still moves or if
  the page did not follow the scheme.

**Open.** A run against a Central that keeps addresses (`e2e-real`), which
needs the server's half; the nine translations; `privacy-recipients` does
not name the mail's processor (the new paragraph does); the section says
"15 minutes" and does not show `pending_until`.

## 18. As built: an administrator makes a reset code (12.11.2026)

The first piece of the administration on the new site: an administrator
makes a reset code for an account, which until now only the old build's
`/admin` page could. Nothing on the wire changed (`AdminRequest::Accounts`,
`Account` and `ResetLogin` as they were), and no dependency.

- **The page.** `/{lang}/account/admin` (`Section::Admin`, the island
  `islands/admin.rs` inside the account area's island). An address under a
  language, so Caddy's `/admin*` for the old build is untouched; not
  indexed, in no sitemap. `Section::ALL` stays the navigation, which is the
  same for everyone; `Section::ROUTED` adds the administration for the
  routes and the tests. The overview links to it for an administrator
  only, beside the link to the old build's administration. The server
  cannot know the role: anyone may open the address, and the island shows
  a person without the role one sentence and asks Central nothing more than
  `AccountSummary`. "No page" is the island's doing, not a 404.
- **Its steps.** A search and its list (cards with the facts `AdminAccount`
  carries, 50 a page, "Show more"); one account opened and read afresh with
  `Account` (the only answer that says whether it has a recovery code); the
  warning; the password once more in a Worker (`password::sign_in_again`,
  then a check that the session is still the account the page was opened
  as); the code shown once, with a copy button (`clip::copier`). Each step
  takes the keyboard at its heading or field; "Back" returns to the list at
  the account's button. What the page says and why is in
  [account reset](../account-reset.md), "The administration's page".
- **The code** lives in the step that shows it as a `ResetCode` behind an
  `Arc`, wiped when the step is left (by "Done", or by `pagehide`). No
  download link: a `data:` address would put the code in a URL.
- **Refusals.** One `Rejected` for every refusal; the page tells them apart
  by the account's row (before sending) and by `AccountSummary` and the row
  read again (after). Unit tests cover the mapping (`refusal`, `explain`,
  `at_reset`, `at_sign_in`, `query`).
- **Words.** 48 new messages (`admin-*`), English and German by hand; the
  nine other languages carry the English text under
  `# English for now: to be translated.` The English share stays under the
  gate's 12 in 100 (French 107 of 1011, the most). Reused: `reset-code`,
  `tokens-until`, `profile-*`, `code-shown-once`, `code-copy*`,
  `password-error-current`, `privacy-delete-password`, `signin-working`,
  `error-*`, `account-home`.
- **Tests.** Unit tests of every decision; a server test that the page
  exists in every language, is not indexed, not in the sitemap and linked
  from no page the server renders; `web/e2e-site/admin.spec.ts` against a
  stand-in that refuses as Central's store does, at both sizes, at 320 px,
  in German, with the 44 px targets, the keyboard order and axe in both
  schemes; `web/e2e-real/site-reset.spec.ts` makes its codes through the
  page now (`issue` in `site.ts` drives it), which this change could
  type-check but not run.

**Open.** The rest of the administration (overview, the role, tags,
gateways, audit, the legacy store, reports) is still the old build's; the
nine translations; a run of `site-reset.spec.ts` against a real Central.

## 19. As built: the owner's round of 12.11.2026

Three things the owner asked for after using the site. Each point names
where it is held.

**The colour switch.** It went from the device's scheme to light, to dark
and back, so on a light device its first press chose light, which was
already on screen, and nothing changed; a second press was needed. Now one
press shows the scheme that is not on screen, the one chosen or else the
device's as the stylesheet reads it (light only where the device asks for
light), in both directions, from the first visit and with or without a
choice kept from before. A choice the device would make anyway is not
kept, so the page follows the device again; the key is
`baylee-site-theme` as before, applied by `boot.js` in the head before the
first paint. The switch shows the sun or the moon of the scheme on screen
and names it ("Colours: light", "Colours: dark"); while nothing is chosen
it follows the device changing under an open page. Without script the
switch stays hidden, as before, and the device decides. `theme-system` and
the half-filled glyph are gone. `web/e2e-site/theme.spec.ts` presses it
once on a dark and on a light device, with nothing, light or dark kept,
on both sizes (the phone's switch is in the menu), checks the scheme on
screen, what is kept, and that the kept one is on the root before the body
begins; the old test had looked for the switch under a class the bar does
not have and was skipped everywhere.

**No official gateways.** No card, list, chip or count calls a gateway
official any more, on the start page or in the directory: the gold pill,
the card's stronger edge, the "Official" chip with `?show=official` (an
old address with it shows every gateway) and `data-official` are gone,
with `gws-official` and `gw-show-official` in all eleven languages. The
directory's lead no longer says that Baylee runs the one marked official;
each gateway is run by its operators (English and German by hand; the
nine others carry the English text until it is translated). Central's API is
unchanged: its directory still names an official origin, and the site's
server still lists that one first (`fetch.rs`) without marking it.
`pages.rs` and `site.spec.ts` hold that the word and the marks are absent.

**The strip under the bar** is Baylee in numbers ("Project status" before):
nothing of GitHub's is in it any more (the release, the checks, the last
commit and the stars left; the source section and the download keep
theirs, so nothing fetched went unused and the privacy notice stays true).
What stays is ordered for a reader, the most telling first: the players
the gateways report, of them in a game, the running games and the open
tables (one term of the list, "as they report it", with four values, the
three short ones first), the
gateways online and not, Central's accounts, the playable cards (read from
the release's `sets.json` on GitHub, but a figure of the game, not of the
repository) and the page views. Each figure is a cell of its own, its glyph
running with its words, its label for a screen reader and the tooltip; a
figure nobody reports is no cell, so it leaves no hole. The server writes
how many cells there are (`data-count`, 1 to 8), and `header.css` lays out
that many without script: on a wide screen (1100 px and more) all in one
line, the room shared out evenly; from 600 px at most four to a line, five
and six as three and two or three and three, seven and eight as four and
three or four and four; on a phone two to a line, three in one line as
wide as their words, and five and seven with a first line of the gateways'
three short counts (the gateways' cell needs half a line), so that no line
differs from another by more than one
cell, the shorter is centred and no cell stands alone. A cell is as wide as
its share of the line, so two lines' columns stand in line.
`web/e2e-site/strip.spec.ts` holds the order, the absence of GitHub, axe
and the 44 px targets, and measures the lines for all sixteen sets of
silent sources at ten widths from 320 to 2560 px (and a phone's own) in
all eleven languages: counts per line, centring, equal gaps, nothing
sticking out of its cell and no sideways scroll. A cell's words may
break inside it where its share is narrow (the gateways' two parts one
above the other; in Japanese between phrases, `word-break: auto-phrase`,
as the site's titles do). `pages.rs` holds a cell for each reported
figure under all sixteen sets of silent sources.

**Words.** Removed in all eleven languages: `theme-system`,
`gws-official`, `gw-show-official`, `strip-build`, `strip-releases`,
`strip-on-github` and `strip-stars` (the build does not refuse a message
nobody calls, so they went by hand). Changed: `status-label` (the strip's
name) and `gw-lead` (its sentence on the official gateway), English and
German by hand; the nine other languages carry the English text of both
under "# English for now: to be translated.", as with §17's changed
messages. No new message.

## 20. As built: the album opens as a book, its photographs turn (12.11.2026)

The owner asked for better movement where Baylee's album opens and shuts,
and for a beautiful change between the photographs in the full view. Both
are `home.js` and `album.css`; neither has words, so no language changed.

- **The book.** The logo lays the book down shut: a cover of cloth the
  colour of the site's evening, the spine's hinge pressed into it, a gold
  rule, and her name as the hero's picture shows it. The cover swings up
  on the spine towards the reader and over to the left (Web Animations,
  0.9 s after 0.16 s) while the book comes nearer; the light comes from
  over the reader's left shoulder, so the cloth catches it as it lifts and
  darkens as it turns away, the endpaper is dark as it comes round and
  brightens as it comes down, and the cover's shade on the first page
  narrows to the spine. The first page lifts a little and settles. The
  close button and Escape run it back (0.65 s) and take the book away;
  shut while it opens, the same movement goes back from where it is. The
  veil over the page fades in with the book and out with it, and goes
  back with it too (a Web Animation of `::backdrop`, made at the first
  frame of an opening, as the veil exists only once the dialog is). The
  light and the shade are keyframes at the cover's angle every 15° under
  one easing for the whole swing, which a CSS animation cannot give (it
  eases each keyframe on its own). The cover is made by the script and is
  there only while the book folds (`data-folding`); without the script the
  logo opens the plain dialog, as before. An Escape the browser does not
  let a page hold (pressed again at once) shuts it at once. While the
  cover swings beyond the book's edge the book does not cut it off, but
  the pages are still cut where the book cuts them at rest (a window too
  short for them).
- **The photographs.** The viewer's frame holds three prints, the one
  shown and, hidden, the ones before and after (the album's smaller
  pictures; the one shown takes the full photograph once it has loaded).
  Each is the size its photograph takes in the frame (container units and
  the photograph's ratio in `--ratio`), so its place is kept before it
  loads and its shadow falls from the photograph's edge. Going on, the
  next print comes in from the right over the one shown, a little turned
  and lifted, while the one shown sinks back into the dark; going back,
  the one shown is lifted off to the right. A finger moves the prints as
  it moves and ends the turn past a third of the way or with a flick;
  short of it they fall back. Zoomed in, the finger still moves the
  photograph. A sideways stroke on a touchpad turns once (zoomed in, it
  moves the photograph sideways; it used to zoom). A key pressed mid-turn
  ends that turn and starts the next. Only one image is the viewer's
  (`data-viewer-image`), the one the zoom acts on. The photographs still
  load only once the book is open.
- **The rules held.** Only transform and opacity move, the frame and the
  controls never; no inline style in the markup (the script sets
  transform, opacity and `--ratio` through the CSSOM); nothing hovers into
  motion. With less motion asked for, the book is open or shut and the
  photograph changed at once, and nothing follows a finger.
- **Tests.** `web/e2e-site/album.spec.ts`: every way in and out of the
  book with the order of its states, focus back on the logo, every
  photograph by the buttons, the arrow keys and synthetic touch strokes,
  the falls back, the zoomed finger, the browser's own input (a mouse's
  drag, a touch screen's finger through the DevTools protocol, a
  touchpad's wheel), focus back on the photograph's frame, less motion, no
  script, and the keyframes of what runs read in the same task as the
  press that starts it (only `transform` and `opacity`). `site.spec.ts`'s
  page-turn test reads its leaf the same way.

**Measured** (headless Chromium on the M1 Max's GPU, the machine loaded;
the gaps between animation frames on the main thread, not the
compositor's own): at 1440 px the median gap was 16.7 ms while the book
opened, shut and a photograph turned, in three runs with at most one gap
of 33 to 50 ms each (a run before them, its browser just started, stalled
once for 0.55 s). At 375 px with the CPU slowed fourfold the median
was 16.7 ms as well, with one to four gaps of 50 to 270 ms per run, the
longest the first open's first frame, which took about 0.2 s before this
change too (the album's first layout and paint) and comes before anything
moves. **Open:** a real phone, Safari and Firefox.

## 21. As built: ways to keep the recovery code (12.11.2026)

The owner's decision on the code's step, after the trade-off was explained
("Wie empfohlen + Mail an mich"). What each way does and keeps is
[web-accounts](../web-accounts.md), "Keeping the recovery code"; the three
passes are in [security assurance](../security-assurance.md), "Ways to keep
a recovery code".

- **The step** (`islands/auth.rs`, `code_view`, with `islands/keep.rs`):
  under the code, Copy, Save as a text file, Print and, only where the
  browser has `PasswordCredential`, Save in password manager, in one row
  that wraps; under them, apart, the warning and "Mail it to myself"; then
  the e-mail field (a registration only, where Central mails); then the box
  "I have saved my recovery code" and Continue, as before. The same step
  after a sign-in that makes a code for a moved account has the ways and
  no field.
- **Nothing before the click.** The file's `data:` link, the `mailto:`
  address and the print's sheet are made when their button is pressed;
  before that the code is in the page once, as the text of the step. The
  password manager and the print are reached through `js_sys`
  (`Reflect`), as the clipboard is: no `web-sys` feature was added. The
  file and the mail are opened through `web-sys` types Leptos already
  builds (`session::open`, shared with the Impressum).
- **Print.** `pages.css` shows the sheet alone while it is in the page,
  from the page and from the registration's dialog (which a print hides
  otherwise); on a screen it is never shown.
- **The address.** Checked by Central's rule on Continue; named after the
  code is kept and Central's terms are accepted, on a step that is the
  account area's e-mail section (`email.rs`, `Begin::Name`): `Set` without
  the password while the registration's sign-in is fresh, the code's field
  at once, "Continue to your account" to skip or to go on. Where Central
  refuses `Set`, the section's own form asks for the password.
- **Words.** 18 new messages (`code-print`, `code-manager`,
  `code-manager-id`, `code-manager-offered`, `code-manager-failed`,
  `code-mail`, `code-mail-warning`, `code-mail-subject`,
  `code-sheet-title`, `code-sheet-name`, `code-sheet-what`,
  `code-sheet-keep`, `code-email-label`, `code-email-hint`,
  `code-email-step-title`, `code-email-step-lead`, `code-email-step-done`,
  `email-again`) and `privacy-device-manager`, a second paragraph of the
  privacy notice's "What your browser keeps"; `code-lead` changed in
  English and German ("somewhere safe where you will find it even without
  this device" for "away from this device", which the file and the
  password manager contradicted). The nine other languages carry the
  English text of the new ones under `# English for now: to be
  translated.` and their old `code-lead`.
- **Reduced motion** now stops the dialogs' backdrop too (`*::backdrop` in
  `base.css`): its colour faded under reduced motion, which `schemes`
  found once the registration's dialog was measured in both schemes.
- **Tests.** `web/e2e-site/recovery.spec.ts`: the whole registration in
  the real island against a stand-in Central and a canned crypto Worker;
  every way, the code once in the page before any click, the `mailto:`
  address (to the typed address and to nobody), the file's content, the
  password manager's offer and its absence, the print in print media from
  the page and from the dialog, the address named with and without terms
  in force, a stale sign-in, a refused address, skipping; axe in both
  schemes, the targets, 320 px and the keyboard's order, and both steps in
  German at 320 px on the page and in the dialog. `e2e-real`'s
  `register()` and `site-email.spec.ts` were adapted (the address named at
  registration, another in the account area) and type-check; they were not
  run here.

## 22. As built: the nine languages translated (12.11.2026)

- **What.** Every message that §17 to §21, the strip and the mails' notices
  had left in English under `# English for now: to be translated.` now
  has its own text in Spanish, French, Italian, Japanese, Korean,
  Brazilian Portuguese, Russian and both Chinese scripts: the
  administrator's page, the recovery code's ways and its e-mail step,
  `status-label`, `gw-lead`, `privacy-device-manager`, and in the mails
  `confirm-link` and the nineteen notice ids. The comments are gone.
- **Who.** Translated by a cheaper model, as the owner asked for words
  (Claude Sonnet), then checked here: every language has every id of the
  English file, the same placeables and the same tags in each message;
  the Italian file's messages were read against their English ids one by
  one; the page named in `admin-refused-own` is the account area's own
  name for its security page in each language.
- **What stays English.** Names and technical words (Baylee, GitHub, the
  platforms, SHA-256, Argon2id, OPAQUE, Central, the README anchors), and
  short words a language's UI keeps as they are (Online, Gateway, Beta in
  some). The gate's share of English messages is now 25 to 58 in about
  1,020 per language (French the most, many of them French words that
  are spelt as in English), under its limit of 12 in 100.
- **Not done.** No native speaker has read them. Wording the translator
  marked as unsure (for example the Italian `code-sheet-keep` and
  `admin-lost`, the French `admin-warning-takeover`) is listed in the
  pull request for a reader of that language.

## 23. As built: the BIMI mark (12.11.2026)

- **What.** Baylee's mark for BIMI (SVG Tiny PS, Baylee on the moon,
  31,288 bytes, SHA-256 `0a52483c…1f17b`), approved by the owner, is
  served at `/bimi/baylee.svg` for mail providers and shown in every
  page's footer beside the sunlight wordmark, 72 px, the same file by the
  same address (the owner, through the session that asked: "Wenn es
  wichtig ist, dann ja"; a mark certificate wants the mark public on the
  domain for a year, and the clock starts when it shows).
- **How.** The file lives in `crates/central-web/assets/bimi/` and is
  marked `-text` in `.gitattributes`; `cargo xtask site` copies it byte
  for byte to `target/site/bimi/baylee.svg` (and makes its `.br` and
  `.gz` beside it, as for every text file); `central-web-server` lists it
  with the fixed-name files (`files::ROOT`), so it is read once at start
  and answered before any language redirect, `image/svg+xml`,
  `no-cache` with an entity tag, the site's security headers. It is never
  optimised: a tool such as svgo would break the profile.
- **Tests.** `xtask`: the file's SHA-256 is the approved one, it begins
  with the Tiny PS root, and it holds no script, handler, external link,
  image or import (its `url(#…)` name its own gradients). Server: 200,
  the type, the exact bytes, no redirect, no cookie, revalidated, in both
  modes; `/en/bimi/…`, `/bimi/`, a trailing slash and another name are
  not it. `web/e2e-site/bimi.spec.ts`: the same over HTTP with and
  without `br` and `gzip`, and the footer's mark visible, loaded and at
  least 64 px on five pages in four languages at both sizes.
- **Not done here.** The DNS record (`docs/operations.md`, "The BIMI
  mark") waits for this site's deploy; the deploy is the owner's.

## 24. As built: the overlays' scrollbars and a wider registration (12.11.2026)

- **The owner:** with a scrollbar, the overlays (the sign-in and
  registration dialogs, the download's other systems) looked broken at
  their right edge; the registration may be wider where there is room.
- **Scrollbars.** What floats and scrolls on its own (`.auth-dialog`,
  `.menu`, the phone's `.nav[popover]`, `.terms-text`) has a thin bar in
  the page's ink on no track (`scrollbar-width: thin`, `scrollbar-color`;
  for a Safari without them, the same by `::-webkit-scrollbar`). A
  dialog's bar ran through its rounded corners: the dialog now has a
  transparent border as tall as its corner radius, so the bar runs between
  the corners, and its edge is drawn by an outline at the border's outer
  line (`outline-offset: -1px`); in Windows' high contrast the border is
  the ground and the outline the edge. Scrolling the card instead of the
  dialog was tried and dropped: Chrome makes a scrolling element
  focusable, and the dialog then gave the keyboard to the card instead of
  its first field.
- **Width.** The registration dialog is up to 42rem wide (signing in stays
  31rem); on a phone both take the screen's width less the gutters.
- **Tests.** `dialogs.spec.ts`: at 1300 × 560 and on a phone, the
  registration dialog scrolls, its bar is thin on no track, its border is
  transparent and at least the corner's radius, its edge a 1 px outline,
  the keyboard is in its first field, it is wider than signing in where
  there is room, its last control and the close button are reachable, and
  every target is 44 px.

## 25. As built: the administration's other pages (12.11.2026)

The owner: "Erst Admin neu, dann löschen" (the administration rebuilt
first, then the old React site deleted). Everything the old build's
`/admin`, `/admin/users`, `/admin/users/<id>`, `/admin/gateways`,
`/admin/audit` and `/admin/legacy` offered is now on this site; the
feedback's triage (`/admin/feedback`) is rebuilt separately. Nothing on
the wire changed (`central-api`, `central-client`, `central-crypto`,
migrations and `Cargo.lock` are untouched); no dependency.

- **The pages.** Under the account area, each an island page that is not
  indexed and in no sitemap: `/{lang}/account/admin/overview` (the
  counts: accounts, gateways awaiting approval, open reports and new
  crashes, gateways in the list, administrators, what is left to move;
  each tile leads to its work, a tile whose read gives nothing is left
  out), `/users` (newest first, 50 a page, a search checked by
  `AccountQuery`'s rule that follows the field once typing pauses, Escape
  empties it, Ctrl+K or ⌘K reaches it), `/users/<id in hex>` (the facts
  with copy buttons for the sign-in name and the id, the tag, the role,
  the way to a reset code, the gateways it registered, its latest twenty
  audit events), `/gateways` (by state, the queue first, counts on the
  filters, owners, tokens on request; approve, disable, enable again,
  revoke a token), `/audit` (today's names beside the ids, the end of an
  id where a name is gone, an action the page does not know as Central
  names it) and `/legacy` (deadline, progress, what holds a move back; a
  Central without a store says so). `/{lang}/account/admin` stays the
  reset code's page, and opens at the account an account page's link
  names (`#account=<id>`, removed from the address once read).
- **Who sees them.** The server cannot know the role, so every address
  answers everyone with the same frame and the island shows the page only
  to a session whose `AccountSummary` says `admin`; anyone else gets one
  sentence and no administration's request is sent. The administration's
  own navigation (Overview, Accounts, Gateways, Audit log, Legacy accounts,
  Feedback, Reset a login) is drawn by the island for an administrator,
  on every page of it, the reset code's included: no page the server
  renders links there. The account overview's "Administration" leads to
  the overview here.
- **One account's address** is the only one with a part the visitor
  chooses (`Section::AdminUser([u8; 16])`): the id as 32 lowercase hex
  digits and nothing else (`data::id_from_hex`); `Page::path` writes it
  back, so the language menu keeps the account. The server renders that
  page for its request and never keeps it (`Page::kept`): ids cannot fill
  the table of rendered pages, whose bounds hold every other page there
  is.
- **A change** (`common::Flow`: the role, a tag, a gateway's approval, a
  token) opens its confirmation in the card of what it changes, with what
  follows (the tag's warning about whispers, the approval's check of who
  runs the address); only the confirmation's button sends, and the
  confirmation gives way to "Asking Baylee Central …" before anything
  leaves, so a second press finds nothing. `AccountSummary` first: another
  account in this browser sends nothing, no role sends nothing, a sign-in
  older than 29 minutes by the browser's clock asks for the password once
  more (in a Worker ended at once, with the session's account checked),
  as the old pages' "password again" did. Central refuses every one of
  these changes with the same `Rejected`; the page explains it by what
  stands afterwards (the role, then whether the account or gateway still
  exists, then the sign-in, asked for once and never in a loop); a tag's
  `Conflict` is "taken, or kept by Central chat". An answer that does not
  arrive (no network, a 5xx, an answer of another kind) is never a
  refusal: the page reads what it shows again and says that it cannot
  tell; where that reading fails too, it says to load the page. After a
  gateway's change the keyboard goes on at the gateway now in its place.
- **Reads** go through `common::read`: a `Rejected` is a lost session only
  where `AccountSummary` is refused too, the role gone where it says so,
  and for an administrator "what was asked for is not there" (an account
  or gateway gone, no legacy store).
- **Not rebuilt, and why.** The navigation's counts (gateways awaiting
  approval, unread reports): they would make every page ask Central, the
  reset code's page included; the overview and the gateways' filters show
  the counts. The search in the address (`?q=`): decided against for the
  reset code's page (§18, `docs/security-assurance.md`); the gateways'
  filter stands in the fragment instead (`#state=approved`), which never
  reaches the site. The reports tile's link with its filters
  (`?status=new&kind=crash`): the feedback's page and its addresses are
  another change's, so the tile leads to its page. The tiles' counting-up
  animation and the skeletons: "Loading …" instead. The modal
  confirmations: inline in the card, as the reset code's page does it.
- **Wire limits found** (none worked around): `OwnedGateway` says only
  `approved`, so a disabled gateway an account registered reads "Awaiting
  approval" on its page, as on the old one; `Rejected` for a change says
  nothing of why (explained as above; a token gone while its gateway
  stays would be taken for the sign-in's age, which no store path produces
  today).
- **Words.** 106 messages (`admin-*`, sections `admin-ov-`, `admin-gw-`,
  `admin-audit-`, `admin-legacy-`, `admin-tag-`, `admin-role-`,
  `admin-change-`), English and German by hand. The nine other languages
  were to carry the English text for a later pass, but that broke the
  gate's 12 in 100 (French 14.5 %); they are translated (Claude Sonnet,
  each language's own terms from its file, the French read against the
  English), in a commit of their own that a revert turns back into the
  English. No native speaker has read them. Reused: `gateways-*`,
  `tokens-*`, `gws-online`, `profile-*`, `admin-state-*`, `admin-pill`,
  `admin-refused-*`, `admin-search*`, `admin-count`, `admin-more`,
  `error-*`, `account-*`, `code-copy*`.
- **Look.** `style/admin.css`, last in `STYLES` (the reset code's block
  moved there from `app.css`): the old build's tiles, tables in a box that
  scrolls on its own (inline-size containment, so a table never widens the
  column), filter chips, pills; nothing moves or grows under the pointer.
- **Tests.** Unit tests: the id's address, the fragment, every change's
  request, what stands before a change, a lost answer never a refusal,
  the explanation of a refusal, every outcome's own sentence in every
  language, every audited action's words, the filters, a tag as Central
  takes it, what holds a legacy move back, the reports at a glance. Server:
  every page in every language, not indexed, an island, linked from no
  page the server renders, one account's address and its 404s, never
  kept. Browser (`web/e2e-site/admin-*.spec.ts` with the stand-in
  `admin-central.ts`, 54 tests, both sizes): every page with and without
  the role, axe in both schemes, 44 px targets, 320 px in German, the
  keyboard's order, each change's exact request after its confirmation
  and nothing before, a double press sending once, the password's step,
  refusals and lost answers; `admin.spec.ts` (20) unchanged but for the
  overview's link.
- **Open.** The feedback's page (another change); a native reader for the
  nine languages (unsure wording is listed in the pull request); a run
  against a real Central and PostgreSQL; deleting the old build once the
  feedback is here.

## 26. As built: the feedback moderation (12.11.2026)

The old build's report moderation (`web/src/admin/feedback/`, removed 12.11.2026, its routes
`/admin/feedback`, `/admin/feedback/<hex>` and the `/admin/reports`
alias) rebuilt on the new site, so the old React site can go (owner,
12.11.2026: "Erst Admin neu, dann löschen"). Nothing on the wire changed:
`AdminRequest`'s report actions and `POST /api/v1/reports/<hex>/<part>`
as they were; no dependency, no `web-sys` feature (the browser's
`DecompressionStream`, `Blob`, `URL` and `createImageBitmap` are reached
through `js_sys`, as `clip` does).

- **The pages.** `/{lang}/account/admin/feedback` (`Section::Feedback`)
  and `/{lang}/account/admin/feedback/<32 lowercase hex>`
  (`Section::Report`), islands of the account area as §18's page is: for
  an administrator only (anyone else gets §18's one sentence and Central
  is asked nothing but `AccountSummary`), not indexed, in no sitemap, in
  no navigation, linked from no page the server renders. Every report's
  address is one id-less page (`Page::from_slug` matches the hex; the
  island reads the id from `location`), so the render cache never holds
  an id; anything but 32 lowercase hex digits is a 404. Each page links
  back to `/{lang}/account/admin/overview`; the shared administration
  navigation is wired in when the two branches meet.
- **The list** (`islands/feedback/list.rs`): the seven views with their
  counts (the open reports' facets and the trash's total), the search
  (it follows the typing after a pause, Enter at once, Escape empties
  it), every filter of `ReportFilter` with its facet count (status, kind,
  priority as chips; assignee, label, gateway, channel, record, reporter,
  days, order as fields; only unread), the active filters as pills that
  take themselves out, sortable column heads, pages and their size, saved
  lists (read when the menu opens, saved, removed), the chart of the last
  30 days (inline SVG and a table, read when unfolded), crash groups with
  "show its reports", the CSV of a selection or of everything the filter
  matches (up to 5,000), and a selection's actions in one request each
  (status, priority, assignee, add or take off a label, merge into one,
  the trash with Undo, restore, delete for good, opened or not, copy the
  ids). The query string is the old one byte for byte (keys, order,
  `URLSearchParams` encoding), so lists saved in the old build still open
  and match; a part out of Central's bounds is dropped, never sent.
- **One report** (`report.rs`): the way back to its list and that list's
  previous and next report with its place (`ReportNeighbours`); its
  pills, first line, short id and a copied link; what was written (the
  client's `[Card]` and `[@Player]` references as chips in their place),
  the crash; what the client sent (inflated in the browser, at most 16
  MiB; the window's picture drawn into a canvas from its checked PNG
  bytes; build, game, backtrace, log, system, settings, and the whole
  object as a tree that draws a fold's children only when it opens);
  the facts (the gateway's address as text, never a link); the record
  handed over as Central keeps it (`.jsonl.gz`); the history in words;
  the triage (status, priority, assignee, labels with the labels in use
  offered, duplicate of, the GitHub issue and GitHub's own "new issue"
  form filled with the administrator's neutral words only); notes, added
  and removed by their author; the related lists; the trash with Undo,
  restore and delete for good. Opening a report marks it opened. Every
  change names the revision it was read at; a conflict reads it again and
  says so.
- **Untrusted words.** A report is a game client's: every field is drawn
  as text (Leptos text nodes; no `inner_html` anywhere, and Trusted Types
  would refuse one). Nothing of a report goes into an address but its id
  and the list's own filter; nothing is logged or written to the console.
  The only addresses built from report data are the old ones: Baylee's
  issue by its number (1 to 10,000,000) and GitHub's new-issue form from
  the administrator's summary, a category and a build that reads as a
  version.
- **Deleting for good** is asked first (nothing is sent before the
  confirmation), wants a password sign-in younger than Central's 30
  minutes (asked once more in a Worker where `AccountSummary` says it is
  older, the session's account checked as §18 does), and is sent once
  however often it is pressed. Central refuses a stale sign-in and a
  report out of the trash with the same 401: the page tells them apart by
  what `AccountSummary` says afterwards.
- **Refusals and lost answers.** A refused read is explained by
  `AccountSummary` (no session: the area says so; no role: one sentence;
  else "does not exist"); an answer that never came is never a refusal: a
  change is read again and said as possibly made, a note's draft stays.
- **Look.** `style/feedback.css`, last of `STYLES`: the old look on the
  site's tokens; 44 px for every control; a table scrolls in its own box
  (`contain: inline-size`, and it is the containing block of its hidden
  words, which would otherwise widen the page); the pill tones mixed a
  little toward the ink so a small bold word keeps 4.5:1 on its tint.
- **Words.** 297 messages (`fb-*`), English and German by hand; the nine
  other languages carry the English text under
  `# English for now: to be translated.` That puts every one of them past
  the gate's English share (`xtask` `no_language_is_english_under_another_name`:
  24 to 27 in 100, the limit is 12): the translations are owed before
  this merges.
- **Tests.** Unit tests of the query string (the old encoding, saved
  views of the old build round-tripping byte for byte, every bound),
  the filter, the CSV (formula injection `=+-@`, tab and CR, quoting),
  the alias, sizes and times, the chart's days, history words, ids as
  pasted, the GitHub addresses, the client part's readers and the
  reference pieces, and that a lost answer is no refusal; a server test
  that both pages exist in every language, are not indexed, not in the
  sitemap, linked from nowhere, and that a malformed id is a 404;
  `web/e2e-site/feedback.spec.ts` and `feedback-report.spec.ts` against a
  stand-in Central (`feedback-stand-in.ts`) with markup and script in
  every text field: each action's exact request, nothing before a
  confirmation, a purge sent once on a double press and only after the
  password where the sign-in is stale, refusals and lost answers, the
  CSV's defused formula, the canvas drawn, axe in both schemes, 44 px,
  320 px in German, the keyboard order, at both sizes.
- **Not rebuilt**, being second presentations or conveniences of what is
  here: the drawer beside the list (a report opens on its own page, which
  steps through the list); the single-key shortcuts and their dialog
  (every action is a control in the tab order); the quiet 30-second
  re-read with its "new reports arrived" line (the Refresh button);
  relative times ("5 minutes ago": absolute UTC as the rest of the
  site); Scryfall's card pictures and links on the chips (the site's
  `img-src 'self'` admits no other origin, and the chips stay text); the
  "clear the filters" button repeated on an empty result (the pills'
  own is there); a record written by the client has no button label and
  notice of its own (the facts say who wrote it). The `/admin/reports`
  alias and the old `/admin/feedback…` addresses get no page here: when
  the old build goes, a redirect to `/{lang}/account/admin/feedback…`
  keeps their query strings, which are the same. As in the old build,
  only deleting for good is confirmed; moving to the trash has Undo, and a
  selection's changes apply at once. The navigation's unread count and
  the overview's report figures belong to the shared administration
  navigation and overview.
- **Wire, as it is.** A purge's 401 does not say whether the sign-in was
  stale or a report had left the trash (the page reads the reports again
  to tell); `AddReportNote` has no idempotency key, so a retry after a
  lost answer could add the note twice (the page reads the report first
  and keeps the draft, and the administrator decides).
- **Open.** The nine translations (the gate fails until then); the
  language menu of a report's page leads to the list in that language
  (the page has no id of its own); a run against a real Central.

## 27. As built: the administration's own islands module (12.11.2026)

The site had one islands module, and every page with an island loaded
it: the contact page, the sign-in and registration dialogs, the account
area, the messenger. Rebuilding the administration (§18, §25) and the
feedback moderation (§26) took it from 614 KB to 913 KB Brotli (4.6 MB
raw). Now the administration's islands are a module of their own, which
only `/{lang}/account/admin…` loads, and the public module is back where
it was. Nothing on the wire, in the CSP or in `Cargo.lock` changed; no
dependency, no tool.

- **The split.** `central-web` has a feature `admin`. The target still
  picks the half (§3): the server renders every island whatever the
  features; in the browser's build the administration (`islands/admin.rs`,
  `islands/admin/`, `islands/feedback/`) and what only it uses
  (`focus::after_render_first`, `fragment::take_account` and
  `fragment::replace`, the account id in the area's `Summary`) exist only
  with `admin`. The feature only adds, so `--all-features` builds stay
  whole. `central-web-islands` passes it on (`admin =
  ["central-web/admin"]`).
- **An island of its own.** The administration's pages had been sections
  of the area's `Account` island, which called into `admin` and
  `feedback`: whatever module had `Account` had the administration. They
  now have `Administration` (`islands/admin.rs`), which draws the
  administration's and the feedback's pages for an administrator and §18's
  one sentence for anyone else. Both islands sit in one frame
  (`account::frame`): `AccountSummary` read in the browser, signed out,
  a failure with "again", and a mail's code removed from the address on
  every page of the area as before. `Account` draws nothing for those
  sections, as it draws nothing for the messenger's, and reaches into
  nothing of them. The server puts `Administration` on every
  `Section::administration()` page (every address under
  `/{lang}/account/admin`, the feedback's included; `Section::admin()`
  alone leaves the feedback out), and `shell::islands_script` gives those
  pages `admin-islands.js` and every other page with islands `islands.js`.
  A page loads one module, never both, so no page runs two Leptos
  runtimes; account pages carry no auth dialogs, so an administration's
  page has its one island only.
- **The build.** `cargo xtask site` builds `central-web-islands` twice, in
  two cargo runs (in one, Cargo would unify the features and the public
  module carry the administration again): without features `islands.wasm`
  and `islands-glue.js`, with `--features admin` `admin-islands.wasm` and
  `admin-islands-glue.js`, each run through the pinned wasm-bindgen CLI
  before the next build overwrites Cargo's output. Each bootstrap is
  `js/islands.js` with its module's hashed addresses, so
  `data-islands="ready"`, the waiting for the largest paint and the
  dialogs' late hydration are the same on both. Every file is hashed and
  has its SRI value in `manifest.json`; the service worker's stamp is the
  digest of every asset's address, so a change to either module installs
  a new worker; its offline list names no islands module and is
  unchanged; its `HASHED` pattern matches the new names.
- **The guard.** The build fails when the public module holds one of the
  administration's marks (`site::ADMIN_ONLY`: `admin-stat-sub` and
  `fb-legend`, classes written only in `islands/admin/` and
  `islands/feedback/`), or the administration's lacks one; an xtask test
  keeps the marks inside those folders. A public island that reaches into
  the administration again fails `cargo xtask site`, not a size budget.
- **The administration's module is the whole set.** It exports the public
  islands too: every `#[island]` of `central-web` is a wasm-bindgen
  export, and the linker keeps every export of every crate it links.
  Leaving the public islands out of that build would mean a second
  feature, the public islands and their helpers behind it, and the dead
  code that follows in every module they share with the administration.
  The pages it serves are an administrator's; it is fetched once and
  kept (an immutable hashed file). Accepted: the target was the public
  module.
- **Not used: Leptos' lazy islands** (`#[island(lazy)]`). They rest on
  `wasm_split`: a splitter cuts the linked module into chunks after the
  build (cargo-leptos' `--split`, with `wasm-split-cli`), and a generated
  loader fetches them by addresses of its own. That is a new tool, which
  needs the owner's acceptance, and chunks outside the manifest's hashes
  and SRI values. Two builds of one crate needed nothing new.
- **Sizes** (`cargo xtask site` on this Mac, `brotli -q 11`, `gzip -9`;
  bytes):

  | Module | Raw | gzip | Brotli |
  | --- | ---: | ---: | ---: |
  | Before: `islands.wasm` (`fe100ce`) | 4,579,552 | 1,273,708 | 912,627 |
  | After: `islands.wasm`, public | 2,976,870 | 856,819 | 629,785 |
  | After: `admin-islands.wasm` | 4,587,079 | 1,274,113 | 912,336 |

  The public module is 282,842 bytes (31 %) smaller in Brotli, back at its
  size before the administration (614 KB): 630 KB. Its glue went from
  59,119 to 56,609 bytes (8,132 Brotli). What of the administration it
  still holds is Central's wire: `call` parses every `Response`, the
  administration's answers among them, and the account's home keeps its
  registration switch for administrators. `central-api` was not
  restructured for it.
- **Tests.** Server (`central-web-server/tests/pages.rs`,
  `only_the_administration_s_pages_load_its_module`): in every language,
  every page under `/account/admin` (one account's and one report's
  included) has the administration's island and script and not the
  public one; the forms, the pages that open the dialogs, the area and
  the messenger have the public script and no administration's island;
  `Section::administration()` holds exactly the addresses under
  `account/admin`. It has teeth: with the feedback's pages given the
  public module, it fails. xtask: `the_islands_are_two_modules_with_one_bootstrap`.
  Browser (`web/e2e-site/modules.spec.ts`, both sizes, by the requests
  the page makes): the contact page, the sign-in dialog over a page, the
  account area and the messenger (signed in as an administrator) fetch
  the public module's bootstrap, glue and module and nothing of the
  administration's; the overview, the reset code's page, the feedback's
  list and a page refused to someone without the role fetch the
  administration's and nothing of the public one; and each page's
  islands work. The whole `web/e2e-site` suite ran on the split site
  (`--workers=4`): 644 passed, 4 skipped by their own conditions (a test
  for phones only, one desktop-only test, and the screenshots, which need
  `BAYLEE_SRC`): the administration's pages, the feedback's, the area,
  the messenger, the dialogs and the service worker's update among them.
- **CI** lints the administration's module in a step of its own (`cargo
  clippy … --all-features`), beside the public one's.
