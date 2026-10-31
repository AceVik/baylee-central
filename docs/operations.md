# Deployed operation

This runbook covers the deployed launcher and its database connector. It is an
operator procedure, not a release approval: the gates in [status](status.md)
still apply. Registration and sign-in on `baylee.app` are served by the browser
listener ([below](#the-browser-listener)) once it is configured; nothing here
enables password reset or browser SSO for gateways. Sessions renew
([below](#sessions)).

## Launcher modes

`central-service` takes exactly one mode argument and no other argument. Every
secret arrives as one bounded JSON document on standard input (at most 128 KiB),
never in argv or the environment. Unknown fields fail closed.

- `--accounts-stdin` serves the live native account protocol. The endpoint is
  always `https://central.baylee.app`, and the official gateway is the compiled
  `OFFICIAL_ORIGIN` with its compiled aliases (`central_api::OFFICIAL_ALIASES`,
  its old address, which is also the legacy migration's); none is configurable.
  Input:
  `listen`, `database`, `opaque_setup`, `bridge`, and optionally
  `browser: {"listen": …}` for the browser listener (absent means off) and
  `legacy: {"database": …}`, the legacy store's own login (absent means no
  legacy sign-in; [the legacy store](#the-legacy-store)).
- `--migrate-stdin` applies pending migrations once under the migration login
  and exits. Input: `database` only. The service never migrates by itself.
- `--issue-beta-keys-stdin` issues beta keys under the migration login (see
  [beta keys](#beta-keys)) and prints each once. Input: `database`, `count`,
  `uses`, `note` and optionally `expires_days`.
- `--legacy-import-stdin` and `--legacy-deadline-stdin` import the official
  gateway's legacy accounts once and set the legacy store's deadline, under the
  migration login ([the legacy store](#the-legacy-store)).
- `--import-reports-stdin` copies the feedback service's open reports once,
  under the migration login ([reports](#the-one-time-copy-of-the-feedback-services-reports)).
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
clears it. Sign-in and session status answer there with the account's
canonical username (`BrowserSession`, `BrowserStatus`). The site is fixed to
`baylee.app` in this mode; only the development launcher takes another.

Per-address budgets read the last `X-Forwarded-For` hop, which Caddy sets to
the client it saw: 5 registrations an hour, 30 discoveries and 30 sign-in starts
a minute, and 60 reads of the gateway directory a minute (`GatewayDirectory`,
which needs no session; the website's `/gateways` page). IPv6 addresses count by their /64. The counters live in memory only,
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
hostssl   baylee_central central_migrator,central_runtime,central_legacy all scram-sha-256
```

## Provisioning (once, as a database superuser)

Generate each password with a CSPRNG (`openssl rand -hex 32`) and keep it only
in the configuration files below. The runtime role must neither own nor be a member
of a role owning anything it uses; the store refuses to start otherwise.

```sql
CREATE ROLE central_migrator LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS PASSWORD '<migrator>';
CREATE ROLE central_runtime  LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS PASSWORD '<runtime>';
CREATE ROLE central_legacy   LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS PASSWORD '<legacy>';
CREATE DATABASE baylee_central OWNER central_migrator;
REVOKE CONNECT, CREATE, TEMP ON DATABASE baylee_central FROM PUBLIC;
GRANT CONNECT ON DATABASE baylee_central TO central_migrator, central_runtime, central_legacy;
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

## Administration

Central's administrators act on the website ([administration](administration.md));
the operator appoints the first one, and the way back in when none can sign
in, with the migrator login from a file the operator keeps:

```sh
central-service --appoint-admin-stdin < appoint.json
```

```json
{"database": {"host": "…", "port": 5432, "name": "baylee_central", "user": "central_migrator",
  "password": "…", "ca_certificate": "…"},
 "username": "…"}
```

The account must be enrolled. It prints `administrator appointed`, or
`already an administrator; nothing changed`, on standard error, and writes an
audit event with the operator as its actor. Keep the username out of the
repository and of shell history: the file holds it.

The audit keeps administrators' and the operator's actions for three years;
a daily timer deletes older events, the digests of expired reset codes and
expired pairings, with the migration document
(`/etc/baylee-central/migrate.json`), under the same `systemd-run` user as
the deploy hook's migration:

```ini
# /etc/systemd/system/baylee-central-sweep.service
[Service]
Type=oneshot
User=central
ExecStart=/opt/baylee-central/bin/central-service --sweep-stdin
StandardInput=file:/etc/baylee-central/migrate.json
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
PrivateTmp=yes

# /etc/systemd/system/baylee-central-sweep.timer
[Timer]
OnCalendar=daily
RandomizedDelaySec=1h
Persistent=true

[Install]
WantedBy=timers.target
```

The runtime role may grant and remove administrator roles and append audit
events, never rewrite or delete one (`ops/grants.sql`, checked at startup).
What a taken-over service gains by that is in [administration](administration.md)
and the [threat model](threat-model.md).

## Deleting and exporting accounts

A person deletes and exports their own account ([account
deletion](account-deletion.md)); nothing is deployed for it, no migration and
no grant. What stays the operator's:

- **An administrator who wants to leave:** another administrator removes the
  role first; the last one is replaced by an appointment (above) first.
- **An export that says `complete: false`:** a list passed its 256 entries.
  Answer the rest by hand, read-only, under the migrator login: the
  account's rows in `terms_acceptances`, `gateway_admins` and
  `audit_events` (as `account_id` and as `actor_account`).
- **A moved account's copy at the official gateway:** a move keeps the
  source data there, its password retired; a deletion at Central does not
  reach it. Until M8 removes it, delete that copy at the gateway when the
  person asks (the official gateway's own runbook).
- **An origin registered again after its owner was deleted:** the old
  owner's terms for that origin are still stored. Before approving the new
  registration, delete them under the migrator login (their texts go with
  them):

  ```sql
  DELETE FROM central_identity.gateway_terms WHERE audience = '<origin>';
  ```

- **Backups** keep a deleted account until they rotate out, at most 12
  months, as the privacy notice says.

## Gateways and their tokens

Since 30.10.2026 Central's administrators approve, disable and enable
gateways on the website (`/admin/gateways`), and owners mint and revoke their
gateways' tokens there ([gateway registry](gateway-registry.md) §"Gateway
tokens"). The deploy hook applies `ops/grants.sql` after the migration,
which gives the runtime `INSERT` and `UPDATE (enabled)` on
`gateway_approvals` and its rights on `gateway_tokens`; a binary from before
migration 000014 refuses to start over the wider grants, so roll back the
grants with the binary. The operator's SQL stays the fallback when no
administrator can sign in:

```sql
INSERT INTO central_identity.gateway_approvals (gateway_id)
    SELECT id FROM central_identity.gateways WHERE origin = 'https://…';
UPDATE central_identity.gateway_tokens SET revoked_at = clock_timestamp()
    WHERE gateway_id = (SELECT id FROM central_identity.gateways WHERE origin = 'https://…')
    AND revoked_at IS NULL;
```

## The official gateway

The directory lists the official gateway (the bridge's,
`https://gateway.baylee.app`, `central_api::OFFICIAL_ORIGIN`) from a row of its
own: no owner, no credential, so the bridge stays the only authority for its
origins ([gateway
registry](gateway-registry.md#discovery-heartbeats-and-administrators)).
Migration 000009 makes room for it; listing it is the operator's, under the
migrator login (`central_migrator`, which owns the tables), once:

```sql
INSERT INTO central_identity.gateways (official, origin, name, version, website)
    VALUES (true, 'https://gateway.baylee.app', 'Baylee', '0', 'https://gateway.baylee.app/');
INSERT INTO central_identity.gateway_approvals (gateway_id)
    SELECT id FROM central_identity.gateways WHERE official;
```

Until its first heartbeat the entry shows these values and offline. The
official gateway's `BAYLEE_CENTRAL_CONFIG_FILE` gains `metadata` beside its
`bridge` (Baylee `docs/central-login.md`), for example `"metadata": {"name":
"Baylee", "version": "0", "website": "https://gateway.baylee.app/"}`; the website
must be on one of its origins, and the gateway sends its own binary's version in
place of `version`. It then sends `GatewayHeartbeat` with the bridge credential every
30 seconds, which replaces the name, version and website and marks it online.
Without `metadata` the entry stays listed and offline. A second official row is
refused, as is an owner or a credential on it; `RegisterGateway` for the
origin answers `Conflict`.

**The move to `gateway.baylee.app` (30.10.2026).** The official gateway's
canonical origin is `OFFICIAL_ORIGIN`, and its old address is an alias
(`OFFICIAL_ALIASES`), both compiled: a ticket an installed client asks for the
old address is the bridge's, bound to that address; the bridge redeems it and
checks its leases under either; an acceptance of the gateway's terms under
either counts; the old address cannot be registered. The heartbeat finds the
row under either origin, so the operator moves the row in the deploy's window,
after checking that no registered gateway holds the new origin:

```sql
SELECT id FROM central_identity.gateways WHERE origin = 'https://gateway.baylee.app';  -- no rows
UPDATE central_identity.gateways SET origin = 'https://gateway.baylee.app',
    website = 'https://gateway.baylee.app/' WHERE official;
```

Until then the directory shows the old address, which the website does not
mark official. Dropping the alias is a release that empties
`OFFICIAL_ALIASES`, once installed clients ask for the new address. To hide the
entry, disable its approval:

```sql
UPDATE central_identity.gateway_approvals SET enabled = false
    WHERE gateway_id = (SELECT id FROM central_identity.gateways WHERE official);
```

## Display names and tags

Migration 000011 (the accounts and administration design, step 3; [web
accounts](web-accounts.md) §"Display name and tag") draws a tag for every
registered account from `0x1000` up, and moved accounts keep the official
gateway's tags, which must stay below that floor
(`profile::REGISTERED_TAG_FLOOR` in `central-api`, and the `4096` of the
migration's backfill). Before the migration runs in production, read the
official gateway's highest tag, guests included, on its own database:

```sql
SELECT max(tag), count(*) FROM account;
```

It answered `13 | 7` on 23.10.2026, so the floor stays. Were it `0x1000` or
more, the floor would move to the next multiple of `0x1000` above it, in
both places, before 000011 is applied. The official gateway makes no local
accounts once bound to Central, so the number cannot grow afterwards; a
carried tag that is taken anyway is replaced by a drawn one.

No grant changes. The deploy order: Central first (the hook does so: it
installs the revision the gateway links, migrates and grants before the
gateway starts), then the gateway whose bridge sends the name and tag
(Baylee pinned to this revision). The previous Central binary keeps working
on the migrated schema (it registers without a tag; the new one draws it at
that account's next sign-in), and a bridge from before keeps reserving
without name and tag. The privacy notice's new version (display name and
tag, `legal/privacy.*.md` §3, §6 and §9) has to be published before the
website offers the field.

## The legacy store

From the cutover until its deadline, Central signs in the official gateway's
accounts that had not moved, from its legacy store ([native
login](native-login.md#legacy-login)), and the gateway has no password route.
The store is schema `central_legacy` (migration 000015) with a login role of
its own; the runtime role has nothing in it.

**Once, before deploying a release with migration 000015** on a database
provisioned before it, as a superuser (`openssl rand -hex 32` for the
password), and add `central_legacy` to the `pg_hba.conf` line above:

```sql
CREATE ROLE central_legacy LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS PASSWORD '<legacy>';
GRANT CONNECT ON DATABASE baylee_central TO central_legacy;
```

`ops/grants.sql` grants it exactly what its startup check requires, and
nothing while the role does not exist; a service whose `central.json` has a
`legacy` block then refuses to start.

**The cutover,** in the official gateway's maintenance window, in this order
(Baylee `docs/central-login.md` §"Cutover"): stop the gateway and deploy its
release without password routes; then, as the migrator, in one pipe and
without a file in between:

```sh
{ cat /etc/baylee-central/migrate.json; sudo -u baylee env DATABASE_URL=… baylee-gateway legacy export; } \
  | central-service --legacy-import-stdin
```

The first document is the migration document (`{"database": …}`), the
second the gateway's export, which holds every remaining password hash and is
refused on a terminal. The import checks every row (canonical username, the
display-name and tag rules, the snapshot's shape and bound), refuses the whole
import for one bad row, for a username another Central account holds, for a
move closed already, for a non-empty store, or for a count or digest that is
not the export's, and stores nothing then. In one transaction it stores the
rows, reserves each account's identity under its name, and computes the count
and digest again from what it stored. Compare its line on standard error with
the export's: the counts and digests are equal. Then add the `legacy` block
(the `central_legacy` login, `{"database": {…, "user": "central_legacy", …}}`)
to `central.json`, restart Central, start the gateway.

**The deadline** is the owner's date, **30.06.2027** (end of day,
Europe/Berlin): migration 000015 sets it, so it holds without a step. The
operator moves it, or clears it, only on the owner's word:

```sh
central-service --legacy-deadline-stdin < legacy-deadline.json
```

```json
{"database": {"host": "…", "port": 5432, "name": "baylee_central", "user": "central_migrator",
  "password": "…", "ca_certificate": "…"},
 "deadline": "2027-06-30"}
```

The date is the last day a legacy sign-in is taken: the deadline is the end
of that day in Europe/Berlin; `null` clears it (the store then stays until a
date is set again). From then no legacy sign-in is taken, and the daily sweep (`--sweep-stdin`, [administration](#administration))
deletes every row left and every legacy reservation that never enrolled; it
deletes the row of a closed move at any time. Administrators see what is left
on the website, at `/admin/legacy` (`AdminRequest::LegacyAccounts`). After the deadline's sweep,
remove the `legacy` block, revoke the role's grants and drop the role; a later
release removes the schema.

**Refused moves.** A forced password change or a ban in force at the gateway
refuses the move, after a correct password, with the same answer as a wrong
one; the listing shows them. Ten failed sign-ins in a row lock an account out
for an hour (the listing shows until when).

## Reports (migration 000018)

Bug and crash reports ([reports](reports.md)) arrive on the native listener
beside `/v1/native`, from the bridge (the official gateway) or an approved
gateway's token with the `sso` scope:

- `POST /v1/reports`, JSON at most 64 KiB: `201 {"report_id": "<32 hex>"}`.
- `PUT /v1/reports/{id}/client` and `…/record`, `application/gzip` with a
  `Content-Length` (`411` without, `413` past 64 MiB): the credential and
  the report's announcement are checked before the body is read, which must
  be exactly the announced length; at most two uploads at once, each within
  60 seconds (the native listener's connections may last that long; every
  other request keeps its 5 seconds); `204`.

The browser listener adds `POST /api/v1/reports/{id}/client|record` for an
administrator's page, with every check of `POST /api/v1`, 120 a minute per
address; the answer is the gzip as stored. The proxy compresses the JSON
answers; the gzip parts it passes as they are.

The runtime's grants (`ops/grants.sql`, checked at startup like the
others): `SELECT, INSERT, DELETE` on `reports` and `UPDATE` of its triage
columns only; `SELECT, INSERT, DELETE` on `report_parts`, `report_notes`,
`report_seen` and `report_views` (with `UPDATE (query)`); `SELECT, INSERT`
on `report_audit`. The daily sweep (`--sweep-stdin`) deletes reports past
their retention and says how many. No secret and no setting is new.

### The one-time copy of the feedback service's reports

After the gateways file reports with Central (Baylee `BAYLEE_REPORTS_TO=central`),
the reports still open at the feedback service are copied once, with their
notes and history (Baylee `docs/feedback.md` §"The one-time copy to
Central"):

```sh
{ cat /etc/baylee-central/migrate.json
  echo '{"author": "<an administrator's account id, 32 hex digits>"}'
  sudo -u baylee-feedback sh -c 'set -a; . /etc/baylee-feedback/env; exec baylee-feedback export-reports'
} | central-service --import-reports-stdin
```

The migration document first, then the administrator the copied notes are
filed under on a line of its own, then the service's lines; its database
address is read from its own environment file, never from a command line.
In one transaction the import checks every report by the intake's rules
(and its status open, its labels, its issue), files it under the official
gateway with its id, times, triage and parts as they were, each note under
the author named, its first author's name in its text, and each line of the
old history at its time, its actor named in its detail; a report whose id is
here already is skipped. A count or SHA-256 that is not the lines', a line
out of the rules, a copy cut short or anything after its last line stores
nothing. Standard error says how many were stored and how many were here
already; run it again and every report is skipped.


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

For the 30 October 2026 release, the service executable and website package
are versioned `0.1.1-beta.9b`. Reusable library versions and the authentication
protocol are unchanged, and the wire changes since `0.1.1-beta.9a` are only
additions, so the `0.1.1-beta.9b` gateway, which still links `0.1.1-beta.9a`
(3ae5035), works beside it. The release is main at c531e9c (#42 to #50: password
change, administration, login reset, deletion and export, the website's design
guidelines, account area, gateway tokens and administration) plus this version
commit on `codex/release-0.1.1-beta.9b`, deployed by hand (`10-central manual`);
#51 (the official gateway's new origin) waits for the gateway that accepts it.
The 28 October release (`0.1.1-beta.9a`) was the sunlit Baylee memorial,
parchment story and photo viewer.


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

## Sessions

A sign-in issues a session for 30 minutes. A live session renews
(`RenewSession`, on both listeners) to 30 minutes from the renewal, but never
past 12 hours after the password sign-in that issued it (`created_at`); its
gateway leases move with it in the same transaction, so a game on a gateway
lasts as long as the session (migration 000008). The game client renews every
ten minutes while it runs, the website while a page is open and in view; after
the 12 hours the password is needed again. An expired, replaced or revoked
session renews nothing, and a renewal needs no accepted terms. The browser
cookie lives the 12 hours of the cap: an expired session's cookie finds no
session, so a renewal never sets the cookie again. The runtime role may update a
lease's `expires_at` and no other column of it (`ops/grants.sql`, checked at
startup).

## Known limits of this deployment

Request budgets on the native listener are global and per account or bearer,
not per client address: behind the proxy every client shares one address, so a
flood can exhaust the shared login budget for everyone (service denial, not
account takeover). The browser listener adds per-address budgets in front of the
same global ones; a client with many addresses still shares the global budget.
Central sessions last 30 minutes unless renewed, and at most 12 hours. Password change
is served on both listeners since 28.10.2026 ([native login](native-login.md)
§"Password change"; no migration or grant), with no form on the website or in
the client yet; reset and recovery are not served. Discovery reveals enrollment state for a supplied
username (see [native login](native-login.md)), on both listeners.
