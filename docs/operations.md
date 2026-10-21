# Deployed operation

This runbook covers the deployed launcher and its database connector. It is an
operator procedure, not a release approval: the gates in [status](status.md)
still apply. Registration and sign-in on `baylee.app` are served by the browser
listener ([below](#the-browser-listener)) once it is configured; nothing here
enables password reset, session renewal or browser SSO for gateways.

## Launcher modes

`central-service` takes exactly one mode argument and no other argument. Every
secret arrives as one bounded JSON document on standard input (at most 128 KiB),
never in argv or the environment. Unknown fields fail closed.

- `--accounts-stdin` serves the live native account protocol. The endpoint is
  always `https://central.baylee.app` and the sole legacy gateway is the compiled
  `https://baylee.acevik.de`; neither is configurable. Input:
  `listen`, `database`, `opaque_setup`, `bridge`, and optionally
  `browser: {"listen": …}` for the browser listener (absent means off).
- `--migrate-stdin` applies pending migrations once under the migration login
  and exits. Input: `database` only. The service never migrates by itself.
- `--issue-beta-keys-stdin` issues beta keys under the migration login (see
  [beta keys](#beta-keys)) and prints each once. Input: `database`, `count`,
  `uses`, `note` and optionally `expires_days`.
- `--generate-secrets` writes a fresh `{"opaque_setup": [...], "bridge": [...]}`
  document to standard output and refuses when standard output is a terminal.
- `--dev` and `--dev-accounts-stdin` are the local development modes described
  in [dev mode](dev-mode.md) and [native login](native-login.md).

`listen` is a numeric loopback socket address. The listener speaks plain HTTP
on loopback only; a TLS reverse proxy terminates `https://central.baylee.app`
and must pass the original `Host` header unchanged (Caddy's `reverse_proxy`
does). Every other Host, an `Origin` or Fetch-Metadata header is refused.
Forwarded headers confer nothing. `/health/live` answers; `/health/ready`
deliberately answers 503, so no proxy or supervisor health check may use it.
SIGTERM or Ctrl-C closes the listener, drains briefly and closes the pool.

## The browser listener

`browser.listen` in `central.json` (`{"browser": {"listen": "127.0.0.1:28791"}}`)
starts a second listener in the same process, for the website's registration
and sign-in ([web accounts](web-accounts.md)). It must be a numeric loopback
address and differ from `listen`; both are bound before the database is
reached. One signal stops both, and either one stopping stops the other.

It serves one route, `POST /api/v1`, with the operations of
`central_identity::browser_operation`, and answers 403 (`request not
permitted`) to anything that is not the site's own page: a `Host` other than
exactly `baylee.app`, an `Origin` other than exactly `https://baylee.app`,
`Sec-Fetch-Site` other than `same-origin`, a content type other than
`application/json`, no `X-Baylee-Central: 1`, any `Authorization`, a peer that
is not loopback, or an operation outside the list. No CORS header is ever sent.
Bodies are at most 64 KiB. Errors keep the native route's statuses and fixed
text. The browser session is the cookie `__Host-central` (`Secure`, `HttpOnly`,
`SameSite=Strict`, `Path=/`, `Max-Age=1800`), never in a body; signing out
clears it.

Per-address budgets read the last `X-Forwarded-For` hop, which Caddy sets to
the client it saw: 5 registrations an hour, 30 discoveries and 30 sign-in starts
a minute. IPv6 addresses count by their /64. The counters live in memory only,
at most 4096 addresses per budget, and forget an address when its window has
passed. A request without a well-formed last hop is refused (400). Do not set
Caddy's `trusted_proxies` for this site unless a CDN in front of it appends its
own hop: without it Caddy ignores a client's own `X-Forwarded-For`.

The `baylee.app` site block (`web/README.md` §"Serving it") passes `/api/*`
to it instead of answering 404, with the `Host` unchanged (Caddy's default for
a plain-HTTP upstream):

```caddyfile
	# Central's browser listener (docs/web-accounts.md).
	handle /api/* {
		request_body {
			max_size 64KiB
		}
		reverse_proxy 127.0.0.1:28791
	}
```

## Database transport

`database` holds `host`, `port`, `name`, `user`, `password` and
`ca_certificate` (a PEM bundle). The connector (`central_postgres::ServerConfig`)
always uses `sslmode=verify-full`: the server certificate must chain to the
configured authority and carry the configured address as an IP subject
alternative name. A server that declines TLS ends the attempt; no plaintext
startup message or password follows (`tests/tls_refusal.rs`). After connecting,
the pool checks `pg_stat_ssl` for its own session. The password and authority
reach the driver through typed options, never through the connection URL.

The host must be a loopback or private IP literal. The driver's trust store is
the public web roots plus the configured authority, not that authority alone;
public authorities do not certify loopback or private addresses, so only the
configured authority can vouch for such a server. DNS names are refused because
any public authority could vouch for one. A CA-only verifier would lift this.

Server side, PostgreSQL needs `ssl = on` with a certificate for that address
and its key owned by the database user with mode `0600`. Central's database must
refuse unencrypted sessions in `pg_hba.conf`, above every other `host` line:

```text
hostnossl baylee_central all all reject
hostssl   baylee_central central_migrator,central_runtime all scram-sha-256
```

## Provisioning (once, as a database superuser)

Generate each password with a CSPRNG (`openssl rand -hex 32`) and keep it only
in the configuration files below. The runtime role must neither own nor be a member
of a role owning anything it uses; the store refuses to start otherwise.

```sql
CREATE ROLE central_migrator LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS PASSWORD '<migrator>';
CREATE ROLE central_runtime  LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS PASSWORD '<runtime>';
CREATE DATABASE baylee_central OWNER central_migrator;
REVOKE CONNECT, CREATE, TEMP ON DATABASE baylee_central FROM PUBLIC;
GRANT CONNECT ON DATABASE baylee_central TO central_migrator, central_runtime;
\connect baylee_central
REVOKE CREATE ON SCHEMA public FROM PUBLIC;
```

Then run `--migrate-stdin` with the migrator login, and grant the runtime role
exactly the privileges its startup checks require: `ops/grants.sql`, as
`central_migrator`, which owns the tables, or as a superuser, whose grant
PostgreSQL records as the owner's. It is idempotent; the deploy hook runs it
after every migration.

A later migration that adds a table also adds its grant to `ops/grants.sql`; the runtime
store's startup check names every table and privilege it needs. Upgrading a
running service across such a migration: stop it, migrate, grant, start it;
started before the grant, it refuses to run (the check fails closed).

The terms tables (`docs/terms.md`) get `SELECT, INSERT` only: published
gateway versions and acceptances are immutable records, and an account's
acceptances go with the account by cascade.

## Beta keys

Registration on `baylee.app` needs a beta key ([web accounts](web-accounts.md)).
Issue them with the migrator login, from a file the operator keeps, as for
`--migrate-stdin`:

```sh
central-service --issue-beta-keys-stdin < issue.json
```

```json
{"database": {"host": "…", "port": 5432, "name": "baylee_central", "user": "central_migrator",
  "password": "…", "ca_certificate": "…"},
 "count": 10, "uses": 1, "expires_days": 30, "note": "October playtest"}
```

`count` is 1 to 100 keys, `uses` 1 to 1000 registrations per key,
`expires_days` (optional) 1 to 3650 days, `note` at most 200 characters without
control characters. Everything is checked before the database is reached; the
keys of one run are written in one transaction, all or none. Each key is printed
once, one per line, as `CENTRAL-XXXX-XXXX-XXXX-XXXX` (80 random bits in
Crockford base32); a count goes to standard error. Central keeps only the
SHA-256 digest: a lost key cannot be printed again, only replaced.

Keys are read back case-insensitively, with or without the `CENTRAL` prefix and
the hyphens, `O` read as `0` and `I` or `L` as `1`. A registration reserves one
use while its enrollment ticket lives (10 minutes) and spends it when the
enrollment commits; an expired reservation gives its use back. Revoke a key as
an operator, which ends its pending registrations too:

```sql
UPDATE central_identity.beta_keys SET revoked_at = clock_timestamp() WHERE note = 'October playtest';
```

The runtime role may only `SELECT` keys and `UPDATE` their `uses_left`; it can
neither issue, revoke, re-date nor delete one, and the store refuses to start
with any wider or narrower grant.

## Secrets

Run `umask 077; central-service --generate-secrets > secrets.json` once and merge
the two values into the service configuration. The OPAQUE setup must never be
regenerated: replacing it locks every enrolled account out. Keep an offline
copy. The bridge credential is shared only with the official gateway's
`BAYLEE_CENTRAL_CONFIG_FILE`, and only when legacy migration is enabled.

The OPAQUE setup together with a copy of the credential table permits offline
password guessing. Treat any backup that holds both as credential material.

## Supervision

Keep the configuration root-owned `0600` and let the supervisor open it, so the
service identity never reads the file itself:

```ini
[Service]
User=central
ExecStart=/opt/baylee-central/bin/central-service --accounts-stdin
StandardInput=file:/etc/baylee-central/central.json
Restart=on-failure
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
PrivateTmp=yes
PrivateDevices=yes
CapabilityBoundingSet=
RestrictAddressFamilies=AF_INET AF_INET6 AF_UNIX
MemoryDenyWriteExecute=yes
SystemCallFilter=@system-service
```

## Deploying with the gateway

`ops/deploy-hook` keeps Central and the website (`web/`) in lockstep with the
official gateway on the same machine. It is a Baylee deploy hook (Baylee's
`docs/deploy-hooks.md`): installed root-owned, `0755`, as
`/etc/baylee/deploy-hooks.d/10-central`, it runs in every gateway deploy,
including the one the gateway's timer starts for a new release tag.

- **Which revision.** The one the gateway being deployed links
  (`central-api`'s `rev` in its `Cargo.toml`), so the service speaks that
  gateway's protocol. Central never goes back: when the installed revision
  (`/opt/baylee-central/state/deployed`) already contains it, the hook does
  nothing, so a newer Central deployed by hand stays.
- **`prepare`.** As the build user it fetches, checks the revision out in
  `/opt/baylee-central/src`, builds `central-service` and the website (`npm
  ci`, `npm run content` from the gateway's checkout, `npm run build`), and
  stages both with `ops/grants.sql` in `/opt/baylee-central/staged`. A failed
  fetch answers "not ready" (75), so the next timer run tries again.
- **`before-switch`** (no game is running, admission is held): stops the
  unit, installs the binary (the old one kept as `central-service.previous`),
  migrates (`--migrate-stdin` from `/etc/baylee-central/migrate.json` under
  `systemd-run` as `central`), grants, starts it and waits up to 30 s for
  `/health/live`; then swaps the website into `/opt/baylee-app/web`. A failed
  migration, grant or start puts the previous binary back, starts it and
  fails the hook, which stops the deploy (migrations are transactional).
- **By hand,** for a Central or website change between gateway releases:
  `sudo /etc/baylee/deploy-hooks.d/10-central manual <central revision>`
  stages and switches that revision at once; nothing holds games for it.

Settings, all optional, in `/etc/baylee-central/deploy-hook.env` (root's,
not writable by others): `BUILD_USER`, `BAYLEE_SRC`, `CENTRAL_ROOT`,
`WEB_ROOT`, `UNIT`, `LISTEN`, `DB_CONTAINER`, `DB_SUPERUSER`, `DB_NAME`; the
defaults are this deployment's. The website's site block is the one in
`web/README.md` §"Serving it", with `root` on `WEB_ROOT`.

## Known limits of this deployment

Request budgets on the native listener are global and per account or bearer,
not per client address: behind the proxy every client shares one address, so a
flood can exhaust the shared login budget for everyone (service denial, not
account takeover). The browser listener adds per-address budgets in front of the
same global ones; a client with many addresses still shares the global budget.
Central sessions expire after 30 minutes without renewal. Password change and
recovery are not served. Discovery reveals enrollment state for a supplied
username (see [native login](native-login.md)), on both listeners.
