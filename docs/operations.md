# Deployed operation

This runbook covers the deployed launcher and its database connector. It is an
operator procedure, not a release approval: the gates in [status](status.md)
still apply, and nothing here enables registration, password reset, session
renewal or browser SSO.

## Launcher modes

`central-service` takes exactly one mode argument and no other argument. Every
secret arrives as one bounded JSON document on standard input (at most 128 KiB),
never in argv or the environment. Unknown fields fail closed.

- `--accounts-stdin` serves the live native account protocol. The endpoint is
  always `https://central.baylee.app` and the sole legacy gateway is the compiled
  `https://baylee.acevik.de`; neither is configurable. Input:
  `listen`, `database`, `opaque_setup`, `bridge`.
- `--migrate-stdin` applies pending migrations once under the migration login
  and exits. Input: `database` only. The service never migrates by itself.
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
exactly the privileges its startup checks require (as `central_migrator`, which
owns the tables):

```sql
GRANT USAGE ON SCHEMA central_identity TO central_runtime;
GRANT SELECT, INSERT ON central_identity.pending_accounts TO central_runtime;
GRANT SELECT, INSERT, UPDATE, DELETE ON central_identity.accounts, central_identity.sessions,
  central_identity.gateway_tickets, central_identity.private_objects TO central_runtime;
GRANT SELECT, INSERT ON central_identity.migration_archives TO central_runtime;
GRANT SELECT, INSERT, UPDATE ON central_identity.gateways TO central_runtime;
GRANT SELECT ON central_identity.gateway_approvals TO central_runtime;
GRANT SELECT, INSERT, DELETE ON central_identity.gateway_admins, central_identity.gateway_leases TO central_runtime;
GRANT SELECT, INSERT ON central_identity.gateway_terms, central_identity.gateway_terms_texts,
  central_identity.terms_acceptances TO central_runtime;
```

A later migration that adds a table also adds its grant here; the runtime
store's startup check names every table and privilege it needs. Upgrading a
running service across such a migration: stop it, migrate, grant, start it;
started before the grant, it refuses to run (the check fails closed).

The terms tables (`docs/terms.md`) get `SELECT, INSERT` only: published
gateway versions and acceptances are immutable records, and an account's
acceptances go with the account by cascade.

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

## Known limits of this deployment

Request budgets are global and per account or bearer, not per client address:
behind the proxy every client shares one address, so a flood can exhaust the
shared login budget for everyone (service denial, not account takeover). Central
sessions expire after 30 minutes without renewal. Registration, password change
and recovery are not served. Live discovery reveals enrollment state for a
supplied username (see [native login](native-login.md)).
