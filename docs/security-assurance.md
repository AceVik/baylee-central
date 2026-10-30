# Security assurance and review record

Security > QA > performance > maintainability. Owner requested multiple review
passes and a deliberately high standard on 2026-10-19.

Target: OWASP ASVS **5.0.0 Level 3** for applicable controls, with a traceable
applicability matrix before release. This is an engineering target, not a claim
of certification or that every control is implemented. Non-applicable requirements
need a written rationale. Source: https://github.com/OWASP/ASVS/tree/v5.0.0

## Three-pass method for every security milestone

1. Design: identify assets, caller trust, failure modes and invariants before code.
2. Implementation: inspect boundaries, default-deny behavior, resource limits,
   secrets/diagnostics, ownership, cancellation and dependencies.
3. Adversarial validation: attack assumptions with negative tests, real transport
   checks and failure injection; fix findings and repeat affected checks.

These are engineering passes, not independent external audits. Separate agent
reviews and their scope are recorded below. Release still needs independent review
and operational/legal gates.

## Foundation review

Design pass: no production listener, password protocol or human replay intake;
policy library has no dependencies. Dev mode shares the real policy code and
cannot bypass future authentication. Reusable libraries do not expose signing
secrets or require consumers to link server/database infrastructure.

Implementation pass: checked explicit crate-root/compiler unsafe prohibitions,
configuration bounds, no value echo in errors, account preference revision
overflow and stale writes, HTTP Host/origin restrictions, body/connection/header
limits, fixed connection lifetime, bounded shutdown and always-not-ready status.

Findings addressed: the initial configuration test needed an explicit str slice;
dependency resolution must retain approved transitive versions instead of silently
selecting a newer libc; request middleware alone does not bound pre-header sockets,
so the server explicitly bounds connections and header-read time.

Validation results and remaining gaps are recorded in status.md. Code review is
not proof that anonymization, SSO, migration or production security is complete.

## Cryptographic spike review

Design pass: fixed OPAQUE suite and KDF, authenticated deployment/account/epoch,
client export-key wrapping instead of shared-session-key wrapping, immutable
database-allocated account ID, fail-closed parsing and no public auth endpoints.

Implementation pass: reviewed upstream state zeroization and local secret ownership,
fixed wire lengths, randomness, nonce generation, purpose separation and server
feature boundary. Removed a Default constructor that could encourage accidental
deployment-key regeneration. Found Argon2's internally allocated scratch memory
was not wiped; now use its safe caller-owned-memory API with an unwind-safe wipe
guard and compare the result with the upstream KSF.

Adversarial pass: published RFC registration vector; real serialized exchanges;
wrong password/missing account/account or issuer substitution; invalid and
cross-attempt final proof; malformed sizes; mutation at all wrapper byte positions;
wrong context/key; password rewrapping; redacted Debug; work-buffer interruption.
The application still needs full fuzzing, full vectors, constrained execution,
server-secret lifecycle and independent review before any production auth.

## PostgreSQL / SeaORM foundation review

Owner selected PostgreSQL 18+, server-generated UUIDv7 and SeaORM 2 migrations;
the inventoried SQLx/Rustls/ring internals were accepted subject to checks.

Design pass: separate server-only adapter and migration registry; UUID allocation
precedes enrollment but grants no session; runtime credentials do not own schema.
Normal development connections are fixed to loopback and synthetic fixtures.

Implementation pass: bounded pool/timeouts, escaped credentials without ambient
DATABASE_URL, no SQL logging, generic errors, UUID version/variant constraints,
explicit migration transaction/advisory lock and no automatic down/reset.
A direct role-attribute check missed potential SET ROLE paths through NOINHERIT
membership; runtime roles now reject memberships as well, with an adversarial test.
Cleanup now attempts container removal even if child-process cleanup fails and
requires a successful Docker inventory response to confirm removal.

Adversarial pass: PostgreSQL 18.6 fixture covers failed/repeated/locked migrations,
unknown history, forbidden runtime DDL, migration-owner and inherited-role rejection,
invalid UUID version/variant, rollback and dropped transactions, 64 concurrent
ORM allocations with database-generated UUIDv7, expiry/default catalog checks.
Normal and deliberately failed fixtures both verify owned-process/container/port
cleanup. No unrelated client or service was stopped.

Source/advisory/license and secret scanners pass on the combined graph. Repeated
crypto generations have exact, documented duplicate exceptions; no advisory
ignores. Full native platform CI is required for this new milestone.
Production TLS identity validation, account isolation and credential-change
transactions are still unimplemented and are not claimed by these tests.

## Follow-up adversarial hardening, 2026-10-19

Design pass: reviewed the current foundation end to end, with emphasis on canonical
wire encodings, vault-generation agreement, least-privilege database credentials,
aborted operations and CI enforcement. Scope remains the reusable development
foundation; no production authentication or user-data endpoints were added.

Implementation findings and corrections:

- OPAQUE 4.0.1 parsers accept trailing bytes on several message types. The wrapper
  now checks exact suite-derived sizes for all six messages before parsing/KDF.
  Regression tests with valid messages plus suffixes failed before this fix.
- OPAQUE identifiers bound account/issuer, but the vault epoch was only bound in
  key derivation/wrapping. A mismatched epoch could still complete client login.
  Transcript v2 now includes the epoch; both mismatch directions and u64::MAX
  are tested. This is the library's standard application-context facility, not
  a replacement cryptographic primitive. Existing undeployed v1 is not retained.
  Source: https://docs.rs/opaque-ke/4.0.1/opaque_ke/struct.ClientLoginFinishParameters.html
- Runtime roles could hold extra CREATE/TEMP, mutation or delegation rights.
  The adapter now verifies the narrow INSERT/SELECT contract, including column
  grants and every writable schema. Fixtures revoke default PUBLIC TEMP.
  Source: https://www.postgresql.org/docs/18/functions-info.html#FUNCTIONS-INFO-ACCESS-TABLE
- The original dropped-transaction test could observe another pool connection.
  An actual aborted task now forces reuse of the same backend and verifies
  rolled-back rows/settings plus lock release through a distinct connection.
- Declared oversized HTTP bodies previously occupied a connection until input
  or timeout. They now receive an immediate generic 413 without body buffering.
  Chunked bodies retain independent streaming limits.
- Unreviewed local path dependencies could evade the registry inventory. They
  are now forbidden outside checked workspace members; source targets must stay
  under the workspace root. A disposable synthetic workspace tests the rejection.
- Compiler probes now include macro expansion and include!-generated code.
  All-feature docs and release builds cover the server feature too.
- Fixture admin credentials are restored immediately after docker create and
  removed from the Cargo child environment. Version-check errors explicitly
  close the new database pool.
- CI uses the verified official checkout v7.0.1 commit, keeps credentials
  unpersisted and permissions read-only. PR/main triggers avoid duplicate branch
  builds while workflow_dispatch permits an explicit feature-branch check.
  Source: https://github.com/actions/checkout/releases/tag/v7.0.1

Adversarial pass: targeted failures were reproduced before the crypto fixes.
Tests now exercise suffixes, epoch disagreement, excess SQL privileges, task
cancellation, raw oversized/chunked HTTP bodies, duplicate Host/conflicting
Content-Length, header-count/buffer limits, pipelining, overload and active-socket
cancellation. Fixed-size authenticated-context buffers remove heap allocations
without changing the wrapper bytes or weakening KDF parameters.

These passes are by the implementing agent and are not an independent audit.
No full fuzz campaign, production TLS, load-capacity certification, account
isolation, protocol state machine or recovery implementation is claimed.

Local validation for this pass: all 36 ordinary Rust tests, two compile-fail API
contracts, the real PostgreSQL integration fixture, all-feature Clippy/release,
policy/compiler probes, dev smoke and deliberate fixture-failure cleanup passed.
RustSec and cargo-deny passed on the unchanged 346-package lockfile. Gitleaks
found no leaks in the working tree or complete local Git history; CI fetches full
history for this check. Native-platform results must be verified on this commit.

Platform validation found one test portability issue: Windows Server reported
the intentionally dropped overload socket as ConnectionAborted/WSAECONNABORTED
(10053), while the test only accepted EOF/ConnectionReset. The transport fixture
now recognizes both closed-connection outcomes; read/write timeouts and unexpected
responses still fail. This changes test interpretation, not server behavior.
Reference: https://learn.microsoft.com/en-us/windows/win32/winsock/windows-sockets-error-codes-2

## Iterative review, 2026-10-19

The owner requested repeated review until a complete additional pass found no new
actionable defects. This review is limited to the implemented foundation. Planned
authentication, recovery, storage isolation and production services are not implied
to exist. Separate agents reviewed crypto, persistence and QA; the primary agent
reviewed HTTP/configuration and integration, then reviewers exchanged areas.
These are AI-assisted engineering reviews, not an independent external audit.

First iteration findings, fixed with regression coverage:

- Noncanonical reserved environment names silently selected default configuration.
  Windows environment names are case-insensitive; all platforms now detect the
  reserved namespace without regard to case and require its documented uppercase
  spelling. Unknown variants fail closed without echoing names/values. Process
  tests check configuration and argument redaction and own/reap their children.
  Source: https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_environment_variables
- Malformed chunked bodies were mislabeled as size violations. The body boundary
  now distinguishes the upstream length-limit error (413) from decode errors (400).
  Raw TCP tests reproduced the old result. A stalled-body test also verifies 408,
  security headers and capacity reuse after the five-second body deadline.
  Source: https://docs.rs/axum/0.8.9/axum/body/fn.to_bytes.html
- The public SeaORM Migrator allowed consumers to bypass the version/lock/atomic
  entry point and invoke destructive convenience operations. The registry is now
  private and a compile-fail API contract protects that boundary.
- A caller-controlled search path could select a forged migration history and
  report success without the identity schema. This failed in a real PostgreSQL
  fixture before the fix. History now uses the trusted public schema with local
  search-path settings and catalog-qualified gate functions. Tests cover alternate
  history, TEMP shadowing, setting restoration and actual migration cancellation
  with backend reuse and advisory-lock release.
- Policy scans covered crates/ while Cargo targets could live elsewhere under the
  workspace. Root native code was demonstrably accepted. The scan now covers the
  first-party tree including hidden files and additional native-source suffixes;
  targets cannot point into excluded tool/output directories or linked sources.
- PowerShell compared Cargo feature names without case sensitivity. A synthetic
  approval of ALLOC wrongly accepted alloc. Exact feature matching and a fixture
  now reject it. Indexed graph lookups reduced one local policy-check measurement
  from 3.04 to 1.37 seconds despite the wider scan; this is not a portable benchmark.
- Test helpers could accept arbitrary I/O/probe failures as proof of closure or
  skip later cleanup after an earlier cleanup error. Smoke tests accept only
  explicit socket reset/abort for header shutdown; cleanup requires refused
  connections, has bounded waits and attempts owned resources independently.
  Deliberate development-server failure cleanup now runs on every CI platform.

