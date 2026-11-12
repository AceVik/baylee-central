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
   `ops/grants.sql` as the database owner, and `--accounts-stdin` with
   `"listen": "127.0.0.1:47920", "browser": {"listen": "127.0.0.1:47921"}`.

**The old build's specs.** `account`, `chat`, `area` and `budget` register
through the old build's page (`dist/`) with a beta key (`REAL_KEYS`). Since
the open beta (11.11.2026) Central issues no keys and wants its proof of work
instead, so they no longer run against a current Central; they go with `web/`
(`../docs/proposals/frontend-rewrite.md` §8, phase 5). `site-chat.spec.ts`
below registers through the new site.
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

```bash
REAL_KEYS=<keys file> REAL_OUT=<empty folder> REAL_SERVICE=<central-service> REAL_MIGRATE=<the migrator document> \
  npx playwright test -c e2e-real/playwright.config.ts e2e-real/area.spec.ts --project desktop
```

the account area and the administration (30.10.2026): an owner registers,
is appointed by the operator's own `--appoint-admin-stdin` (the spec runs it
with the migrator document, which it reads from the file and never prints),
registers `https://qa-gateway.invalid`, mints a token (kept in
`REAL_OUT/token`, mode 0600, masked in the pictures), changes its own tag and
approves the gateway; every page is photographed into `REAL_OUT`. Paste the
token into a local QA gateway's console (`PUT /admin/central/token`, the
gateway's `BAYLEE_CENTRAL_ENDPOINT` a loopback stand-in for Caddy that sets
`Host: central.baylee.app` in front of the native listener), then

```bash
REAL_OUT=<that folder> npx playwright test -c e2e-real/playwright.config.ts e2e-real/linked.spec.ts
```

sees the directory list it online with its deep link, in both schemes and at
both widths, and the owner's token used.

The rows Central stored, the native client's sign-in to such an account and
the recovery code's use were checked from outside the page; the run of
21.10.2026 is in `../docs/security-assurance.md` §"Account pages".

## The new site (`crates/central-web`)

The same Central serves the server-rendered site
(`../docs/proposals/frontend-rewrite.md`). Build it with `cargo xtask site`,
start `target/debug/central-web --dev target/site
crates/central-web-server/tests/data` (it listens on `127.0.0.1:28792`), and
put the front before it instead of `dist/`:
`node e2e-real/front.ts --tls <its folder> --site 28792`. Then

```bash
REAL_FRONT_SPKI=<pin> REAL_OUT=<empty folder> npx playwright test -c e2e-real/playwright.config.ts e2e-real/site-chat.spec.ts
```

`REAL_FRONT_SPKI` is the base64 SHA-256 of the throwaway certificate's
public key (`openssl x509 -in cert.pem -pubkey -noout | openssl pkey -pubin
-outform der | openssl dgst -sha256 -binary | base64`): Chromium registers
no service worker on an untrusted certificate, and the pages register one.

registers two accounts at each width without a key (the open beta; the page
solves Central's proof of work), makes them contacts by handle, has
them write to each other (markup in a message stays text), reloads one page
and finds the chat open again without a password (the identity the sign-in
kept in this browser), compares the safety number on both sides, creates a
group, invites, joins, writes to it and leaves it (the history stays, the
composer goes), switches a notification category off and on across a reload
(the device's own Web Push needs a push service and is not run), and checks
that localStorage holds neither a message nor the password.

```bash
REAL_FRONT_SPKI=<pin> REAL_SERVICE=<central-service> REAL_MIGRATE=<the migrator document> REAL_NATIVE=<the native listener> \
  npx playwright test -c e2e-real/playwright.config.ts e2e-real/site-reset.spec.ts e2e-real/site-password.spec.ts --project desktop
```

the reset page and the password's change (12.11.2026; steps shared in
`site.ts`). `site-reset.spec.ts` registers an administrator, whom the
operator's `--appoint-admin-stdin` appoints as in `area.spec.ts`, and an
account. The administrator makes reset codes on `/en/account/admin` (the
search, the account, the warning, its password once more, the code shown
once and copied), and the account redeems them on `/en/reset`: a recovery code
that is not its own stops before the code is spent; its own keeps the vault;
the used code is refused in the words of any wrong one; without the recovery
code a new vault begins at the next epoch and its new recovery code is shown
once; the old recovery code opens nothing any more and the new one does.
`site-password.spec.ts` registers one account and changes its password in
the account area: a wrong current password changes nothing, the right one
leaves this browser signed in and ends the client's session, the old
password is refused and the new one signs in. Both read what became of the
vault from outside the page: a sign-in over the native listener as Baylee's
client makes it (the committed module, `REAL_NATIVE`, default
`127.0.0.1:47920`) tells whether the session lives, the fingerprint of the
chat identity, which is derived from the master key (the same after a kept
reset and a password change, another after a new vault), and the stored
document with its revision. A passing run prints no password, code or
token and writes none; a failing one may, in its assertion's message and in
its trace (`web/test-results-real`, kept on failure), which holds what was
typed and sent, the reset code in `ResetStart` included. All of it is
synthetic; delete the folder after reading. One project registers three
accounts and starts six resets (ten an hour from one address): run one
project against a Central started afresh.

Central allows five registrations an hour and ten a day per address, and
counts them in memory: one run of `site-chat.spec.ts` registers four, so start Central afresh
between runs. Since 11.11.2026 it also counts sign-ins that were not
completed with the right password (30 an hour from one address, 10 an hour
at one account from one address; `docs/native-login.md` §"Guessing a
password online"), and every local run comes from one address: a Central
started afresh has counted none.

## Tearing down

Stop `front.ts` and Central (SIGTERM; their ports must refuse), remove the
container by its label, and delete the keys file, the configuration
documents, the passwords and the certificates' keys.
