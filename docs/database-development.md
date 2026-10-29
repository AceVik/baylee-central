# Database adapter and migration development

The server-side libraries use SeaORM 2.0.4 and sea-orm-migration 2.0.4.
PostgreSQL 18 is the minimum; the pinned local fixture reports 18.6.
Client, policy and crypto crates remain independent of database implementation.

## Current API and deliberately limited scope

- central-postgres::DevConfig accepts a port, bounded database/user identifiers
  and a password. It always targets 127.0.0.1, builds an escaped URL, disables SQL
  logging and bounds pool/connections/statements. It never reads DATABASE_URL.
- connect_dev is a low-level local fixture/operator connection; it does not grant
  authority or silently migrate anything. PendingAccountStore checks the runtime
  role before use, rejecting elevated attributes, direct role memberships,
  any schema CREATE, database CREATE/TEMP and owned identity tables. It requires
  SELECT/INSERT and denies mutation, maintenance, reference/trigger and grant-option
  privileges, including column grants. This is a startup check of this adapter's
  narrow role contract, not a complete audit of arbitrary database objects or
  security-definer functions. Only trusted operators may change roles/grants.
- central-migration::apply is an explicit operator API, separate from the normal
  service. It validates PostgreSQL 18+, takes a transaction-scoped advisory lock
  and runs the versioned registry under an outer transaction and timeouts.
  Unknown migration history is rejected. Failed DDL rolls back history too.
  History is fixed to the trusted public schema with a transaction-local search
  path and catalog-qualified gate functions. The raw SeaORM migrator is private;
  consumers cannot call its destructive or unguarded convenience methods.
  Missing/inaccessible public is an error, even if a temporary history table exists.
- The first migration creates central_identity.pending_accounts. PostgreSQL
  generates the primary UUID with uuidv7(); UUIDv7/variant and expiry constraints
  also apply to direct SQL. Reservations expire after 15 minutes.
- reserve uses SeaORM INSERT DEFAULT VALUES RETURNING. Public output contains
  only UUID bytes and expiry, not an ORM model. No caller chooses the ID.

This is an enrollment allocation primitive, not a registration API. Tickets,
quotas, expiry cleanup, activation, ownership and credential/wrapper transactions
are still required before exposing it. There is no automatic schema-sync.
The migration library takes a caller-owned SeaORM connection; that caller must
establish verified TLS outside explicit disposable fixtures.
- central-postgres::ServerConfig is the deployed connector: a loopback or private
  IP literal, bounded identifiers, a password and a PEM authority bundle, always
  `sslmode=verify-full`, the password and authority passed as typed driver options
  rather than in the URL, and a `pg_stat_ssl` check of the session. Both stores
  offer `connect` beside `connect_dev` and run the same role and grant checks on
  either path. See [operations](operations.md) for provisioning and limits.

## Running real database checks

Install Docker with Linux-container support and OpenSSL. Run:

```sh
cargo xtask gate
cargo xtask postgres
```

([xtask](xtask.md)). The latter creates a fresh pinned official PostgreSQL image with ephemeral data,
a loopback-only random published port, memory/CPU/process limits and random
synthetic credentials. DDL and runtime logins are distinct. PUBLIC database TEMP
is revoked; the test subprocess does not inherit the bootstrap administrator
password, PGPASSWORD or DATABASE_URL. No production
configuration is read. The fixture runs the explicitly ignored Rust integration
test and asserts that it actually executed successfully.

The exact owned child process, container and listener are cleaned and checked in
finally blocks, including failures. Its generated container name is printed so an
agent can identify that exact fixture after an external hard interruption. Docker
daemon failure is reported as unverified cleanup, never a successful removal.
To exercise failure cleanup, invoke `cargo xtask postgres --exercise-failure` and
expect the deliberate Synthetic fixture failure error after cleanup; CI checks
that path.

Coverage: failed migration leaves no history; repeat migration is idempotent;
parallel migration lock conflicts; unknown history; restricted and NOINHERIT
privilege paths; DDL denial; invalid UUID version/variant; explicit rollback and
dropped transaction; concurrent ORM allocations with unique DB UUIDv7 and expiry.
An actual task abort occupies the other three pool connections, forces reuse of
the cancelled connection's backend PID, checks rolled-back data/session settings
and verifies advisory-lock release from a separate connection. Privilege tests
include column UPDATE/REFERENCES, TEMP/CREATE, grant delegation and missing SELECT.
Migration checks also cover forged history in an alternate schema, temporary-table
shadowing before first installation, missing/inaccessible public, restored caller
settings after success/failure and an actual abort
while migration history is locked. The aborted migration's backend is reused and
its advisory lock must be available from a distinct connection.
The fixture serves TLS from a synthetic authority with a certificate for
127.0.0.1 only (openssl on the host; the key is copied into tmpfs as the database
user). The suite runs both stores, the migrator and an idempotent re-migration over
`connect_server`, and refuses a certificate from an unrelated authority and a wrong
password. `tests/tls_refusal.rs` needs no database: a listener answers the TLS
request with `N` and asserts that no plaintext startup follows; downgrading the
connector to `prefer` fails it. A certificate naming another address is refused
by the driver's verify-full mode but is not separately exercised here.
The fixture does not prove reset/recovery, credential atomicity, row-level
account ownership or PostgreSQL 19 compatibility.

## Migration rules

Append a new named migration; never edit an applied one. Keep database-specific
static SQL next to its SeaORM MigrationTrait implementation when it makes security
constraints clearer. User values go through bound parameters; user input never
becomes DDL. Production upgrades must have backup/restore evidence and explicit
operator execution under separate credentials.

The current migration's down operation deliberately rejects deletion. Its registry
is private, so SeaORM fresh/reset is not available through this library. Prefer
reviewed forward corrections.
Provision deployment roles/grants by the operations runbook; fixture bootstrap is
not an installer. New schema versions must pass fresh-install and prior-version upgrade,
partial-failure, lock, compatibility and privilege tests.

The adapter and the migrations belong to the server, which is built for Linux
x86-64 only (owner, 29.10.2026): CI compiles and unit-tests them there. Windows
and macOS CI compile them only as development dependencies of central-client's
tests and run none of their tests ([xtask](xtask.md#which-runner-runs-what)).
Until 29.10.2026 their unit tests also ran on Windows and both macOS
architectures. The Docker integration job runs on Linux CI and has also been
validated locally from Windows. Production secret loading remains explicit
follow-up validation.