Cryptographic QA and memory improvements: RFC 9807 C.1.1 now checks KE2, KE3,
export and session keys on both peers, with mutations at all 320 KE2 and 64 KE3
byte positions. The test-only Identity KSF is separate from the fixed application
suite. All 584 individual wrapper bits are tested. Fixed zeroized AEAD buffers
replace temporary heap buffers and match the previous upstream allocating API's
wire bytes. No algorithm, wire version, parameter or dependency graph was changed.
Source: https://www.rfc-editor.org/rfc/rfc9807.html#appendix-C.1.1

The exchanged-review iteration found one additional migration boundary: PostgreSQL
ignores missing/inaccessible schemas in search_path. With public missing and a
forged TEMP history, the initial search-path fix could still report success without
installing the application schema. The real fixture reproduced that result before
the second fix. apply now checks pg_catalog.current_schema() against public and
fails closed otherwise. Regression coverage includes TEMP history before first
installation with public present, and rejection with public absent or without USAGE.
Source: https://www.postgresql.org/docs/18/runtime-config-client.html#GUC-SEARCH-PATH

A later policy consistency pass also made local Cargo package-ID membership
case-sensitive. A Linux-specific disposable Cargo graph checks distinct paths and
package names differing only in case before testing rejection of the external
package. Windows cannot exercise that filesystem case; its execution is checked
in Linux CI rather than reported as a local reproduction.

The closing scan review reproduced another QA gap in an isolated Git repository:
working-tree exclusions for local settings/build outputs also suppressed secrets in
committed history. Ordinary source was detected while synthetic config.local.json
and target/config.json credentials were not. History scanning now uses a separate
configuration without path exceptions. An obligatory scanner regression uses fresh
synthetic values and includes both committed and subsequently deleted secrets.
No real credential or unredacted finding is used in that fixture.
Fixture environment restoration distinguishes a missing variable from an empty
one: PowerShell's plain null argument did not delete POSTGRES_PASSWORD under the
current runtime. Both restoration sites now use NullString for absence and verify
the exact prior value. Scanner snapshots use ordinal name matching for Linux's
case-sensitive environment; repository cleanup remains independent of restoration.

The final scanner cross-review reproduced a false green result when GIT_DIR routed
Git to a different synthetic repository. The scanner now rejects repository-routing
environment overrides before any Git/tool/fixture operation, verifies the checkout
root and requires full history. Regression tests mock Git and require all ten
scope overrides to be rejected before the mock is reached; they do not redirect a
real scan. Source: https://git-scm.com/docs/git#_environment_variables

Closing read-only passes over the corrected crypto, HTTP/configuration, database
and QA boundaries found no further actionable defects in this implemented scope.
Local validation passed: 44 ordinary Rust tests, three compile-fail API contracts,
real PostgreSQL 18.6 integration, all-feature Clippy/release, policy/compiler probes,
normal/injected-failure development smoke, RustSec and cargo-deny on the unchanged
346-package lockfile, and working-tree/full-history secret scans. Database fixture
restoration also passed all four absent/present environment × success/failure cases.
An additional 40 Windows overload/cancellation repetitions and a real IPv6 listener
check passed. Native CI must still pass on the final revision; its evidence is
linked in the PR rather than borrowed from an earlier commit.

Cleanup limitation: automatic tool approval rejected deletion of one extra
synthetic two-repository probe directory without a specific reason. No processes,
listeners or containers belong to that retained directory. Its exact local path
is recorded in target/review-cleanup.txt and reported to the owner. The normal
owned server/database/scanner fixtures were cleaned and verified; this extra
directory is not claimed removed.

Remaining bounds: the upstream OPAQUE OsRng interface can panic on entropy failure;
no fallback randomness was added. Fuzz campaigns, all RFC vectors, constant-time
analysis, production TLS/session integration and external review remain release
gates. A clean finite review pass cannot establish the absence of every bug.

## Native integration review, 2026-10-19

Design pass: one active session per account, including on the same device; no
implicit gateway-admin permission from registry ownership; separate heartbeat,
redemption and migration authorities. Every gateway action requires a live,
purpose-bound lease. Private-history chunks remain account-encrypted and old
devices' secrets are retained for restoration. No operator receives vault keys.

Implementation pass: account locking followed by a fresh session query prevents
an UPDATE waiting behind a login from using a revoked snapshot. A unique session
index backs the application rule. UUID constraints use IS NOT DISTINCT FROM 7
to reject PostgreSQL's NULL version result for invalid UUID variants. Runtime
permissions reject grant options and column-level approval/archive writes.
Migration invalidation preserves old ciphertext, advances epochs and revokes
sessions; permanent close blocks subsequent legacy resets. Session status lets
fully migrated clients avoid depending on the old gateway during later logins.

Adversarial pass: real PostgreSQL tests force the row-lock race before revocation,
compete concurrent logins/writes, remove admins with live/pending capabilities,
try foreign accounts/gateways/purposes and replay, and exercise lost replies,
tampered content, bounded quota reclamation and dangerous role grants. The native
SDK restores a private chunk after a new-device login has revoked its source
session. Separate login/data/control budgets prevent bulk transfers from
consuming the session-check budget; adding a fake bearer cannot bypass the login
budget. Wire tests include worst-case JSON expansion, not only random ciphertext.

This review does not replace independent cryptographic review, production
capacity tests or an external audit. Content confidentiality does not imply
hidden metadata, guaranteed availability or detection of a complete old snapshot
on a device without a trusted revision checkpoint.

## Deployed launcher and database connector review, 2026-10-20

Design pass: assets are the OPAQUE server setup, the bridge credential, database
logins and the account tables. The deployed mode must not be steerable to another
endpoint, gateway or plaintext database. So the endpoint and legacy gateway are
compiled (`Endpoint::live`, `LEGACY_ORIGIN`), the input schema has no field that
could name them, secrets come only from standard input, and the database
transport is verify-full TLS to a loopback or private IP literal. Migration is a
separate mode and login; the service still never migrates. Secret generation
refuses a terminal so secrets are not left in scrollback.

Implementation pass: `ServerConfig` keeps the password and authority out of the
URL (typed sqlx options through SeaORM's options hook), validates every field
before use, rejects a pasted private key, and has no Debug or Clone. Both stores
share one checked constructor, so `pending-role.sql` and the grant contract run
on the deployed path exactly as on the development one. The pool additionally
checks `pg_stat_ssl` for its own session. Configuration types derive Zeroize; the
password and authority are moved, not copied, into the connector. The service
binds before connecting, handles SIGTERM for supervised stops and closes the pool
on every exit path. Errors stay generic; no value is echoed.

Finding recorded rather than fixed: the driver's rustls trust store is the public
web roots plus the configured authority. Restricting hosts to loopback/private IP
literals makes the configured authority the only one that can certify the server;
DNS names are refused until a CA-only verifier exists. A certificate naming a
different address is refused by verify-full but is not separately exercised.

Adversarial pass: a listener that declines TLS receives no plaintext startup
(injecting `sslmode=prefer` fails that test); the real fixture now serves TLS and
refuses an unrelated authority and a wrong password while the runtime, inherited
and migrator roles behave as before over TLS (injecting a success expectation for
the unrelated authority fails the suite). Process tests reject extra or unknown
arguments, empty and malformed input without echoing canaries, and confirm that
generated secrets restore and differ. Unit tests reject development-shaped,
extended and plaintext documents, public addresses, DNS names and URL injection.
This is engineering review, not independent audit or production approval.

## Development task runner (`cargo xtask`), 2026-10-20

Owner request: the PowerShell checks in Rust, one implementation for every
platform, the scripts kept for Windows development ([xtask](xtask.md)).

Design pass: the asset is the gate itself; a weaker port would pass forbidden
code silently, so every rule and message is the PowerShell one and each has a
regression that breaks exactly that rule. Inputs are untrusted (a pull request's
tree, `cargo metadata`, Git variables): unreadable or unexpected input fails
closed with a fixed sentence. Fixture secrets stay out of arguments and output.
Cleanup must be proved, not assumed: owned names, labels, process groups,
refused ports, temporary paths checked before removal. Dependencies are only
already-approved crates with approved features; no HTTP client or shell.

Implementation pass: programs run from argument lists; passwords reach Docker
and the tests through child environments and `psql` through standard input;
the task never alters its own environment. Randomness is `getrandom`. A test
process runs in its own process group and is stopped with its tree on timeout.
Container removal requires the run's random name, the ownership label and the
created id, and a second inventory. Temporary directories are removed only
under the temporary directory, never through a link. The gitleaks archive is
checked against a pinned SHA-256 on every run; the added macOS and Linux ARM64
pins come from the release's checksum file, whose Linux and Windows x64 entries
equal the already pinned values. The compiler probes are text files, since the
tree scan reads every `.rs` file for exactly that code.

Findings addressed: the PowerShell check accepted a member with no `publish`
setting (cargo reports null, whose count is zero), which allows crates.io;
both checks now refuse it, with a regression each. The tree scan flagged the
port's own probe sources and documentation, which led to keeping marker-bearing
probes and test cases in data files rather than weakening the scan.

Adversarial pass: on macOS the policy (with compiler probe), the secret scan
with its positive controls, the development smoke and the `PostgreSQL` fixture
pass, and the smoke and fixture failure paths report only their synthetic
failure after proved cleanup (no container, listener or temporary directory
left). Injected weakenings each fail a test: dropping the `web/node_modules`
skip, matching it in any case, dropping it from the excluded roots, accepting
a null `publish`, removing the link check, dropping the `asm!` marker, and a
probe that compiles. Windows and Linux runs are CI's (the verify, postgres and
secrets jobs run both implementations). Engineering review, not an audit.

## Terms of use and their acceptance, 2026-10-20

