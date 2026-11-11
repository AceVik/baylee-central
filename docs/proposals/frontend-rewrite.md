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
Caddy site block exists only as documentation (`web/README.md` §"Serving it").

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
`deletion.rs`, `pairing.rs`).

### 1.4 Browser crypto and the chat

- `central-web-crypto` (Rust, `#![forbid(unsafe_code)]`) is committed as
  WebAssembly with its wasm-bindgen glue in `web/src/central-wasm/`, checked
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
  canonical rebuild): password change on the website, reset with a recovery
  or reset code, and approving a pairing. The security page says that
  password changes stay in the client for now. Notification settings come
  with the PWA (phase 4).
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
  gets nothing stored (owner's decision, 11 November 2026).
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
- Password change and reset on the website (new exports too).
- The administration still comes from the old build (§11, Caddy `/admin*`).
