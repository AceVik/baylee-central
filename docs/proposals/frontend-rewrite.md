# Proposal: the website rebuilt as server-rendered Rust

*10.11.2026.* The owner asked on 10.11.2026 for the whole frontend of
`baylee.app` to be rebuilt as server-side rendered Rust (Leptos or similar):
search-engine optimised, accessible by keyboard alone, the legal texts
reworked, a fully built-out signed-in area and a redesigned chat that stays an
installable PWA. This document is the inventory of what exists, the plan, and
the decisions the owner has to take before code that needs them lands. It is
the contract for the branch `claude/frontend-leptos-af71a6`; a change of plan
changes this file first.

Status: **proposal**. Nothing here is accepted until the owner says so. The
dependency graph it needs is in [dependency-proposal/leptos.md](../dependency-proposal/leptos.md)
and is **not** in `approved-packages.json`.

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

**Leptos 0.8.23** (the newest stable; 0.9 is a release candidate) in
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
- **Map:** `/{lang}/` home; `download`; `cards`, `cards/<set>` (each set its
  own server-rendered page, searchable without script through `?q=`);
  `gateways`, `gateways/host`; `baylee` (the remembrance as a page; its
  presentation needs the family's agreement again); `faq`; `contact`;
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

## 10. Owner decisions

1. **Accept the dependency graph** of [leptos.md](../dependency-proposal/leptos.md)
   with the three exceptions (`paste` unmaintained, `xxhash-rust` BSL-1.0,
   `base16` CC0-1.0) — or choose the framework-free fallback (§2.2).
2. **Impressum:** plain server-rendered text from deploy configuration
   (proposed), or keep the proof of work (not WCAG AA, needs script).
3. **Card images:** none, with links to Scryfall (proposed), or an exception
   to the no-third-party rule for `cards.scryfall.io`.
4. **The hero's WebGL cloud simulation:** a still painting (proposed), or
   kept as an enhancement with a pause button.
5. **Contact form** through `/site/contact` on this origin: needs
   `https://baylee.app` in the feedback service's `FEEDBACK_CONTACT_ORIGINS`
   and a Caddy route (deployment, Baylee repository).
6. **Pairing UI:** build the page now (it cannot be used until the client
   parses `baylee://pair`), or wait for the client.
7. **The remembrance as its own page** (`/{lang}/baylee`): its presentation
   changes, which needs the family's agreement.
8. **The Fan Content notice inside an open conversation on a phone:** in the
   conversation's details, one tap away (proposed), or a one-line strip
   under its head.