Design pass ([terms](terms.md)): assets are acceptance records (who agreed to
which text, when), gateways' published texts and the refusal of sessions that
have not accepted. Callers are a central session, a gateway credential or the
bridge, and nobody for the public reads. Invariants: a gateway cannot accept
for a player (acceptance needs the player's central session); a published
version never changes and acceptances name its digest; currentness is version
and digest; enforcement lives in Central and the gateway, never in the client;
drafts (`<!-- version: draft -->`) are never in force; responses refuse
unknown fields, so new operations are new variants and the old lease check
stays for gateways not yet updated. A reviewer found before code that the
legal drafts would have been in force once compiled in, that a terms verdict
on every socket recheck would end running games, that migration needed
`ConfirmRestore` while unaccepted, and that currentness must include the
digest; each is in the design.

Implementation pass: three tables with `SELECT, INSERT` only for the runtime
role, checked at startup with the existing grant contract; every check and
write is one statement with bound values; a dead session is `Rejected`
before any terms answer; the gateway's verdict is computed in the lease
statement itself; served texts are re-validated and re-hashed against their
stored digest; bounds keep the largest document within the wire limit with
worst-case escaping; public reads fall under the global limit, lease checks
under the control limit. Central's own texts are compiled in, both languages
must name one version, and a parse failure stops the service from starting.

Adversarial pass: over real HTTP and `PostgreSQL` the e2e run keeps a fresh
account from tickets and object writes (403) until it accepts exactly the
current version and digest (a flipped digest bit and a renamed version are
`Conflict`, accepting again is `Done`), while reading its own data stays
possible and a random bearer is `Rejected`; a gateway's publish is refused
without its credential, for `draft`, for an invalid version and for unordered
texts, idempotent when repeated, `Conflict` when changed; acceptance of a
forged digest, of an origin that published nothing and of a superseded version
is `Conflict`; `GatewayAuthorize` turns true only for the exact accepted
version and digest. At the store the runtime role cannot update or delete any
terms record, tampered texts are not served, the 257th version is refused and
acceptances go with their account. Injected faults each fail a test: lifting
the ticket gate, and skipping the digest check when serving. Engineering
review, not an audit.

## Recovery code, 2026-10-21

Owner decision: registration on the website hands out a recovery code, shown
once ([web accounts](web-accounts.md)). This pass covers the code and its
wrapper in `central-crypto` ([crypto protocol](crypto-protocol.md)
§"Recovery code"); storing it is the server's work, reset by it is M1's.

Design pass: the asset is the master key, which the code opens without the
password, so the code must carry full strength on its own (160 random bits,
no stretching needed or added) and its wrapper must never be mistaken for
the password one. The key derivation and the wrapper each take their own
domain label (salt `baylee-central/recovery/v1`, associated data
`baylee-central/recovery-wrap/v1\0`) and bind the issuer, account and epoch
as the password wrapper does. The design's "the context's AAD as info" could
be read two ways; the exact bytes (the password wrapper's 86) are now spelled
out in the protocol document and pinned by a known-answer test, since M1's
reset has to reproduce them. `WrappingKey` keeps having no raw-key
constructor: the code derives its own key privately and shares only the
sealing routine.

Implementation pass: the code, its symbols, the 40-bit work values and the
written text live in zeroizing buffers; the text's buffer is allocated at its
final length so it never moves and leaves a copy. Input is bounded (128 bytes)
before it is read; only 32 symbols of the alphabet are accepted, and the
confusable letters are mapped as Crockford's base32 says. No Clone,
Serialize or Display; Debug is redacted. The refactor of the password wrapper
into a shared seal and open routine keeps its bytes (the existing
compatibility test with the upstream allocating API still passes). Not
addressed, and recorded: parsing and writing look symbols up by their value
(a 32-byte table), which is not constant-time; the code is typed and shown in
the owner's own browser, where an observer of that timing could read the
page anyway.

Adversarial pass: two vectors computed apart from the crate (Python's base32
in Crockford's alphabet, pyca/cryptography's HKDF, libsodium's
XChaCha20-Poly1305; `crates/central-crypto/tests/recovery-vectors.py`) pin the
text, key and wrapper; all 584 wrapper bits, a changed issuer, account, epoch
or code bit, the same key under the other wrapper's domain, and a password
wrapping key are refused; 32 symbols in lower case, with `O`, `I`, `L`,
spaces, line ends and doubled hyphens read back, while `U`, `_`, `.`, a NUL, a
Kelvin sign, a full-width letter, 31 or 33 symbols and 129 bytes are refused.
Injected faults each failed a test: deriving with the recovery domain as info
(the known answer), and dropping the `L` mapping (the read-back test).
Engineering review, not an audit.
## Registration and browser sign-in, 2026-10-21

Owner decisions and the contract: [web accounts](web-accounts.md). Built in
three PRs (migration and store, wire and identity, browser listener); each
records its own implementation and adversarial findings below.

Design pass. Assets: the beta keys (who may register), the username namespace,
OPAQUE records, the wrapped master key and its recovery wrapper, the browser
session cookie and the per-address counters (memory only). Callers: anyone on
the Internet through `baylee.app`'s Caddy for the browser listener, native
clients and the bridge on `/v1/native`, and the operator (migrator login) for
issuing keys. Invariants and the choices made before code:

- A bad, used-up, expired or revoked key is `Rejected` before the name is
  looked at; only a valid key learns `Conflict` for a taken name. Keys are 80
  random bits, stored as SHA-256 digests only, so neither guessing nor a copy
  of the table yields one.
- A reservation counts against its key only while its enrollment ticket lives:
  a key admits a registration while `uses_left` exceeds its live reservations,
  and a use is spent in the enrollment commit. An expired reservation so gives
  its use back without any clean-up job, and two registrations racing for a key's
  last use are serialized by the key's row lock. The link between key and
  account is cleared at enrollment: the privacy notice lists no such record.
- Legacy rules (`legacy_closed`, the bridge's operations) apply to legacy
  accounts only, in the schema too: `source` pairs with `legacy_id` by CHECK,
  `legacy_closed` is legacy-only, an enrolled registered account must have a
  recovery wrapper and a legacy one must not. Every `legacy_closed` predicate
  in the store gains its registered counterpart, otherwise a registered account
  could never use private objects; the bridge's redemption answers
  `legacy_id: None` for it instead of failing.
- One session per account and kind. A browser session is looked up by a digest
  in its own domain (SHA-256 over a fixed label and the token), not by the
  native bearer digest, so a cookie value presented as a native bearer finds no
  session and a native bearer is no browser session. Binding the kind in every
  session query would have touched fifteen statements, any one of which could
  have been missed open; separate digest domains fail closed by construction and
  cost no query.
- `Register` is served on the browser listener only this round: its
  per-address budget is the design's control against key guessing and name
  probing, and the native surface has none (native registration is M6).
- The browser listener is default-deny: one route, the listed operations, exact
  `Host`, `Origin`, content type and `X-Baylee-Central`, no `Authorization`, no
  CORS answer. Every refusal happens before the identity service is called and
  is tested without a database. The token travels only in the `__Host-` cookie
  and never in a body. `X-Forwarded-For` counts only from a loopback peer.
- Migration 000007 keeps defaults for `source` (`legacy`) and `kind`
  (`native`), so the previous binary, which the deploy hook restarts when a new
  one fails to start, still serves legacy migration; it fails closed on every
  registered account (CHECKs and NOT NULL reads), which the new code names
  explicitly in every write.

**Migration 000007 and the store.** Implementation pass: every `legacy_closed`
predicate (object writes and reads, `SessionStatus`) and the bridge's
redemption read of `legacy_id` were found holding registered accounts back;
the redemption read a NULL into a string and would have answered 503 at the
official gateway. Each now names the registered case. The fixtures granted
hand-copied lists; they now run `ops/grants.sql` itself, so the startup check
is tested against what the deploy hook grants. `beta_keys` grants a column,
which the generic check did not model: it now requires `UPDATE` on exactly
`uses_left` and refuses it on any other column or the table. The fixture found
that issuing read the clock twice (`created_at` after `expires_at`), so a
30-day key lasted microseconds less than 30 days; one timestamp now serves
both.

Adversarial pass: a prior-version upgrade runs the exact SQL of 000001 to
000006 in a scratch schema, writes legacy accounts, a session and a ticket as
the previous binary did, applies 000007, and checks the data, the defaults and
the previous binary's own statements, all in one rolled-back transaction. The
schema refuses each broken pairing of source, legacy ID, key link, ticket and
recovery wrapper. Seven injected faults each failed the PostgreSQL fixture at
the assertion written for it, after a clean baseline run on the same commit: a
browser digest equal to the native one (a cookie accepted as a bearer), a
sign-in replacing sessions of any kind (a website sign-in ending the client's
session), the legacy-only object and `migration_closed` predicates, a grant
check accepting `UPDATE (note)`, a migration without the recovery rule, and
key issuing outside one transaction (a run repeating a digest left its other
key behind). Engineering review, not an audit.

**`Register`, the recovery wrapper and the identity service.** Implementation
pass: registration and enrollment both lock the key's row before an account's,
so the two cannot deadlock; the sweep of abandoned registrations runs as a
statement of its own with `SKIP LOCKED`, holding no account row while waiting
for a key. A malformed name is refused before the database is reached, then the
key, then the name: a bad key and a malformed name both answer `Rejected`, so
neither tells anything about the name. A legacy migration takes a name back
from an abandoned registration and never the reverse. `recovery` is skipped on
the wire when absent, so a legacy enrollment from a new client still reaches a
server built before it. The browser operations are an exhaustive match rather
than a list (a positive list says nothing about an operation added later); a
gateway's terms cannot be accepted from the browser surface; a sign-in attempt
is bound to its surface. A legacy enrollment carrying a recovery wrapper was
refused only by the schema (as `Unavailable`); the test now pins the store's own
`Rejected`, so the schema stays a second line.

Adversarial pass: nine injected faults each failed the PostgreSQL fixture at
the assertion written for it, after a clean gate and baseline on the same
commit: `Register` accepted on `/v1/native`, the name looked at before the key,
reservations not counted, enrollment not spending the use, revocation ignored at
enrollment, a legacy enrollment taking a recovery wrapper, a sign-in attempt
finishing on the other surface, no sweep, and the name reclaim taking a legacy
name. One run (revocation) first hit the fixture's 300-second limit under a
concurrent workspace build; alone it failed at its assertion in 96 seconds.
Engineering review, not an audit.

**The browser listener.** Implementation pass: the listener's own refusals
come before decoding, the allowlist before the budgets, the budgets before the
identity service; the service never sees a refused request. The budget lock's
poison error held its guard across the service call, which made the handler's
future not `Send`; the compiler refused it, and the guard now ends before
anything is awaited. The cookie value is written and read through `Token::hex`
and `Token::from_hex` (the bearer's parser, zeroizing buffers), never through
`format!`; the header copy itself is marked sensitive. Caddy's documentation was
checked for `X-Forwarded-For`: without `trusted_proxies` it ignores a client's
own value and sets the peer it saw, so the last hop is the client; the runbook
says not to set `trusted_proxies` for the site. The first budget test built a
fresh listener, with fresh budgets, per request and so could never fail; it
now shares one listener, and its assertions failed before that change.

Adversarial pass: every header condition, a non-loopback or missing peer, an
absolute URI, other methods and paths, oversized and malformed bodies, unknown
and non-browser operations and malformed forwarded addresses are each tested,
without a database, to be refused before the service is called. Fourteen
injected faults each failed those tests: an `Origin` compared as a prefix,
fetch metadata ignored, the custom header or the `Authorization` refusal
dropped, any peer accepted, the first hop or the first `X-Forwarded-For` line
counted, the allowlist skipped, `Logout` not clearing the cookie, the cookie
without `HttpOnly`, IPv6 counted by its host half, discovery and sign-in sharing
one budget, a duplicate cookie accepted, and a full budget table refusing new
addresses. Over real HTTP and PostgreSQL the website's whole flow passes:
registration with a beta key, enrollment with a real recovery wrapper that the
code opens to the stored document's key, sign-in into the cookie only (exact
attributes, no token in the body), the cookie refused as a native bearer, terms,
sign-out clearing the cookie, the fifth registration of an address refused
while a neighbour's is not, and the same account admitted at the official
gateway without a legacy ID. Engineering review, not an audit.

**The username in the browser answers, and a development origin.** Design
pass: the page asked for the account's name so it need not keep it in
`localStorage`; the session's own account name is no new disclosure to the
holder of its cookie. `Response` refuses unknown fields, so the native
`SessionStatus` could not gain one without breaking clients pinned before:
the browser surface answers a variant of its own, `BrowserStatus`. A local run
of the pages needs the listener on an origin other than `baylee.app`; the
site became a value (`BrowserOrigin`), and only the development launcher can
name another one, a numeric loopback `http` origin validated as
`Endpoint::development` validates the development endpoint. No check is
skipped for it, and the deployed launcher's input has no field for it.
Implementation pass: the two-listener runner moved to a module both launchers
share. Adversarial pass: four injected faults each failed their test (the
fixture ran clean first): the native and browser status answers swapped (a
native client could no longer read its status), the sign-in or the status
answering another column than the canonical name, and a development origin
taken without validation. In this round an injection dry run restored files
with `git checkout -- crates` over uncommitted work and so undid it; the work
was re-applied from its scripts and committed before any further injection.

## Browser crypto (`central-web-crypto`), 2026-10-21

Owner decisions: browser crypto is a WebAssembly crate built from
`central-crypto`; `wasm-bindgen` 0.2.129 as a direct dependency, getrandom
0.4.3 `wasm_js` and 0.2.17 `js` ([web accounts](web-accounts.md)).

Design pass: the assets are the password, the OPAQUE export key, the master
key, the recovery code and the states between two messages. The page is
trusted code from this origin under the site's CSP; Central's answers are
not, so the module checks every server message (exact OPAQUE sizes, the
KE2 MAC) and the returned wrapper. The module makes no network call, binds
the live issuer at compile time (no parameter can name another Central) and
keeps no key: the master key exists only inside a step and is wiped when it
ends. A state is an opaque exported object, consumed by the step that takes
it (wasm-bindgen empties the JavaScript object) and wiped on drop, also when
the page frees it unused. Findings before code: the design's
`login_finish(state, response, wrapped_master)` cannot exist, since
`wrapped_master` arrives in the answer to the KE3 that `login_finish` makes;
the step is split into `login_finish` and `login_confirm` and the design
says so. A `development` feature for a local Central's issuer would have
broken every flow test under the gate's `--all-features`; it is left out
until the run against a real local Central, and the committed module is
bound to the live Central by construction (`central_origin()` is tested).
Exported classes instead of plain objects keep `js-sys` out of the direct
dependencies, which the owner's approval did not name.

Implementation pass: `#![forbid(unsafe_code)]` holds on the crate with the
`#[wasm_bindgen]` expansions; the policy, its compiler probe and the textual
scan pass. The graph gains no package: wasm-bindgen 0.2.129 and js-sys
0.3.106 were already in the lockfile; the inventory gains `getrandom@0.4.3`
`wasm_js` and `getrandom@0.2.17` `js`, plus `js-sys` and `wasm-bindgen`, the
names cargo gives the two optional dependencies 0.2's `js` turns on (recorded
under `approvals` in `approved-packages.json`). Both backends call
`crypto.getRandomValues`; the glue holds no `eval` or `Function`
constructor, and the `module.require` and `process` it references belong to
getrandom 0.2's Node path, which a browser with Web Crypto never takes. The
password crosses into the module as an owned string and goes straight into
`SecretString`; the recovery code leaves it read from its zeroizing buffer
(`JsValue::from_str`), not as a buffer handed to the glue and freed unwiped.
Not wipeable, recorded: the page's JavaScript strings (password, code), and
the bytes wasm-bindgen's string passing may leave in freed module memory
when it grows a buffer for non-ASCII input; the pages therefore run one
module instance per flow in a Worker and terminate it, which discards its
whole memory. Panics abort (profile `wasm`); a trapped instance is never
reused. The build is reproducible on x86-64 Linux, CI's host: repository and
Cargo home are remapped out of the module (a module still naming either
fails the task), wasm-bindgen strips the name and producers sections, and
`cargo xtask wasm --check` compares a fresh build byte for byte. Finding,
from CI: a module built on macOS differed from CI's although no path or flag
did; cargo folds the build host into every crate's `-C metadata`, which
renames the symbols and changes the layout under link-time optimisation.
The committed module is therefore the x86-64 Linux build, the task refuses
other hosts, and other machines use the pinned container (`docs/xtask.md`);
a rustc-argument rewriter that would have forced the metadata was rejected
as first-party code editing compiler arguments. CI's native Ubuntu build and
the documented container run on macOS both equal the committed module. That
container run also found the local-path check matching `/central` inside
`https://central.baylee.app` when the repository sits at `/central`; the
check now looks for a directory followed by its separator, with a test.
Git never converts the committed files' line ends (`.gitattributes`), so a
Windows checkout keeps the bytes the sums name.

Adversarial pass: on the host, against `central-crypto`'s server side with
real messages and the fixed Argon2 profile, a registration then a sign-in
then `login_confirm` succeed, and the sealed document opens to exactly the
design's bytes under the password's master key and under the recovery
code's, typed back in lower case; two registrations differ in code, wrappers
and document, and one's code does not open the other's wrapper; a wrong
password fails at `login_finish`, before KE3 exists; another epoch and an
unknown account fail; `login_confirm` refuses the recovery wrapper, another
registration's wrapper, flipped bytes across the wrapper and lengths 0, 72
and 74; short, long and non-v7 accounts, epoch zero, an empty or 1025-byte
password and a response with a suffix are refused. In JavaScript the real
module (vitest) is bound to the live Central, makes 32-byte registration
requests and 96-byte KE1s, different each time, refuses the same malformed
input with `rejected`, and a consumed state cannot be used again. Injected
faults each failed a test: `login_confirm` ignoring the unwrap result, and
in `--check` one changed byte, a wrong sum and an extra file, and the macOS
build itself, which CI's Linux check refused. Engineering
review, not an audit; the full round trip against Central's browser surface
is the account pages' run against a real local Central.

## Account pages on baylee.app, 2026-10-21

Owner decisions: registration and sign-in on the website, a recovery code
shown once, a strictly necessary `__Host-` session cookie, and
`'wasm-unsafe-eval'` in the site's CSP ([web accounts](web-accounts.md),
`web/README.md` §"Accounts").

Design pass: the assets are the password and the recovery code while the
page holds them, the session (Central's HttpOnly cookie, never seen by the
page) and the person's consent to the terms. Callers: the page is trusted
code from this origin; Central's answers are checked, not trusted. Every
request carries `X-Baylee-Central: 1` and JSON, goes `credentials:
same-origin` with `redirect: "error"`, and never an Authorization header;
an answer must be exactly its shape. The crypto runs in a Worker per flow
that is terminated at the flow's end. An unknown name and a wrong password
read the same; only a valid key learns that a name is taken (the server's
rule). A terms version is accepted only when the served texts hash to the
digest in force, so a forged or stale text can never be accepted in the
person's name. The page remembers only the name and the session's end, to
show them; Central decides whether the session lives. Not addressed,
recorded: ASVS 5.0 V6.2.4 (checking a new password against the 3000 most
common) needs a list in the page, whose source and licence are the owner's
to choose; the strength hint lowers a few common words and the name only.

Implementation pass: no HTML is ever interpreted (React text, Fluent
messages through `rich`, terms through `LegalText`, names in `<bdi>`);
the CSP still forbids inline script and `eval` of JavaScript, and frames.
The password lives in the form's state and one closure until the sign-in
after registration has handed it to its Worker; the recovery code lives in
the page's state until the person confirms it was kept, its file through a
Blob URL revoked at once; leaving earlier asks first. Findings fixed: the
provider's default client was a new object on every render, which would
have asked Central for the session in a loop; the terms step restarted on
every parent render; a focus effect keyed on a fresh object moved the focus
from "Password again" back to "Password" on every keystroke (a component
test caught it); a scrolling terms box needed a focus stop the lint rules
forbid, so the terms now flow in the page. Signing out while Central cannot
be reached forgets the session here, but the HttpOnly cookie lives until its
end: the account page now says so, with the time, instead of claiming the
session ended.

Adversarial pass: unit tests hold every answer to its shape (an extra
field, a 63-byte or 256-valued byte array, a non-v7 account, epoch 0 or
2^53, a terms version `draft`, unordered or unknown languages) and every
status to its meaning; the digest equals a value computed with Python's
hashlib; a foreign identity or a wrapper that does not open ends the new
session with `Logout`; every flow's Worker is closed on every path. Against
the build under its CSP, at a phone's and a desktop's width, a strict mock
of the browser surface refuses any request with wrong headers, origin,
fields or byte lengths and none occurs; the real WebAssembly starts both
exchanges in its Worker with 32- and 96-byte messages and refuses a forged
registration response and KE2; with `'wasm-unsafe-eval'` removed from the
Worker's CSP the module cannot start and the page says so. Injected faults
each failed a test: no `X-Baylee-Central` header, `credentials: "include"`,
no digest check, no identity check, extra fields accepted. Engineering
review, not an audit.

Adversarial pass against a real Central, 21.10.2026 (`web/e2e-real/`): main
with the browser listener of PR #25 at its head `7bc7df7` (merged unchanged
as `7bada2c`), in the deployed
mode (`--accounts-stdin` with `browser.listen`) over verify-full TLS to
PostgreSQL 18, provisioned, migrated, granted and keyed as
`docs/operations.md` says, behind a stand-in for the site's Caddy; Chromium
at `https://baylee.app` (mapped to the stand-in), at a phone's and a
desktop's width. The deployed mode compiles in the live issuer, so the
committed module ran unchanged; the planned development build was not
needed. Results:

