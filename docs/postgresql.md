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

- The legacy store (schema `central_legacy`, [native login](native-login.md#legacy-login))
  has its own login role, the only one that reads it; the runtime role has
  nothing in it, and the legacy role nothing outside it, both checked at
  startup.
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

## Account chat identifiers

The social tables added by migrations 000019–000021 use account UUID foreign
keys, never public tags as authorization. Chat devices, messages, deliveries,
groups and member records have database UUIDv7 primary keys. A message's separate
`correlation` UUIDv7 is generated by its offline-capable sender before sealing and
is bound by the signed header, checked on ingestion and unique per sender. This is
the explicit offline-client exception above, not a replacement for INSERT RETURNING.
The delivery `seq` identity column is a JavaScript-safe ordering cursor. Recipient
account locks cover allocation through commit; no cross-recipient ordering or
unguessability is claimed. See central-chat.md for migrations and exact grants.

## Gateway activity (migration 000025)

`m20261111_000025_gateway_activity` adds five nullable columns to
`central_identity.gateways`: `activity_players`, `activity_playing`,
`activity_tables` and `activity_games` (`integer`) and `activity_at`
(`timestamptz`), for what a gateway reports with `GatewayPulse`
([gateway registry](gateway-registry.md#activity-and-the-public-figures)).
Three CHECKs hold the shape: all five are set or all five are NULL
(`gateway_activity_whole`), each count is between 0 and 100 000
(`gateway_activity_bounds`, the wire's `GatewayActivity::MOST`), and the
players in a game are at most the players connected
(`gateway_activity_playing`: `activity_playing <= activity_players`). Existing
rows keep every column and start with no activity.

`activity_playing` and its CHECKs were written into this migration on the day
it was made (11.11.2026), not added by a later one: 000025 had run on
throwaway databases only. A database that ran its first form has the
migration recorded and lacks the column, and the service's heartbeat and its
read of the figures fail on it; such a database is made anew, not repaired.

- **Grants.** None are added. The runtime's table-level `UPDATE` on `gateways`
  covers the new columns, and the startup check's list is unchanged.
- **Writes.** Only the heartbeat's own `UPDATE` touches them, in the statement
  that sets `last_seen` under the five-second condition: a pulse sets the
  four counts and the time, a plain heartbeat sets all five to NULL. No
  other statement of the service writes them.
- **Reads.** `PublicFigures` reads one page of at most 100 rows by primary
  key, joined to enabled approvals, with `last_seen` and `activity_at` inside
  90 seconds. There is deliberately no index on the new columns: every pulse
  writes `activity_at`, and an index over it would take the in-place (HOT)
  update from each of them. The read walks the table as the directory's
  does; rows that are pending or silent are passed over, so a table with
  very many of them makes both reads slower in the same way (an account
  registers at most eight gateways).
- **The account count.** `count(*)` over `accounts` with `credential IS NOT
  NULL` is one scan of that table. The service runs it once in 60 seconds
  and keeps the number in memory: when the minute is over, the one request
  that notices counts and the others answer the earlier number meanwhile.
  Only before a service's first count has returned does every request
  count, bounded by the public budget. No table or column holds the number.
- **Earlier binaries.** The previous release's heartbeat names none of the new
  columns and runs unchanged over the migrated table (its statement is in the
  upgrade fixture). It leaves a stored activity as it is, which matters only
  if the two releases beat for one gateway in turn; the 90 seconds bound it.
- **Number.** 000024 is left free: the session-token line of work has its own
  migration with the number 000023 beside this line's `open_registration`,
  and one of them takes the next number when the two meet. The order that
  counts is the list in `central-migration/src/lib.rs`; this migration
  depends only on the `gateways` table of 000004 and 000009.

## Report levels, a bearer's owner and an inbox's room (12.11.2026)

Three reads were added and no migration: each reads tables the runtime role
read before, so `ops/grants.sql` and the startup check are unchanged. The
number 000027 was kept for this work and is not used.

- **Report levels** ([reports](reports.md#what-the-reports-may-take-12112026)).
  `file_report` weighs before it inserts. In its transaction it first takes
  `pg_try_advisory_xact_lock(1862384219, 2)`: the migrations' first key with
  a second key of its own (theirs is 1), so the two never meet. It does not
  wait: if another filing holds the lock, the answer is `Busy`, and the
  lock ends with the transaction. Then one aggregate sums what the gateway
  holds and what all hold: for each row of `reports`, 65,536 bytes and the
  size it announced for each part that is stored or still awaited
  (`report_parts`, grouped by report). No index serves it and no counter
  is kept: it reads every report row there is, so its cost grows with the
  reports kept (up to three years of them). The parts' bytes are not read.
  A counter kept by triggers would need a migration and was not built.
  `ops/report-levels.sql` is the same sum for an operator, by gateway.
- **A bearer's owner** ([native login](native-login.md#a-bearer-nobody-issued)).
  `bearer_owner` is one statement of three lookups by a unique digest each
  (`gateways.credential_digest`, `gateways.heartbeat_digest`,
  `gateway_tokens.digest`), asked when an answer first vouches for a
  bearer. It answers nothing for a session and for the official gateway.
- **An inbox's room** ([chat](central-chat.md#full-inboxes-and-one-senders-share-12112026)).
  A send counts, for each recipient device, its deliveries and how many of
  them are the sender's: `chat_deliveries` by `(device, message_id)`,
  joined to `chat_messages`, of which one sender has at most 5,000
  (`(sender, correlation)`). Before, it counted the deliveries only. The
  count still reads every delivery of every recipient device, 5,000 each
  at the most. The plan was not looked at under load.


## Profile pictures (migration 000027)

[Profile pictures](profile-pictures.md) add four tables to
`central_identity`, each hanging on the account's row and gone with it;
the previous binary names none of them, so it runs unchanged over the
migration. (The report-levels section above says 000027 was kept and not
used there; this work takes it.)
- **`pictures`**: one row per account: a 16-byte random `key`, unique and
  new at every upload; the still's `avif` (32 B to 160 KiB) and `png`
  (67 B to 320 KiB); an optional `sequence` (32 B to 512 KiB) for a picture
  that moves; `sha256`, the digest of the PNG master followed by the
  sequence, computed by the service; `side` (256); `set_at`. The three
  files are stored out of line (`STORAGE EXTERNAL`).
- **`picture_reports`**: `uuidv7()` ids, the reported digest, the reporter
  (set to NULL when that account is deleted), a reason from the closed
  list, an optional note of at most 1,000 characters, the state and its
  decision; one open report per reporter and picture.
- **`picture_notices`**: the last removal's reason and note, and a block
  until a time or until lifted.
- **`picture_budgets`**: the hour's and the day's upload counts.

The runtime may `SELECT`, `INSERT` and `DELETE` pictures and update only
`key`, `avif`, `png`, `sequence`, `sha256`, `side` and `set_at` (never the
account); it files and decides reports by column but never deletes one;
the operator's sweep deletes decided reports a year after their decision
(`ops/grants.sql`, checked at startup). The upgrade fixture runs 000027
over 000026 with its bounds, the sequence's among them, and the cascade.

## An account's e-mail address (migration 000026)

`m20261112_000026_email` adds two tables in `central_identity`, for an
account's optional address ([e-mail](email.md)). Nothing else in the schema
knows an address. Both tables have the account's id as primary key and
`REFERENCES accounts(id) ON DELETE CASCADE`, so an account has at most one
row in each and both go with it. The `accounts` table is not changed.

`account_emails` holds the address an account confirmed:

- `address` (`text`): 5 to 254 bytes of printable ASCII around one `@`.
  The CHECK is looser than the wire's rule (`central_api::email::address`),
  which the service applies before a row is written; it keeps out what is
  no address at all. There is **no unique constraint**: two accounts may
  hold the same address.
- `lang` (`text`): the language tag the confirming code was asked in, which
  a later notice to the address is written in.
- `confirmed_at` (`timestamptz`), from `clock_timestamp()`.
- `reset_digest` (`bytea`, 32 bytes or NULL): the digest of the reset code
  Central last mailed for the account.
- `reset_mailed` (`timestamptz[]`, at most 3): when it mailed the last
  ones.

`account_email_pending` holds an address that waits for its code: `address`
and `lang` as above, `code_digest` (`bytea`, 32 bytes;
`MailCode::digest`, SHA-256 over a label, the account's id and the code),
`expires_at`, and `tries` (`smallint`, 0 to 4): the fifth wrong code deletes
the row. An index on `expires_at` serves the sweep. No code is stored, only
digests.

- **Whose reset code.** The reset code stays where migration 000013 put
  it, in `accounts.reset_digest` and `reset_expires`, and the statements
  that use, clear and sweep it are unchanged. A code on the account's row
  was mailed exactly when `account_emails.reset_digest` holds the same
  digest. Every other code is an administrator's. The statement that makes
  a mailed code requires that the account's row holds no code, an expired
  one or a mailed one; so it never replaces an administrator's code that
  lasts. An administrator's new code replaces the digest on the account's
  row and not the one beside the address, and is an administrator's by the
  same rule.
- **How often.** The same statement requires that no time in
  `reset_mailed` is younger than 10 minutes and fewer than 3 are younger
  than 24 hours. A code that is made appends the time and drops those older
  than 24 hours. The times are kept when the mail was not taken, and when
  the address is replaced; they end with a removed address.
- **Locks.** Every write of an account's address, and the making of a
  mailed code, first locks the account's row (`FOR UPDATE`). A mailed code
  also locks the administrators' rows as `set_admin` does, so no account
  becomes an administrator while a code is made for it.
- **Time.** Expiry is compared with `clock_timestamp()` in the statement
  that reads the row. A row past its end counts as absent everywhere.
- **Confirming** is one transaction: the waiting row is deleted, the
  address row written, a mailed reset code that went to the old address
  cleared from both rows. A wrong code's count is committed, not rolled
  back with the refusal.
- **Grants.** `SELECT, INSERT, UPDATE, DELETE` on both tables for the
  runtime role (`ops/grants.sql`). The startup check's list has 37 entries
  with them.
- **The sweep** (`--sweep-stdin`, the migrator login) deletes waiting rows
  past their end, clears a `reset_digest` whose code is no longer on the
  account's row, and drops times older than 24 hours.
- **Earlier binaries.** The previous release names neither table. Its
  reset statements run unchanged over the migrated schema (they are in the
  upgrade fixture): it makes, redeems and sweeps an administrator's code,
  and it would replace a mailed code like any other, since it does not
  know one. Its account deletion removes both rows by the cascade.
- **No way back.** The migration has no `down`, as its siblings have none.
