# e2e-real — the account pages against a real Central

The account pages' everyday tests run against a strict mock of Central's
browser surface (`../e2e/central.ts`). This folder runs the same pages against
a real Central in its deployed mode (`--accounts-stdin` with
`browser.listen`), with real OPAQUE and Argon2 in the page's Worker and the
real `__Host-central` cookie. It is not part of `npm run e2e` or CI: it needs a
PostgreSQL and a running Central, set up by hand as below. Synthetic data
only; every login, key and certificate is made for the run and deleted after.

The browser must really be at `https://baylee.app`: the browser listener
takes only that `Host` and `Origin`, and a `__Host-` cookie needs HTTPS.
Chromium maps `baylee.app:443` to a local stand-in for the site's Caddy
(`front.ts`: TLS, `dist/` with the production CSP, `/api/*` to the listener
with the Host unchanged and `X-Forwarded-For` set), and the tests accept its
throwaway certificate. The deployed mode compiles in the live issuer, which
the committed WebAssembly module is bound to: no development build is
needed.

## Setting up (from the repository root)

1. **PostgreSQL 18 with TLS**, as `cargo xtask postgres` makes it: a
   throwaway authority, a server certificate naming only `127.0.0.1`, the
   pinned image (`crates/xtask/src/postgres/container.rs`) started with
   `ssl=on`, published on `127.0.0.1` only, under a label of its own.
2. **Provisioning** as in `docs/operations.md` §"Provisioning", with fresh
   `openssl rand -hex 32` passwords fed to `psql` on standard input.
3. **Central**, each document on standard input with `umask 077`:
   `central-service --generate-secrets`, `--migrate-stdin`, then
   `ops/grants.sql` as the database owner, `--issue-beta-keys-stdin` (count 2,
   uses 3) into a keys file, and `--accounts-stdin` with
   `"listen": "127.0.0.1:47920", "browser": {"listen": "127.0.0.1:47921"}`.
4. **The site**: `npm run build`, a certificate for `baylee.app`
   (`openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:P-256 -nodes
   -subj /CN=baylee.app -addext subjectAltName=DNS:baylee.app`), then
   `node e2e-real/front.ts --tls <its folder>`.

## Running (in web/)

```bash
REAL_KEYS=<keys file> REAL_OUT=<empty folder> npx playwright test -c e2e-real/playwright.config.ts e2e-real/account.spec.ts
```

registers one account at each width with the display name `Real_Runner`,
sees the handle with the tag Central drew on the recovery step, the account
page and the header, keeps its recovery code in `REAL_OUT`, checks the
cookie, signs out and in, and meets a taken name and a bad key.

```bash
REAL_OUT=<that folder> npx playwright test -c e2e-real/playwright.config.ts e2e-real/terms.spec.ts
```

needs Central's terms in force: a service built from a checkout whose
`legal/terms.*.md` name a version instead of `draft` (never committed). One
account accepts, the other declines in German.

```bash
REAL_SECRETS="$(<the secrets document>)" REAL_OUT=<that folder> npx playwright test -c e2e-real/playwright.config.ts e2e-real/moved.spec.ts
```

moves one synthetic account per width from the official gateway as Baylee's
client and the bridge do (`LegacyReserve` with the bridge credential of the
secrets document, which it takes from the environment and never prints,
enrollment without a recovery wrapper, restore,
`LegacyClose`, over the native listener at `REAL_NATIVE`, default
`127.0.0.1:47920`), then signs in on the page, accepts the terms where they
are in force, keeps the code the sign-in made in `REAL_OUT` and sees the
account page say the account has one.

```bash
npx playwright test -c e2e-real/playwright.config.ts e2e-real/budget.spec.ts --project desktop
```

on a freshly started Central: the sixth registration in an hour is refused.

The rows Central stored, the native client's sign-in to such an account and
the recovery code's use were checked from outside the page; the run of
21.10.2026 is in `../docs/security-assurance.md` §"Account pages".

## Tearing down

Stop `front.ts` and Central (SIGTERM; their ports must refuse), remove the
container by its label, and delete the keys file, the configuration
documents, the passwords and the certificates' keys.