- Registration ran real OPAQUE and Argon2 in the Worker (0.2 to 0.4 s from
  the click to the code) and showed a well-formed code; the account then
  signed in at once. The cookie was `__Host-central`, 64 hex digits,
  HttpOnly, Secure, SameSite=Strict, Path=/, and absent from
  `document.cookie`; a reload was confirmed by `SessionStatus`; signing out
  removed it; a wrong password sent nothing past `LoginStart`; the name in
  capitals signed in; a taken name with a valid key was 409 and a bad key
  401, and neither spent a use.
- The rows: `source = registered`, no legacy id, a UUIDv7, epoch 1, a
  192-byte credential record, 73-byte password and recovery wrappers, the
  166-byte document (125 + 41) at revision 1, the reservation and enrollment
  digest cleared, the key's uses down by exactly the two registrations, and
  only browser sessions.
- The native client's SDK, through the native listener, signed in to both
  accounts (a wrong password refused), decrypted the document the browser
  had sealed to exactly the design's bytes and confirmed the restore; the
  account's browser session survived it. The recovery code the page showed
  opened the stored recovery wrapper and the same document, and did not
  under another account.
- With Central's terms in force (a test build naming a version, never
  committed): the page's digest of the served texts equalled Central's, so
  `AcceptTerms` was recorded (one row, that version, a 32-byte digest) and
  not asked again; the German text was shown in German; declining signed
  out and recorded nothing.
- Per-address budget: five registrations from one address, then 429, which
  the page names as too many attempts.

Finding fixed: after "Not now (sign out)" on `/signin` or `/register`, the
terms step stayed with its accept button, which would then fail as if
Central had not answered. The step now says the person is signed out, with a
link to sign in; a session that ended before accepting says the same. Unit
tests for both, and the German decline re-run against the real Central.
Everything the run started was stopped and checked: the stand-in and Central
by SIGTERM with their ports refusing, the container by its label, and the
passwords, keys and configuration documents deleted.

