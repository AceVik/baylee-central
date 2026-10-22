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
