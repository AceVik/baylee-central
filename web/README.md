# web/ — the browser test suites

The site at https://baylee.app is rendered by the server, in the `central-web`
crate (`../crates/central-web`, `../docs/proposals/frontend-rewrite.md`), with
its administration. The old React app that lived in this folder was removed on
12.11.2026. Nothing here builds or serves the site: this folder holds only the
Playwright suites that test it in a real browser, and the TypeScript and lint
checks for them.

| path | what |
| --- | --- |
| `e2e-site/` | the site in Chromium at a phone's and a desktop's width, against the development server of `central-web` and synthetic data; every page, both colour schemes, axe-core, targets, keyboard paths, the islands, and no request to another origin |
| `e2e-real/` | the same site's account, reset, password and chat steps against a real Central, run by hand (its `README.md`) |
| `e2e-real/front.ts` | the TLS stand-in for baylee.app's Caddy site block, for `e2e-real` only |
| `e2e-real/mailer.ts` | the stand-in for `baylee-mailer`, for `e2e-real` only |
| `e2e-real/site.ts` | the steps the `e2e-real` specs share; loads the committed WebAssembly crypto from `../crates/central-web/crypto` |
| `tsconfig.json`, `.oxlintrc.json` | the typecheck and lint settings of both suites |

## Commands

Node 22.18 or newer (CI uses 24.21.0), in this folder:

```bash
npm ci
```

```bash
npm run check
```

`check` is the typecheck and the lint (`oxlint --deny-warnings`) of both suites.

The browser suite needs Chromium once, and the site built from the repository
root, because its `webServer` runs `target/debug/central-web`:

```bash
npx playwright install chromium
```

```bash
cd .. && cargo build -p central-web-server && cargo xtask site && cd web
```

```bash
npm run e2e:site
```

runs every spec in `e2e-site/` at both widths, on port 28792. A single spec runs
with `npx playwright test -c e2e-site/playwright.config.ts e2e-site/site.spec.ts`.
If the port is in use, wait for the other run to finish; do not kill a process
you did not start.

The screenshots in `site.spec.ts` come from a Baylee checkout (`BAYLEE_SRC` for
`cargo xtask site`); without one, that test is skipped.

`e2e-real` is not run by `npm run`: see `e2e-real/README.md`.
