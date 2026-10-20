# PostgreSQL and identifier policy

Owner decision, 2026-10-19: PostgreSQL **18 or newer** is the transactional
database. PostgreSQL 18 is the minimum compatibility target; later stable major
versions require the same migration and integration suite before support is
claimed. Pin the deployment to a reviewed minor release and container digest,
with regular security updates. No SQLite substitution in integration tests.

## IDs and client-side encryption

Use native `uuid` columns with `DEFAULT pg_catalog.uuidv7()` and primary/unique
constraints. Fetch the allocated ID with `INSERT ... RETURNING id`; avoid an
extra select, timestamps used as uniqueness substitutes, UUIDv4 defaults and
application-side generation for normal central writes.

PostgreSQL 18 provides UUIDv7 generation without an extension. A version check
at migration/startup must reject PostgreSQL < 18. Validate stored central IDs
with `CHECK (pg_catalog.uuid_extract_version(id) IS NOT DISTINCT FROM 7)`.
A UUID is public metadata, reveals approximate creation time and is neither an
authorization check, an unpredictable token, nor a global event-order guarantee.
Source: https://www.postgresql.org/docs/18/functions-uuid.html

New-account enrollment first allocates an immutable pending account ID in
PostgreSQL, under a bounded single-use registration authorization. The client
uses this ID and its trusted deployment identity in the OPAQUE/vault context.
Only successful enrollment atomically activates that reserved account and stores
its credential and encrypted master-key wrapper. No active account or session
is created merely by reserving a UUID. Pending records expire; rate limits and
quotas prevent unbounded reservations. Never reassign an expired identity.

Local/offline objects may need a client-generated UUIDv7 before any network
access (e.g. an encrypted offline deck). This is an explicit exception, validated
as UUIDv7 on import, collision-checked and bound to the authenticated owner.
Never silently remap an ID already included in authenticated encryption:
reject a conflict, or have the client decrypt/re-encrypt under a new context.
IDs from outside systems stay in separate legacy/external reference fields.

Unknown-account login must not return a fresh random fake account ID on every
attempt while real IDs remain stable: that would defeat enumeration defenses.
The account-discovery/fake-record design needs a separate adversarial test before
login endpoints are exposed. ID allocation does not resolve this protocol gate.

## Transaction and privilege requirements

- The runtime role is not a database owner, superuser, migration role or
  BYPASSRLS role. Separate credentials and explicit schema grants for identity,
  personal metadata, directory, intake and migrations; revoke public creation.
- All dynamic values use bound parameters. Object names come from fixed code.
  Do not allow callers to supply schema/search_path/SQL fragments.
- Remote connections require verified TLS and hostname validation. No opportunistic
  plaintext fallback. Loopback-only throwaway fixtures may use an explicit dev
  transport exception with synthetic data.
- A password change commits the new OPAQUE record, the wrapper for the same
  master key, credential revision and revocation state in one transaction.
  Compare-and-swap detects stale writes; bounded retries handle serialization
  conflicts, with an idempotency record for unknown commit outcomes.
- Reset without a usable recovery method preserves the old encrypted vault.
  A new vault requires an explicit client choice and a distinct epoch.
- Explicit foreign keys, ownership constraints and transaction locks protect
  cross-account isolation and last-owner invariants; authorization also occurs
  in the service. Any RLS policy is defense in depth and must be tested under the
  real restricted role, including pool reuse.
- Bound pool size, statement/lock/idle-transaction timeouts, connection lifetime,
  result sizes and pagination. Cancellation must release/rollback the connection
  before reuse. Never log SQL bind values, credentials or private records.
- Migrations use a separate operator command and lock. Application startup never
  silently migrates production. Restore/backup procedures must preserve encrypted
  wrappers, OPAQUE server-key version references and schema compatibility.
  Database backups alone must not contain vault plaintext or client keys.

## Integration gate

SeaORM 2.0.4 and sea-orm-migration 2.0.4 are selected. Versioned migrations live
in a separate crate; no entity schema-sync or automatic destructive schema reset
runs in the service. Pending migrations fail readiness; an explicit operator
command applies them under a migration lock and separate DDL credentials.
Every migration needs forward/rollback policy, failure-atomicity and upgrade tests.

The owner accepted the reviewed PostgreSQL/TLS dependency graph after its
inventory and checks. The local adapter, the verified-TLS deployed connector and
versioned migration are implemented (deployed transport: [operations](operations.md));
see database-development.md for executed checks and remaining production gates. Keep the adapter in its own library;
shared policy, crypto and client APIs must not depend on database implementation.

Use disposable PostgreSQL 18+ fixtures with exact owned-container cleanup.
Test database-generated v7 IDs, concurrent uniqueness, rejecting v4/null-version
IDs and unsupported database versions, enrollment rollback, atomic credential
change, idempotent retry, foreign-owner access, role restrictions and cancelled
transactions. Run native clients on Linux, Windows and macOS; Linux-hosted
PostgreSQL in CI may serve that cross-platform adapter suite.