The account's name, since the browser surface sends it (server PR #27): the
page takes only the named answers and shows Central's canonical name, held
in memory; the browser keeps nothing but the session's end, and the key
that also held the name is deleted on sight. Tests refuse the unnamed
shapes, keep the name out of storage, and show "Your account" while a
remembered session is unconfirmed. Run again with `web/e2e-real/` against a
real Central at main `8af36f4` (with #27) in the deployed mode, at both
widths: the account and the header named the account as Central does, after
the sign-in and after a reload, the browser kept only
`baylee-central-session` with `expiresAt`, and a planted
`baylee-central-account` was deleted on the next page; everything started
was stopped and checked, and the synthetic secrets deleted.

## The gateway directory on the website, 2026-10-22

Owner decision of 22.10.2026: the website's gateway list works and lists the
official gateway (`docs/accounts-admin-proposal.md` on its design branch, step 1).

Design pass: the assets are the bridge's authority over the official origin
(its tickets must stay redeemable once, by the bridge) and the directory's
truth (what is listed and whether it is online). Callers: anyone on the
internet for the directory, which is public and holds only approved metadata;
the bridge credential for the official heartbeat. Invariants: the official row
holds no owner and no credential (a CHECK) and is unique (a partial index), so
no registered credential authenticates it and no registered ticket names its
audience; registration of the bridge's origin is a `Conflict`; the bridge's
heartbeat touches that row alone, under the five-second limit, with metadata
on that origin; approval stays the operator's, the runtime gains no grant. The
directory on the browser listener uses no session and keeps every check of
that listener; its per-address budget is 60 a minute.

Implementation pass: the official row is chosen by `official` and the bridge's
origin together, which a live store holds to `LEGACY_ORIGIN`; the registered
heartbeat, gateway authentication and registered tickets add `NOT official`
beside the NULL digests that already exclude it. The bridge is checked in
constant time before the heartbeat token is looked up; a wrong token is
`Rejected` as before. The page posts with `credentials: "omit"`, `redirect:
"error"` and `X-Baylee-Central: 1`, keeps every check of the answer, and marks
the official gateway by the origin it knows, never by the answer. Not
addressed, recorded: the directory shares the unauthenticated global budget
with sign-in on both listeners, as on the native route before.

Adversarial pass: the upgrade over 000008's registered gateways (they keep
owner and credentials; the prior binary's registration still works; a second
official row, an owner or a credential on it, a registered row without them
are each refused); the store (registration of the bridge's origin at a full
quota is `Conflict`; a bridge heartbeat before listing, before approval, after
disabling, for another origin or with metadata elsewhere is refused; it moves
the official row and leaves the registered one unchanged; a registered
token and registered tickets do not reach it; moved to another origin it is
no longer official in a live store); the SDK over HTTP (the bridge's heartbeat,
`Conflict` on registration, the directory equal on both listeners, the
heartbeat refused on the browser listener); the listener's budget per address
apart from the others; the page's request, the mark and paging after
Central's last id. Engineering review, not an audit.

## The account page and recovery codes for moved accounts, 2026-10-22

Owner decision (22.10.2026): moved accounts have no recovery code, and the
website must stop telling them to keep one safe; the accounts and
administration design's step 2 ([web accounts](web-accounts.md) §"A
recovery code after a sign-in").

Design pass: the assets are the master key, which exists only inside one
sign-in's Worker and is why the code is made right after a sign-in and never
later; the new code, held by the page until the person decides and never
stored by it; the 73-byte wrapper, which opens the master key with that code;
and the browser session. The page is trusted code from this origin, Central's
answers are checked, the cookie may be stolen. A stolen cookie cannot make a
wrapper of the real master key (it opens nothing without the password), so
what it could do is put bytes of its choosing on an account and block the
owner's code; `SetRecovery` therefore takes a wrapper only within 10 minutes
of the session's password sign-in (`created_at`, which renewal never moves),
only while the account has none, only with Central's terms accepted, and
only on the browser listener; other bytes later are `Conflict`, and
replacing a code waits for password reset (M1). Not addressed, recorded:
within those 10 minutes a stolen cookie can still store a wrapper that opens
nothing, and the owner learns of it from the account page and the
`Conflict`; Central cannot tell a good wrapper from a bad one without the
key, and must not have it. Findings before code: `require_terms` looked the
token up as a native session, harmless only because no browser operation
needed accepted terms before this one, and now takes the listener's kind; the
design's German wording says "Konto" where the site's glossary
(`web/README.md` §"The words") says "Account", so the page says "Account";
the design left the codes for a stale sign-in and an unfinished move open,
both are `Rejected`; a registered account's own wrapper sent again is
`Done`, any other `Conflict`.

Implementation pass: `set_recovery` checks the wrapper's format before the
database, then reads the browser session's account `FOR UPDATE` in one
transaction: two codes sent at once store one and refuse the other. The
same bytes are compared after the lock, so a retry after a lost answer is
`Done` even past the 10 minutes. The store's `account_summary` and
`set_recovery` take a browser session only, matched by its digest domain and
its kind, so no caller can hand them a native bearer; the identity service
refuses both on the native listener before it looks at the terms. `invalidate_legacy` clears the
wrapper in the statement that raises the epoch; the schema would refuse the
credential's removal otherwise. Migration 000010 only widens the check, so
the previous binary, which the deploy hook restarts when a new one fails,
still runs. `recovery_create` opens the master key before it asks for
randomness, so a wrong key fails as `login_confirm` does; the key is wiped
when the step ends, and the code leaves the module read from its zeroizing
buffer, as at registration; the Worker frees the result once read. The page
keeps the code and the wrapper in its state only, sends the wrapper only
after "I kept it", sends the same bytes on a retry, withdraws "Not now" once
a request may have stored the code, and says so honestly when a refusal
follows a lost answer. A summary naming another account than the sign-in
ends the new session. No new dependency and no grant: the runtime already
updates `accounts`.

Adversarial pass: over PostgreSQL 18.6 (`cargo xtask postgres`), the
upgrade applies 000010 over legacy and registered rows written by the
previous versions, keeps every row's wrapper or its lack, then lets the
closed legacy account take one while a pending account, a registered account
losing its own, a 72-byte wrapper and a credential removed under a wrapper
stay refused. `SetRecovery` refuses an unfinished move (also once restored
but not closed), malformed wrappers (72, 74 and 0 bytes, version 2), a native
bearer and an unknown session, stores once, is `Done` for the same bytes
again (also after 11 minutes) and `Conflict` for others, refuses a fresh
wrapper at exactly 10 minutes and takes it at 9:50, stores nothing after
sign-out; two codes sent at once store one; a registered account keeps its
own; `invalidate_legacy` clears a wrapper and raises the epoch. The summary
reads the browser session only, its sign-in exactly 30 minutes before its
first end, unmoved by a renewal. Over the real browser and native listeners
(`central-client`), a registered account's summary says so, `SetRecovery`
answers `TermsRequired` until the terms are accepted and then keeps the
registration's wrapper; an account moved as Baylee's client moves one signs
in on the website, its summary says `legacy` without a code, the native
listener refuses both requests with its own session and with the cookie,
the code is stored once after the terms and opens the stored wrapper to the
account's master key. In `central-web-crypto`, against `central-crypto`'s
server side, a code made after a sign-in opens to the master key the
password wrapper holds (for a moved and for a registered account), is bound
to the epoch, differs each time, and is made only from the account's own
password wrapper. The website's tests (stand-ins and the strict mock of the
browser surface under the production CSP) cover both wordings in English and
German, both ways to make the code, "Not now", the retry with the same bytes
after a lost answer, a refused and a conflicting code. Findings fixed: the
native listener first answered `TermsRequired` to `SetRecovery` from an
account without accepted terms, since the terms were checked before the
surface (the integration run caught it); the store first took any session
kind, so a native session could store a wrapper had a caller passed one.
Injected faults each failed a test: the terms looked up as a native session
again, no 10-minute rule, `invalidate_legacy` keeping the wrapper,
`recovery_create` wrapping a fresh key, and one opening any wrapper, a
sign-in that never makes a code, no identity check of the summary, and a
retry with other bytes. Engineering review, not an audit.

Against a real Central, 23.10.2026 (`web/e2e-real/moved.spec.ts`): this
branch's release build in the deployed mode (`--accounts-stdin` with
`browser.listen`, the committed terms in force) over verify-full TLS to
PostgreSQL 18.6, provisioned, migrated (000001 to 000010, rebased onto step 1) and
granted as `docs/operations.md` says, behind the stand-in for the site's
Caddy; Chromium at `https://baylee.app`, at a phone's and a desktop's width.
Per width, one synthetic account was moved as Baylee's client and the bridge
move one (`LegacyReserve` with the bridge credential, enrollment without a
recovery wrapper, sign-in, `ConfirmRestore`, `LegacyClose`, over the native
listener with the committed WebAssembly); then the page signed in with real
OPAQUE and Argon2 in its Worker, asked `AccountSummary`, showed the terms,
accepted them, showed the code the sign-in made, sent `SetRecovery` once
after "I kept it", and the account page said the account has a code, also
after a reload. The rows: `source = legacy`, the move closed and restored,
73-byte password and recovery wrappers, epoch 1, only browser sessions left;
Central's log named no token, password or key. Everything started was
stopped and checked (both listeners and the stand-in refusing connections,
the container removed by its label), and the passwords, secrets document and
certificates, which lived only in that run's shell and its owned directory,
are gone.

## Display name and tag at Central, 2026-10-23

Owner decision 1 of 22.10.2026: display name and tag live at Central, a
moved account keeps its gateway's, a new one picks a name and draws a tag
(the accounts and administration design's step 3; [web
accounts](web-accounts.md) §"Display name and tag").

