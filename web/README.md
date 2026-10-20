# web/ — the web app at https://baylee.app

The public part of milestone M4 in the accounts and web roadmap
(`docs/roadmap-accounts-web.md`, PR #5):
what Baylee is, the download, the card status, the gateway directory,
contact, the legal notice and the privacy notice. It replaces Baylee's static
`web/site` (served at baylee.acevik.de), whose words it keeps and whose
layout it redraws. Account, sign-in and administration come later, lazily
loaded inside this same app, once Central has its browser sign-in (M3).

| path | page |
| --- | --- |
| `/` | what Baylee is, the download button, screenshots, opponents, online play, the repository |
| `/download` | every platform's files from the latest GitHub release, install notes |
| `/cards` | the card status: totals, finished and next sets, every set by era, a set's cards |
| `/gateways` | Central's public gateway directory, page by page, and how to run a gateway |
| `/contact` | the contact form and how to report a bug |
| `/impressum` | the legal notice (§ 5 DDG), details behind a proof of work |
| `/privacy` | the privacy notice (Art. 13 GDPR) |

English and German, one at a time: `?lang=de|en` wins, then the choice made
with the buttons (localStorage `baylee-site-lang`, the old site's key), then
the browser's languages. `?theme=dark|light` forces a colour scheme (for
screenshots); otherwise `prefers-color-scheme` decides. `public/boot.js` makes
both choices before the first paint.

## The stack

The one of Baylee's `web/feedback`, at its versions: React 19.3, TypeScript
7.0, Vite 8, oxlint, vitest with Testing Library and jsdom, Playwright. Every
version in `package.json` is exact and `package-lock.json` is committed. No
router, i18n or CSS library: a handful of pages need none (`src/router.tsx`,
`src/i18n.tsx`), and both languages stand side by side in the components
(`<T en={…} de={…} />`) so a sentence and its translation change together.

## Layout

One folder per part of the site, each file one component or one concern,
none much past 200 lines:

| folder | what |
| --- | --- |
| `src/` | `main.tsx`, `App.tsx` (the frame and the page the address names), `router.tsx`, `i18n.tsx`, `motion.ts`, `github.ts`, `config.ts` |
| `src/components/` | the bar, the footer, sections, the logo, icons, the modal |
| `src/home/` | the start page, one file per section |
| `src/download/` | the release and the machine (`release.ts`, `machine.ts`), the button, the platform cards, the page |
| `src/cards/` | the card status: data and parsing (`model.ts`, `numbers.ts`, `card.ts`, `data.ts`), the rows, the summary, the full list, the card grid |
| `src/central/` | Central's directory client (`directory.ts`) and its checks (`validate.ts`) |
| `src/gateways/` | the directory page |
| `src/contact/` | the contact form and the reporting help |
| `src/impressum/` | the blob, the proof of work and its worker, the scrambled drawing, the page |
| `src/privacy/` | the privacy notice, its sections in two files |
| `src/styles/` | the stylesheets by area; the global ones are imported by `main.tsx`, the others by their feature |
| `test/` | the unit and component tests, mirroring `src/`, one file per unit; `test/fixtures/` (synthetic data, also used by `e2e/`) and `test/support/` (the stand-in network) |
| `e2e/` | the Playwright smoke tests and their mocks |
| `scripts/` | the local server, the CSP, the content copy |

## Commands

Node 22.18 or newer (CI uses 24.21.0), in this folder:

```bash
npm ci
```

```bash
npm run check
```

`check` is typecheck, lint (`oxlint --deny-warnings`), unit tests (vitest) and
the production build, in that order. The smoke tests need a Chromium once:

```bash
npx playwright install chromium
```

```bash
npm run e2e
```

`e2e` builds and serves the site as Caddy would (`scripts/serve.ts`, the same
CSP and headers) and runs `e2e/smoke.spec.ts` at a phone's (Pixel 7) and a
desktop's width: every page draws, nothing is refused by the CSP or thrown,
nothing reaches past the screen's sides, the download offers GitHub's file,
a set opens and shows a card large, the directory pages, the Impressum reads
right on screen and wrong in the DOM, the contact form sends, the language
stays chosen, light and dark both draw, reduced motion hides nothing. Every
outside service is answered in the browser by `e2e/mocks.ts` with synthetic
data; a request to any other host fails the test.

`npm run screens` writes pictures of every page at both widths and in both
schemes into `screens/` (gitignored), for a review or a pull request.

To look at the built site by hand:

```bash
npm run build && npm run serve
```

and open <http://127.0.0.1:4173/>. Served from this machine, the contact
form and the Impressum talk to a feedback service on `127.0.0.1:28780`, as
the old site did (Baylee's `docs/feedback.md`).

`npm run dev` is not wired: Vite's dev server injects inline scripts that the
CSP refuses, so only the build shows what a visitor gets.

## Content from Baylee

Two things the pages show are Baylee's and carry Wizards' card names and card
text, so they stay out of this repository (README.md at the root: no Wizards
assets here):

- the card status, `data/sets.json` and `data/sets/set-<code>.json`, written
  by Baylee's `baylee-tools site card-status` from one git revision
  (Baylee's `web/site/README.md` §"The card status");
- the client's screenshots (`shot-*.avif|webp`), which show its text view.

```bash
npm run content -- ~/Projects/baylee
```

copies both from a Baylee checkout into `public/data/` and `public/media/`
(gitignored), and the next build carries them in `dist/`. A deployment runs
it before `npm run build`; refreshing the card status after a card batch is
that and a new build, or copying the data straight into the served `data/`.
Without them the pages say the list did not load and the screenshots stay
empty; the tests mock both.

The logo, the painting and the fonts (Faustina and Alegreya Sans, SIL OFL 1.1,
licences beside them in `public/fonts/`) are our own or free, and committed.

## Settings

Build-time, through Vite's environment (`VITE_…`), each with a default:

| variable | default | what |
| --- | --- | --- |
| `VITE_FEEDBACK_ORIGIN` | `https://feedback.baylee.acevik.de` | the feedback service the contact form posts to |
| `VITE_CENTRAL_API` | `/api` | Central's API on this origin, for the directory |

Nothing in the build is a secret; everything in it is downloaded by every
visitor.

## The gateway directory

`src/central/directory.ts` is a typed client for Central's public
`GatewayDirectory { after }` ([gateway registry](../docs/gateway-registry.md)),
on the native wire vocabulary of `central-api`: it posts
`{"operation": "gateway_directory", "after": null | [16 bytes]}` to
`${VITE_CENTRAL_API}/v1/native` with one `Content-Type: application/json`, no
credential and no cookie, and reads `{"result": "gateway_directory",
"gateways": [...]}`: at most 100 approved gateways in UUID order, the last
id the next page's `after`. A refusal is Central's plain text and status,
never parsed. Each answer is checked as `central-client`'s
`gateway_directory` checks it (UUIDv7 ids in strictly ascending order after
the cursor, canonical HTTPS origins, bounded metadata whose website is on its
own origin, no unknown field) and an answer that fails shows nothing of it.
Names and versions are rendered as text (`<bdi>`, never HTML); a website is
a link only after those checks, with `rel="noopener noreferrer nofollow"`.

**Not live yet.** Central's native boundary refuses every request carrying
`Origin` or `Sec-Fetch-Site` (`crates/central-http/src/boundary.rs`), and a
browser always sends both on a POST. The directory therefore needs the
reviewed same-origin browser API of M3 at `/api` (the roadmap's "Target
shape"); until it exists the page says that Central did not answer, and
offers to try again. The tests use `test/fixtures/directory.ts` and never call a
real Central.

## The Impressum

As on the old site (Baylee's `web/site/README.md` §"The Impressum" and
`docs/feedback.md` §"The Impressum"), exactly as bot-safe:

- the operator's details are in no file of this repository; Baylee's
  `baylee-tools site impressum-blob` writes an obfuscated blob from a
  private file, the server keeps it outside the web root, and only the
  feedback service hands it out, for a solved proof of work;
- the page asks its own origin (`/site/impressum/challenge`, then `POST
  /site/impressum`), which Caddy passes on to the feedback service, and only
  when the visitor presses *Show the details*; the puzzle runs in a worker
  (`src/impressum/worker.ts`, a file of its own for the CSP), and the page
  reports the same three facts about its browser as before (`s`);
- `src/impressum/blob.ts` decodes the blob with `crypto.subtle` (a secure
  context) and `SCRIPT_KEY`, the published half of the obfuscation key, which
  must stay byte for byte the value in Baylee's
  `crates/baylee-tools/src/site/impressum.rs` (a test pins it);
- the details are drawn one character per span, shuffled among decoys, in
  order only on screen (`src/impressum/Scrambled.tsx`, the order classes in
  `src/styles/impressum.css`); the block is `aria-hidden`, a sentence tells a
  screen reader why, and the copy, write and call buttons read the characters
  back only in their click handler: no `href` ever holds an address;
- the honeypot link to `/site/impressum/kontakt` is on the page and in
  `index.html`'s `<noscript>`, hidden from people; `robots.txt` disallows
  `/impressum` and `/site/impressum/`, the page adds `<meta name="robots"
  content="noindex, noarchive, nosnippet">`, and Caddy sends the same
  `X-Robots-Tag`.

Tests encode made-up details (`test/fixtures/impressum.ts`), never real ones.

## Serving it

The build is static: `dist/` behind Caddy, with no access log (the privacy
notice says so; adding one means changing it). The shape of the site block,
for the deployment step (not done here, and not approved by this change):

```caddyfile
baylee.app {
	# The Impressum's proof of work and blob: the feedback service.
	@impressum path /site/impressum /site/impressum/*
	handle @impressum {
		request_body {
			max_size 8KiB
		}
		reverse_proxy <feedback service>
	}
	# Central's reviewed browser routes (M3). Until they exist, nothing here.
	handle /api/* {
		respond 404
	}
	handle {
		root * /opt/baylee-app/web
		@page not path *.*
		rewrite @page /index.html
		@hashed path /assets/*
		header @hashed Cache-Control "public, max-age=31536000, immutable"
		@html path /index.html
		header @html Cache-Control "no-cache"
		file_server
	}
	header {
		Content-Security-Policy "<scripts/csp.ts>"
		X-Content-Type-Options nosniff
		Referrer-Policy no-referrer
	}
	@impressumPage path /impressum
	header @impressumPage X-Robots-Tag "noindex, noarchive, nosnippet"
	encode zstd gzip
}
```

The CSP is `csp()` in `scripts/csp.ts`, which the local server and the tests
send too: `script-src 'self'; style-src 'self'` (no inline script or style;
React sets styles through the CSSOM, which the CSP allows),
`connect-src 'self' https://api.github.com <feedback service>`,
`img-src 'self' data: https://cards.scryfall.io`, `worker-src 'self'`.

The feedback service needs `https://baylee.app` in `FEEDBACK_CONTACT_ORIGINS`
for the contact form (it answers CORS only to those) and the proxy's address
in `FEEDBACK_TRUSTED_PROXIES` for the Impressum's allowance (Baylee's
`docs/feedback.md`).

## Privacy

The app sets no cookie and has no analytics or tracker. It keeps two things
in the visitor's browser: the language chosen (localStorage) and GitHub's
answers for ten minutes (sessionStorage). It talks to GitHub's API, to
Scryfall's image host for an opened set, to the feedback service for the
contact form and the Impressum, and (through this origin) to Central for the
directory. `src/privacy/` says all of that; a new host in the CSP, a
cookie, a log or a new form means a new sentence there. Its hosting paragraph
(netcup, no access log) is the old site's and must be confirmed for
`baylee.app` before the page is published; the notice is a draft for the
owner's lawyer, as the old one was.

## Accessibility and motion

Mobile first; WCAG 2.1 AA is the aim, not an audit: a skip link, one `h1`
per page and focus moved to the content on a page change, landmarks, every
control at least 44 px, visible focus, `aria-pressed`/`aria-expanded` on
toggles, `<output>` for what changes, contrast from the old site's measured
palette. A set's card grid is one stop for Tab (arrow keys, Home, End; Space
shows a card large, Enter opens it on Scryfall). Motion is transform and
opacity, 160–360 ms, and `prefers-reduced-motion` stills all of it.

## Known gaps

The policy check (`cargo xtask policy`, `scripts/check-policy.ps1`) skips
`web/node_modules` (npm's install, whose `.bin` holds symbolic links) and
nothing else under `web/`; a Cargo target inside it is refused, and the
policy regressions prove both.

- The directory waits for M3's browser route (above).
- No automated accessibility audit (axe) yet: it would be a new dependency.