Design pass: the assets are the tag's uniqueness and permanence (chat on a
gateway is keyed by it), the moved account's own name and tag, the names of
the people who sign in at a gateway (which that gateway may learn, and no
other), and what a tag tells about the number of accounts. Callers: anyone
with a beta key registers (the browser listener's per-address budget), the
cookie's session chooses a name, the bridge credential carries names, a
gateway's redemption credential (or the bridge) asks for a lease's profile.
Invariants: the name's rule is a CHECK as well as code, and the gateway's
rule exactly; a tag is UNIQUE, positive, fits a gateway's column, and no
statement changes one once held; a registered tag is drawn from
operating-system randomness at or above the floor, so it is no counter;
`GatewayProfile` answers only through `GatewaySession`'s lease check, so a
gateway learns only the names of its own sessions; a name a person chooses
is never one of the reserved eight; nothing new is logged, and names are no
secrets but stay out of every log as before. Findings before code: the
gateway's rule was 3 to 32 characters without the edge rule until
15.09.2026, and its importer copied names unchecked, so a legacy name may
fail the CHECK and a bridge sending it would block that account's move: the
bridge sends none for it, and `LegacyProfile`'s once-marker became the tag
(a moved account has a name only with its tag); `SetDisplayName` refuses a
moved account before its tag is carried, or the website would let a person
replace the name decision 1 says it keeps; `Response::Enrollment` is the
bridge's too, so the tag reaches the page through the summary, not there.

Implementation pass: draws are rejection-sampled from `getrandom`, each in a
savepoint, and a unique violation counts as a taken tag only when another
account holds that tag afterwards, so any other violation (a taken name on
the bridge's reservation) keeps its own answer. The registration checks the
display name after the key, inside the key's transaction, so a refused name
reserves nothing. A carried tag is tried first and replaced by a draw only
when taken; a renewed reservation keeps a held tag (`COALESCE`). The backfill
is a bounded `DO` block that raises rather than loops once no range is left.
`set_display_name` reads the browser session's account `FOR UPDATE`, and its
`UPDATE` repeats `display_name IS NULL AND tag IS NOT NULL`;
`legacy_profile` locks the legacy row and repeats `tag IS NULL`.
`GatewayProfile` is a request and an answer of their own, counted with the
control requests and refused on the browser listener; `SetDisplayName` is
browser-only, uses the cookie, needs the terms and counts with the data
requests. Old requests decode: `Register` and `LegacyReserve` gain only
optional fields, absent on the wire when `None`. The page holds the same
rules, sends the name with the form, and shows the handle (never the
username) in the header. No new dependency, no grant: the runtime updates
`accounts` already.

Adversarial pass: over PostgreSQL 18.6 (`cargo xtask postgres`), the upgrade
applies 000011 over legacy and registered rows the previous versions wrote
(registered rows, pending and enrolled, get distinct tags of four hex
digits; legacy rows none; the previous binary still registers), the CHECKs
refuse the gateway's table of bad names, a name without its time, tag 0, a
negative and a duplicate tag, and the backfill run again over a half-taken
first range draws a five-digit tag. The store: refused display names at
registration (reserved, malformed, and an unknown key first) reserve
nothing; a name and tag chosen at registration; a name chosen later once,
the same again `Done`, another `Conflict`, with a native bearer, without a
session, reserved or malformed `Rejected`; the previous binary's untagged
account draws its tag at its sign-in; a moved account without a tag cannot
choose until `LegacyProfile` carried its own, one with a tag but no name
chooses; `LegacyReserve` stores both, refreshes the name and keeps the tag,
keeps a reserved name, refuses a malformed name or tag, draws when the tag
is held; `LegacyProfile` once only (`Conflict` with the same and with other
values), unknown or registered accounts and malformed values refused, a held
tag drawn anew; with the floor injected as 1 and the draws given, seven of
fifteen taken still draws there, taken draws are drawn again, eight taken
widen to the next range, eight taken draws are `Busy` and reserve nothing, a
draw outside its range is refused, and once `MAX_TAG` is held no range is
left. `GatewayProfile` names the lease's account, its name and tag once
carried, and refuses another gateway's lease, the bridge asking for a
registered gateway's, `admin` and `export`, an unknown lease, a disabled
gateway and an expired lease. Over the real listeners (`central-client`),
the registered account's summary has the name it registered with and a tag
of four digits, refused names are `Rejected` with a valid key, the moved
account's summary has the bridge's name and tag 13, and `SetDisplayName`
needs the terms, takes the same name, refuses another (`Conflict`) and a
reserved one, and is refused on the native listener with either token.
The website's tests (stand-ins, and the strict mock under the production
CSP at both widths) cover the field and its rules, the handle on the
recovery step, in the header and on the account page, choosing a name
once the terms are accepted, a name chosen elsewhere meanwhile, and the
moved account's note, in English and German. Injected faults each failed a
test, and were taken out again: a name of 17 characters allowed (the shared
table), `LegacyProfile` without its once-check and its `tag IS NULL`
(`Conflict` expected, `None` came), `GatewayProfile` without the lease's
gateway in its check (the bridge read a registered gateway's lease), draws
below the floor (a tag of three digits in the summary), and the page's
reserved names compared without stripping `_` and `-`. Engineering review,
not an audit.

## Central sunlight website — 27 October 2026

Design pass: adapt the website to the owner-approved Baylee memorial artwork,
including responsive light/dark palettes and a local photograph album. Keep
account protocols, server boundaries, CSP, dependency versions and external
origins unchanged. Photographs are the existing metadata-stripped derivatives;
no production account, private database or credentials enter the test setup.
Asset provenance is in `web/assets-sunlight.md`.

Implementation pass: decorative images ignore pointer input; the interactive
cloud is a labelled native button opening the shared native dialog. No HTML
injection, remote media embed, new dependency or inline script/style exemption.
Photos load only as the album is opened and traversed. The shared dialog now
explicitly returns focus when React unmounts it; previously removing an open
dialog could lose the opener. Reduced motion freezes all idle animation.
The mobile layout keeps the cloud before the longer project description.

Validation: `npm run check` passed (typecheck, lint, 215 tests and production
build), `npm run e2e` passed 30 tests, and `npm run screens` rendered all 40
combinations of ten routes, two widths and two themes. The new browser tests
exercise image decoding, photo wrapping in both directions, Escape and button
closure, opener focus, motion preference changes and the hover halo. Existing
browser tests continue to use synthetic external responses under the production
CSP. Desktop/mobile hero, album, sign-in, registration and card views were
visually inspected; the album was also operated in the in-app browser.

`cargo xtask gate`, `cargo xtask postgres`, `cargo xtask smoke` and
`cargo xtask secrets` passed. PostgreSQL and smoke fixtures verified removal
of their owned processes/listeners. Pinned `cargo-audit 0.22.2` and
`cargo-deny 0.20.2` both failed their advisory checks on **RUSTSEC-2026-0387**
(`crypto-bigint 0.5.5`, timing variability; advisory dated 23 October 2026).
The Rust and npm manifests/lockfiles are byte-identical to base `efc6731`;
this is a pre-existing dependency finding, not a dependency introduced here.
No advisory was ignored and no security gate was weakened. The local website
change is reviewable, but the full release checks are not green and no push,
deployment or production migration was performed. This is an engineering
review, not an independent security audit.

## Baylee’s complete remembrance — 27 October 2026

Design pass: replace the album’s short draft with the owner-approved family story,
in eight chapters and 37 paragraphs, alongside the existing five photographs.
The same German prose and English translation are embedded in the native client.
These are deliberately public, owner-supplied memories. No new personal data is
collected; account/session boundaries, requests, CSP and dependencies are unchanged.

Implementation pass: the reading column is ordinary escaped React text, with no
HTML injection. Native dialog ownership/focus restoration remain in the shared
component. Chapters and photos have independent state; photo changes preserve the
reading offset. The date range says “With us”, not a fabricated birth date. The
new WebGL canvas is limited to the masthead, 960 × 300 pixels and 20 draws/second;
it releases GPU objects and listeners on unmount, pauses for hidden documents,
stills for reduced motion and rebuilds after context restoration. CSS supplies
the fallback. No remote asset, analytics, unsafe API, dependency or CSP exception.

Adversarial/QA pass: `npm run check` passed typechecking, lint, 215 unit tests and
the production build. Browser checks pass at desktop/phone widths in light/dark,
including all 37 paragraphs, the last words at the scroll end, independent photo
navigation, chapter boundary controls, keyboard focus/escape, Home/End on tabs,
320-pixel layout, WebGL disabled, real context loss/restoration and no continuing
GPU draws while reduced-motion or closed. Shader alpha was corrected after visual
inspection; the header cloud and warm light do not cover the prose. All 34 normal
E2E tests pass; the 40 opt-in whole-site screenshot tests were not rerun because
the changes are confined to this album. No production accounts or data were used.

The Rust gate, isolated PostgreSQL 18.6 fixture, development smoke and working-tree/
history secret scans passed again. Each fixture verified its owned process,
listener and container cleanup. Pinned `cargo-audit 0.22.2` and `cargo-deny 0.20.2`
still report the previously documented **RUSTSEC-2026-0387**, `crypto-bigint 0.5.5`;
licenses, sources and dependency bans pass. Manifests/lockfiles are unchanged.
The advisory is not ignored: full release checks remain blocked and nothing was
pushed, deployed or migrated in production. This is the implementing agent’s
engineering review, not an independent security audit.

### Parchment memorial UI review — 27 October 2026

Design pass: local static owner-supplied photos, decorative preview effects only,
uncropped originals loaded on demand, no protocol/account/storage change and no
new dependency. Preserve the approved 37 paragraphs and family-requested wording.
Implementation pass: nested native dialogs isolate Escape and restore focus;
backdrop dismissal requires a pointer-down on the backdrop; late image loads do
not discard zoom; bounded panning resets on photo/fit/zoom/resize. All transient
listeners/observers/timeouts are cleaned up. Ten images have metadata stripped.
Adversarial pass: desktop/phone tests exercise rapid chapter reversal, reduced
motion during a turn, no-WebGL/context recovery, all ten photos, delayed full
image loading, zoom bounds, dragging, stable hover geometry and nested dismissal.
`npm run check` passes; full mocked browser suite: 40 passed, 40 opt-in screen
capture cases skipped. Rust gate, PostgreSQL, smoke and secrets checks pass.
Pinned audit/deny still fail on the pre-existing `crypto-bigint 0.5.5` advisory
RUSTSEC-2026-0387, also present before this UI change. No exception or dependency
change was introduced. This is feature-branch work, not external release approval.


## Bounded bigint advisory handling, 28 October 2026

Design pass: the affected operations from RUSTSEC-2026-0387 are not reached by
the fixed Ristretto255/SHA-512 OPAQUE suite. Reviewed both upstream Ristretto
implementations and the hash-expansion/domain helpers. The current dependency
requirements cannot take fixed bigint >=0.7.6 independently. Preserve protocol,
stored credentials and vetted cryptography; do not introduce a fork or new
primitives. This is an unreachable-code exception, not a patched-library claim.
See [the source review and removal condition](crypto-proposal/bigint-advisory.md).

Implementation pass: exact registry versions/features and reverse consumers are
checked with all-feature metadata including inactive optional packages. Bind
the local crypto member to its manifest location and fingerprint its entire
reviewed crate. Both the complete gate and dependency CI job require the guard
before accepting the narrowly named audit/deny exception. No dependencies,
cryptographic code, browser module, data formats or KDF parameters changed.

Adversarial pass: regression tests reject additional consumers at each affected
boundary, changed sources/versions/features, missing and duplicate packages/nodes,
non-workspace crypto, edited/added/removed/renamed source files and links. A test
found that duplicate central-crypto resolution nodes were initially accepted;
that is now rejected too. LF and CRLF source text produce the same fingerprint.
All six scope regressions pass. Full local gate, PostgreSQL fixture, development
smoke, history/worktree secret scan, pinned audit 0.22.2 and deny 0.20.2 pass.
These are engineering checks; independent protocol review and release gates
remain separate requirements.

An additional Argon High review of the supplied guard and reached upstream
sources found no confirmed findings. It did not run commands or independently
certify the protocol; local/CI checks supply the execution evidence.

## Password change, 28 October 2026

Roadmap M1, step 1 ([native login](native-login.md) §"Password change").

Design pass: the assets are the account's login (the OPAQUE record), its
vault (the master key, reachable through the password wrapper and, for
accounts that have one, the recovery wrapper), and the sessions and gateway
leases that act for it. Callers: a session holder on either listener, who may
be the owner or a thief of a bearer or cookie. Central cannot check that a
new wrapper holds the same master key, so the decisive finding before code
was that a session alone must not change the password: a first proposal,
a sign-in less than 10 minutes old (the `SetRecovery` rule), would
let a thief with a fresh token install their own credential and a wrapper of
a random key, taking the login and leaving the owner's data reachable only
through a recovery code. The change instead carries a fresh OPAQUE proof of
the current password inside the exchange, bound to the starting session and
to the account's login budget. That budget counts OPRF evaluations, not
requests: the start answers a registration response besides KE2, both
evaluations of the account's OPRF key, and with the registration response's
output for a guessed password the masked envelope in KE2 can be opened and
checked offline, so one start tests two guesses. A start therefore costs two
of the eight units a minute, and a stolen token guesses no faster than
anyone can through `LoginStart` without a session, and gains nothing alone
(found in the second review, below). Invariants: the account is taken from the
session, never from the request; a change commits only at the credential
revision its proof was made against, with the session still live; every
session ends with it (a stolen session too), and the caller's next session
is issued in the same transaction; vault epoch, document, objects and
recovery wrapper do not change; a legacy account changes its password only
once its move is closed, so the bridge's `LegacyInvalidate` never meets a
changed Central credential; nothing new is logged. Accepted limits: the
client is trusted with its own vault's continuity (a modified client can
only lose its own data); devices learn of the change when refused (an
e-mail notice comes with step 4); no audit entry yet (the audit log comes
with step 2's roles).

Implementation pass: `Pending` gained a purpose, a sign-in or a change bound
to the digest of the starting token; `LoginFinish` refuses a change's
attempt and `ChangePasswordFinish` a sign-in's, each after consuming it.
The start reads the credential through the live session of the listener's
kind and a finished move, then charges two units of the per-account budget
(`Limits::account` takes the units and spends nothing when they do not fit)
before the worker permit, then runs KE2 and the registration response in one blocking
task. The finish verifies KE3 off the async workers, then the store parses
record and wrapper, locks the account through the session at the start's
revision (`FOR UPDATE OF a`), updates record, wrapper and revision, deletes
all of the account's sessions and inserts the new one. The lock and the
session are read in two statements, as `objects::locked_owner` reads them:
a joined `FOR UPDATE` that waited re-evaluates only the locked account row,
so a session replaced meanwhile would still have looked live (the
adversarial pass below proved it). `dispatch` answers
the finish beside `LoginFinish`, so the browser listener sets the new cookie
as for a sign-in; the terms are checked at the start (an acceptance is never
withdrawn, and a new version needs a restart, which forgets every pending
attempt). Both requests are browser operations (the exhaustive
`browser_operation` match) and data requests per session; the native
listener serves them too. The SDK checks the answer's identity and that the
stored wrapper is the one it sent, logs a mismatched new session out, and
resolves a lost answer by the old session's status. No new dependency,
migration or grant: the runtime already has `UPDATE` on `accounts` and full
rights on `sessions`. The identity service's session-binding and listener
checks are also enforced by the store's join on the session's digest, kind
and account, so a fault in either alone is caught by the other.

Adversarial pass: over PostgreSQL 18.6 (`cargo xtask postgres`), the store
refuses a password credential while the legacy move is open, for a token of
the other kind and for an unknown token; refuses a commit with a record one
byte short or long, a wrapper one byte short, a next session of the other
kind, another account's session and a credential revision other than the
account's, each changing nothing; one commit raises the revision, ends both
kinds of session, issues one, keeps epoch, document and its revision, and
the replaced credential commits nothing again; a change waiting on the
account's lock while its session is deleted is `Rejected` and changes
nothing. Over the real listeners (`central-client`): the SDK is refused
while the move is open, refuses a seven-character password before sending
anything and a wrong current password at its own KE2 check, the session
staying live; a sign-in begun before the change and finished after it is
`Rejected`; the old password fails after the change and the document
reads; a lost answer after a commit is `Rejected` (the old session ended)
and the newer password signs in. By hand: no session and an unknown bearer
start nothing; a change finished with another bearer, then (consumed) with
its own and with none; a change's attempt as a sign-in's and the reverse;
a proof with one bit changed; a short wrapper beside a good proof; each
refused and consumed, the credential revision unchanged; with seven units
spent a sign-in's start fits and a change's does not (`Busy`, nothing
spent), and once eight are spent the next sign-in is `Busy`. On the
website, with a registered account playing natively at the official
gateway: no change before the terms (`TermsRequired`), the cookie as a
native bearer starts nothing, a change begun on the website cannot finish
on the native listener; the change answers a browser session with a new
cookie, the old cookie, the native session and the gateway's lease end,
the summary still has its recovery code, the stored recovery wrapper is the
registration's and opens the first document, the new password signs the
client in (leaving the website's session) and the old one fails. A change
whose session a later sign-in replaced cannot be finished by the
replacement. Each test asserts the exact answer (`Rejected`, `Busy`), so a
spent budget cannot pass for a refusal. Injected faults each failed a test and were taken out again:
`LoginFinish` taking a change's attempt, the start charging no account
budget, an open legacy move allowed, the lock's revision check dropped
(the update's own check still refused: `Unavailable` where `Rejected` was
expected), the revision not raised, only native sessions deleted, no
wrapper length check, the SDK reporting a lost answer as `Unavailable`,
no terms needed, the finish not bound to its starting session (first run:
undetected, since the store's join refuses a foreign session; the test of a
replaced session was added and catches it), and the commit without its
session recheck (a real finding: it committed after the session was
deleted; fixed as above). Clippy (pedantic, all targets and features), the
unit suites and the gate pass.

Second review, from scratch at the owner's required effort (the first
implementation was written before the session's effort was raised): the
budget finding above, fixed with a unit test of the two-unit cost (and a
change charged one unit fails the wire suite's budget test); tests
that accepted any error where a spent budget could hide a wrong answer now
assert the exact one, and each part of the wire suite has an account of its
own with its units counted; the record's wording of the lock was brought in
line with the two statements. The lock order matches the other account
mutations (account row first, then sessions); walking it against a sign-in,
a renewal (session row, then its leases) and a logout found no lock cycle.
That is reasoning, not a concurrency test. Engineering review, not
an audit.

## Central administration, 28 October 2026

Roadmap M1 step 2a ([administration](administration.md)): the accounts and
administration design's step 5, server side.

Design pass: the assets are the administrator role (which approves
nothing yet but will lock, ban and reset), the audit's completeness, and
what administrators see of accounts. Callers: an administrator's browser
session, any other session, and the operator under the migrator login.
The owner decided (22.10.2026) that the runtime login writes administrators'
decisions; what a taken-over service gains is recorded in the design and
the threat model. Invariants: `Admin` only on the browser listener with an
administrator's browser session; the role checked in the statement or
transaction that acts; a change only within 30 minutes of a password
sign-in, renewal not counting; every change writes its event in its
transaction; the audit holds ids, never words, and the runtime can neither
rewrite nor delete it; nobody removes their own role, so one administrator
always remains; the answers have no field for a credential, wrapper or
ciphertext. Finding before code: two administrators removing each other at
once would each see the other still present and leave none; changes are
serialized on the administrators' rows and the acting role is read again
after the lock.

Implementation pass: migration 000012 (roles, audit, `last_sign_in_at`),
exact grants and their startup check (two more tables, `count(*) = 15`); the
store's reads check the administrator in their transaction, and `SetAdmin`
refuses anyone else before taking a lock, then locks the administrators'
account rows in id order and reads the acting role in a fresh statement.
The design's lock on the role rows needs an `UPDATE` grant there, which the
runtime does not hold; the first run of the role tests found it (the change
failed as `Unavailable`), and the account rows are locked instead. A search
is a plain `strpos` substring or a tag, never a pattern. The operator's
`--appoint-admin-stdin` and `--sweep-stdin` read the migrator login from
standard input, as `--migrate-stdin` does, and print fixed messages; the
username arrives at run time. No new dependency.

Adversarial pass: over PostgreSQL 18.6 (`cargo xtask postgres`), the
runtime refuses to start with `UPDATE`, `DELETE` or `TRUNCATE` on the audit,
an `UPDATE` column on roles, or without `INSERT` on the audit or `DELETE` on
roles, and its own login cannot update, delete or truncate an event; the
operator's appointment refuses an unknown, pending or malformed name,
appoints once and writes one operator event, and again changes nothing; a
non-administrator, a native bearer and an unknown token change and read
nothing; a pending or unknown account is not appointed; nobody removes
their own role; refusals write no event; appointing and removing again
write nothing new; a removed administrator's next read is refused; a
sign-in 31 minutes old reads but does not change, and a renewal does not
freshen it; two administrators removing each other while their rows are
held leave exactly one; the search finds newest first, pages, matches a
display name without case and a tag exactly, and treats `%` as text; the
account view and the audit name the acting administrator and nothing else;
a password sign-in of either kind sets the sign-in time and renewal keeps
it; the sweep deletes the event older than three years and keeps the one a
day younger, and the runtime cannot sweep; the upgrade applies 000012 over
000011's rows, the previous binary still signs in, and the CHECKs refuse a
role other than `admin`, an administrator's event without an actor, an
operator's with one, an action in words, an unknown actor kind and a detail
over 1024 bytes; an account's roles go with it and its events stay. Over
the real listeners: the operator appoints, the summary says so, the native
listener refuses `Admin` with the cookie and with any token, the list,
account and audit answers carry what the store tests showed, and a stale
sign-in and a malformed search are refused. Injected faults each failed a
test and were taken out again: a stale sign-in allowed to change, removing
one's own role, no role check after the lock, the account list without a
role check, a change without its event, a pending account appointed, the
grant check skipping the audit table, no sign-in time, and the summary never
saying administrator. The identity service's refusal of `Admin` on the
native listener is also enforced by the store (a native bearer is no browser
session), so a fault in either alone is caught by the other; the unit test
names `Admin` among the browser's own operations. Engineering review, not an
audit.

## Resetting a login, 28 October 2026

Roadmap M1 step 2b ([account reset](account-reset.md)).

Design pass: the assets are the login (a reset replaces it), the vault (a
new one destroys what the old key opened), and the recovery wrapper.
Callers: an administrator who issues a code, whoever holds the code (the
person, or an administrator keeping it), and a device still signed in.
Invariants: only an administrator with a fresh sign-in makes a code, never
for an administrator, oneself, a pending account or an open legacy move,
and every code is audited; Central keeps the code's digest only, for 24
hours, and a new code, its use or a password change ends it; a code finds
its account only with the right username and an unexpired digest, and a
wrong code spends nothing of the account's budget; a start that answers a
registration response (one OPRF evaluation) spends one unit; a finish
commits under the account's lock with the code, revision and epoch checked
there, spends the code and ends every session; keeping changes only the
credential; a new vault raises the epoch, replaces document and recovery
wrapper, deletes the objects the old key encrypted and empties their quota.
Findings before code: a reset code's start is an OPRF oracle for its holder
(charged a unit, after the code is found, so it is no worse than
`LoginStart`, which needs no code); the recovery wrapper leaves Central
before any password proof (authorized by the code's 160 bits, and opened
only by the recovery code's own; recorded as a deliberate departure);
issuing ends nothing, so until locks exist a reset answers a forgotten
password, not a takeover (said in `status.md`). Accepted: whoever holds a
code takes the login, and with a new vault destroys the data; the audit
makes an administrator who keeps a code visible, it does not prevent it.

Implementation pass: `ResetCode` (central-api) is 160 bits of
operating-system randomness, read only with its `RESET` prefix, without
Clone, redacted in Debug, carried as its printed form only in the
administrator's answer; `ResetStart` decodes it, so a malformed code never
reaches the store. Issuing locks the administrators' rows as `SetAdmin`
does, so the target cannot become an administrator meanwhile, then checks
the acting role after the lock. Pending resets have a map of their own (32,
60 seconds, bound to account, digest and listener). The finish's lock is a
single-table `SELECT … FOR UPDATE` whose recheck sees the row's latest
version. Two findings in the code, fixed before tests: migration 000005's
`account_closed_enrolled` requires `restored` for a closed move, so a moved
account's new vault keeps it (the design cleared it for every account); and
`object_bytes`, the quota's byte counter, must be emptied with the objects.
A registered account's new vault without a recovery wrapper is refused in
Rust, so the client sees `Rejected`, not the schema's `Unavailable`. A
password change clears an outstanding code; a sign-in does not.

Adversarial pass: over PostgreSQL 18.6, refused issuers (a non-administrator,
a native bearer, a sign-in 31 minutes old) and targets (oneself, another
administrator, an open move, a pending and an unknown account) write no
event; a code is audited once, its expiry is 24 hours ahead and shown on the
account's row; it finds the account by name without case and nothing with
another code or name; a second code replaces it, and issuing ends no
session. Keeping: a short record or wrapper and a stale revision change
nothing; the finish changes only the credential, ends both sessions and
spends the code, after which neither the finish nor the start works. A
registered account's new vault: refused without or with a short recovery
wrapper and with a malformed document, each changing nothing; then the
whole row compares equal to the expected one (revision, epoch, document,
revision 1, the new recovery wrapper, `restored` false, no objects, an empty
quota, no session, no code); the old epoch finds nothing, the new one signs
in, takes no gateway ticket until it confirms and then does. A moved
account's new vault keeps `restored`, may go without a wrapper and compares
equal in full. A password change clears a code; an expired one finds
nothing, and the sweep clears it. The upgrade applies 000013 over 000012
and its CHECK refuses a digest without expiry, a code on a pending account
and a short digest. Over the real listeners: an administrator's code from
the website; a wrong recovery code refused at the client with the code
still usable; the right one keeps the document under the new password, and
the old one and the spent code are refused; two starts of one code, the
finish on the other listener refused, the first finish spending it and the
second refused; eight wrong codes spending nothing of the account's budget
before a new vault, which signs in at the next epoch with its own document
and a new recovery code; a signed-in device keeping its data, refused with
another account's name, its own session ended; and a start with a good code
`Busy` once the account's eight units are spent. Injected faults each
failed a test and were taken out again: an administrator's login reset, a
stale sign-in issuing, an expired code found, a finish that does not spend
its code, a new vault keeping the objects, the quota or `restored`, a
registered account's new vault without its wrapper passing the Rust check
(the schema refused it as `Unavailable`), a start spending nothing, a
password change keeping the code, a finish on the other listener, issuing
without its event, and the sweep keeping expired codes. Not detectable, on
purpose: refusing one's own login, since an actor is always an
administrator and an administrator's login is refused anyway; the explicit
check stays as defence in depth. Engineering review, not an audit.

## Deleting and exporting an account, 28 October 2026

Roadmap M1 step 3 ([account deletion](account-deletion.md)), with the
owner's decisions of the same day.

Design pass: the assets are the account's existence (a deletion cannot be
undone), the data it takes with it (documents, objects, sessions, owned
gateways), and the metadata an export hands out. Callers: the person with a
live session, a thief holding only that session, and the account's own
concurrent operations (a sign-in, a password change, an administrator's
appointment, a reset). Invariants: a deletion carries a fresh OPAQUE proof of
the password, bound to the session, the listener and the purpose, charged one
unit of the account's login budget; it commits under the account's lock with
the session, credential revision and epoch rechecked in a fresh statement
after the lock, and is refused for an administrator (checked after the lock,
so an appointment racing it either waits or is seen) and for an open legacy
move; foreign keys remove every row naming the account, under the runtime's
grants (no new grant: a cascade needs no privilege on the child tables), and
the audit stays; Central's terms are not required to leave or to export; an
export needs a password sign-in under 30 minutes, names only the session's
own account, carries no credential, wrapper, digest or ciphertext, and every
list in it has a stated bound. Findings before code: deleting an owner frees
its gateways' origins, but their published terms are keyed by origin and stay
(the runtime may not delete them), so a later registrant of the same origin
would inherit them as current (documented for the operator and the owner;
no cascade can fix it under the current grants); the gateways the account
administers lose its row, so their `admin_revision` is raised in the same
transaction and an owner's next list change sees `Conflict` instead of a
stale list; a deletion's lost answer cannot be told from a replaced session,
so the SDK says `Unavailable` only while this session lives. Accepted: a
gateway registered between the start (whose answer names the owned gateways
for the warning) and the finish is deleted without having been named; the
official gateway keeps a moved account's source data (its own copy, password
retired), which a Central deletion does not reach.

Implementation pass: the three requests are in `browser_operation` and
`uses_session`, not in `browser_only` and not in `needs_acceptance` (a test
names each). The start finds the account deletable before it charges the
unit, so an administrator or an open move spends nothing; the attempt is a
`Pending` of its own purpose (`DeleteAccount` with the starting session's
digest), consumed before every check, so a sign-in's attempt finishes no
deletion and a deletion's no sign-in. The finish's lock is the password
change's (session, revision, epoch, a finished move, `FOR UPDATE OF a`); the
recheck after it is a fresh statement with the role condition, since an
appointment's insert holds the account row's key-share lock and the lock's
own recheck sees only the account row. The other owners' gateways the
account administers raise `admin_revision` in the same transaction; the
`DELETE` names the revision again. The runtime's grants are unchanged: the
cascades into `gateway_approvals`, `migration_archives` and
`terms_acceptances`, which the runtime may not delete from, run without that
privilege, and the store's tests run under the runtime login to show it. The
export is one read-only repeatable-read transaction whose queries name their
columns; its lists are cut at 256 with `complete` false, and a worst-case
export encodes within the wire's limit. On the browser listener a deletion
that succeeded clears the cookie; a refused one leaves it. The SDK proves the
password at the start (a wrong one is `Rejected` there), returns the owned
gateways for the warning, and resolves a lost finish by the session's status.
Two departures from the design, recorded there: the challenge's gateways are
`OwnedGateway` (id, origin, approved) rather than a name, and the export's
`restored` flag became a `state`.

Adversarial pass: over PostgreSQL 18.6, under the runtime login with the
deployed grants. One account was given a row in every table that can name
it: two sessions, an outstanding ticket and a lease, its own approved gateway
with another account on its list, a place on another owner's list, objects
(live, a reservation, a deleted one), an archive, an acceptance, a reset code
and audit events. Each of those columns was seen to hold a row first. After
the deletion no uuid column of the schema (apart from the audit and
`granted_by`) and no session digest names the account or its gateway. The
audit kept its events, the other owner's `admin_revision` rose by one, and its
session lived. Refused, each changing nothing: an open move, an administrator
at the start, an administrator appointed after the start at the finish, a
session of the other kind, an unknown token, another account's session with
this account's credential, and a revision or an epoch one ahead. Races, each
by holding the account row from the operator's connection until the
deletion was seen waiting: an appointment committed meanwhile, and the
account's sessions deleted meanwhile. Both delete nothing. The export,
field by field; refused for a sign-in 31 minutes old, a token of the other
kind and an unknown one; cut at 256 terms with `complete` false. Over the
real listeners:
- the SDK on a moved account with a gateway and an object (refused while the
  move was open, a wrong password changing nothing, the warning naming the
  gateway, which left the directory);
- an account that had not accepted the terms in force (an object refused with
  `TermsRequired`), exporting and deleting;
- a lost answer (`Rejected`, account gone);
- an attempt finished on the other listener, then consumed;
- a deletion's attempt as a sign-in's and a sign-in's as a deletion's;
- a newer session of the same account finishing an older one's attempt;
- an administrator refused before anything was spent, with an export saying
  so;
- six starts and a seventh `Busy`;
- the website's export and deletion, the cookie cleared only on success.

Fault injection, each fault reverted after its run. Nineteen faults were
injected, and eighteen were caught by the assertion meant for them:
- the role condition removed;
- the recheck removed, or without the role, or without the session (caught
  only by the replaced-session race);
- the role checked in the lock statement alone (caught only by the
  appointment race: the lock's recheck does not see a role committed while
  it waited, which is why the fresh statement exists);
- open moves made deletable;
- no `admin_revision` raised;
- a start that costs nothing;
- the session binding disabled;
- a sign-in's attempt accepted as a deletion;
- the terms required for leaving;
- the cookie cleared on the wrong outcome;
- the freshness dropped;
- `complete` always true;
- a lost answer called retryable;
- the export forgetting the events the account acted in;
- the warning naming every owner's gateways;
- the export reading another account's row (the SDK's account check refused
  it first).

Not detectable, on purpose: the listener binding of the attempt. A token is
a session of one kind only, and the store finds no session of the other kind
for it, so the deletion is refused there too; the check stays as defence in
depth. The local runs used a raised fixture timeout, because the shared
machine was loaded; CI ran the real five-minute limit (the postgres job took
173 seconds). Engineering review, not an audit.

## The website's design guidelines and public pages, 30 October 2026

Design pass: [website design](website-design.md) writes down the look the
pages already had and the rules for the account area and the administration
to come: semantic tokens, a type and spacing scale, components, motion and
accessibility. The dark scheme becomes the blue hour; the footer becomes a
three-column map; the gateway directory gets cards with a deep link that
opens the client on that gateway, in the grammar of Baylee's deep-link design
(`baylee://gateway/add?origin=…`). No request, answer, boundary, CSP,
dependency or stored value changes.

Implementation pass: the deep link is built only from an origin the
directory's checks have already passed, and `central/deeplink.ts` refuses
anything but a bare https origin (a loopback http origin only on a page
served from this machine): no path, query, credentials or operator text
reaches the link. The link carries nothing secret, and a client that does
not know the scheme leaves it unanswered; the page says so and keeps "copy
the address". Page changes cross-fade through the View Transitions API only
where the browser has it and motion is welcome; the update draws the new
page synchronously, because the browser runs no frame callbacks during it.
Staggered arrival is a backwards-filling animation, so a card's own hover
transition is untouched, and the global reduced-motion rule stills it.

Validation: `npm run check` (typecheck, lint, 248 unit tests including the new
contrast test of both schemes, the palette rule and the deep-link refusals,
and the build) and `npm run e2e` (40 passed under the production CSP) pass.
The first e2e run caught the cross-fade waiting on a frame callback that
never comes inside a transition; fixed as above. Engineering review, not an
audit.
