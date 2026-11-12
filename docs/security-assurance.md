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

## Session tokens, 6 November 2026

Design pass: assets are the Ed25519 signing key, the Central token and each session's capability.
Hostile callers considered: a gateway, a client with a real token, a holder of a copied token, an
old client, a gateway operator choosing its own address. Invariants: nothing in a token selects an
algorithm or key; a token is good for one audience; no path that ends a session can forget to
revoke it; a revocation lost in transit is not lost; a destination that comes from outside reaches
no private address. Implementation pass: the header is compared with fixed bytes; every end of a
`sid` is a trigger rather than a call; the push queue is the log and a cursor, so it holds no more
than the log; address checking and dialling are one step; a bearer keeps to its side by an exhaustive
match, so a new operation does not compile until it is decided. No package was added.

Adversarial pass, what it found and what changed. A capability copied out of a Central token opened
the session bare, which made the signature decorative: bare credentials are now refused for client
operations. A stream left at the attempt limit by a crash was never claimed again; a page that aged
out was never passed; a clock step could leave a hole no message could cover: each is fixed and has a
test. Tests that used a random bearer as a session stopped reaching the checks they were written for
once the gate answered first; they now use real sessions on the wrong side, and a token whose session
ended has its own test. `SignedToken` read from the wire skipped its bounds. A decoded payload holding
a capability was not wiped. A missing `token_keys` at a gateway deploy would have left Central down
after the migration, with the previous binary unable to start either; the hook now asks first.

This is the implementing agent's engineering review. It is not an independent audit, and the owner's
release gates still apply. No real gateway has been called, and `official_address` has not been
exercised against the real host.

## Dependency migration for 0.1.0-beta.9c, 2 November 2026

Design pass: retain existing credentials, fixed Ristretto OPAQUE suite, chat
known answers, protocol domains and resource limits while upgrading maintained
dependencies. No application-specific cryptography or first-party unsafe is
introduced. The owner accepted the 45 exact package/feature changes and their
dependency-internal unsafe/native implementation scope in
[the dependency review](dependency-proposal/beta9c.md), subject to gates.

Implementation pass: consolidated duplicate SHA/HKDF/HMAC aliases onto the
current zeroizing implementations, kept OPAQUE's required digest/RNG generation
explicit, and migrated the wiped Argon2 work buffer to 0.6. The previous Argon2
integration is enabled only for compatibility tests. Checked new HMAC key
initialization, unchanged KDF parameters, and the bounded bigint reachability
exception; neither its caller/feature checks nor its mutation tests were
relaxed. Reviewed changed build scripts, package sources and licence results.

Adversarial pass: all 168 crypto tests pass, including old/new Argon2 output
equivalence, published vectors, malformed/tampered messages, secret redaction
and unwind wiping. The full Rust gate, real PostgreSQL enrollment/login and
migration tests, HTTP boundary smoke, secret scan, crypto scope, audit and deny
checks pass locally. Deliberate PostgreSQL/HTTP fixture failures also return
their expected errors and remove their owned services. The canonical Linux WASM
build and byte comparison pass. Web typecheck, lint, 373 unit tests, the
production build and 58 browser smoke tests pass (76 existing opt-in cases
skipped). Four real-service browser tests pass at desktop and phone widths:
OPAQUE registration/unlock, contacts, encrypted direct/group chat, trust state,
PWA installation, notification category consent after page closure and a public
offline shell without cached account data. Provider delivery is injected through
CDP; this does not claim a live third-party push-provider test. All owned test
processes, ports, containers, VM and secret material were removed. Remote
platform checks remain required before merge. This is an engineering review,
not an independent audit or a production release approval.

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
Asset provenance is in `web/assets-sunlight.md` (since 12.11.2026 `docs/assets-sunlight.md`).

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

## Gateway tokens, the account area and gateway administration, 30 October 2026

Design pass. Assets: a gateway's ability to sign players in for its own
audience and to appear in the public directory, an account's sessions, and
the administrators' decisions. The owner asked for tokens that let a user host
a gateway, for the gateway to connect with one, and for administrators to
edit tags. A token is the existing primitive (`Token`, 32 random bytes,
SHA-256 digest only), scoped (`sso`, `listing`; the two existing credentials'
roles), named, expiring within 366 days (a CHECK bounds both ends) and
revocable. It authenticates only through the two queries that already decide
every gateway request (`authenticate_gateway`, `gateway_heartbeat`), with
revocation, expiry, scope and approval checked in the same statement, so
there is no cache to go stale and no third path that half-works. Approval
stays a person's decision; Central fetches no registered address. Minting is
a change and needs a password sign-in under 30 minutes old, so a stolen
cookie mints nothing later. The account area is browser-only and acts
through a live browser session of the current credential (`kind =
'browser'`); a native bearer acts on nothing there. Administrators' changes
reuse the role-after-lock pattern and write their audit in the same
transaction; the tag change keeps both tags (numbers) in `detail`.

Implementation pass: owners and administrators read token rows without the
digest; the token itself appears once, in `OwnerAnswer::TokenCreated`, and
its text form (`bgt_` + hex) is formatted by the same rule on both sides
(a shared vector). `last_used_at` is touched on heartbeats, redemptions and
status checks, at most once a minute, never by the per-request lease checks.
Disabling deletes the gateway's leases in the transaction that disables it.
The runtime gains exactly `INSERT, UPDATE (enabled)` on `gateway_approvals`
(never `DELETE`) and `SELECT, INSERT, UPDATE (revoked_at, last_used_at)` on
`gateway_tokens`; the startup check now names a list of updatable columns
and still refuses any other column, `DELETE`, a grant option or ownership.
Migration 000014 relaxes the official CHECK so a registered row may have no
credential pair, never half of one; the previous binary's lookups never match
a NULL digest, but it refuses to start over the wider grants, so a rollback
rolls the grants back with it ([operations](operations.md) §"Gateways and
their tokens"). Reading one's sessions and gateways, ending the client's
session and revoking a token need no accepted terms; registering and
minting do.

Adversarial pass (PostgreSQL fixture, `tests/support/accounts/gateways/
tokens.rs`, and the upgrade test): the official origin, malformed origins,
a control character in a name, a native bearer, a taken origin; bounds of
name, scopes and lifetime; another owner's gateway; a stale sign-in; the
ninth live token; a pending gateway's token (status yes, authentication
and heartbeat no); approval; scope separation both ways; an expired and a
revoked token; disabling (leases refused, directory hidden, status says
disabled), the same state twice writing one event, enabling, an
administrator's revocation, a stale administrator; a taken tag, out-of-range
tags, the same tag twice writing one event with both tags; sessions listed
with the lease's origin and the client's session ended. The grant contract
is probed both ways (each wider privilege and each missing one refuses
startup). The upgrade test adds a token-only row and refuses half a pair, a
pair on the official row, an ownerless registered row, a token without
scope, longer than 366 days, empty-named, of the wrong digest length, a
repeated digest and a revocation before creation. The website's area is
covered by unit tests of every answer's exact shape and by page tests with
stand-ins (`npm run check`: 287 tests). Engineering review, not an audit.

## Website follow-ups: gateway links behind a switch, 30 October 2026

Design pass: the gateway cards offered "Open in Baylee" through
`baylee://gateway/add` before any released client reads that link, so the
public page had a button that did nothing. The link is now behind a
build-time switch (`VITE_DEEP_LINKS`, off), and copying the address is the
card's first action until a client release with gateway links is out. The
link is also refused for an https origin whose host is an IP address or a
single label, which the client ignores (Baylee's `docs/proposals/deep-links.md`).
The account area's token table takes the table styles, the administration's
account page drops an actions panel with nothing in it, and a gateway's
placeholder version shows as "—" until its first heartbeat.

Implementation pass: `deepLinks()` reads the build's environment on each
call, so the cards and the directory's hint follow one switch; with it off
no `baylee:` URL is in the page at all.

Adversarial pass: unit tests for both switch states, for the IP-literal and
single-label refusals beside the earlier ones, and for the empty actions
panel (an injected always-shown panel fails the test). The smoke test and
the real-Central run check that the default build has no "Open in Baylee"
link. Engineering review, not an audit.

## The official gateway's move, and approval for the directory only, 30 October 2026

Design pass. Two owner decisions of 30.10.2026. First, the official gateway
moves from `https://baylee.acevik.de` to `https://gateway.baylee.app`, and
installed clients, which ask Central for tickets at the old address, must keep
working. The official set is compiled (`OFFICIAL_ORIGIN`, `OFFICIAL_ALIASES`),
as the official origin always was. No configuration names another gateway
the bridge's authority; the production config still has no gateway field.
The development launcher's `gateway_aliases` takes numeric-loopback origins
only, and a live service refuses `with_aliases`.

Second, a gateway token signs players in without an administrator's approval,
which now governs the directory only. A token names an owner who answers for
the gateway, and that owner or an administrator revokes it at once. A
credential pair, which its owner cannot revoke, keeps needing the approval.
Disabling still stops both and ends the gateway's leases.

Implementation pass.
- The bridge's authority carries the set: `gateway_authority` answers the
  canonical origin first, and every lease query reads
  `audience = ANY(string_to_array($n, ' '))`. A validated origin holds no
  space, and `origins()` refuses an empty set or one with a space. No
  `postgres-array` feature, so no dependency change.
- A redeemed lease keeps the ticket's own audience: it is selected with the
  spent ticket, not taken from the caller.
- Gateway terms: the authorization reads an acceptance under any origin of
  the set. `GatewayTerms` and `AcceptTerms` for an alias use the canonical
  origin, where the bridge publishes.
- The store refuses to register any compiled official origin, beside the
  identity service's check of the configured set. So nobody can take the new
  origin before the official row moves to it.
- Pending gateways: the token arms of `authenticate_gateway` and
  `gateway_heartbeat` test "not disabled" (`NOT_DISABLED`) instead of joining
  an enabled approval. The credential arms keep the join. Redemption, lease
  checks, profiles, admin checks and registered tickets use "not disabled",
  so they are never stricter than the authentication before them. The
  directory still joins an enabled approval.

Adversarial pass (PostgreSQL fixture):
- Both official origins are refused to `RegisterGateway` and to the
  website's `AddGateway`, in both modes.
- A live heartbeat set with a foreign origin is refused. The heartbeat finds
  the row under either origin, before and after the operator moves it, and
  takes a website on the old one. The directory then shows the new origin.
- A ticket asked for each official origin is redeemed under the set, and
  its lease is checked under the set and under its own origin, but not under
  the other one alone.
- Terms accepted under the old address count under the set, and not for a
  lease checked under the new address alone.
- A pending token gateway admits a player and beats, unlisted, and is listed
  once approved. A disabled one's token neither authenticates nor beats.

No migration, no grant change, no dependency change. Engineering review, not
an audit.

## The legacy store's sign-in, 30 October 2026

Owner's decision of 30.10.2026: Central takes the official gateway's old
accounts' sign-in and move over, and the gateway loses its password routes
([native login](native-login.md#legacy-login)). This is the one place Central
verifies a password itself.

Design pass: assets are the six remaining Argon2id verifiers, their source
snapshots and the accounts' names. The store is its own schema with its own
login role; the runtime role is closed out of it and it out of everything
else, both checked at startup. Only the gateway's parameters exist (m=19456,
t=2, p=1, 32-byte output, 16-byte salt), kept as fixed-length bytes, so Central
parses no PHC string and a row chooses nothing. Every refusal before the proof
(unknown, locked-out, closed, registered name, wrong password) is one
`Rejected` after one derivation and one failure statement; a forced change or
ban is refused after the proof. The bridge's legacy operations are refused
once the store exists, and the bridge's admission of an open move is refused
at all times, so a `legacy_id` handed to the official gateway means a closed
move. Rows are deleted at the close or after the deadline, never marked; the
import reserves each name at Central so nobody registers it meanwhile.

Implementation pass: `LegacyPassword` is the only password wire type, without
`Debug`, wiped on drop, native listener only, and its sign-in is not retried by
the client's transport on `Busy`. The derivation runs on the worker semaphore
over a caller-owned buffer wiped on success, error and unwind; argon2's own
H0 and final blocks are wiped by argon2 itself, its `zeroize` feature on for
the OPAQUE stretch too. The comparison is
`subtle`'s. Limits: the global request budget, 30 legacy sign-ins a minute in
all and five per name, ten failures lock an account for an hour. The import
runs under the migrator in one transaction, refuses the whole batch for any
bad row, a taken name, a closed move, a non-empty store or a count or digest
mismatch, and recomputes count and digest from the stored rows; the snapshot's
SHA-256 is a constraint. The grant block is conditional on the role existing,
so a deployment without it fails only the legacy store's own role check.

Adversarial pass (PostgreSQL fixture, `tests/support/accounts/legacy_store.rs`;
real HTTP, `central-client/tests/support/legacy.rs`): each wider privilege of
either role and a missing one refuse startup, and the statements fail under
the real roles; each malformed field, a duplicate, a count and a digest
mismatch, a taken name and a closed move refuse the import and store nothing;
the lockout's tenth failure, a locked account read as unknown, the hour
passing; the deadline (malformed dates refused, rows hidden at once, the sweep
deleting rows and unenrolled reservations); the sweep of a closed move whose
deletion was lost. Over HTTP: the good path from discovery to the closed move
and the official gateway's admission with the legacy id; a wrong password; a
replayed enrollment ticket; a second legacy sign-in after enrollment
(`Conflict`); the close before restore and before the terms; a wrong digest;
the close repeated; export and sign-in after the close; the bridge's own legacy
operations refused; the bridge's admission of an open move refused and then
granted after the close; the per-name budget (`Busy`, even for the right
password); a locked-out account refusing its right password; a ban and a
forced change refusing theirs; a Central-only account and an unknown name on
the legacy path, and a registered account's session refused export and close;
an empty password refused before lookup; the administrator's listing refused
to a non-administrator. The gateway's own hash is reproduced bit for bit
(`central-crypto` test vector, also checked with OpenSSL's Argon2id). An
independent agent reviewed the `central-crypto` change for the bigint scope
([bigint advisory](crypto-proposal/bigint-advisory.md)). Known limits: timing
differs between a proof and a refusal by the success path's statements; the
lockout lets anyone who knows a legacy username lock it for an hour at a time;
the store's verifiers, like the gateway's database before, can be attacked
offline from a copy. Engineering review, not an audit.

## Pairing a signed-in website to Baylee's client, 30 October 2026

The owner's decision of 30.10.2026 (Baylee `docs/proposals/deep-links.md`
§"Protocol specification"; here [web accounts](web-accounts.md) §"Pairing"
and [the protocol](crypto-protocol.md) §"Derived from the master key" and
§"Pairing").

Design pass: the assets are the master key, the account's native session and
the key check. Callers: the page with its cookie, who may be the owner or a
cookie thief; whatever local program receives the `baylee://pair` link, which
on Windows and Linux can be another application that registered the scheme
(the named attacker); anyone reaching the native listener; and Central
itself, which must never learn the master key. Invariants: Central sees
neither the link's secret nor the master key; a native session leaves
Central only together with the sealed key, once, for the verifier whose
challenge was claimed; a cookie alone cannot approve, since the proof needs
the key check, which needs the master key; a stolen session cannot plant a
check (two minutes from a password proof, never overwritten); a paired
session's 12 hours count from the page's password proof. A hijacking
application that claims first makes the real client's claim a `Conflict`,
so the real client shows an error instead of a code, and the hijacker's code
on the page matches nothing on the person's screen. Accepted limits, decided
by the PM: a reader of the same user's memory is out of scope; approval is one
click beside a comparison code read at a glance; the claim, polls and redeem
carry no session and are bounded by the handle's 32 random bytes, 300 polls
and five minutes.

Implementation pass: migration 000016 adds `accounts.key_check`,
`sessions.proven_at` (NULL meaning the creation, so every existing insert
stays valid) and `pairings`, keyed by the handle's digest, deleted with the
browser session that started it, at most five minutes long and 300 polls.
The store starts at most three per account and a thousand in all after
sweeping ended ones; claims once within two minutes; shows a claim and takes
an approval only from the starting session, once; and deletes the pairing on
every redeem, so a wrong verifier ends it. A redeem checks the challenge, the
epoch and that the proof leaves at least a minute, then issues a native
session in place of the account's last one, resting on the page's proof. The
identity service checks exact lengths and the approval proof against the
stored check before the seal is stored. The browser listener serves the
page's four requests and the key check's two, the native listener the
client's three, under a budget window of their own so polling cannot spend
the sign-ins'. A reset to a new vault clears the check (found while writing
the store: the old vault's check would otherwise have outlived its key). The
SDK opens the seal under its binding before it keeps a session and ends a
session whose seal does not open.

Adversarial pass: `tests/support/accounts/pairing.rs` runs every refusal
against PostgreSQL: no check, no pairing; a second check, a late one, one
without a session; a native bearer starting; a fourth open pairing; another
account's cookie reading, approving or cancelling; a second claim (the
hijacking application); a second approval; a wrong verifier, then the right
one; a second redeem; another epoch between approval and redeem; a proof too
old; a late claim and an ended pairing; and that signing out ends the page's
pairings. The listener lists name every one of the 59 operations (62 since
the merge with the legacy store, whose sign-in, export and link they had not
named). The crypto
tests are those of the two commits before this one.

Evidence on macOS, 30.10.2026: `cargo xtask gate` (format, policy, crypto
scope, clippy, tests, doc tests, release build), `cargo xtask postgres`,
`smoke`, `secrets` (tree and history), `cargo deny` and `cargo audit` 0.22.2
pass. `cargo xtask wasm --check` builds on Linux x86-64 only and did not run
here; `central-web-crypto` depends on `central-crypto`, so the module takes
`hpke` and `curve25519-dalek` 5 too (corrected after the review below; this
sentence first called it a development dependency). The first PostgreSQL run
found the migration count still at 13 and an ambiguous column in the test's
own helper, both fixed. With the
store made to keep a pairing after a wrong verifier, the test fails at
"ended by the wrong verifier". Engineering review, not an audit; a second
model reviews these three commits before they land.

Second-model review, 30.10.2026 (read-only, of `c41/pairing` at 6a6f99f): no
critical or high finding. Fixed in the commits after it:
- The service and the SDK had no attack tests of their own. Now
  `central-client/tests/support/pairing.rs` runs the page's half as the
  website's crypto does and the client's through the SDK, over both real
  listeners: the key check set once after a password sign-in and never
  replaced by the cookie's holder; a proof under another check, a proof over
  another seal, a seal or proof one byte short, another handle and a second
  approval refused; a resent claim answered the same, another one a
  conflict; a verifier of the wrong length refused, a wrong one ending the
  pairing; the redeem replacing the account's earlier native session; and a
  seal to a stranger's key or bound to another expiry, which Central accepts
  with a valid proof, refused by the SDK, which signs the session out, so the
  account keeps no native session.
- A redeem refused after the deletion (another epoch, a proof too old) rolled
  the deletion back. The session's issue now runs in a savepoint: a refusal
  undoes the issue and keeps the deletion, and only a storage failure leaves
  the pairing. The fixture checks that the pairing is gone and the account's
  native session stays.
- Step-ups and "signed in at" counted from a session's creation, which for a
  paired session is the redeem: `AccountExport` would have taken a pairing
  as a fresh password sign-in for 30 minutes. Every such check now reads
  `COALESCE(proven_at, created_at)`, as the key check and renewal did.
- A claim resent after a lost answer was a conflict, the hijack's signal. The
  same triple, compared in constant time, now gets the same answer.
- The redeem's challenge is compared in constant time; `KeyCheck` no longer
  derives equality.
- Expired pairings went only with the next start; the operator's daily
  sweep deletes them too.
- The redeem answers the account's display name and tag beside its username,
  so the client's question can name the account as other players see it.
Recorded, not changed: the native listener's one window for claims, polls and
redeems and the thousand open pairings are global, as its other budgets are
(availability only); the migration number left 000015 to the legacy store,
which landed first. Open: the page's MAC check, which lives in
`central-web-crypto` and is reviewed when it is built.

The review's one medium finding about the design, decided by the owner the
same evening ("Yes, PSK mode"): in HPKE's base mode Central, or anyone
writing its database, knows everything a seal binds and the client's public
key, so it could seal a master key of its own that the client would open
and then run under, writing its local vault wrapper and new objects under a
key Central knows. The seal is now RFC 9180 PSK mode with the link's secret
`S` as the pre-shared key and the handle as its ID. Central never sees `S`,
so a seal it makes does not open; the client keeps `S` from its claim to
the redeem and wipes it once the seal was tried, and with the pairing when
it gives up. Tests: RFC 9180 Appendix A.2.2 (this suite in PSK mode) whole;
a seal under another secret, one made in base mode with everything Central
knows and one under a guessed secret do not open; over both listeners, a
seal approved with a valid proof but made without the link's secret is
refused by the SDK, which signs the session out. The page must seal under
the same `S` it checks the claim's MAC with (`central-web-crypto`, not built
yet).

## The website's polish: keyboard, focus, policy and passwords, 31 October 2026

Design pass. The owner asked for a clearer, faster, more accessible site with
more conveniences, in the order security, performance, clean code. Assets on
the page: the secrets shown once (a gateway token, a reset code, a recovery
code), the session, and what an injected script could reach. Three read-only
reviews (security, accessibility and keyboard, performance) and a design
review fed one list; the server findings (sign-in budgets, format characters
in names) are a change of their own to Central, not this one.

Implementation pass:
- **Policy.** `connect-src` names GitHub only at Baylee's repository
  (`api.github.com/repos/AceVik/baylee` and below, `raw.githubusercontent.com/
  AceVik/baylee/`), so a script that got in cannot post to another repository
  or gist through it; `img-src` drops `data:`, which nothing used. The local
  server sends `Permissions-Policy` (no camera, microphone, location, payment
  or device access) and `Cross-Origin-Opener-Policy: same-origin`; the
  README's Caddy block names both. `test/csp.test.ts` holds the policy.
- **Links from GitHub.** A CI run's and a commit's page become an `href` only
  under `https://github.com/`, as releases already did.
- **Passwords at registration.** A password guessers try first is refused in
  the page (`guessable`: one or two characters repeated, one run, the
  username, a common word with digits or marks around it), the blocklist
  NIST SP 800-63B §5.1.1.2 asks for; the strength meter stays a hint and
  there is still no composition rule. Central cannot check it (OPAQUE: the
  password never leaves the page), so a client that skips the page skips the
  check too.
- **Showing a password** turns the field to text with spellcheck and
  autocorrect off, so it never goes to a spelling service; it hides again
  once the field is emptied.
- **Secrets shown once.** A dialog the browser closes by itself (Chromium's
  second Escape skips `cancel`) asks its owner, and one that keeps it open
  (the token, the reset code) opens again instead of losing the secret.
- **Sign-in.** The crypto Worker starts while Central looks the name up; it
  receives the password only after the lookup and is closed on every way out
  (the flow tests count opened and closed Workers).

Adversarial pass (`npm run check`: 299 unit tests; `npm run e2e` under the
production CSP at both widths, now with every page of the account area and
the administration signed in): policy directives exact; `javascript:`,
`data:`, `http:` and look-alike hosts refused as GitHub links; the blocklist
both ways (refused patterns, accepted passphrases and non-Latin passwords);
the colour switch's stored value only `light` or `dark`. Checked and not
changed: the recovery file names the sign-in name beside the code, but a
password reset needs an administrator's reset code as well
(`account-reset.md`), so the file alone does not reset a password; clearing
the clipboard after a secret was copied would also clear whatever the person
copied next, so it stays theirs. Trusted Types need a browser run against
the WebAssembly glue and wait. Engineering review, not an audit.

## Reports in Central, 31 October 2026

Design pass ([reports](reports.md) passes 1 to 3; the owner's go on
31.10.2026, with "the feedback traffic should be sent gzipped"). Assets: what
players write and attach (their words, the client's details with the
window's picture, a game's record), the reporter's pseudonymity, and the
administrators' decisions. Callers: a gateway with the bridge's credential
or an approved gateway's `sso` token (a pending one files nothing, so a
stranger cannot use Central as a dropbox), and an administrator's browser
session. Invariants: no column joins a report to an account; the heavy
parts are gzip that Central never unpacks (no decompressor, no bomb, no new
crate), checked against the announced length and SHA-256 in constant time;
a stored report's words and parts are never rewritten (the runtime holds no
such grant); every triage change is a line of the report's history.

Implementation pass:
- The intake is bounded JSON (`deny_unknown_fields`, 64 KiB, every field's
  bounds, no invisible formatting characters in what another person
  reads), within the public budget and 120 reports a gateway an hour.
  `direct` is the bridge's alone.
- A part's upload: credential and announcement first, then the declared
  length must be the announced one, then one of two slots, then exactly
  that many bytes within 60 s; written once (`Conflict` again), within an
  hour of its report.
- Triage: the filter is typed and becomes fixed SQL with numbered values;
  the administrator is checked in each statement's transaction; the
  feedback service's rules (revision `Conflict`, one duplicate level,
  assignees who are administrators); a purge needs a fresh password and
  releases the duplicates of what goes; the sweep runs before every list and
  in the operator's daily sweep.
- The browser's part route has every proof of `POST /api/v1`, an
  administrator's cookie and a budget of its own; it answers the gzip as
  stored, which the page unpacks with a bound.

Adversarial pass: the tests named in [reports](reports.md) §"Pass 3", among
them a pending gateway's token, another gateway's report, a forged hash, a
wrong length and wrong magic, a part twice, a stranger's session on every
read, a stale session's purge, a second duplicate level, another admin's
note, the runtime trying to rewrite a report's words, a part or the history,
and each wider or missing grant refusing startup. Not covered here: the
website's section (its own pull request) and the gateways' switch (Baylee,
landed by the PM). Engineering review, not an audit.

## The reports' one-time copy, 31 October 2026

`--import-reports-stdin` writes reports the intake never saw, so it holds
them to the intake's own check (`ReportIntake::validate`) and the triage's
rules (status open, labels, issue) before one row is written, under the
migration login and in one transaction. Its input is bounded line by line
(two record sizes as hex and a margin), the reports to 100 000, and the copy
must end with a count and SHA-256 over its lines, so a truncated or padded
copy stores nothing. The migration document's password is zeroized after
the connection is made; the operator's command reads the feedback service's
database address from its environment file, never from an argument. The
gzip is hex: no new decoder or dependency, and Central still never unpacks
it. Tests: the hex reader's bounds, the documents read no further than
their line, a line as a report with its parts, and in PostgreSQL the
refusals (no administrator, a count or digest that is not the lines', a
label outside the rule) storing nothing, a copy stored whole with its parts,
notes and history, and a second run skipping it. Engineering review, not an
audit.

## Sign-in capacity and invisible characters in names, 31 October 2026

Design pass. A read-only review of the website and the browser surface (the
website's polish, the entry before) found two server matters. The table of
unfinished OPAQUE exchanges (sign-ins, password changes, deletions) held 128
entries for 60 seconds, shared by both listeners, while the public budget
lets 32 requests a second through: about two `LoginStart`s a second with
made-up, well-formed identities (OPAQUE answers an unknown one with a
stand-in record, so each takes a slot) kept every real sign-in, and every
password check before a token mint or an administrator's change, at
`Busy`. And a gateway's or a token's name could hold invisible characters
that change how the text reads (a right-to-left override, isolates, marks):
cards draw names in `<bdi>`, but a name also stands in sentences such as an
administrator's "Approve …?", where an override could make it read as
another.

Implementation pass:
- **Capacity from the budget.** `limits::PENDING` is 4096, more than
  `GLOBAL_PER_SECOND` (32) × `PENDING_SECONDS` (60) = 1920, which a constant
  assertion holds; the three places that insert, and the prune, use it, and
  the lifetime is the same constant. No sequence of public requests fills the
  table before it spends the budget everyone shares (the documented limit);
  a password change or a deletion needs a signed-in account and its eight
  units a minute. An entry is about a kilobyte.
- **Names.** `central_api::gateway::shown_char` refuses control characters
  and U+061C, U+200B, U+200E–F, U+202A–E, U+2028–9, U+2060–4, U+2066–9 and
  U+FEFF; the zero-width joiner and non-joiner stay (emoji sequences, some
  scripts). It is the rule of `GatewayMetadata::validate` (registration and
  every heartbeat) and of `token_name`. Strictly narrower: a gateway built on
  the old rule sends nothing the new one would need, and one that sent such a
  name has its heartbeat refused. Rows stored before keep their name; the
  website's directory parser keeps the old rule, so one old row cannot fail
  the whole list.
- The documents gave the browser cookie `Max-Age=1800`; the code sends
  43200, the session's 12-hour cap (`web-accounts.md`, `operations.md`).

Adversarial pass: unit tests refuse each class (override, isolate, mark,
invisible space, line separator, byte order mark, control) in a gateway name
and a token name, and accept German, Japanese and Arabic names and an emoji
sequence with its joiners. `cargo xtask gate` and `cargo xtask postgres`.

Recorded, not changed, for the owner: the per-account budget is keyed by the
account alone, so whoever knows an account's identity can keep its sign-ins
busy, and `Discover` says whether a username is enrolled (`native-login.md`
§"Implementation status"). Keying the budget by account and address instead
would end that lockout but let many addresses guess faster; a stable fake
identity for unknown names (derived with a server key) would end the
enumeration but needs its own design with the legacy discovery. The native
listener still sees one address behind the proxy. Engineering review, not an
audit.

## The administration's overview and named audit pages, 31 October 2026

Design pass. Two reads for the administration's pages, no change: counts for
a first view (`AdminRequest::Overview`) and today's names beside a page of
the audit log. Assets: what an administrator may read about accounts and
gateways, which the existing reads already show row by row. Caller trust:
only an administrator's live browser session, checked in the read's own
transaction like every other read; the native listener refuses `Admin`
before anything else. The record itself does not change: it keeps ids; the
names are joined when it is read and never stored with it.

Implementation pass:
- `Overview` is one statement of counts (`FILTER` aggregates, one pass over
  `accounts`, `account_roles`, `gateways` with their approvals, and the live
  `gateway_tokens`); no row, name or id leaves it. The official gateway is
  not counted among the registered ones. A read, so `changes()` says false
  and no fresh password is asked.
- `Audit` joins the actor's and the subject's accounts and the gateway in
  the statement that reads the events, so names and events agree; the answer
  lists each named account (id, display name, tag) and gateway (id, name,
  origin) once, only those that still exist. Display names and tags are what
  `Accounts` already shows; no username, credential or note is read.
- The website parses both answers by their exact keys and bounds (a page's
  names at most two accounts and one gateway per event), asks for the counts
  only once the summary says administrator, and draws names as text in
  `<bdi>`. Pages: `/admin` is the overview, the list of accounts moved to
  `/admin/users`.
- The owner's and the administrator's forms check a new gateway's or token's
  name against Central's new rule (`shownName`, the mirror of
  `central_api::gateway::shown_char`); names read back keep the older rule.

Adversarial pass: the PostgreSQL fixture compares every count with the
tables, refuses the overview without an administrator's session, checks
that a page names exactly the accounts its events name, each once, and that
the operator's event names only its subject; the client SDK's test reads the
names of an administrator's change over the browser listener; the wire tests
refuse an unknown field in the overview and in a named audit page. The
website: page tests with stand-ins, and the e2e mock's answers in the exact
shapes. Engineering review, not an audit.

## The administration's feedback section, 31 October 2026

Design pass. The website's side of bug and crash reports
([reports](reports.md) §"The website's section"): `/admin/feedback` and a
page per report, every triage request of `AdminRequest`, and the two raw
part reads (`POST /api/v1/reports/{id}/client|record`). Assets: the
players' words and pseudonyms, the client's details (a screenshot, the
table, the log), a game's record, and the administrators' notes. Caller
trust: an administrator's browser session, which Central checks on every
read and change; the page asks only once the summary says administrator.
Untrusted input: everything a report holds, including what a gateway says
of itself, and the client part's gzip, which a modified client can shape.
Not built here: Central's routes and their budgets (reviewed with #58).

Implementation pass:
- **Text only.** No report field, note, history line, gateway string or
  client value is drawn as HTML; names are `<bdi>`, ids and versions
  `<code>`, the player's words a text node (`dir="auto"`). A card the text
  names is a link built from its Scryfall id alone (checked as a UUID), its
  picture from `cards.scryfall.io` with `referrerPolicy="no-referrer"`.
- **Shapes.** Every answer is parsed by its exact keys and central-api's
  bounds (`admin/feedback/api.ts`); a filter is sent with its set parts only
  and mirrors `ReportFilter::validate`, so an address with a bad part drops
  that part instead of sending it.
- **The client part.** At most 2 MiB of gzip are read (the body counted
  while it streams, `Content-Length` checked first, gzip's magic required),
  inflated through `DecompressionStream` and stopped past 16 MiB of output
  while it streams (`MAX_CLIENT_JSON`, the design's bound), decoded as
  strict UTF-8 and only then parsed. The JSON tree draws children only when
  unfolded, 200 at a time, at most 32 levels deep.
- **The screenshot.** Shown only when its base64 decodes to a PNG by its
  eight-byte signature and an `IHDR` of 1 to 16,384 pixels a side; the
  bytes become a `Blob` shown through `URL.createObjectURL`, revoked when
  the view goes. `img-src` gains `blob:` for this and keeps `data:` out
  (`scripts/csp.ts`, `test/csp.test.ts`).
- **The record.** Never unpacked in the page: read with a 64 MiB bound and
  handed to the browser as the `.gz` Central keeps; a client's record says
  it is unverified.
- **GitHub.** An issue link is `ISSUES/<n>` for Central's number (1 to
  10,000,000); a new issue's form carries only the administrator's own
  summary, a neutral category and a build that reads as a version.
- **CSV.** Made in the browser; a cell starting with `=`, `+`, `-`, `@`, a
  tab or a carriage return gets an apostrophe in front.
- **Reauthentication.** Only a purge (one report's or a selection's) runs
  through `useChange`, so a stale sign-in asks for the password first;
  triage, notes, the trash, views and the unread mark go straight, as
  `AdminRequest::fresh` rules.
- **Requests.** The search waits for a pause in typing; facets are read once
  per filter; the views' counts are two reads; the quiet refresh runs every
  30 seconds only while the page is in view; the drawer reads a client part
  only once the reader stops on a report.

Adversarial pass: unit tests refuse answers with an unknown field, a bad
label, signature, id or word, a control character in an excerpt and an
oversized part; a gzip bomb stops at its bound; a GIF and broken base64 are
no screenshot; a CSV cell starting a formula is defused; a pasted issue
from another repository is refused. The e2e mock refuses any report
request with a field too many or too few and checks the part route's
headers and empty body; a spec shows the screenshot through `blob:` under
the production policy with no console error. Engineering review, not an
audit.

## Account-owned chat and groups, 2026-11-02

Design pass: personal relationships and ciphertext delivery belong to Central's
account boundary, independent of gateway bearer credentials. Preserve the native
wire crypto and make private key ownership explicit. Server-visible membership
metadata is not claimed encrypted. Exact recipient sets, durable replay, session
revocation, epoch reset, TOFU changes and bounded resource consumption are invariants.

Implementation pass: ordered account locks serialize sends and contact transitions;
group locks bind sends to committed membership. Native/browser session kinds are
checked separately. No private key, plaintext message or bearer token is logged.
The owner approved the exact Ed25519 additions and curve digest feature. Imported
crypto sources, zeroization features, source fingerprints and graph boundaries were
reviewed. First-party unsafe remains forbidden; accepted dependency internals are
not claimed unsafe-free. Tag reassignment after chat enrollment is refused because
signed device statements and historical conversation IDs bind the public tag.

Adversarial pass: real PostgreSQL tests exercise forged registration, foreign inbox,
wrong recipient sets, duplicate IDs, concurrent sends, blocks, epoch rotation,
private invites, owner-only actions, simultaneous leave/send, and membership quotas.
SDK tests check sticky key-change barriers across rollback/restart and injected
own-account keys. Browser regressions cover stale account state, cookie-only
transport, response bounds, acknowledgement identity and worker teardown. Full
Rust gate passed including the group backend; final native transport and browser
integration with the canonical Linux WASM build also pass. See central-chat.md for
the precise protocol, limitations and outstanding release checks.


Integration follow-up, 2026-11-02: the sibling's real migration fixture exposed
that the closed-move admission predicate also denied purpose-bound Export/Link
tickets needed before the move can close. The store now exempts only those two
nondelegating purposes. Design review retained the closed-move Admit gate,
official-bridge authority, audience/session binding and one-use deletion. The
PostgreSQL regression checks open-move transfer, no delegated lease, wrong audience,
wrong purpose, registered-gateway refusal and replay, alongside the existing
refused-before-close and accepted-after-close admission test.

## Browser chat and notifications — 2026-11-02

Design pass: personal identity, ciphertext delivery and contacts belong to Central;
game chat stays scoped to its gateway. Private keys stay in the unlocked worker;
rendered plaintext stays in volatile page memory. Account generations invalidate
delayed results, and trust changes need explicit comparison. Push is opt-in and separate from the encrypted chat
transport: only generic category hints, explicit device ownership, current roles,
no arbitrary webhook destination, no private API caching and no automatic reload.

Implementation pass: checked state and cookie races, group membership immediately
before sealing/retrying, bounded queues and device caps, worker termination, exact
ciphertext retry, strict input/URL parsing, DNS filtering, disabled redirects/proxies,
credential revision, session-bound logout, atomic preference bits and unique outbox
leases. Fixed a late acknowledgement releasing its successor's lease, stale group
membership sealing for an incomplete recipient set, and profile state surviving an
account switch. The first push dependency graph was rejected because its old P-256
made the bigint advisory reachable; the selected AWS-LC provider has no such edge.

Adversarial pass: real PostgreSQL 18.6 and TLS browser/native integration, RFC crypto
vectors and tampering, concurrent delivery/lease replacement, ownership theft,
revoked admin/credentials, offline opt-out and worker payload/cache tests. Rust gate,
smoke, secret scanner, cargo-audit 0.22.2 and cargo-deny 0.20.2 pass. Browser/native
interop and desktop/phone chat plus persisted preference changes pass against the
actual service using the final canonical Linux x86-64 WASM artifact. Its rebuild and
byte comparison passed in an isolated VM with the pinned toolchain; the VM and
all owned fixtures were cleaned up. The final web suite has 365 passing tests.
Real external push and platform-specific installation remain staging checks. These are engineering reviews, not an independent
audit or a production rollout approval.


PWA activation follow-up, 2026-11-02: browser permission is not evidence of a live
subscription. The offer now handles reauthentication after logout, ignores the old
permanent prompt dismissal, observes local/cross-tab installation changes, and
respects deliberate opt-out within the current page's authentication generation.
A failed/expired subscription clears its stale local marker. Installation is offered
only after a browser installability event; each prompt is consumed once by a click,
with bounded component lifetime and handled rejection. Regression tests cover these
transitions and no automatic permission/install prompt. Real desktop/phone worker
push delivery and the public-only offline cache still pass with the canonical WASM.
The owner's permission to reset existing live messages/contact links is recorded in
central-chat.md; this follow-up executes no production deletion.

## Integration review after the administration merge, 2026-11-02

Design pass: retain main's administration overview and account panels while adding
chat, contacts and push. Authentication generations must also isolate the new
overview's asynchronous results and navigation badges.

Implementation pass: rebased the seven feature milestones onto main, preserving
both sets of routes and the generation-bound terms acceptance. Adapted the reports
destination to the shared administration API. Found that a delayed overview read
could repopulate counts after a different sign-in or overwrite a newer refresh.
Counts now carry their authentication generation, with lifetime and request-order
checks for success and failure. Separate Playwright output directories prevent
the ordinary and real-service suites from removing one another's trace files.

Adversarial pass: four regression tests cover pending/loaded counts crossing a
sign-in and stale success/failure after a newer refresh. All four fail against the
previous implementation and pass with the correction. The combined web check
passes 373 tests, type checking, lint and production build. The rebased Rust gate,
real PostgreSQL fixture, smoke, secrets, pinned audit and deny checks pass. Crypto
sources, dependency graph and the canonical Linux WASM artifacts are unchanged.
This is an engineering review; production rollout and external push delivery
remain separate checks.

Final lifecycle review found a further issue: chat unlock performs a verified
browser sign-in, replacing the cookie, while push subscriptions retained the old
session digest. A later server-side logout could therefore miss that installation
if the browser's separate unsubscribe failed. Browser login now atomically
rebinds only subscriptions matching the verified account, credential revision and
presented old browser-cookie digest. No cookie alone authorizes reauthentication;
foreign-account and other-device consent remain separate. Regression coverage uses
real PostgreSQL for device/account/session-kind boundaries and the actual HTTP
listener with OPAQUE reauthentication and server-only logout.
The HTTP regression also exposed the browser dispatch dropping that cookie before
verification; it now forwards it as consent context while keeping the anonymous
login budget. The regression failed before the complete fix and passes afterward.
Both real PostgreSQL suites, the full Rust gate, smoke and secret checks pass on
the corrected implementation. No dependency or cryptographic protocol changed.

The final CI then exposed a pre-existing randomized scanner-control failure:
Gitleaks 8.30.1's generic-key allowlist ignores substrings such as `dead` and
`feed`, which can occur in random hexadecimal canaries. A local batch reproduced
misses in 4 of 2,000 synthetic values. The control now uses the pinned dedicated
GitHub-PAT rule with a balanced prefix and an OS-random suffix. Its 18 adversarial
controls include every repeated hex digit and both stopwords; a separate batch of
2,016 values was fully detected. Working-tree path exclusions and complete-history
scanning are unchanged, and neither scanner configuration gains an exemption.
The actual scanner, including these controls and both repository scans, passes.
Source: [pinned Gitleaks rules](https://github.com/gitleaks/gitleaks/blob/v8.30.1/config/gitleaks.toml).

## The chat unlocked by the sign-in, 3 November 2026

Design pass. The owner asked that a signed-in page's chat work without asking
for the password again. Assets: the chat identity derived from the master key,
the plaintext the page shows, the password. Caller trust: the page's own sign-in
(OPAQUE, `__Host-central`). Options weighed: keeping a derived key at rest in the
browser (localStorage, IndexedDB, also under a non-extractable WebCrypto key)
would leave the chat key readable to whoever holds the browser profile after
sign-out, and `docs/central-chat.md` keeps only encrypted peer trust at rest; a
key split with Central needs a new protocol operation and a WebAssembly rebuild.
Chosen: the sign-in's Worker, which opens the master key anyway, keeps the chat
identity instead of discarding it (`chat_unlock` in place of `login_confirm`),
and the page keeps that Worker for the sign-in's life. No new protocol, no new
stored secret, the committed WebAssembly unchanged. Invariant: the chat is
unlocked only by a password proven in this page, and closed when the sign-in
ends; a reload asks once. (Superseded the same day for reloads by the owner's
decision below: the identity is now kept, sealed, while the sign-in lasts.)

Implementation pass:
- `chat_unlock` consumes the sign-in exactly as `login_confirm` does and fails
  the same way on a wrapper that does not open; the sign-in then fails as
  `mismatch` and ends the new session. Accounts without a recovery code keep
  `recovery_create`; an account without a tag keeps `login_confirm`.
- The Worker leaves the flow only after `chat_unlock` answered; the flow's
  `close` then no longer terminates it, and nothing of the sign-in's protocol
  reaches it again (`account/crypto-worker.ts` answers the account protocol by
  `op` and the chat's by `id`; the chat refuses the sign-in steps).
- `chat/held.ts` keeps one chat, by account: another account's is never handed
  out; another account's sign-in closes it; a same-account reauthentication
  keeps the open one and closes the new Worker. `AccountProvider` locks it when
  the session ends for any reason, including a sign-out in another tab.
- The peer trust is restored as its own step after the unlock: a damaged
  record fails the chat's start closed (no registration, no inbox, no seal)
  without failing the sign-in, and a retry does not restore twice.
- Before its start the session neither sends nor reads. The five-minute hidden
  lock and the page-hide lock are gone: the keys live as long as the page and
  its session, which a reader of the page's memory holds anyway.
- The held chat keeps types only from the chat, so the first page's bundle
  does not grow by the chat's code.

Adversarial pass: unit tests for the Worker handover (closing the flow leaves
the chat's Worker, closing the chat ends it), the sign-in keeping a chat only
on success, the same-account and other-account cases, the start without a
password (restore, register, own keys, inbox), damaged trust, Central's refusal
at the start, a tab hidden for an hour, and the unread state. The e2e mock now
answers the chat and the groups in their exact shapes with a stand-in Worker:
a sign-in reaches the chat with no password field, a message goes out and its
answer comes back, the contacts tab answers a request, and after a reload a
wrong password is refused before the right one unlocks. Not run here: the real
WebAssembly against a real Central (its `chat_unlock` and steps are the same
exports the chat page used before). Engineering review, not an audit.

## Review of the website overhaul (PR #62), 3 November 2026

A review of the overhaul's commit before it reaches main. What it changed:

- **No unverified endorsements.** The start page's gateway showcase was a
  hard-coded list: three gateways on domains this project does not control,
  with invented player counts, links to them and `baylee://gateway/connect`
  links that the client does not handle. Anyone registering one of those
  names would have had the official site send players there. The section
  now shows the first gateways of Central's own directory through the
  directory's card (an "Open in Baylee" link only behind the deep-link
  switch), and `gatewayConnectLink` is gone. The self-hosting snippet no
  longer names two environment variables no gateway reads.
- **Nothing made up.** The status strip showed constant games and users, a
  per-browser view counter, a "registered" figure computed from the
  constants and an invented player list, and the footer and strip claimed
  GDPR compliance. The strip now shows only what a source says (the release
  and CI from GitHub, the gateways online and not from Central's
  directory); the claims are removed (AGENTS.md: no compliance claim
  without evidence).
- **No half protocol.** The commit added `GatewayStats` to the heartbeat and
  the directory listing of `central-api`, the crate the client links: player
  handles with their presence for a public listing, "opted-in" asserted in a
  comment without a consent mechanism, counts without bounds, accepted under
  `Private`, validated and then dropped (never stored nor served), without
  an entry in the gateway registry's documentation, and a `deny_unknown_fields`
  break for older clients the day it would be filled. The crates are back to
  main's; the feature can return as its own change with storage, consent,
  bounds, documentation and a compatibility plan. The page's mirror of it
  (`validGatewayStats`, the card's counts) went with it.
- Smaller: the download and language lists give the keyboard back to their
  switch on Escape and a choice, close when the keyboard leaves them and
  name what they control; the download's live region is its line of facts,
  not the list; leaving an anchor on the same page goes back to the top;
  the feedback screenshot's `blob:` address is made per image as it is
  attached (React's development double attach revoked a shared one); German
  and English sentences written into components became Fluent messages;
  dead code (an unrouted page, an API only its test used, a constant)
  removed.

Checked and left: the directory's parsing and its origin rules, text-only
rendering of gateway strings, the CSV and GitHub guards, the router's click
handling, the anchors the page scrolls to, and the polling's pauses.
Engineering review, not an audit.

## The chat identity kept while signed in, 3 November 2026

Design pass. The owner, the same day: the chat should not have to be unlocked,
not on the website and not in the client; a signed-in account sees its contacts
and chats, full stop. That needs the chat identity across reloads without the
password, so it is now kept in the browser while the sign-in lasts. Options
weighed: raw bytes in `localStorage` (readable by any copy of it); a key split
with Central (a new protocol operation, a migration and a per-session secret
on the server, for little more: message ciphertext is only readable with a
session anyway, and a compromised Central would hold its half); the identity
sealed in IndexedDB under a non-extractable Web Crypto key bound to account,
epoch and tag, forgotten with the session. Chosen: the last. No custom
cryptography (AES-256-GCM from the Web Crypto API; the identity's derivation
unchanged); the module gains `Chat::keep` and `chat_resume`, the identity's
bytes going only from the module to Web Crypto inside the Worker.

Implementation pass:
- `flow::chat_unlock` keeps the 96 bytes beside the chat (`OpenChat`, wiped on
  drop); `flow::chat_resume` takes exactly 96 bytes and a valid tag and
  rebuilds the same identity. The Worker seals them at once with a fresh
  non-extractable key, then overwrites its copy; resume overwrites the opened
  bytes after `chat_resume`.
- The record names its owner and the AAD binds it again: another account,
  epoch or tag, an altered IV or ciphertext open nothing, and such a record is
  deleted. One record per browser; storing replaces it.
- Forgotten on sign-out, on every end of the session the page sees (expiry,
  Central's "no" at renewal or summary, a sign-out in another tab) and at
  start when no session is remembered. The chat page resumes once per mount,
  shows "loading" meanwhile and asks for the password only when nothing opens;
  a password unlock keeps the identity for the next reload.
- The WebAssembly module was rebuilt with `cargo xtask wasm` on x86-64 Linux
  (the pinned Rust image under Rosetta) and checked with `--check`.

Adversarial pass: host tests that a kept identity resumes as the same device
and registration, that the same sign-in derives the same bytes, that other
lengths, tag 0 and another epoch give nothing alike; unit tests for the seal
(non-exportable key, wrong owner, swapped claim, altered ciphertext and IV),
the resume (no password, nothing registered when nothing is kept) and the keep
after a sign-in and after a password; e2e: the chat opens after a reload with
no password field, IndexedDB is empty after sign-out, and with nothing kept the
password opens it once and the next reload needs none. Not run here: a real
Central with the real module end to end. The native client is the client's own
change. Engineering review, not an audit.


## Open registration: the proof of work and the end of the beta keys, 11 November 2026

Design pass. The owner, 10 and 11.11.2026: the site is for the open beta,
the closed beta's keys go, in the backend too, on conditions (the client's
and the gateway's API compatible; a migration that deletes nothing else;
the replacement protection in place before the key stops being one), and
an administrator keeps a simple switch to close registration during an
attack. The replacement is the owner's design of 28.10.2026: a proof of
work issued and checked by Central, per-address budgets with a daily one,
a honeypot and a minimum fill time. Options weighed for the challenge: a
signed stateless token (a token format of our own, excluded by the
project's rule against custom token algorithms) against random nonces held
in memory (bounded, one use); chosen: the latter. The puzzle is the one the
Impressum already uses (SHA-256 over `nonce:part:counter`, leading zero
bits, eight parts), so the page's Worker solves both and nothing new runs in
the browser. A closed registration answers the challenge with a new response
(`RegistrationClosed`) instead of a new error kind, which would have broken
exhaustive matches in the game client.

Implementation pass:
- `central_api::register` checks and solves; `RegisterChallenge` and its
  answer, `Register { nonce, counters }`, `RegistrationClosed`, and the admin
  requests `Registration` and `SetRegistration` are additive to the wire;
  `beta_key` left `Register`, and `central_api::beta_key` went (the gateway's
  own integration tests use it; they need the proof when they update their
  pin, and the game repository's session was told).
- The identity service holds the challenges (at most 10,000, the oldest
  making room, five minutes, one use, three seconds at the soonest) and the
  last hour's registrations for the difficulty (16 bits, one more per
  doubling from 60 an hour, three at most); it spends the proof before the
  store sees the name. Tests may build it with an easy base; the service's
  own launch never does.
- The browser listener's budgets: `Register` 5 an hour and 10 a day,
  `RegisterChallenge` 20 an hour.
- Migration 000023 drops `beta_keys`, `accounts.beta_key_id` and the two
  constraints naming it, re-creates `account_registered_pending` without the
  key, keeps the audit's `beta_key_id` (it never had a foreign key), and
  adds `registration` (one row, open). The runtime role gets `SELECT` and
  `UPDATE (open, changed_at, changed_by)` on it, nothing else, checked at
  startup. `register` reads the row for share in its transaction, the switch
  for update, so a registration in flight finishes before a close and none
  starts after it.
- `--issue-beta-keys-stdin` went; the CLI refuses it as an unknown mode.
- The register page solves the proof in `pow-worker.js`, waits out the
  three seconds, keeps a hidden field for form robots and refuses a form
  sent within three seconds; administrators find the switch on the account
  overview, with a fresh sign-in when theirs is older than 30 minutes.

Adversarial pass: unit tests that a solution counts once, not too soon,
not after five minutes, not for an unknown nonce, that a full table drops
the oldest, that the difficulty rises with each doubling and falls with the
hour; that every part must be solved and a wrong counter fails; the wire's
shapes, a `Register` without the proof and one with the old `beta_key`
refused; the budgets per hour and day and the challenge's own. On PostgreSQL
18: registration open, closed (refused before the name, also for a taken
one), reservations, names, races and the sweep without keys; the switch
read and set by an administrator only, a stale sign-in refused, the same
state writing nothing, each change one audit event; the runtime's exact
privileges on the switch, both ways; the keys' table and column gone, the
audit's column kept; and 000023 over the upgrade chain's data, where every
account, the pending registrations and every audit event (one naming a key)
survive. The client's integration tests register through the real listener
with a proof each. Engineering review, not an audit.

## The sign-in form before the islands run, 11 November 2026

Design pass. The website's sign-in sends Central only OPAQUE messages: the
password goes from the field to the crypto Worker and nowhere else
(`docs/web-accounts.md`). The form is rendered by the server and taken over
by its island once the WebAssembly runs. The islands start after the load,
and since `cb1df03` after the first frame too. Until then, the browser's
own submit applied: `method="post"` to the page's address, with the fields
named `username` and `password`. Pressing Enter that early posted the
password in the request body to `central-web`, which answers every POST
with 405 and reads no body, through Caddy, which logs no bodies. Nothing
stored it, but the password left the Worker's boundary. The contact form
posted its fields to its page the same way, and they were lost. Options
weighed:
- no form element, which would cost the implicit submit and the password
  managers' recognition;
- an `action` the server refuses, which still sends the body;
- holding the submit back until the islands run, and leaving the fields
  without names.

Chosen: the last, both layers.

Implementation pass:
- `boot.js`, the one script that runs before the first paint, cancels a
  submit from a form inside a `<leptos-island>` while `<html>` does not yet
  say `data-islands="ready"`. A form outside an island (the album's
  `method="dialog"`) is untouched.
- The sign-in's and the contact form's fields lose their `name`
  attributes, so even a submit let through sends an empty body. The islands
  read the fields through their own signals, and password managers go by
  `autocomplete`.

Adversarial pass: a browser test blocks the WebAssembly (and the service
worker, which would fetch it past the page's routes), fills the sign-in,
and submits with Enter and by script. It sees no request other than GET or
HEAD, and the page stays where it was. The fields' lack of names is checked
too. With the guard taken out, the test fails. Engineering review, not an
audit.

## The start page in the old site's look, 11 November 2026

Scope: the start page's new sections (screenshots and their dialog, the
gateways with a copy button, what Baylee is, the opponents, the card
numbers, the contact form, the repository card), the GitHub snapshot's new
fields, and the islands starting only near the reader.

Threat-model pass:
- The repository card shows a commit subject, star and issue counts from
  GitHub's API: text from outside, fetched by the server.
- The contact form, an island, is on the start page now, far below the
  first screen; until the islands run it must send nothing.
- Copying a gateway's address writes to the visitor's clipboard.
- The screenshots' recordings and the dialog are built by `home.js`.

Implementation pass:
- `fetch.rs` reads `GET /repos/AceVik/baylee` (same host, agent, timeouts
  and size limit as before) into two integers, and the newest commit's
  message into its first line: control characters dropped, at most 120
  characters. Leptos writes it as a text node inside `<bdi>`, so markup or
  a direction override in a subject stays text and stays in its box.
- The islands' loader waits, after the load and the first frame, for the
  page's largest paint (a second at most), then until an island is within
  a screen of the window or a pointer or the keyboard reaches into one; an
  island without a box, or a page without one, starts at once. The submit
  guard of the previous entry holds the start page's form back until then,
  and the CSP's `form-action 'none'` holds it back anyway.
- Text typed before the islands ran (the password among it) is read from
  the page's own fields into the islands' signals when they start
  (`islands/early.rs`); it stays in the page as typed text does, nothing is
  sent before the form is submitted, and the earlier guard still holds a
  submit back until the islands are ready.
- `boot.js` shows a gateway's copy button only in a secure context with
  `navigator.clipboard.writeText`, and writes only the origin the server
  put on the button (Central's directory checked it), on a click.
- `home.js` draws each screenshot (`<picture>`, `<img>`, `<source>` made
  with `createElement`, their addresses from the server's `data-`
  attributes) once its card comes near, and copies a figure's caption into
  the dialog with `cloneNode`; no markup strings, nothing for a Trusted
  Types policy. Without script a `<noscript>` picture shows the still.
- The built stylesheet loses its comments, blank lines and indentation
  (`lean_css` in `xtask/src/site.rs`, with a test); strings are copied as
  written and a string broken over a line fails the build. The SRI hash
  is taken of what is served.

Adversarial pass: browser tests check that the start page fetches no
WebAssembly until the form comes near and that the form then refuses an
empty message in place; that the copy button copies exactly the listed
origin and says so; and that the screenshot dialog steps and closes with
focus returned. axe, the 44 px targets and the "nothing from elsewhere"
check pass on every page in both colour schemes. Engineering review, not an
audit.

## Connection places, panics and the website server's answers, 11 November 2026

Design pass. A read-only audit made seven claims; each was checked in the
code before anything changed, and all seven held.
- **Connections (medium).** Both listeners drop a new connection once 32
  are open (`Config` default; the deployed launcher never raised it). The
  browser listener read a request's body, for up to the request's five
  seconds, before it looked at a header, its client address or a budget.
  Caddy passes a body on as it arrives unless told to buffer it (assumed
  from its documentation, not tried here), so about seven body-less
  `POST /api/v1` a second (a `Content-Length`, then nothing) held every
  connection: nobody could sign in or chat. The native
  listener checks `Host`, `Origin` and Fetch Metadata before the body and
  an upload's credential before its bytes, but it has no client address
  (forwarded headers confer nothing there), and the same stall holds its
  32 connections for five seconds each.
- **Panics (low).** A connection task that panicked made the accept loop
  return an error, and `listeners::together` stopped the other listener
  too: one reachable panic in any handler ended the service.
- **Website server.** `/cards` (unprefixed) was a 301 chosen by
  `Accept-Language` without `Vary`; `Accept-Encoding` was matched by
  substring (`br;q=0` and `brotli` got Brotli); every page and the
  sitemap deep-cloned the whole snapshot under its lock; every GET
  rendered its page again.
- **Budgets.** `Book::charge` walked all of a budget's (up to 4096)
  addresses under the budgets' one lock on every browser request.

Implementation pass:
- **Browser listener** (`browser.rs`). For `POST /api/v1` and a report's
  part, the loopback peer, the site's exact headers and the client
  address are checked on the headers alone, first. Then the address takes
  one of four places in flight: a table in memory of at most 1024
  addresses, an RAII permit that gives the place back however the request
  ends (answered, refused, timed out, its connection gone), no eviction
  (a full table refuses newcomers). Past four, 429 `Busy` at once, before
  the body, the service or a budget. The body has its own two seconds
  (408), the service its own five after it; a seven-second layer is only
  a backstop. Other paths and methods are answered without reading a
  byte. The deployed browser listener takes 128 connections, the policy's
  most (64 KiB bodies each), so one address holds at most four of them;
  the policy default stays 32 (development; the deployed account listener
  followed the same day, next section). The wire is unchanged: routes,
  statuses and fixed texts.
- **Panics** (`server.rs`). A connection task that panicked is counted
  and reported with a fixed line (no payload, no request data; the
  launcher's panic hook prints none either), and the listener goes on,
  while serving and while draining. A task that ended otherwise
  (cancelled) still stops the listener as before.
- **Budgets.** The caller's own window is checked lazily (passed: it
  starts again); every other passed window is forgotten once a period, or
  when a newcomer finds the book full, before the oldest window makes room
  as before. Every decision is the one the old code made: an address is
  charged against its own window only, and eviction still follows the
  removal of every passed window.
- **Website server** (`central-web-server`). `/cards` is a 302 with
  `Vary: Accept-Language` and `no-cache`, as the proposal's §5 asks for
  every unprefixed address; `/{lang}/cards` stays 301. `Accept-Encoding`
  is parsed (RFC 9110 §12.5.3): tokens without case, a coding's own
  weight else that of `*`, `q=0` or a weight that does not parse refuses,
  `x-gzip` is gzip; the heavier of Brotli and gzip is sent, Brotli on a
  tie. The data is `RwLock<Arc<Snapshot>>`; a request clones the `Arc`,
  and a change installs a new one only when it changes anything. A page
  is rendered once per (page, language, machine) for the snapshot it came
  from; finding another snapshot drops them all, a render that raced a
  replacement is not kept, at most 1024 pages and 32 MiB (a page is about
  20 KB, the start page 70 KB). HTML, the fixed-name files, robots.txt,
  the sitemap, the manifests and the TDM file carry a strong ETag of the
  bytes sent (one per encoding); `If-None-Match` (a list, `*`, weak
  comparison) gets 304 only where the unconditional answer is a 200 with
  that tag, with ETag, Cache-Control, Vary and the security headers and
  without the body's own headers.

Adversarial pass: four stalled bodies from one address hold four places, a
fifth is refused at once while another address is served, and the four
end at the body's deadline; the same over a real socket, with a
connection that goes away mid-body (its place comes back). Places come
back after an answer, a malformed body, a refused operation, an
oversized body, a spent budget and a service that never answers (408 at
its own deadline). A full table refuses newcomers and keeps every place.
With the per-address limit taken out, both stall tests fail. A route that
panics, over a real socket: the next two requests are served, shutdown is
clean and the panic is counted once; a cancelled task still stops the
listener. The existing budget tests pass unchanged, and a new one shows
an address's own passed window starting again between sweeps while
another waits for the next sweep. Rendering is checked to be pure: every
page in both languages for three machines, twice and on another thread,
byte for byte. New data changes the page and its tag and drops the kept
pages, the same data keeps them; with the snapshot check taken out that
test fails. Each encoding case, `brotli` and `br;q=0` among them; one tag
per encoding, a tag confirming only its own form; 304 with the security
headers for pages and the server's documents; another tag, language or
machine gets the page; a 404 or a redirect is never a 304. A refusal
sent before the body is read (429, 403) still arrives whole over a real
socket when the body came with the headers, also one of 60 kB past
hyper's 16 KiB read buffer (macOS loopback). `cargo fmt`, clippy with
`-D warnings` and the tests of `central-http`, `central-web-server`,
`central-policy` and `central-service` pass, and `cargo xtask policy`.
Not run: `cargo xtask gate`, `postgres` (among it `central-client`'s
test of this listener against a real Central, which sends one request
at a time with small bodies), `smoke`, `secrets`, `cargo audit`,
`cargo deny`, and the browser suites.

Recorded, not changed, for the owner:
- 32 addresses (IPv6 /64s are cheap) stalling four bodies each still fill
  the browser listener, each place for two seconds at most. This raises
  the cost; it does not end the attack. Buffering bodies in Caddy would,
  and Caddy's configuration was out of scope.
- The native listener stayed exhaustible by stalled bodies in this
  change (no client address to key places by, and a deadline of two
  seconds would break slow uplinks for its bodies of up to 1 MiB). It
  was closed the same day: the next section.
- A panic no longer stops the service. The identity service's locks fail
  closed (503) when poisoned, so a panic while one is held would leave
  that operation unavailable until a restart, where before the process
  exited. The fixed log line counts panics. The listener's own critical
  sections (budgets, places) cannot panic; the identity service's were
  not all reviewed for this.
- Four places per address is a constant to tune. The islands ask one
  thing when they start (the account summary) and the chat works
  strictly in turn, so a tab rarely has more than one request in flight;
  but an address many people share (a NAT), with a slow service holding
  requests longer, can meet the limit, and a 429 when the account page
  starts shows the busy message until it is reloaded. Every local and
  end-to-end run, and the website's own directory fetch, arrive as
  127.0.0.1 and share its four places; the browser suites were not run.
- A refusal now goes out with the body unread. Hyper reads what has
  arrived when it drops the body; a larger body still on its way when
  the connection closes could reach the proxy as a reset, and the
  visitor would see a 502 in place of the 4xx. The page's bodies are
  small, and the test above passes, but Caddy was not tried.
- Caddy's `encode` may rewrite or weaken the tag of compressed HTML; that
  was not checked here. The server confirms only the tag of the bytes it
  would send now, so a rewritten tag can cost a 304, never serve a stale
  page.

Engineering review, not an audit.

## The account listener's places for late and large bodies, 11 November 2026

Design pass. The section above left the account listener exhaustible: a
`POST` that announces a body and sends none held one of its 32
connections for the request's five seconds, so about seven a second held
them all, and no client or gateway could sign in. Its two obstacles, and
what answers them:
- **No client address.** The listener binds loopback only (the policy
  refuses anything else) and is deployed behind Caddy on the same host,
  which sets `X-Forwarded-For` to the client it saw and, without
  `trusted_proxies`, ignores a client's own (Caddy's documentation; Caddy
  was not run here). So the last hop of a request from a loopback peer is
  the proxy's word. It is used for one thing, to share out places; no
  identity, credential, budget or log line follows from it. A client beyond
  the proxy cannot choose it. A process on the host can send its own and
  so use other addresses' shares, but never more than all addresses
  together may hold (a quarter of the connections), where today it could
  hold every connection. A request without a well-formed hop counts by its
  peer.
- **A short deadline breaks slow uplinks.** No deadline changed. What a
  withheld body may hold is bounded instead, so how long it holds it no
  longer decides anything.
- **Places for whole requests, as on the browser listener, would refuse
  honest bursts**: a gateway makes its calls from one address, many at
  once. So a request whose small body is there with its headers is never
  counted: a body of at most 64 KiB has a quarter of a second to arrive
  free. Only a body that is late, or large (over 64 KiB, or without a
  length), needs a place: four an address, a quarter of the listener's
  connections together.
- **Memory.** A large body keeps its place until its request ends, so at
  most a quarter of the connections hold a body of up to 1 MiB. With the
  deployed listener raised from 32 connections to the policy's most, 128,
  that is 32 such bodies, as before, and at most 128 small ones (8 MiB)
  beside them. The table holds no more addresses than there are places.
- **Uploads.** A report part's upload is authenticated before its body is
  read and takes one of two slots; a gateway that withheld the body held
  the slot for the upload's minute. The body must now bring 16 KiB in
  every five seconds until it is whole.
- **The wire** is unchanged: routes, statuses and fixed texts. The new
  refusal is the route's own answer for `Busy` (429, `account request
  rejected`), which the client's transport already reads as a refusal
  before execution and may ask again.
- Decided against: a shorter body deadline (the obstacle above, and not
  needed once places bound the hold); a place for every body from its
  first byte (honest bursts); more connections alone (the attack scales
  with them).

Implementation pass:
- `in_flight.rs`: the browser listener's table and its `X-Forwarded-For`
  reading, moved unchanged in what they decide; an address's share is an
  argument and the table can bound all places together (the browser
  listener leaves that unbounded).
- `boundary.rs`, `admitted`: a declared oversize is refused first (413, as
  before). A body whose size hint is at most 64 KiB is read for the grace
  (250 ms) without a place; when it is not whole by then, or for any other
  body at once, the address takes a place or the request ends with 429
  before another byte is waited for. The place is an RAII permit: a late
  body's is dropped when the body is read, a large one's when the
  handler has answered, both when the request times out, fails or its
  connection goes. The request's five seconds cover all of it as before.
  `client`: the last forwarded hop when the peer is loopback, else the
  peer; a request without a peer (a test's) counts as `0.0.0.0`.
- `boundary.rs`, `paced`: reads frame by frame under the body's bound; a
  window that ends with the body unfinished and fewer than the pace's
  bytes ends it (408); a window also closes when frames are always ready;
  empty frames are dropped; more than one frame is gathered in one buffer
  whose capacity never passes the bound.
- `native.rs`: a part's body is read through `paced` (16 KiB per five
  seconds); a stalled one answers 408 with the boundary's fixed text,
  every other failure as before.
- `production.rs`: the deployed account listener takes 128 connections.
  Development listeners keep the policy's default of 32 (eight places).
- Nothing new is logged. An address is held only in the table, only while
  it has a place.

Adversarial pass:
- In the router: four withheld bodies of one address take its four
  places; a fifth is refused when its grace ends, long before the
  request's time; a request of the same address and one of another whose
  body is there are served meanwhile and take no place; another
  address's late body waits, arrives and is served; the four end with 408
  at the request's time and the table is empty. With eight connections
  (two places) two addresses hold both: a third address's late body is
  refused, its large one at once although all of it is there, its small
  one that is there is served. A large body, and one of unknown length,
  keeps its place while its handler runs; a late one has given it back by
  then; a fifth large one of an address is refused. A place comes back
  after a broken body (400), a body past the bound (413), a connection
  that goes and a timeout.
- The address: the proxy's hop is the last whatever a client wrote before
  it; a hop that is no address, or none, counts by the peer; a peer that
  is not loopback is not believed; IPv6 by its /64.
- Over real sockets against the listener itself, once with a forwarded
  address and once by the peer alone: four withheld bodies, the fifth
  answered 429 with the fixed text and the security headers within two
  seconds, another address's late body waiting and then served when its
  byte comes, the stalling address's own complete request served, and
  the places back once the four connections are gone. With eight
  connections: two addresses (IPv4 and IPv6) hold the two places, two more
  are refused, twelve complete requests are served meanwhile, and the two
  end with 408 at five seconds as a withheld body always did.
- The pace: a body at twice the pace over ten windows is read whole and in
  order; a tail shorter than the pace's bytes ends the body; one that is
  withheld, stops after a good window, or trickles a byte at a time ends
  at its window; the bound (413) and a broken body (400) hold; frames
  that are always ready and empty still end.
- Mutations, each caught: no share per address, no total, the forwarded
  hop ignored, a large body's place given back before its answer (unit
  tests, the first two also over sockets); with the pace's check removed
  the withheld-body test never ends.
- The browser listener's tests pass unchanged but for the table's
  constructor.

Run on this change: `cargo xtask gate` (format, policy, clippy, tests, doc
tests and the release build), `cargo xtask postgres` (PostgreSQL 18.6;
`central-client`'s sign-in, restore and migration against a real Central
over this listener among it), `cargo xtask smoke`, `cargo xtask secrets`,
`cargo xtask crypto-scope` and `cargo deny check` pass. `cargo audit
--deny warnings` reports RUSTSEC-2026-0459 (`libsqlite3-sys` 0.37.0), as
it did before this change; `Cargo.lock` is untouched here.

Not tested, and recorded for the owner:
- **Caddy was not run.** That it sends the client as the last
  `X-Forwarded-For` hop for `central.baylee.app` is taken from its
  documentation, and the live Caddyfile was not seen. If the proxy sends
  no such header, every client counts as `127.0.0.1` and all share four
  places: small requests go on, large writes and late bodies meet 429
  early. The runbook says so.
- **A 429 sent with the body unread.** For a late body nothing more is
  coming, but a large body refused at once is still on its way; the
  proxy could pass that on as a broken connection (502) in place of the
  429. Either is a refusal before execution to the client's transport,
  which treats a broken connection as no answer.
- **The game's client and gateway were not run against this build**
  (their suites live in the other repository; `central-client`'s test
  here ran in `cargo xtask postgres`). Every local run is one address,
  `127.0.0.1`: a suite that sends more than four bodies over 64 KiB at
  once, or four late ones, from one machine would now meet 429.
- **What stays open.** Eight addresses that withhold four bodies each
  hold all 32 places, each for five seconds; until one comes back, late
  and large bodies of everyone else are refused, so an encrypted object
  over 64 KiB cannot be written and a body delayed past the grace is
  turned away, while sign-in and every request whose small body is there
  go on. One address that opens about 400 connections a second, each
  with a body withheld for its grace, can still hold the other 96: that
  is a flood, which the proxy or the network must limit by rate.
  Buffering bodies in Caddy (`request_buffers`, documented, not tried)
  keeps withheld bodies from Central altogether; the runbook has the
  line.
- **Constants to tune** (`boundary.rs`): the grace (250 ms; shorter
  raises the flood's cost and makes more honest bodies late), 64 KiB,
  four places an address (an address many people share, on a slow
  uplink, can meet it), a quarter of the connections, 128 connections,
  and the upload's pace (a gateway on a line slower than about
  26 kbit/s could upload before and cannot now; a part under 16 KiB must
  be whole within five seconds).
- No slow or lossy real uplink was tried; the times are from reasoning,
  the tests use shortened ones in the router and the real ones over
  loopback sockets.

Engineering review, not an audit.

## The public figures: a gateway's pulse and what Central publishes, 11 November 2026

The owner wants the website to show how many players, tables, games and
accounts there are, with the numbers coming from where they arise: "the
gateways tell Central, Central tells the website". This is Central's side
of that chain (wire, store, service, both listeners, the SDK). The website
reading the figures and the game's gateway sending them are not in this
change ([gateway registry](gateway-registry.md#activity-and-the-public-figures)).

Design pass. Nothing secret is added, so the questions were what the two
new operations let a caller do that it could not do before:
- **Callers.** A pulse comes from whoever holds a gateway's heartbeat
  credential, which is all a heartbeat needs. The figures are for anyone.
- **The pulse must not be a second way into the gateway's row.** It is the
  heartbeat itself with three numbers added: one function in the service
  and one `UPDATE` in the store serve both, so there is no second
  credential check, limit or condition to keep equal. What a pulse can do
  beyond a heartbeat is write three bounded integers and a time on its own
  row.
- **The wire the game links must not move.** The client and the gateway of
  the game build on `central-api`, and every type refuses unknown fields.
  An activity as an optional field of `GatewayHeartbeat` would have changed
  that request and, sent to an earlier Central, been refused as a whole, so
  a reporting gateway would have gone offline there. Hence two new
  operations and two new types, and no change to an existing one.
- **What is kept is the least that shows the numbers.** Three counts and a
  time on the gateway's own row, overwritten by each pulse: no history, no
  player, no address. Reporting is the operator's choice, and the way out
  needs no new operation: a plain heartbeat clears the counts.
- **What is published.** For a gateway: its id, which the directory
  publishes already, and its three counts, only while it is approved and
  enabled, seen within the directory's 90 seconds and reported within
  them. For Central: the number of accounts that can sign in (a stored
  credential; the administration's "enrolled").
- **Lies.** Central cannot check a gateway's counts. It bounds them (100 000
  each, in the wire type's check, in the store before anything is read,
  and in a CHECK) and never publishes them apart from the gateway's id, so
  whoever shows them can say whose claim they are.
- **Cost.** The count of accounts is a scan, so it is not run per request:
  the service keeps it for a minute. A read of the figures is one page of
  the gateway table, as the directory's is, under the directory's budgets
  on both listeners.
- Decided against: an optional field on the heartbeat (above); a table of
  reports or any series (history of when people play, and a second write
  path); Central counting players itself from tickets (it would have to
  follow players; it does not see tables at all); an index on the report's
  time (every pulse writes it); an estimate of the accounts from the
  planner's statistics (another notion: it counts unfinished registrations
  too); the count held in a table (a write path for a number that a minute
  in memory serves).

Implementation pass:
- `central-api`: `GatewayActivity` (three `u32`, `MOST`, `validate`),
  `GatewayFigures`, `Request::GatewayPulse`, `Request::PublicFigures`,
  `Response::PublicFigures`; all refuse unknown fields. Nothing existing
  was edited.
- Store: `counts` validates a pulse's activity before the first statement,
  so an over-bound pulse reads and writes nothing and marks no token used.
  The beat's one `UPDATE` sets the four columns from bound parameters (a
  plain heartbeat's are NULL) under the conditions it already had: its own
  row, the five seconds, not disabled. `gateway_heartbeat` and
  `official_heartbeat` keep their signatures. `gateway_figures` reads ids
  and counts only, joined to enabled approvals, both times inside 90
  seconds, 100 a page by primary key, and checks each row's bound again on
  the way out. `account_count` is one `count(*)`.
- Migration 000025: four nullable columns and two CHECKs (all or none; each
  count from 0 to 100 000). No grant changes: the runtime's `UPDATE` on
  `gateways` is table-wide, and the startup check's list is the same. No
  index.
- Service: `GatewayPulse` is decided in the exhaustive `browser_operation`
  match as native only, `PublicFigures` as served on both listeners
  without a session or accepted terms. The figures spend the public
  budget, a pulse the bearer's budget its heartbeat spends. The account
  count sits in `Counted`: fresh for 60 seconds; after that one caller
  counts while the others answer the earlier number; the lock is not held
  across the query.
- Browser listener: the figures are charged to the directory's book, one
  60 a minute per address for the two.
- `central-client` (AGPL, built on `central-api` only): `gateway_pulse`
  checks metadata and bound before it sends; `public_figures` refuses an
  answer with more than 100 entries, ids that are not ascending UUIDv7s
  after the cursor, or a count above the bound.
- Nothing is logged for either operation, and no dependency was added
  (`Cargo.lock` is unchanged).

Adversarial pass:
- **The earlier wire.** The exact JSON of `gateway_heartbeat`,
  `gateway_directory` and a listing is asserted, and each refuses an
  `activity` or `accounts` field; the new shapes are asserted byte for
  byte as well. A `public_figures` without `after` reads as the first
  page, as the directory's does (an absent option); kept, not changed.
- **The pulse against the store** (PostgreSQL 18): a count above the bound,
  the redemption credential, an unknown token, an `sso`-only token and
  metadata on another origin are refused and leave the whole row and the
  tokens' last use as they were; the bound itself and zeros are taken.
  Inside the five seconds a pulse is `Busy` and the counts stay, and a
  plain heartbeat refused the same way clears nothing. A later pulse
  replaces the counts; a plain heartbeat clears all four columns and the
  gateway stays online without a figure.
- **The 90 seconds**: seen and reported 85 seconds ago is shown; a report
  91 seconds old is not, although the gateway is online; a gateway silent
  for 91 seconds is not, although its report is fresh.
- **Approval**: a pending gateway's pulse (by `listing` token) is kept and
  not shown, then shown once approved; a disabled gateway is hidden at
  once and its pulse refused, by token and by credential pair; enabled
  again within the window it is shown as it last reported.
- **The official gateway**: unlisted or unapproved its pulse is refused as
  its heartbeat is; the bridge's pulse moves its own row and no other, for
  the bridge's origins only; its plain heartbeat clears the counts; a
  registered gateway's row is unchanged through all of it.
- **Paging**: 101 reporting gateways give 100 and then one, ascending,
  none twice; a cursor that is no UUIDv7 is refused.
- **The count**: an unfinished move is not counted, a finished enrollment
  is, a deleted account no longer. `Counted`: no number before the first
  count; kept for the minute; then one caller is told to count and the
  rest are not, also when that count never comes back.
- **The migration** over the upgrade fixture's rows: every earlier column
  of every gateway unchanged (a digest of the rows), no activity anywhere,
  the previous binary's heartbeat statement still runs and leaves a stored
  activity alone, partial activities and counts outside the bound are
  refused by the table.
- **Over real HTTP** (`central-client` against a real Central): a silent
  online gateway has no figure; a pulse inside five seconds is `Busy`; the
  redemption credential, an unknown token, no
  bearer and an over-bound count are refused; then the pulse is taken and
  anyone reads the gateway's counts, with the cursor, on the native route
  and on the browser listener with and without a cookie, the same answer;
  the directory shows the pulse's metadata (it is a heartbeat); a plain
  heartbeat withdraws the figure. The bridge's pulse works for the
  official gateway. On the browser listener a pulse is refused whatever
  the token.
- **Budgets**: the figures and the directory spend one public budget, and
  a bearer sent along buys none of its own; on the browser listener 30 of
  each use up an address's 60, both are then refused, other addresses and
  other budgets are untouched, and the window ends for both together.
- **Found on the way and put right.** The upgrade step first addressed
  gateways that an earlier step had removed; its updates would have
  changed no row and its refusals proved nothing. Every statement of the
  step now asserts that it changed one row. The first form of the kept
  count let every request that arrived while a count ran count as well;
  `Counted` lets one.
- **The checks have teeth**: with the report's 90 seconds taken out of the
  read, with the clearing taken out of the plain heartbeat, and with the
  approval taken out of the read, the fixture fails each time (one change
  at a time, each undone).

Run on this change: `cargo xtask gate` does not pass. It stops at clippy
on a function this change does not contain: `strip` in `central-web`'s
start page has 111 lines where 100 are allowed, in the commit this work
started from (2293736); the website's crates were not this work's to
touch. Its format check, policy and crypto scope pass before that, and
its remaining steps pass when run by hand: clippy over every crate that
does not build on `central-web`, the workspace's tests (471), the doc
tests and the release build; `cargo xtask gate --client` passes. `cargo
xtask postgres` (PostgreSQL 18.6), `cargo xtask smoke`, `cargo xtask
secrets`, `cargo xtask crypto-scope` and `cargo deny check` pass. `cargo
audit --deny warnings` reports RUSTSEC-2026-0459 (`libsqlite3-sys`
0.37.0), as it did before this change; `Cargo.lock` is untouched here.

Not tested, and recorded for the owner:
- **The earlier binary over the migrated table** was not started; that it
  runs there is read from its statements and its startup check
  ([operations](operations.md#what-gateways-report-migration-000025)).
- **The commits one by one.** The gates ran on the last of them; the
  commits before it were ordered so that each builds, and were not built
  singly.
- **The kept count over HTTP.** That the number of accounts is not counted
  again within the minute is tested on `Counted` alone; over HTTP the
  fixture checks that both listeners answer the same number.
- **No gateway sends a pulse yet.** The game's gateway and client were not
  run against this build; the gateway's side is described in a brief for
  that repository, not written. Until it sends pulses the figures list no
  gateway.
- **An earlier Central.** That it refuses the two new operations as it
  refuses a wrong credential (`Rejected`, HTTP 401) follows from its
  decoder and was not run against an earlier binary. The gateway's
  fallback rests on it.
- **The session-token line of work** (the revision the game pins) answers
  a heartbeat with its own response, and has a migration with this line's
  number 000023. This change was not merged with it. The pulse shares the
  heartbeat's path so that it answers whatever the heartbeat answers after
  the merge; `central_client::gateway_pulse` expects `Done` and has to
  follow then. This migration is 000025 and leaves 000024 free.
- **What stays open.** An approved gateway can claim any numbers up to the
  bound. The number of accounts is exact and public, so Central's growth
  can be watched minute by minute. A reporting gateway publishes how busy
  it is to anyone who records it. On the native route the figures have no
  per-address budget, like the directory: one client can spend the public
  32 a second, which sign-in draws from too; that was so before, with the
  directory, and is the proxy's to limit. The website should therefore
  read the figures about once a minute for everyone and keep the last
  answer, never once per page view.
- **Load.** No measurement with many gateways or many accounts; the cost
  statements are from the statements' shape.

Engineering review, not an audit.

## Registration hardened: reviewed as an attacker, a price by look, a longer rise, 11 November 2026

Design pass. The owner, 11.11.2026: repair and improve the protection of
registration as the Impressum's is protected, against accounts made
automatically, with good mechanisms. First a review as an attacker who
reads the source and speaks the listener's protocol directly; its questions
and answers are in `docs/registration-abuse.md` §"Hardened". Then three
changes, each weighed:

- *A request that looks automated.* Refusing it was excluded by the owner's
  rule for the Impressum that no person is ever locked out; chosen: a harder
  challenge only, up to four bits, counted as the Impressum's service
  counts. The listener has the headers and the identity service issues the
  challenge: the listener hands over plain facts (`central_identity::Seen`),
  never a header's text and never the address.
- *What the page reports of its browser* needs room on the wire. A field on
  `RegisterChallenge` would have parsed on both sides (serde takes extra
  fields on a unit variant, which a test holds), and for that reason would
  have been ignored silently by a Central from before. Chosen: a request of
  its own, `RegisterChallengeFor { signals }`, with the old one unchanged
  byte for byte; a Central from before refuses the new one as it refuses
  every body it cannot read, and the page then asks the old way.
- *A longer rise.* Six bits for a flood and four for a look would add up to
  ten: half an hour of the phone that needs two seconds for the base, a
  lock-out by another name. Chosen: both add up and stop at six, 22 bits a
  part, which `central_api::register::MAX_DIFFICULTY` now is on both sides
  (the page took up to 30 before). A slow phone needs longer than five
  minutes for the two hardest steps, so those challenges live ten and twenty
  minutes, and the table holds 40,000: more than the shared budget lets all
  callers ask for in twenty minutes.
- *A block of addresses.* The budgets count IPv6 by /64, and whoever holds
  a /48 is a new address with every request. A hard budget per block would
  refuse people who share one (a carrier, a school); chosen: the block
  (IPv6 /48, IPv4 /24) is counted for the difficulty only. A hard budget
  stays the owner's decision.

Implementation pass:
- `central_api::register`: `MAX_DIFFICULTY` 22 and the two signal bits;
  `wire`: `RegisterChallengeFor { signals: u8 }`, additive, answered with
  the responses that existed.
- `central_identity::challenge`: `points` (the facts and the report to 0 to
  4 bits), `issue(now, points)`, the rise to six bits, the common ceiling,
  a lifetime per challenge checked when it is spent, a sweep at most once a
  second; the constants are checked against the wire's ceiling and the
  shared budget when the crate is compiled. `spend` still removes the
  challenge before it judges it, and checks the difficulty stored with the
  nonce.
- `central_identity::limits`: both challenge requests are named in the
  shared budget's list, so a token sent along does not move them to the
  budget for sessions.
- `central-http`: `seen.rs` reads three headers into `Seen`;
  `Budgets::admit` charges the request and, for a challenge, counts the
  haste of the address and of its block (`Client::block`), the block never
  being a reason to refuse; `Accounts::browser` carries `Seen` to
  `IdentityService::handle_browser_seen`. Nothing of it is logged, and
  nothing is stored but the counts the budgets keep for an hour.
- The page: `pow::browser_signals` (`navigator.webdriver`, no
  `navigator.languages`), the new request with the old as its fallback, the
  three seconds counted from the answer's arrival (`handover_wait`; counted
  from the request, a slow connection handed its solution in too early and
  lost the challenge), and after three seconds of work a line with the
  share done and a bar in an `aria-live="polite"` region
  (`register-solving`). No new control.
- The words in English and German; the nine other languages carry the
  English line until they are translated.

Adversarial pass:
- Over the real listener, the identity service and PostgreSQL 18
  (`crates/central-client/tests/support/registration.rs`, in
  `cargo xtask postgres`): a solution for less work than was asked is
  refused, and the right one after it finds the challenge gone; a good proof
  with a taken name is a conflict and is spent by it; a nonce Central never
  issued, in the issued shape and solved, is refused the same for a taken
  and a free name; three registrations with one solved challenge at the
  same instant make one account; the native listener refuses both challenge
  requests, `Register` and a reservation without the bridge, and made-up
  tickets enroll nothing; the price by look (a tool's user agent, missing
  headers, the page's report, bits Central does not know, all together at
  four), the same for both ways to ask; an address pays one bit more with
  every five challenges and is refused at its twenty-first, both ways
  counted together; the twenty-first challenge from one /48, each from
  another /64, costs one bit more, and the block beside it nothing.
- Unit tests: a solution once, not too soon, not after its time, not for an
  unknown nonce, not for less than the difficulty stored with it; the rise
  at each doubling to six and its fall with the hour; a flood and a look
  together never over six; every combination a challenge the page takes;
  lifetimes of five, ten and twenty minutes; a table filled at the shared
  budget's full rate for twenty minutes drops nothing that still lives; the
  header facts and the user-agent markers, where a person's browsers score
  nothing; the steps of haste; the new request's exact bytes, the old one's
  unchanged, signals a `u8` cannot hold refused; a challenge request with a
  token still spending the shared budget (it did not before); the page's
  wait counted from the answer's arrival.
- In a real browser (Chromium at both widths, `web/e2e-site`, run on a port
  of this checkout's own): the page asks with `RegisterChallengeFor` and
  reports the browser a program drives, stays quiet for three seconds, then
  shows the line and the bar in the polite region, the bar named by the
  line, with the page's WCAG rules and its 44 px targets holding while it
  shows, hands the proof in and clears the line; the whole site suite
  passed with it (278 passed, 6 of its own skipped). The Worker's real
  speed was measured there (`docs/registration-abuse.md` §"A flood"), which
  corrected this change's first figure for a desktop from seconds to a
  minute.
- Against a real Central in one piece (`web/e2e-real`: a throwaway
  PostgreSQL 18 with TLS, the deployed `--accounts-stdin` with its browser
  listener, `central-web` and the stand-in for Caddy, on ports of this
  checkout's own, stopped and deleted after): Chromium with a device's
  profile is asked for 18 bits, two for the driven browser the page reports
  and none for its headers, which arrive through the proxy as a person's do
  (`Accept-Language`, `Sec-Fetch-Site: same-origin`, a user agent that
  names no tool); the proof the page's Worker solved is taken four seconds
  after the challenge, and registration, enrollment and sign-in go through;
  `site-chat.spec.ts` passes at both widths with its four registrations.
- Not run: a browser no program drives (it would be asked for 16 bits), a
  phone, Safari and Firefox. The nine other languages' words are English.
- Left open and written down (`docs/registration-abuse.md` §"Hardened"):
  the budget all callers without a session share can be used up from few
  addresses; a block's people share its count; a hard budget per block; a
  proof that costs memory, since SHA-256 costs a native program a
  thirteenth of what it costs a page.

Engineering review, not an audit.

## The site's polish: links in new tabs, movement, the words, 11 November 2026

The owner asked for a polish of the Leptos site (layout, movement, every
link away in a tab of its own, the words and the legal texts put right).

**Pass 1, what changed.** Every off-site anchor carries `target="_blank"`
beside `rel="noopener noreferrer"`, release files included
(`central-web-server/tests/pages.rs` fails on one that does not, and on a
link within the site that does); a CSS arrow with an alternative text marks
such a link in running text. A platform's further files fold under a native
`<details>`. Pages cross-fade through `@view-transition` (same-origin
navigations only, by the standard), a page's head settles by `transform`
alone, a line under the bar follows the scroll through a scroll timeline, and
`home.js` counts the card numbers up from a `data-count` the server wrote.
The words of both languages were proofread, the privacy notice on the site
gained three sections, and `legal/*.md` moved to version 2026-11-11.

**Pass 2, the attacker's view.** `target="_blank"` alone would hand the opened
page `window.opener`; `noopener` on every such link (and the site's
`Referrer-Policy`) keeps the opened site from reaching or learning of this
one, and the test holds the pair together. The counting script writes only
`textContent`, from a number formatted by `Intl.NumberFormat`: no markup
sink, so Trusted Types has nothing to refuse; a `data-count` that is not a
positive finite number is left as the server wrote it. No new script file, no
inline style or script, no new origin in the CSP; view transitions and
scroll timelines are CSS and reach no data. The new words promise less than
before where the code did not bear them out (what a recovery code saves, what
an export holds, what deleting leaves, who receives what).

**Pass 3, what was tested.** The page tests in both languages; `e2e:site`
with the wording changed in the specs; the real-Central chat suite on the new
Central build; Lighthouse on the start page, download and register (mobile
performance 96–99, accessibility and best practices 100). Movement is off
under `prefers-reduced-motion` (the global rule and the media queries around
each new animation), and `tests/hover.rs` still holds: nothing moves or
grows as a whole under the pointer. **Not evidence of legal compliance:** the
texts were reviewed by a model, not a lawyer; what is the owner's to confirm
stands in `docs/proposals/website-text-review.md`.

## The service worker only for those who use it, 11 November 2026

**Pass 1, what changed.** `pwa.js` registered the service worker for every
visitor with JavaScript, which put a copy of the site's public files on the
device of someone who only read a page. It now registers it only when the
browser remembers a live sign-in (`baylee-central-session`), runs the
installed app (`display-mode`), or has notifications on for this device
(`baylee-push-installation-v1`, since notifications arrive through the
worker). Otherwise it unregisters a worker left from earlier and deletes the
`baylee-site-*` and `baylee-public-*` caches. The owner chose this on
11 November 2026; the privacy page's "What your browser keeps" says it.

**Pass 2, an attacker's view.** Nothing is granted by the three signs: they
decide only whether this origin's own, integrity-checked `/sw.js` is
registered, through the same Trusted Types policy as before. A script that
could write localStorage could already register the worker. Removing the
worker cannot lose data: its cache holds public files, and the push state it
keeps is cleared by sign-out before the sign leaves.

**Pass 3, what was tested.** `e2e-site/pwa.spec.ts`: a reader has no
registration and no cache; after a sign-in the worker and cache exist, and
the first page after the sign is gone removes both; a device with
notifications on keeps the worker without a sign-in; the offline page and the
waiting update still work signed in. Not tested: an installed app on a real
phone (the `display-mode` branch is exercised by no test).

## The security page and what it claims, 11 November 2026

The owner asked for animated diagrams that explain the sign-in, what stays
in memory, the chat's encryption and signing the client in by a link.

**Pass 1, what was built.** `/{lang}/security`: four sequence charts as
inline SVG beside the same steps in words, each flow ending in its limits
and the page in a list of what none of it can do. No script drives them: a
radio group picks "play" or a step and `style/security.css` moves the rest,
so the page needs no new entry in the CSP and holds no inline style (every
position is an SVG attribute). A chart is hidden from assistive technology
because the list beside it says all of it. Every sentence and every arrow
maps to a row of `docs/security-page.md` with the file and lines it rests
on; a claim changes there first.

**Pass 2, a reader told to assume every sentence false.** A second reader
who had not written the page checked each row against the code and changed
the rows A7, A8, B2, B5 to B7, C1 to C7, D9, E2, E5 and E7 and the
sentences on them. What that took back: the chat wraps for one key per
account and not per device, and whoever has Central's database with its
keys and guesses the password, or has the recovery code, reads the stored
chats; an old gateway account's client sends the typed password at every
attempt until the account has moved, not once; a paired session counts as
no newer than the page's password sign-in, it is not "never fresh"; what is
overwritten names only what the first-party code holds itself (not what
`opaque-ke` holds inside, not the string's copy on its way into the
module); the RFCs are called what they are (IRTF, informational).
`boot.js` now deletes a stale chat record also where the browser cannot
list its databases, because the page says the record goes.

**Pass 3, what holds it.** `security.rs`: `no_promises` reads the page's
words in every language's file and refuses "unbreakable", "guarantee",
"GDPR", "100 %" and their German forms, and "secure", "sicher" and "safe"
outside the page's own name and the safety number (the list holds German
and English words only, so it does not read the other nine languages'
own); `charts_are_drawable` and `names_fit_with_room_to_grow` hold the
drawings; `e2e-site/security.spec.ts` plays and steps each chart
with the keyboard and runs axe; the nine translated languages were spot-read
for a claim made stronger than the English one (the limits of every flow,
the memory sentence), by a model and by no speaker of the language. **Not
an audit and not evidence of security:** the page says of itself that only
the people who built Baylee have reviewed the implementation.

## The Impressum's puzzle on the new site, 11 November 2026

The owner asked to repair and improve the protection of the legal notice's
details. The service that guards them is the game repository's
`baylee-feedback` (`/site/impressum*`, unchanged here); the page is this
repository's.

**Pass 1, what was wrong and what changed.** The rewrite had lost two of
the service's signals and weakened a third property: the hidden link to the
service's honeypot (`/site/impressum/kontakt`) was gone, the answer "the
graphics come from a program" (bit 4 of `s=`) was never sent, and the
solved details stood in the page as `mailto:` and `tel:` links. Now
`legal.rs` draws the bait link again (`hidden`, `display: none`,
`tabindex="-1"`, `aria-hidden`, `rel="nofollow"`); `islands/impressum.rs`
asks WebGL for its renderer after the click (never while the page loads)
and sends the three yes-or-no answers; the e-mail address and the number
are drawn as text with one hidden piece between their halves
(`invalid.example `, `00 `), and two buttons build `mailto:` and `tel:`
only when pressed. The privacy page names everything the puzzle keeps and
asks.

**Pass 2, the collector's view.** What a program gets without a browser:
no detail in the page, in any language, and a link only a program follows,
which marks its address for 24 hours. With a headless browser: the service
sees `webdriver` and a software renderer and answers with made-up details
of the same shape. Both answers are the browser's own word and a patient
attacker lies about them; they raise the price, they are no barrier, and
the puzzle's work is the only part that cannot be talked away. A harvester
that reads `textContent` or the markup after a solved puzzle gets
`…@invalid.example …` and a number with two digits too many; what a person
sees, selects, copies and hears is the true value (`hidden` content is in
neither `innerText` nor the accessibility tree). That is a hurdle against
ready-made harvesters, not against one written for this page. No inline
style or handler was needed, so the CSP is unchanged; the page speculates
on no link (no prefetch, prerender or speculation rules), so no browser
follows the bait for a person, and `robots.txt` keeps honest crawlers off
`/site/`.
Findings on the service's side, which this repository cannot change, are
written up for the game repository and the owner: the made-up details also
reach real people who share a marked address, and since the page now draws
them as ordinary text they read as real; a challenge is not bound to the
address that asked for it; the price does not rise with the total load;
the mark is read when a challenge is issued, not when it is redeemed.

**Pass 3, what was tested.** Units for the renderer test and the halves;
`central-web-server/tests/pages.rs`
(`the_impressum_holds_no_detail_and_keeps_its_bait`: the link once per
language and on no other page, no `mailto:`, `tel:` or speculation in any);
`e2e-site/impressum.spec.ts` against a stand-in service (the solved text as
seen, selected and announced, the markup without either value, the buttons'
addresses, bit 1 of `s=` in a driven browser, one retry after 410, axe and
44 px). Against the game repository's real service on loopback with
made-up details: a browser driven like a person's got 20 bits, 15 to 19
seconds of work and the true details; a headless one 21 bits and made-up
ones; after one request to the bait every browser on that address got
made-up ones; a blob the service wrote decoded in the island. **Not
tested:** the proxy's route on the live host (`/site/impressum*` must stay
with the feedback service at the switch), real graphics cards on Windows
and Linux, a screen reader by ear.

## The figures on the website, 11 November 2026

The other end of "The public figures" above: `central-web-server` shows
what Central publishes.

**Pass 1.** `fetch.rs` asks Central for the figures with the directory,
once a minute for everyone and never per page view, as that section asks.
It takes at most 20 pages of 100, requires ascending ids and a valid
activity (each count at most 100 000), and shows a gateway's numbers only
while the directory calls that gateway online. The strip adds them up and
shows them, with the number of accounts, only while a source reports them;
when Central stops answering, the numbers go rather than stand still.

**Pass 2.** The numbers are plain text written by the server from integers;
no markup comes from Central or a gateway. A gateway's counts are its own
claim: an approved gateway can show up to the bound, which the privacy page
says in words ("the operator's own statement") and the owner accepted with
the bound. A Central that answers out of order or beyond the bounds is
treated as one that does not answer. The exact number of accounts is public
by the owner's choice.

**Pass 3.** `fetch.rs`'
`only_an_online_gateway_s_own_report_is_shown_and_it_goes_with_central`,
`pages.rs`' `the_figures_are_shown_only_while_a_source_reports_them`, and
the browser suite on pages with and without figures. Not tested: a real
gateway's pulse (none sends one yet), and the strip with six-digit numbers
in every language at every width (checked by eye in German and English).

## Guessing a password online, 11 November 2026

The owner asked for the sign-in to be protected against brute force as far
as it goes, without breaking the released client or gateway.

**Pass 1, design: what was there and what replaces it.** Read: the whole
sign-in path on both listeners (`central-http`'s `native.rs`, `browser.rs`,
`boundary.rs`, `in_flight.rs`; `central-identity`'s `lib.rs`, `dispatch.rs`,
`limits.rs`, `password.rs`, `deletion.rs`, `reset.rs`, `legacy.rs`,
`reports.rs`), the released client's transport in the game repository
(read only) and the documents. Every place a secret can be guessed online:
the password through `LoginStart`, `ChangePasswordStart` (two guesses) and
`DeleteAccountStart`; the legacy password through `LegacyLogin`; a reset
code (160 random bits) and the pairing and session tokens (256), which
guessing does not reach. Found, in the order of what they cost:

1. The native listener counted no client address at all
   (`native.rs`, `process`): one address could send as many sign-in starts
   as the 32 requests a second all clients share let through, at as many
   accounts as it liked, and by doing so alone keep every sign-in at
   `Busy`. The browser listener allowed one address 30 starts a minute
   (43,200 a day) at any accounts.
2. One account's password could be guessed 8 times a minute, 11,520 times
   a day, from anywhere (`limits.rs`, `account`): the only count per
   account was a minute's.
3. That count's table held 1,024 entries, was keyed by whatever id a start
   presented, and refused newcomers when full: about 17 starts a second
   with made-up ids kept every real sign-in at `Busy`.
4. Whoever knew an account's id kept its holder out everywhere with eight
   requests a minute, the holder's own devices included (recorded on
   31.10.2026 as the owner's trade-off).
5. A start was counted before a worker was taken, and the three finishes
   consumed their attempt before taking one: under load a start spent a
   unit for nothing, and a right proof was lost with its attempt.
6. On the browser listener, requests without a session other than the
   budgeted ones (`Terms` or `LoginFinish`, say) had no address budget, and a
   report's two native routes drew on the shared 32 a second before the
   credential was looked at: one address could spend everyone's sign-ins
   through either.
7. Not changed, recorded: requests with a bearer nobody issued draw on the
   windows all sessions share (512 and 256 a second) and on a table of
   1,024 bearers a second that refuses newcomers when full, so a flood of
   them keeps signed-in clients and gateways at `Busy`; sign-in itself goes
   on. The page's only password rule is eight characters. `Discover` says
   whether a name is enrolled (known, `native-login.md`).

The design ([native login](native-login.md) §"Guessing a password online"
has the numbers): a start is a guess until its proof. Counted per account
over all places that are not its own, per account at a place, per place
over all accounts, per IPv6 /48; a proof gives the guess back and makes the
place the account's own for 30 days, where only its own count applies,
until another password is set or the account is deleted; made-up ids
together are one account and take no entry; a reset's start
keeps the minute only; bounded tables that forget and never refuse; address
budgets on the native listener, and a share per address of the requests
without a session on both. Decided against:
- **A lockout** (N failures close the account): it hands the lock to
  whoever knows the id, for as long as they like, at every place. The
  counts above do that only where the account never proved its password.
- **Counting by account and address alone**: many addresses then guess
  without a bound; the account's own count over all places stays.
- **A proof of work or a device token for sign-in**: a released client can
  send neither, and requiring one would lock it out. What a later client
  needs is in the document.
- **Writing the counts or the own places to the database**: a migration
  and a store of who signed in from where; the owner's decision (below).
- **A new answer or a `Retry-After` header**: the released client reads a
  429 as `Busy` and asks again; nothing new is sent.

**Pass 2, implementation.** `central-identity`: `guesses.rs` (new: the
counts, the own places, the tables; no place has a `Debug`, so none is
printed by accident), `limits.rs` (the per-place share of the shared
windows), the starts and finishes in `lib.rs`, `password.rs`,
`deletion.rs`, `legacy.rs` (worker first, then the count; the proof gives
back), `reset.rs` (a finish forgets the account's own places but its own),
`reports.rs` (a place for the two routes). `central-http`:
`addresses.rs` (new: the native budgets), `boundary.rs` (the address
attached to each request: the proxy's last hop from a loopback peer, none
without one), `native.rs`, `browser.rs` (the address handed to the
service). No wire shape, status, text, migration, grant or dependency
changed; `IdentityService::handle` and `handle_browser` keep their
signatures, with `handle_from` and `handle_browser_from` beside them;
`file_report` and `report_upload` take the place. Nothing new is logged or
stored.

**Pass 3, an attacker's view, and what was tested.**
- *One address, many accounts* (spraying, stuffing): 30 unproven guesses an
  hour, 100 a day over all accounts; the /64s of one /48 share four
  places' counts. Unit tests with their own clock.
- *Many addresses, one account*: 30 an hour and 100 a day whatever the
  number of addresses, 10 an hour from any one. Unit tests; over real
  HTTP and PostgreSQL the seven evaluations a minute left after the
  holder's sign-in, then `Busy` from three other addresses and natively.
- *Filling the tables*: made-up ids take no account entry and share one
  count; a full table forgets the entry that spent least, so 16,384
  single guesses do not make it forget the account guessed at most.
  Unit tests.
- *Refunds as a way to guess more*: a proof returns exactly what its own
  start spent, only into the window it was spent in, and an attempt is
  consumed once; proving one's own account from an address frees nothing
  for guesses at others from it. Unit tests
  (`a_later_window_gets_nothing_back`, `a_proven_start_was_no_guess`).
- *Choosing one's address*: the last hop is the proxy's; a forged earlier
  hop changes nothing (tests on both listeners). A process on the host can
  send its own hop and so spend, or use, another address's counts; it is
  the operator's machine.
- *Keeping someone out*: the holder at an own place is served while the
  account's count is spent from elsewhere (unit test and real HTTP: eleven
  right sign-ins in a row during it, more than a place's ten an hour).
- *An own place as a way in*: whoever knew a password and proved it from
  their own addresses kept those as the account's own after the holder
  set another one, each with 30 guesses a day of its own beside the
  account's hundred, for 30 days. Found in the review of the first two
  commits and closed: a change or a reset leaves only the place it was
  made from, a deletion none (unit test). The review of that fix found
  its own way round: a sign-in begun before the change and finished after
  it still proved the old password, and the proof made its place the
  account's own again although the store refused the session. A place is
  now marked only after the store issued the session or stored the new
  password (unit test; real HTTP and PostgreSQL: a stale finish is
  `Rejected`, and its place is refused with the strangers while home is
  served; with the mark put back before the store's call that scenario
  fails at this assertion, run once and restored).
- *Asking what the counts know*: with an address's or an account's count
  spent, a start is answered only at an own place, so whoever sends from
  an address learns whether a given account signed in from it. Not
  closed; under "What stays possible".
- *The account listener behind its proxy*: with the hop a proxy appends,
  over real HTTP: the holder's address is served while a guesser's and a
  request without a hop are refused; 30 discoveries a minute from one
  address, then `Busy` for it alone, a forged earlier hop included, and
  none for a request without a hop.
- *An unknown name treated as a known one*: the same answer of the same
  size after the same work, counted at its place as an account is; all
  made-up ids share one account's count (unit test; real HTTP: answered,
  and refused by the eleventh start at the latest).
- *One address spending everyone's budget*: 600 a minute, then `Busy` for
  it and service for the others, on every request that draws on the
  shared budget, a report's routes included (unit tests).

Run on this change and passing: `cargo xtask postgres` on its last code
commit, the fourth (PostgreSQL 18.6; the whole existing scenario
unchanged, and the guessing steps after it); `cargo xtask smoke` and
`smoke --exercise-failure` on the third; `cargo xtask secrets` on the
last; `cargo deny check`. `cargo audit --deny warnings` reports
RUSTSEC-2026-0459 (`libsqlite3-sys` 0.37.0), as it did before;
`Cargo.toml`, `Cargo.lock` and `deny.toml` are untouched here.

**`cargo xtask gate` did not pass.** Its format, policy, crypto-scope and
clippy steps passed (the workspace, all targets and features, warnings
denied); its test step stops at `xtask`'s
`site::tests::no_language_is_english_under_another_name` ("es: 150 of 836
messages are the English ones"). That test reads
`crates/central-web/locales`, which this change does not touch and which
is the same as at the commit it began from (`95e6185`); it belongs to
whoever edits the locales. Every other test of the workspace passed
(`cargo test --workspace --all-targets --all-features --no-fail-fast`),
and the two steps the gate did not reach were run on their own and
passed: the doc tests and the release build. The gate and the workspace's
tests ran on the second of the four code commits, the doc tests, the
release build, the smoke test and the secret scan on the third; after
the fourth, which touches `central-identity` and `central-client`'s
tests, clippy and the unit tests ran again for `central-identity`,
`central-http` and `central-client`, and `cargo xtask postgres` for the
whole.

Not run: the game's client and gateway suites against this build (the
game repository pins another Central branch, `c42/session-tokens`); a
real proxy in front of the account listener (Caddy's last hop is taken
from the earlier review above); the page's end-to-end suite, whose
requests all arrive as one address.

**What stays possible** (residual risks, for the owner):
- **Guessing.** 100 guesses a day at one account from many addresses,
  36,500 a year, and 30 more a day for whoever shares one of its own
  addresses. That finds a common password. Nothing here checks what
  password is chosen; the page asks for eight characters.
- **Keeping someone out.** 100 requests a day from four addresses close an
  account's password sign-in everywhere it has not proved its password in
  the last 30 days, since the last restart; before, eight a minute from
  one address closed it everywhere. Someone at the holder's own address
  (a household, a campus, a carrier's shared address) closes it there too,
  with 10 an hour. Every restart, so every deploy, forgets the own places:
  a guesser who keeps asking then keeps the holder out again, an
  administrator included, until the guessing addresses are kept away at
  the proxy. Sessions that exist go on.
- **A shared address** that spent its count (30 wrong passwords an hour
  from anyone behind it) refuses its newcomers; those who signed in there
  before are served.
- **Many addresses** are bound by each account's count only: a botnet, or
  an IPv6 provider's range, sprays as widely as the 32 requests a second
  allow, and four addresses at 600 a minute each still spend those for
  everyone. The share is counted by /64 with nothing wider above it, so
  one IPv6 subscriber with a /56 is 256 addresses for it. Rate limits at
  the proxy or the network must answer a flood.
- **What the counts tell.** Whoever sends from an address (a household,
  an office, a campus, a carrier's shared address, a VPN's exit) and knows
  an account's id can find out whether that account proved its password
  from that address in the last 30 days: with the address's hour spent,
  which 30 unfinished starts do, a start at the account is answered there
  only if the place is its own. Nothing about other addresses, and
  nothing to anyone elsewhere. It comes with own places kept by address
  and goes with a device token.
- **Made-up ids cost a worker.** A start at one that only nobody's count
  refuses has taken a worker and a statement before the refusal; the
  address's 30 starts a minute and the shared 32 a second bound them.
- **The counts tell a made-up id from an account's**: eight starts at a
  made-up id, then one at the id in question, is refused only if that one
  is nobody's too. `Discover` says as much for a name; it matters the day
  discovery stops saying it.
- **One process's memory.** A second instance would count on its own.
  Above 16,384 accounts signing in within 30 days the least recent lose
  their own places early.
- **Without the proxy's header** the account listener counts per account
  only; the runbook has the check.
- **Privacy.** For up to 30 days the service's memory holds from which
  address (an IPv4 address or an IPv6 /64) an account signed in, at most
  eight per account, and for up to a day which addresses made unproven
  sign-in attempts at which account. Another password forgets the
  addresses but the one it was set from, a deletion all of them. Nothing
  of it is written to disk or logged. The owner accepted this trade on
  11.11.2026, asked in these terms: Central remembers for 30 days, in
  memory only, from which address an account signed in, and someone at the
  same address who knows the user name can read from the answers that it
  did. The privacy page says both since then (`privacy-counters`), the
  security page what guessing still finds and who can keep whom out
  (`sec-signin-guess`, rows A12 and A13 of `docs/security-page.md`), and
  the message for a refusal no longer promises a minute (`error-busy`).
- **Every local, preview and end-to-end run is one address.**

**Recommended, not built.** A device token and a proof of work for sign-in
in a later client version, and the device token as a cookie on the website
now; the own places kept across restarts (a table of account and a keyed
digest of the address, with its own privacy decision); a check of chosen
passwords against a list of common ones on the page, where the password is
typed; `Retry-After` on a `Busy` answer, and words on the page that do not
promise "a minute"; counting requests with a bearer only after the bearer
is known, per address before that.

Engineering review, not an audit.

## A registration proof that costs memory, 11 November 2026

The owner accepted Argon2id in SHA-256's place for registration's proof of
work on 11.11.2026; the question and the answer are quoted in
[the crypto protocol](crypto-protocol.md) §"Registration's proof of work".
The reason was a measurement from the hardening before
(§"Registration hardened"): one core solved the SHA-256 proof thirteen
times as fast for a script as for the page.
[Registration abuse](registration-abuse.md) §"A proof that costs memory"
has the numbers in force and the measurements.

**Pass 1, design.** Read: `central-identity`'s `challenge.rs`,
`dispatch.rs` and `lib.rs` (the workers, and how sign-in's finish takes one
before it consumes its attempt), `central-crypto`'s `legacy.rs` and
`opaque/stretch.rs` (how Argon2 is called there), `central-api`'s
`register.rs` and `wire.rs`, `central-web-crypto`, the page's `auth.rs` and
both Workers, the crypto scope check and its record. Decided:

1. *One function for both sides*, in `central-crypto`, where the Argon2
   dependency is. `central-api`, from which the page reads its ceilings,
   has none and gets none. No dependency, feature or lockfile change.
2. *A fixed label as the salt separates the domain*; the nonce, the part
   and the counter follow in fixed widths, so no two inputs are the same
   bytes and no output means anything elsewhere.
3. *Central sets the price and keeps it with the nonce.* The page takes it
   up to ceilings both know when they are compiled, each field and the
   whole.
4. *An answer of its own* (`RegisterWork`): the older answer refuses a
   field it does not know. `Register` stays as it is.
5. *The old request is answered "paused"* and not refused: a page from
   before has words for the one and would show the other as a registration
   that failed.
6. *A worker first, then the nonce.* A `Busy` must not cost a solved
   challenge. The check runs off the executor and ends at the first part
   that does not hold.
7. *The numbers by measurement*: 16 MiB, one pass, eight parts, 4 to 10
   bits.

Decided against:
- **A second implementation of the hash** (JavaScript in the page): two
  functions drift, and the page's would be the slow one again.
- **A function made for proofs of work**: a new dependency, and less read
  than Argon2id.
- **More memory**: Central's cost for each check, and phones.
- **Keeping the SHA-256 request alive beside the new one**: the cheap way
  would stay open to whoever asks the old way.
- **Charging `Register`'s budget only for a good proof** (the hand-over's
  open point): not decided by the owner, and with this change it would let
  one address make Central compute without a bound.

**Pass 2, implementation.** `central-crypto`: `proof.rs` (new; the safe
`argon2` interface on memory the caller owns, as `legacy.rs` uses it).
`central-api`: the ceilings, `work_cost`, `work_acceptable`,
`Response::RegisterWork`. `central-web-crypto`: `work_start` and
`Work.search`, and asserts that hold the two crates' ceilings equal; the
committed module rebuilt in the pinned container. `central-identity`:
`challenge.rs` (the price kept with the nonce; `prove`), `dispatch.rs`
(the new answer, the old request, `Register`'s order),
`with_registration_challenge` with a memory. The page: `auth.rs` and
`crypto-worker.js` (the price checked before any work, the search in
turns, the older way at an older Central, one more try on `Busy`). No
migration, grant, setting, log line or stored datum is new; `Cargo.toml`
and `Cargo.lock` are untouched. The crypto scope's fingerprint was pinned
again after the documented source and graph review by the author, and the
pass that took the change into the website's branch read `proof.rs` a
second time and computed the pinned output again with OpenSSL; a review by
another model or a person is open (`Review: pending` in
`docs/crypto-proposal/bigint-advisory.md`).

**Pass 3, an attacker's view, and what was tested.**
- *The cheap way back.* The old request gets no nonce, and a SHA-256
  solution is judged by the new function like any other counters. Over the
  real listener and PostgreSQL (`registration.rs`, `older`): "paused" for
  a person's headers and a script's, `Rejected` for a SHA-256 solution to a
  made-up nonce and to an issued one, no account afterwards. Unit:
  `a_sha_256_solution_is_not_taken`.
- *Less than was asked.* The price is the one kept with the nonce; the
  client sends none. A solution found at a lower difficulty or with less
  memory does not hold (`central-crypto` and `challenge.rs` tests; over
  the listener as before).
- *A proof used twice, raced, or made up.* As before: the challenge leaves
  the table under its lock before anything is judged, and now after a
  worker is held. The listener's tests for these run on the new proof.
- *A price that exhausts the page.* Whoever answers in Central's place and
  asks for more than the ceilings gets no work: the page refuses before it
  starts (site test: one bit above the most, and a kibibyte above 64 MiB,
  with nothing handed in), and the module refuses again in `work_start`
  before it takes memory.
- *Making Central compute.* Only a `Register` that passed the address's
  budget, found a worker and names a living challenge after its three
  seconds is computed for: one evaluation for a wrong answer, eight for a
  right one. A wrong count of counters is refused before any. The bound
  is in the document; it was calculated from the measured evaluation, not
  driven as a flood.
- *Taking the workers, or losing someone's solution.* Without a worker
  the answer is `Busy` and the challenge stays; the same solution is taken
  afterwards (`a_busy_service_keeps_the_challenge_for_its_solution`), and
  the page sends it once more (site test: 429, then the same counters
  again).
- *The name.* A wrong, late or early solution is refused before the name
  is looked at, the same for a taken and a free one (the listener's test
  from before, on the new proof).
- *The test knob.* It only lowers and nothing a client or an operator
  sends reaches it (`the_knob_only_lowers`; the launch does not call it).
- *Two builds drifting apart.* One pinned vector, checked natively,
  through the function the Worker calls, and in Chromium with the
  committed module; its bytes also come out of OpenSSL's Argon2id.
- *What the memory holds.* The inputs are public and the output is
  compared with zero, so nothing secret is in the proof's memory and no
  constant time is needed. The proof has a Worker of its own, ended
  before the password's Worker starts, so the page holds 16 MiB and then
  64 MiB and never both (a Worker's WebAssembly memory does not shrink;
  the first build of the page used one Worker for both and would have
  held about 80 MiB, found when the allocator was read for this review).
  The browser's own figure for the Worker's memory was not measured.
- *The other listener.* The native listener refuses `Register` and both
  challenge requests, as before.

Found on the way:
1. `central-client`'s fixture used one address (`203.0.113.50`) in two
   tests, registration's haste and the sign-in's guessing, on the commit
   this work began from (`757e41a`), so the second found the address's
   twenty challenges of the hour spent. Seen by reading on that commit and
   as a failure on this branch; not run on `757e41a` itself. The haste
   test moved to an address of its own.
2. Chromium's throttle does not slow a Worker (measured, and refused on
   the Worker's own session), so the brief's stand-in for a phone was
   replaced by stopping and continuing the renderer in turns. The
   document says how, and that no phone was measured.

Run on this change and passing, on its last code commit (`701f285`):
`cargo xtask gate` ("Gate passed: format, policy, clippy, tests, doc tests
and release build."), `cargo xtask crypto-scope`, `cargo xtask secrets`,
`cargo deny check` ("advisories ok, bans ok, licenses ok, sources ok"),
`cargo xtask smoke` and `cargo xtask postgres` (PostgreSQL 18: the
registration attacks on the new proof, the older request, and the
sign-in's guessing after them). The same six passed on the documents'
commit before it (`6341223`). `cargo xtask wasm --check` in the pinned
container, on `6341223`: the committed module and glue match a fresh
build, and nothing the module is built from changed after it. The site
suite's account spec on a port of its own: 22 passed. `web/e2e-real`
against a throwaway Central of this tree, release build, three runs: a
timed registration at each width and `site-chat.spec.ts` on the desktop,
twelve accounts in all. The calibration and the native measurement were a
temporary spec and a temporary example, neither committed.

`cargo audit --deny warnings` fails as it did before this change, on
RUSTSEC-2026-0459 (`libsqlite3-sys` 0.37.0); `Cargo.toml`, `Cargo.lock`
and `deny.toml` are untouched here.

Not run: a phone, or any machine but one laptop; a flood over the listener
(the bound is reckoned from the measured evaluation); the machine that
serves `baylee.app`; code with vector instructions or a graphics card;
this page against a real binary of an older Central (the older way is
shown against a stand-in, and that the older listener answers an operation
it does not know with the refusal the page waits for was read in its
source); the game's client and gateway against this build (the game
repository pins an older revision and, by a search, matches none of
registration's answers); the site's and the real suite's other specs; the
browser's own figure for a Worker's memory.

**What stays possible** (for the owner; the document's "What stays" has
the numbers):
- A script solves about as fast as a person's page on one core, and uses
  all its cores where the page uses one. The proof is a price, not a wall.
- Code with vector instructions, and graphics cards, were not measured.
- Central computes for answers now. The bound holds within the budgets;
  it was not measured on the serving machine, and the workers are
  sign-in's too.
- A register page from before, in a tab left open, cannot register until
  it is reloaded; so the site is deployed first
  ([operations](operations.md#registration)).
- A device several times slower than the measured quarter can run out of
  the hardest challenge's twenty minutes.
- The privacy notice's sentence about the puzzle was older than this and
  the hardening before it (five minutes, nothing about what the difficulty
  depends on, nothing about Central computing). On the website's branch it
  now says twenty minutes at the latest, memory and computing time, what
  the difficulty depends on and that Central computes part of it again
  (`privacy-counters`, German and English by hand, the nine other
  languages by machine); the owner has not read the wording, and it makes
  no statement about the law.
- `proof.rs` was read a second time by another pass of the same model
  only; no other model and no person has reviewed it.

Engineering review, not an audit.

## The pulse's fourth count, 11 November 2026

The owner asked for "players in game" beside "players online" on the
website. A gateway's activity gets a fourth count for it:
`GatewayActivity { players, playing, tables, games }`, where `playing` is
the number of connected players seated in a running game. On the wire an
activity is `{"players":12,"playing":8,"tables":3,"games":4}`. The pulse
and the figures were made on this branch earlier the same day and were
neither merged nor released, so their shape changed in place; nothing else
on the wire moved ([gateway
registry](gateway-registry.md#activity-and-the-public-figures)).

Design pass. The count is one more bounded integer on the gateway's own
row, written by the heartbeat's one statement under the heartbeat's
credential and read on the same public page. The questions were what is
new about it:
- **One thing about the counts can now be checked.** The players in a game
  are among the players connected, so `playing <= players`. It is refused
  where a count above the bound is refused: in `GatewayActivity::validate`
  (which the SDK calls before it sends and on what it reads, the store
  before its first statement and again on each row it hands out, and the
  website's reader on each entry) and in a CHECK of the table. A claim can
  no longer contradict itself. It stays a claim: an approved gateway can
  show any number up to the bound with any share of it in a game.
- **No way from three counts to four.** `playing` is required. An activity
  without it is not an activity, and the pulse that carries it is refused
  whole. An optional count read as zero was decided against: a gateway
  that does not say how many play would be shown with "0 in a game", a
  number nobody reported drawn as a zero, which the figures never do.
  Nothing sent the three counts: no gateway sends pulses yet.
- **The migration was edited, not followed by another.** 000025 gained
  the column and its CHECKs in place. That it had run on throwaway
  databases only is the brief's statement for this change and was not
  checked against any deployed database's migration history. A database
  that ran the first form has 000025 recorded and lacks the column; it is
  made anew ([PostgreSQL](postgresql.md#gateway-activity-migration-000025)).
- **What the figures give away grows a little.** For a reporting gateway
  anyone can now read how many of its connected players are in a game. On
  a gateway with one or two players that is closer to saying when those
  people play than the players' count alone was. It is still a count, and
  reporting is still the operator's choice
  ([threat model](threat-model.md)).
- Decided against: the rule folded into the bounds' CHECK (a CHECK of its
  own names the rule a refusal broke, and the store's test reads that
  name); Central working out who plays (it does not see tables); a share
  or a percentage in place of a count.

Implementation pass:
- `central-api`: `playing: u32` between `players` and `tables`.
  `validate` refuses a count above `MOST` and more `playing` than
  `players` (`playing` has its bound through `players`). No other type
  changed.
- Migration 000025: `activity_playing integer`; `gateway_activity_whole`
  counts five columns (none or all); `gateway_activity_bounds` holds the
  fourth count too; `gateway_activity_playing` is
  `activity_playing <= activity_players`. No grant changes
  (`ops/grants.sql` names no column of `gateways`).
- Store: the beat's `UPDATE` binds four counts (parameters 5 to 8),
  `counts` hands them over in that order after `validate`, and
  `gateway_figures` reads the column. A plain heartbeat clears all five.
- `central-identity` and `central-client`: no code changed; the pulse
  passes through the first, and the second's checks are `validate`.
- Website: `Activity` has `playing`, `fetch.rs` copies it, `activity_of`
  adds it up, and `home.rs` draws one more span after the players' in the
  strip and on a gateway's card (`live-playing`: German and English
  written, the nine other languages hold the English words until they are
  translated).
- No dependency changed (`Cargo.lock` is untouched), and nothing is logged.

Adversarial pass:
- **The wire.** The exact JSON of an activity, a pulse and the figures
  with four counts. Refused by the decoder: the three counts from before,
  `playing` alone left out of or doubled in an activity, a negative,
  fractional, too large, quoted or null `playing`, an extra field. More
  `playing` than `players` decodes and fails `validate`, by one and by
  many, inside the bound and outside it; nobody in a game and everybody in
  one pass.
- **The SDK.** `gateway_pulse` sends neither an over-bound activity nor
  one with more playing than players (the stand-in Central is not asked);
  `public_figures` refuses a page that holds one.
- **The store** (PostgreSQL 18.6): more playing than players is refused
  with the heartbeat's credential and with a `listing` token, and for the
  official gateway, and leaves the row and the token's last use as they
  were; four different numbers go in and come out in their places through
  every later step; everybody in a game at the bound is taken.
- **The table.** Written past the store, thirteen in a game of twelve
  connected is refused, and the error names `gateway_activity_playing`;
  twelve of twelve is taken and the store reads it. In the upgrade
  fixture each refused statement breaks one rule and no other: `playing`
  alone set or alone NULL, `playing` below zero, players lowered under
  playing, six in a game of five; five of five and none of five are
  taken.
- **Over real HTTP** (`central-client` against a real Central): with the
  heartbeat's credential, a pulse with more playing than players is
  refused when the SDK is asked to send it and when it is sent past the
  SDK; the same pulse written with three counts, as a gateway built
  before would write it, is refused, and no figure appears; the same
  bytes with the fourth count are taken, so it was the shape; then anyone
  reads the four counts on the native route. On the browser listener the
  four counts of a row written as the operator are read, each in its
  place.
- **The website.** The fixture's gateway is drawn with "8 im Spiel" in
  the strip and on its card on the German start page, and with "8 in a
  game" on its card in the English directory; neither appears while nobody
  reports. The strip's sum of the fourth count is tested with the three
  others.
- **The checks have teeth** (one change at a time, each undone): with the
  rule taken out of `validate`, `central-api`'s test and both of the
  SDK's fail; with `gateway_activity_playing` taken out of the migration,
  the upgrade fixture fails at its first statement of that kind; with two
  of the store's four parameters swapped, the HTTP scenario reads back the
  wrong activity; with the strip's span taken out, the pages' test fails.

Run on this change: `cargo xtask gate` passes (format, policy, crypto
scope, clippy, the workspace's tests, the doc tests, the release build), as
do `cargo xtask postgres` (PostgreSQL 18.6), `cargo test -p central-web -p
central-web-server -p xtask`, `cargo xtask site`, `cargo xtask smoke`,
`cargo xtask secrets` and `cargo deny check`. `cargo audit --deny warnings`
reports RUSTSEC-2026-0459 (`libsqlite3-sys` 0.37.0), as it did before this
change; `Cargo.lock` is untouched here. Clippy's limit of 100 lines moved
two parts of tests into functions of their own (`pulse` in
`central-identity`'s lists of requests, `activity_held` in the upgrade
fixture); no lint was switched off. The gates ran on the second of the two
commits; the first was checked on its own (`cargo check` over the
workspace's targets, and the website crates' tests, which it leaves at
three counts).

Not tested, and recorded for the owner:
- **The browser suites** (`npm run e2e:site`, `e2e-real`) were not run:
  other sessions held their ports. The site suite's specs were read: none
  names the figures' words or counts the pill's parts, and it draws its
  pages from the fixture that now holds four counts. In the suites' place
  the built site was served once with that fixture on a loopback port of
  its own and opened in headless Chromium. At the suite's six widths (320
  to 2560) and two heights, the German, English, Russian and Japanese
  start pages and the English directory need no sideways scrolling, which
  is the suite's own measure; the strip was looked at in German at 320
  and at 1440: four parts, two lines of two on the phone, one line on the
  desktop. Nothing else of the suites was repeated (contrast, the other
  languages' pill, the card's line by eye). The stylesheet was not
  changed, and its comments still say three counts (`gateways.css`,
  `header.css`). The server was stopped and its port seen closed.
- **No gateway sends the count.** The game's gateway does not send pulses
  yet (its repository was searched for one and holds none); when it does,
  it has to send all four.
- **The website's reader against a Central that answers more playing than
  players** has no test of its own, as it has none for the bound: it calls
  the same `validate`, and the reader is not built to be asked without a
  Central.
- **The privacy page's sentence** still says a gateway may tell Central
  "how many players, open tables and games it has". The fourth count is a
  count of players, so the sentence was left; whether it should name the
  players in a game is the owner's to say.
- **The nine translations** of `live-playing` are the English words.
- **A database that ran the first form of 000025** was not tried against
  this build; that it fails there follows from the statements.

Engineering review, not an audit.

## The site's count of page views, and signing in as a dialog, 11 November 2026

The owner asked for the number of page views back in the start page's strip,
and for signing in and registering to open as dialogs over the page. Both
touch what the website's server and the browser hold, so both went through
the three passes.

### The count of page views

Design pass. The old site showed a number a third party counted. The new
site has no third party, so the number is the site's own or none.
- **What is kept.** One unsigned number for every page and every visitor
  together, and the day it began. Nothing of a request: no address, no
  cookie, no identifier, not which page, not when. The privacy notice says
  so, and says it only where a deployment counts (`privacy-views`).
- **Opt-in.** `Config::views` names a file; without it nothing is counted,
  nothing is shown and the server writes nothing anywhere, as before. The
  development and test configurations do not count; the tests' fixture
  shows a fixed number, so their pages stay the same bytes.
- **The server's first writable state.** The file is the one thing this
  server may write. It is read with a bound (1024 bytes), parsed with
  unknown fields refused, and a file that is not this server's own stops
  the start instead of being overwritten. It is written beside itself and
  renamed over, with the bytes synced first, at most once a minute and
  only if the number grew, and once more when the server stops.
- **What counts.** A request the server answers with a page (status 200)
  whose headers are a browser's navigation to a document
  (`Sec-Fetch-Mode: navigate`, `Sec-Fetch-Dest: document`) and not a fetch
  ahead of a visit (`Sec-Purpose`, `Purpose`). A crawler or a script that
  does not send these is not counted.
- **How it reaches a page.** Through the snapshot the pages are drawn
  from, once a minute, never per request. A visit does not change the
  page it gets, a page's entity tag changes with the data only, and
  nobody can watch a single request move the number.
- Decided against: a count per page or per day (a history, and with few
  visitors close to saying when somebody read what); unique visitors (needs
  an identifier or an address); counting in Central (the site's visitors
  are no business of the account service); a cookie to count a person once.

Implementation pass. `central-web-server/src/views.rs`; the counter is an
`AtomicU64` raised with `checked_add`, so it stays at its end instead of
wrapping. The date is computed from the system clock without a new
dependency. No header value is stored or logged; a header that is not text
counts as absent. Saving runs on the blocking pool, off the request path.

Adversarial pass.
- *Can a request make the server write?* Only the number, at most once a
  minute, to the configured path. No request data reaches the file's name
  or content.
- *Can the number be inflated?* Yes, by anyone who sends the two headers:
  they are the browser's word, and a program can say them. The number is
  therefore a count of requests that look like visits, not of people, and
  the strip calls it page views. It is bounded by what the server answers
  and by the proxy's limits; it cannot overflow. Accepted: it is a figure
  for the curious, and no decision of the system reads it.
- *Does it leak a visit?* The number moves once a minute by the sum of all
  visits. On a site with almost no traffic an observer who polls it learns
  that some page was opened by somebody within a minute, not which or by
  whom. Accepted, and said in the threat model.
- *The file as an attack surface.* A larger or malformed file stops the
  start (tested). `File::create` follows a symbolic link, so a directory
  that another account can write would let that account aim the server's
  write at another file the service may write. The runbook therefore
  requires the service's own state directory; with the unit's sandbox
  (`ReadWritePaths` naming only that directory) there is no other file to
  aim at. Open: not enforced in code.
- *A crash between write and rename* leaves `<file>.new` and the old file;
  the next start reads the old file and the next save replaces the
  leftover. At worst a minute's growth is lost.
- Tests: `views.rs` (what is a visit, the file's forms, the write),
  `tests/pages.rs` (`a_visit_counts_once_and_never_changes_the_page_it_gets`,
  `a_views_file_that_cannot_be_used_stops_the_start`,
  `page_views_are_shown_and_told_only_where_they_are_counted`).

### Signing in and registering as dialogs

Design pass. Every page that is not itself a sign-in, registration or
account page now carries the two forms in closed `<dialog>` elements; a
link to `/signin` or `/register` opens the dialog where script runs and
leads to the page where it does not. The forms are the pages' own islands,
so there is one implementation of each flow.
- **No new path for a secret.** The dialogs post nothing themselves; the
  islands talk to Central as on their pages (OPAQUE through the
  WebAssembly, the registration proof, the same origin checks). The CSP is
  unchanged: no inline script or style was added, and the dialog is opened
  by `boot.js`.
- **WebAssembly only when needed.** A closed dialog's islands are not
  hydrated; `islands.js` hydrates them when the dialog first opens. A
  visitor who never opens one loads no WebAssembly, as before.
- **A password must not travel in an address.** Without script a form's
  default submit would put its fields in the URL. The hold-back that
  prevents this is now per island (`data-hydrated`), not per page: a
  form inside an island that is not yet hydrated does not submit at all.
- **The registration's fill-time check** counts from when the form became
  usable; hydrating on open keeps that the time the person first saw it.
- **A password shown by the eye** is the field's type changed in place;
  nothing copies the value. The eye is a button with `aria-pressed`.
  Sending the sign-in form hides its field again, so a failed try does not
  come back with the password's field readable; the registration's fields
  leave the page with their form.

Adversarial pass.
- *Can a page open the dialog over something and take a click?* The dialog
  is modal (`showModal`), the page behind is inert, and it is opened only
  by a same-origin link whose path is exactly a language's `signin` or
  `register`. A link with a modifier key or a new-tab target is left alone.
- *Can a third party frame the site and draw over the dialog?* Framing is
  refused as before (`frame-ancestors 'none'`).
- *Does the hidden form change what a password manager fills?* The forms
  carry the same `autocomplete` values as the pages; a closed dialog is not
  rendered, and browsers do not fill what is not rendered. Not tested
  against individual password managers.
- Tests: `web/e2e-site/dialogs.spec.ts` (opening fetches the WebAssembly
  only then; the eye; axe and 44 px targets inside the dialog; an empty
  form is refused; Escape, the button and the veil close; the pages
  themselves carry no dialog; without script the link leads to the page).

Open after this section: the count's file is trusted to live in the
service's own directory; the dialogs were not tried with password managers
or on a phone.

## An administrator's change and the store's four connections, 11 November 2026

Found in a review of everything a stranger, a signed-in account or a
registered gateway can reach, asked for by the owner on 11.11.2026
(§"The review's other findings" below lists the rest).

Design pass. The store's pool has four connections and waits three
seconds for one (`connection.rs`). Each of the administrators' five
changes (`set_admin`, `set_tag`, `set_registration`,
`set_gateway_approval`, `admin_revoke_gateway_token`) began its
transaction, which takes a connection, and then asked `acting_administrator`
who was acting. That asked the pool (`&self.db`), not the transaction: a
second connection while the first was held. Four changes at once each
held one and waited for a second that none of them would give back until
the pool's three seconds were over. For those seconds every request of
the store waited too, and then each change answered `Unavailable`.
- **Who can ask.** `Admin` is the browser listener's request and needs a
  browser session with Central's terms accepted. It does not need the
  role: the role is what that first reading decides. So any registered
  account could do it. An address has four requests in flight there, which
  is exactly the pool, and `Admin` has no budget by address. One browser
  session, four requests every three seconds: sign-in, the chat's relay,
  every gateway's check of its players and the website's account pages
  answer 503 for as long as it goes on. High: it needs an account and
  nothing else, and stops the whole service.
- **The answer** keeps the order of the checks and moves where the first
  one is asked: before the transaction begins, on a connection that goes
  back to the pool before the next is taken. Anyone who is no
  administrator is refused there and never holds a transaction. The
  transaction, the lock on the administrators' account rows and the
  second reading after the lock, which is the one that decides, are as
  they were, all on the transaction's own connection.
- **The wire** is unchanged. A stranger's change was `Rejected` before
  (when the pool had room) and is `Rejected` now.
- Decided against: a larger pool (the wait scales with it, and five
  requests then do what four did); dropping the first reading (a stranger
  would then take the lock on the administrators' rows before being
  refused); a budget by address for `Admin` alone (it would hide this
  cause, not remove it).

Implementation pass:
- `accounts/admin.rs`, `acting_administrator`: takes the session alone and
  returns the transaction with the acting account. It reads the role on
  the pool, begins, locks, reads again on the transaction and refuses a
  sign-in older than 30 minutes, as before. The five changes take their
  transaction from it; none begins one of its own, so the order cannot be
  got wrong again in a sixth.
- Looked for the same shape elsewhere: every function of
  `central-postgres/src` that begins a transaction was searched for a use
  of the pool before its end. None: `publish_gateway_terms` rolls its
  transaction back before it asks the pool, `issue_reset` asks first and
  begins after.
- Nothing is logged; no statement changed.

Adversarial pass, on PostgreSQL 18 (`cargo xtask postgres`,
`tests/support/accounts/admin.rs`, `crowd`):
- A session of an account without the role asks for eight changes at
  once, twice the pool, each of the five among them, while the same
  session reads its account summary. Every change is `Rejected`, the
  summary is answered, and all of it takes under two seconds.
- Before the fix, the same test failed, and that run is the finding's
  evidence: `assertion left == right failed: refused, not unavailable
  left: Err(Unavailable) right: Err(Rejected)` at
  `tests/support/accounts/admin.rs:94`, after the pool's three seconds.
- An administrator's own eight changes at once (four that are allowed,
  four that the rule against removing one's own role refuses): four done,
  four `Rejected`, none `Unavailable`. Changes still wait for each other
  on the lock, each on one connection.
- The earlier tests of this file (stale sign-in, self and last
  administrator, two removing each other, the audit's rows) pass
  unchanged.

**What stays possible:**
- Four connections are few. Requests that are slow in the database for
  another reason (a statement may run five seconds) still hold them; what
  each caller may ask is bounded by the budgets in
  [operations](operations.md), not by the pool.
- An administrator's changes wait for one another on the lock, as
  designed.

Engineering review, not an audit.

## One session and the budget all sessions share, 11 November 2026

Second finding of the same review.

Design pass. A request with a session is counted twice
(`central-identity/src/limits.rs`, `Limits::request`): against a window all
sessions share (512 control requests a second, 256 data) and against its
bearer's own (128 and 64). The shared window was charged first. A bearer
past its own budget was refused, as intended, but each refused request had
already taken one of everyone's. One session asking for `SessionStatus`
512 times a second was itself served 128 times and left nothing: every
other client's status, renewal and logout, and every gateway's check of
its players, answered `Busy` for as long as it kept asking. Medium: it
needs one registered account (or, with the next section's finding, none)
and one address, and stops everything that needs a session, though not
sign-in.
- **The answer** is the order: the bearer's own budget is looked at
  first, without spending; only a request that fits it draws on the shared
  window, and then on its own. A refusal spends nothing anywhere, as the
  budgets for callers without a session already do.
- **The wire** is unchanged: the refusal is the same `Busy`, for the same
  requests of the same bearer, in the same second.
- **What it does not answer:** four bearers that each keep within their
  own budget spend a shared window between them (4 × 128 = 512, 4 × 64 =
  256). That is the next section and the review's open points.

Implementation pass:
- `Window::fits` reads a window without spending. `Limits::request`
  forgets the bearers of earlier seconds, as before, refuses a bearer
  whose own window is spent or for whom the table of bearers has no room,
  then charges the shared window, then the bearer's own.
- Nothing else moved: which requests are control and which data, the
  numbers, and the requests that never count as a session's.

Adversarial pass (`limits.rs`,
`one_bearer_past_its_own_budget_spends_no_one_else_s`), for both kinds of
request:
- One bearer asks for a whole shared window in one second. It is served
  its own budget and no more; three other bearers are then served their
  whole budgets in the same second; a fifth is refused, the window being
  spent by requests that were served.
- Before the fix the same test failed at the first other bearer:
  `another session is served: Busy`.
- The module's earlier tests pass unchanged.

**What stays possible:** four bearers, as above; and a bearer nobody
issued still counted as a session (the next section).

Engineering review, not an audit.

## A bearer nobody issued, 11 November 2026

Third finding of the same review. [Operations](operations.md) had recorded
it that day as known and unchanged; this changes it. The document is
[native login](native-login.md#a-bearer-nobody-issued).

Design pass. A request that needs a session is counted before it is
served, and the store is asked about its token only when it is served.
That order is right: the count is what bounds the store's work. But the
count took every token for a session's. Any request with a made-up token
drew on the windows all sessions share (512 control requests a second, 256
data) and took a place in the table of 1,024 bearers a second.
- **Who can ask.** Anyone, without an account. On the website's listener
  the requests that act on the cookie's session have no budget by address
  (an address has four in flight, nothing more); on the account listener
  they have none either. One address sending `SessionStatus` with a new
  made-up cookie each time spent a window in a fraction of a second, every
  second. Sign-in went on, having a budget of its own; a signed-in client
  could do nothing with its session, the website's account pages failed,
  and a gateway's checks of its players were refused. Medium: no account
  and one address, and it stops everything behind a session.
- **The answer** is to remember what the store has already said. A bearer
  is counted as a session's once an answer has vouched for it: the sign-in,
  password change or pairing that issued it, or an answered request that
  the store refuses for any token it does not know. Every other bearer is
  served as before, since it may be a session Central has not heard from
  since a restart, but out of a second of its own: 64 requests for all of
  them together, 600 a minute for one place. It spends nothing of the
  sessions' windows, takes no place among their bearers, and nothing of
  the sign-ins' 32 a second either.
- **It fails closed.** The requests that vouch are a short list, each read
  in `central-postgres` to refuse an unknown token; a request that is not
  on it, and any request added later, vouches for nothing until someone
  puts it there. Two requests are answered alike for any token (`Logout`,
  `PairCancel`): they vouch for nothing and end it.
- **It is taken back.** A session that was replaced, ended or expired was
  vouched for once. Four `Rejected` answers in a row take that back, so
  each such token has four requests among the sessions and no more; a
  logout, a password change's finish and a deletion's finish take it back
  at once; and it lapses 30 minutes after the last answer that vouched.
- **It grants nothing.** The list is read by the count alone. Dispatch and
  the store check every request exactly as before; a token on the list
  that the store no longer knows is refused like any other.
- **The numbers.** 64 a second is what the book of places is sized for
  (`PLACES`, asserted). 600 a minute for a place is the share a place
  already has of the sign-ins' budget. After a restart each session needs
  one request among the strangers; 64 a second serves 3,840 sessions a
  minute. 30 minutes is a session's life between two renewals, and the
  client renews every ten. 16,384 bearers are about a megabyte.
- **The released client and gateway** send and receive what they did. A
  `Busy` they did not get before is possible only for a bearer that is not
  vouched for, when more than 64 such requests arrive in one second or
  more than 600 in a minute from its address: without a flood, a restart
  with thousands of clients asking in the same second. The client asks a
  `Busy` again three times within about a second. With a flood of made-up
  bearers, that bearer waits where before the flood every bearer did.
- **Addresses.** One more count by client address, in memory, for a
  minute: the requests whose bearer is not vouched for. It is the kind of
  count a place's 600 a minute without a session already is. The privacy
  notice's sentence lists what is counted by address in a bracket that
  names neither (nor the pairing and report budgets the website's listener
  has had); whether that sentence wants another word is the owner's.
- Decided against: counting such requests against the sign-ins' budget (a
  flood of made-up tokens would then stop sign-in, and every restart would
  set all sessions against those signing in); a share per address of the
  sessions' windows for every bearer (a gateway makes all its calls from
  one address: a limit that can keep a gateway out, so the owner's, and
  listed as open); asking the store before counting (the work the count
  bounds); letting every answered request vouch (a request added later
  that answers alike for any token would vouch for made-up ones).

Implementation pass:
- `central-identity/src/limits.rs`: `class` holds the three lists that
  were in `Limits::request`, unchanged; `tells` is the list of requests
  that vouch and the four that end; `Proven` is the table (digest, time of
  the last vouching answer, refusals in a row; swept once a minute; full,
  it forgets the one whose last such answer is the oldest; the bridge's
  digest is kept apart, compared in constant time, and never forgotten).
  `Limits::request` sends a bearer that is not held to the strangers'
  window and its place's share, and nothing else changed for one that is.
  `told` is asked before a request is served and `answered` after it.
- `central-identity/src/lib.rs`: both entry points
  (`handle_from`, `handle_browser_from`) call `told` before the budgets and
  `answered` after dispatch, with the same token the budgets counted (on
  the website's listener that is the cookie only for a request that acts
  on a session). `issued` reads a session out of the answer itself
  (`Session`, `PairSession`, the browser reply's cookie), so every path
  that hands out a session is covered in one place. A poisoned lock
  changes no answer.
- Only digests are held, in memory; nothing is logged.

Adversarial pass.
- With a clock of its own (`limits.rs`): 2,000 made-up bearers in a second
  from one place pass 64 times, and nothing more passes from 198 other
  places or from none; a session beside them is served its whole 128 and
  64, the table of bearers holds its two entries only, and those signing
  in have their whole 32. One place passes 600 in a minute and no more,
  while another place, a request without a place, a sign-in from the same
  place and a session there are served. An answer vouches only when it is
  one on the list and not a refusal; three refusals leave a bearer, a
  vouching answer starts the count again, the fourth in a row takes it
  back; other errors count for nothing; each of the four endings takes it
  back at once; a public request or a pairing's says nothing about a token
  sent along. The half hour, to the second, and that a lapsed session is
  served as a stranger and vouched for again by its answer. The bridge:
  from the start, through refusals, a logout and ten half hours. The
  table: full, a known bearer needs no room, one more forgets the oldest;
  half an hour on everything is swept.
- On PostgreSQL 18 through the service the listeners serve
  (`central-client/tests/support/strangers.rs`): an account's website and
  native sessions, signed in over the real listeners and never yet
  answered; then 900 made-up bearers from one address in one instant (600
  control, 300 data, on both listeners) and after them the two sessions'
  four requests. All four are answered; no made-up bearer is answered
  anything; a sign-in's request is served beside it. Each request on the
  list, with a token nobody issued, is `Rejected` by the store (18 on the
  account listener, 6 on the website's; a pulse is the heartbeat's path).
  A second service on the same store stands for Central after a restart:
  the website session's first request is served and vouches for it, then
  900 made-up bearers from sixteen addresses leave its two requests
  answered, while the native session, not heard since the restart, is
  `Busy` in that second and served in the next. Four sessions replaced by
  later sign-ins are refused four times each; then each asks for its whole
  128 in one instant and the live session is served beside them.
- Seen failing first, on the tree before this change: `the holder's
  request 0 beside the flood: Some(Busy)` at `strangers.rs:246`.
- That the fixture watches the wiring and not only the count was tried
  by switching off one piece at a time and running it again; each run
  failed where that piece is needed: no vouching at issue, at `the
  holder's request 0 beside the flood: Some(Busy)`; no vouching by an
  answer, at `the session that was heard`; no count of refusals, at `the
  live session beside four ended ones: Some(Busy)`.
- Tried and held: a public request with a token cannot vouch for it (it is
  not counted by the token, so it tells nothing); a native bearer shown as
  a cookie, or a cookie as a bearer, is refused by the store and vouches
  for nothing; signing in again and again to collect tokens that were
  vouched for once gives each four requests; filling the table needs
  16,384 sessions that the store issued within half an hour, which the
  sign-in budgets hold to 30 a minute an address, and a session pushed out
  is vouched for again by its next answer.

**What stays possible** (for the owner; the review's last section has the
options):
- An account is enough again: four vouched-for bearers that each keep up
  their own budget fill a shared window. Two accounts' four sessions are
  four, and one account alone can hold more (two sessions, and up to
  eight gateways with eight tokens each, which `GatewayStatus` answers
  while the gateway awaits approval). From one address. This is narrower
  than the finding, not its end.
- Seven addresses of made-up bearers fill the strangers' second all the
  time, one alone for nine or ten seconds of each minute. A session that
  is not vouched for then waits: after a restart, after half an hour
  without a request, after four refusals in a row. (As first built, a
  gateway whose players presented bad tickets could be made a stranger by
  those players. Since 12.11.2026 a refusal of a request that carries a
  player's lease or ticket is not counted against the gateway that asked;
  the section on the review's leads has it.)
- A request that reaches the account listener without a proxy's address
  has no place: it meets the 64 a second alone.
- A restart forgets the list; every session is a stranger once.
- Central was not flooded over a socket here; the fixture asks the service
  directly, so the listeners' own limits are not part of what was shown.

Engineering review, not an audit.

## The review's other findings, 11 November 2026

The owner asked on 11.11.2026 for Central to be checked for
vulnerabilities and for those to be closed. The three sections above are
what was closed, each with a test that failed first. This section is the
rest: one small budget that was added, what is open and whose decision it
is, what is low, what was read and found sound, and what was not read.

**How.** Everything a stranger, a signed-in account or a registered
gateway can reach was read: `central-http`, `central-identity`,
`central-postgres` with `ops/grants.sql`, `central-web-server`,
`central-service`, `ops/` and what [operations](operations.md) prescribes.
The store's authorization, the administration with the reports, and the
website's server with the service and the deploy hook were each read a
second time by a separate pass of the same model with nothing but the
source; their leads were checked against the code before they were
written down here, and where that was not done the entry says "lead".
Nothing was run against a deployed Central, nothing against an account
that is anyone's. Out of scope and not edited: the website's look and
words, `central-api/src/gateway.rs` and the gateway's pulse,
`central-crypto/src/proof.rs`.

**Added besides** (low). The account listener gave `GatewayDirectory` and
`PublicFigures` no budget by address; a place's 600 a minute was all that
held one address. They now share 60 a minute an address there, the number
the website's listener has (`central-http/src/addresses.rs`, a test that
passed 120 before). The released client asks neither; the website's
server reads both through the website's listener. And
[reports](reports.md) said the log names a report's id and gateway:
Central writes no such line, and the sentence now says so.

**Open, medium or worse.** None of these was built: each needs a number,
a trade or a server's configuration that is the owner's.

*12.11.2026: the owner decided for the recommended variant of each, and
they were built that day, but not all of each: point 6's deleting on
acknowledged delivery was not built, the limit of points 2 and 4 is a
file for the firewall that was written and not loaded, and point 3's
refusal stays the owner's to decide. The titles are struck and each
point names what was built;
the rest of the list is as it was written. What stays possible of each,
which is not little, is in
[the section of 12 November](#the-seven-open-findings-12-november-2026).*

1. ~~*Four vouched-for bearers spend what sessions share*~~ [**Built
   12.11.2026:** (a) at half a window and (b) by owner, point 1 there. Two
   addresses with two sessions each still do it.] (what stays of
   the section above). Four bearers at their own budget fill a window
   (4 x 128, 4 x 64). Two accounts have four sessions between them; one
   account alone holds two sessions and up to 64 gateway tokens (eight
   gateways, eight tokens each, 32 made in a day). From one address, with
   two accounts or with one.
   - (a) A share for one place of the sessions' windows, as the sign-ins'
     budget has: at one bearer's budget (128 and 64 a second) it changes
     nothing for a gateway alone at its address and holds one address to
     a quarter; four addresses then do what one does now. Two gateways, or
     very many clients, behind one address would share it: a limit that
     can keep someone out.
   - (b) Count a gateway's tokens together, by gateway or by owner. No
     address is involved and nobody honest is refused; it needs the store
     to tell the count whose a token is. It ends the way with one account
     only: two accounts' four sessions stay.
   - (c) A rate limit by address in front of both listeners (below).
   - Recommended: (a), at half a window rather than a quarter where two
     gateways may share an address, and (b) besides. Neither ends it for
     four addresses with an account each: a shared window is four
     bearers' budgets wide, and how much wider it may be is a question of
     what the store's four connections serve.
2. ~~*The 32 a second of those who have no session*~~ [**Built
   12.11.2026:** (a) as a file for the host's firewall, which the operator
   loads, and (c), point 2 there. Four addresses still spend the 32; those
   who ask little are served beside them.] is spent by four
   addresses that each keep up their 600 a minute (known since
   11.11.2026). Sign-in, registration and the public pages' data are
   `Busy` meanwhile.
   - (a) A limit on new connections by source address in the firewall in
     front of Caddy (nftables has one; Caddy's own build has no rate
     limit, to this review's knowledge, and was not run here). It answers
     this, the next two points and floods of whole requests at once, and
     costs no code. An IPv6 subscriber must be counted by its /64, as
     Central does.
   - (b) A smaller share for a place (240 a minute needs ten addresses,
     not four). A campus or a carrier's shared address signs in through
     the same share: a limit that can keep people out.
   - (c) Part of the second kept for places that have asked little in the
     minute, so that a household still signs in while a few addresses
     flood. More code; many addresses still win.
   - Recommended: (a) now, (c) when there is time.
3. ~~*A request that reaches the account listener without the proxy's
   address*~~ [**Built 12.11.2026:** the check kept and made exact, and
   the deployed listener counts such requests and says so in its log,
   point 3 there. They are still served; the refusal is still the owner's
   to decide.] is counted by no address (known). The listener binds
   loopback, so this is a process on the host, or a proxy that stopped
   sending the header. The runbook's check after every proxy change (31
   `Discover` in a minute must end in 429) is the control. Refusing such
   requests would refuse the official gateway on the same host and every
   local run; an option is to refuse them on the deployed listener unless
   the bearer is the bridge's. Recommended: keep the check, and decide
   the refusal together with point 2.
4. ~~*The account listener's connections*~~ [**Built 12.11.2026:** the
   firewall's file bounds the connections one source opens and holds,
   point 4 there. It does not end this point: eight addresses still hold
   the places for late and large bodies.] (128, a quarter of them for late
   and large bodies) can be held by many addresses (known). Recommended:
   the firewall's limit of connections per source, with point 2.
5. ~~*Reports.*~~ [**Built 12.11.2026:** (a) with four slots, (b), (c)
   without a migration and (d), point 5 there. Two approved gateways can
   still hold all four slots for a quarter of each hour.] It needs an
   approved gateway. Two things:
   - *Stored bytes.* A gateway may file 120 reports an hour, each with up
     to 66 MiB of parts (one of 2 MiB and one of 64,
     `central-api/src/reports.rs`): about 7.7 GiB an hour from one.
     Nothing bounds what one gateway, or all, have stored; the hour's
     count is in memory and a restart starts it again.
   - *The two upload slots.* A part that was never stored may be sent
     again, and twice at once: one report an hour and two connections
     that keep the pace (16 KiB in five seconds) without ever finishing
     hold both slots, a minute at a time. Every other gateway's parts are
     answered 429 meanwhile; the released gateway sends a part five
     times, with about 70 seconds of waiting in all, and then leaves the
     report without it.
   - (a) One upload slot for one gateway was this review's first thought
     and is not compatible: the released gateway sends two parts at once
     and says it chose two because Central takes two (`baylee-gateway`,
     `report/central.rs`, `UPLOADS`). With one slot it would be refused
     by its own other upload, and with several reports waiting a part can
     use up its five sends. What is compatible: more slots in all and at
     most two for one gateway. Each slot is up to 64 MiB of memory, so how
     many is the server's to say; two approved gateways then hold what one
     holds now.
   - (b) An hour's count for a gateway of uploads that ended unfinished;
     past it, `Busy` for its uploads. It needs a number that a gateway on
     a poor line, or one caught by a restart, never reaches.
   - (c) A bound in the store on the bytes a gateway has stored (a
     migration and a number), refused with `Busy`.
   - (d) A bound on all stored report bytes.
   - Recommended: (c) and (b) with the owner's numbers; (a) where the
     server's memory allows it.
6. ~~*Chat.*~~ [**Built 12.11.2026:** the share, and a group message
   that skips a full device, point 6 there. The deletion was not built:
   the wire has no acknowledgement. Two contacts still fill an inbox for
   good.] A device's inbox and a sender's outbox hold 5,000 deliveries
   and nothing deletes one that was delivered; a send to a group is
   refused whole while one recipient's device is full, so a contact can
   fill an inbox and keep others from being heard. [Central
   chat](central-chat.md) names retention as the owner's pending
   decision; this is what it costs meanwhile. Options: delete on
   acknowledged delivery; a time limit; a share of an inbox for one
   sender. Recommended: the share and the deletion.
7. ~~*The deploy hook*~~ [**Built 12.11.2026:** the first two, point 7
   there. The binary and the website are still what the build made.]
   (`ops/deploy-hook`; not reachable from the internet).
   Root builds as the build user in the source tree and then takes
   `ops/grants.sql`, the binary and the website from that same tree; the
   grants go to the database's superuser. Whatever ran in the build (a
   dependency's build script, `npm ci`'s install scripts) could have
   rewritten them. Options: apply the grants as the migrator, not the
   superuser; read `grants.sql` from the commit that was verified, not
   from the working tree; build as a user without a home or keys;
   `npm ci --ignore-scripts`. Recommended: the first two.

**Low** (listed, not changed):
- `central-api/src/gateway.rs`, `validate_origin`: a host with a trailing
  dot is another origin, so a look-alike of a registered origin can be
  registered (an administrator's approval is the gate). In a file this
  review was told not to edit.
- The website's report-part route passes no budget of the account service
  and no terms check, only the listener's 120 a minute an address.
- `reset_start` spends the account's unit before it holds a worker.
- A push subscription moves to the same browser's next session; a stolen
  cookie's subscription outlives a sign-in (lead).
- `invalidate_legacy` leaves the key check in place (the bridge's path).
- `redeem_pairing` does not ask again whether the session that began the
  pairing still lives (lead).
- Removing and blocking a contact answer differently for a tag nobody has,
  with no budget of their own for looking tags up (lead).
- A gateway's origin is taken by whoever registers it first, with no proof
  of holding it; `Conflict` and `Busy` tell registered origins apart, and
  `Register`'s `Conflict` a pending name.
- 1,000 open pairings for everyone: about 334 accounts reach it (lead).
- A reset code made for an account before it was appointed administrator
  stays valid (lead).
- Triage: the role is read once in a transaction; `DELETE` on
  `report_parts` is granted and unused; a gateway's credential pair files
  reports without the `sso` scope, where the document says token; the
  sweep inside every administrator's read is unbounded; sixteen labels of
  32 characters pass the request's check and fail the column's, answered
  503; a date that is none is answered 503, not `Rejected` (leads).
- [Operations](operations.md): the new site block gives `/admin*` no
  security headers from `file_server`; the feedback service is sent the
  session cookie; two `/api/*` blocks disagree on the body's size; the
  runtime's and the migrator's runs share one system user; HSTS without
  `includeSubDomains` (leads).
- `central-web-server`: no bound of its own on connections and no header
  timeout; the timeout's answer passes outside the security headers;
  release links are held to a prefix only and a tag may be `.` or `..`; a
  404 without a language varies by `Accept-Language` without saying so;
  no panic hook; `session::go` refuses `//` and not `/\` (leads).
- The deploy hook leaves the service stopped after some failures, rolls
  back the binary only, takes no lock and builds as a person's login
  (leads).
- The privacy notice's sentence on counting by address names four kinds
  of request in a bracket; more are counted (a place's share of every
  request without a session, pairing, report parts, and since this review
  the requests whose bearer is not vouched for).

**Read and found sound.**
- *Store.* Every id a caller chooses is bound to the token's principal;
  no read or write across accounts was found. Sessions, tokens, leases
  and tickets are bound to account, gateway and listener; password
  change, reset and deletion check again under a row lock; a reset code
  is used once; the key check is written once behind a fresh proof;
  objects and documents have quotas and compare revisions; push endpoints
  are held to three hosts, https, no redirects and public addresses.
  Statements are constants with bound parameters; none is built from a
  caller's input. No function but the one fixed above asks the pool while
  it holds a transaction.
- *Administration.* The role is read at every request; a change needs a
  password sign-in under 30 minutes and only those count; the last
  administrator stays; the audit's grants are append-only and checked at
  start; no database error's text reaches a caller.
- *Identity.* Dispatch checks the listener, then the terms, then the
  operation's own credential, for every operation; OPAQUE states are
  bounded, bound to listener, session and purpose and used once; the
  proof of work's table is sized by its budget; nothing is logged.
- *Listeners.* One request a connection, fixed texts, bounded and paced
  bodies; the forwarded address is believed from a loopback peer only and
  shares out places and budgets, never identity.
- *Website's server.* No request's path reaches the file system;
  redirects are relative and under known languages; the content policy
  with Trusted Types; the cache's key and `Vary` agree; the fetcher asks
  fixed hosts over TLS without redirects, bounded in size and time;
  gateway data is validated and escaped before it is rendered.
- *Service.* Secrets on standard input only; loopback and `verify-full`
  enforced; fixed texts on standard error; a panic prints no payload.

**Not read or not run.** `central-crypto` and the browser module; the
website's pages and islands; the SDK beyond what the fixture uses; the
dependencies' own code; Caddy, the firewall and the machine that serves
`baylee.app`; any flood over a socket; the released game client and
gateway against this tree (their requests and answers are unchanged by
construction: no type of `central-api` was touched).

**Run.** Each fix's own tests are in its section. The six checks
`AGENTS.md` names (`cargo xtask gate`, `postgres`, `smoke`, `secrets`,
`crypto-scope` and `cargo deny check`) are run on the branch's last tree,
which holds this section, before it is handed over; their last lines are
in the review's hand-over note and not here, since this text is part of
the tree they run on. `cargo audit --deny warnings` fails as before, on
RUSTSEC-2026-0459 alone (`libsqlite3-sys`); `Cargo.toml`, `Cargo.lock`
and `deny.toml` are untouched.

Engineering review, not an audit.

## The review's leads on the website's server and on origins, 12 November 2026

The section above lists what a second reading reported and nobody had
checked ("leads"). Those that touch the website's server, the islands'
redirect, the runbook's site block and a gateway's origin were checked
against the code on 12.11.2026, on the pull request's branch. What held
was fixed; what did not is said.

**Design pass.** Each lead was read as a claim about one function and
checked there before anything was changed.
- *The deadline's answer carried no security headers.* True: the timeout
  layer was added after the header layer and so stood outside it; a 503
  from the deadline left without the content policy. The body is empty,
  so nothing could run in it, but "on every answer" was not true.
- *A 404 under no language depends on `Accept-Language` and did not say
  so.* True. The answer is `no-cache` with an entity tag per language, so
  a cache had to ask again and would have been corrected; the header was
  missing all the same.
- *Release links were held to a prefix.* True: an address that begins as
  Baylee's and climbs (`…/releases/download/../../../other/…`) passed,
  and a browser resolves it to another repository. The addresses come
  from GitHub's API for Baylee's repository over TLS, so this needs
  GitHub's answer to be wrong; the check is cheap.
- *`session::go` refused `//host` and not `/\host`.* True, and not
  reachable: every caller passes a path built from a known language code
  and a fixed word. Fixed as a guard for the caller that comes later.
- *No panic hook.* True. The process holds no secret; a panic's text can
  hold a header or a path of the machine.
- *A host with the root's trailing dot is a second origin.* True
  (`central_api::gateway::validate_origin`): `https://host.` passed and
  is the same host as `https://host`. An approval still stood between
  such a registration and the directory.
- *The feedback service is sent the session cookie.* True of the
  runbook's block: the cookie is `Path=/` (`__Host-` requires it), so the
  browser sends it to `/site/impressum*` and `/site/contact`, and the
  block removed it only on the way to `central-web`.
- *The two `/api/*` blocks disagree on the body bound; `/admin*` from the
  old build has no headers.* True of the text: the new block had no
  `request_body`, and the note on `/admin*` did not say that the old
  block's headers must come with it.
- *No bound on connections and no header timeout of its own.* True and
  left: the listener is refused unless it is loopback
  (`Config::validate`), Caddy stands in front of it and bounds both, and
  `axum::serve` offers neither without a server of our own under it. Open
  below.

**Implementation pass.**
- `routes::router`: the header layer is the outermost, the deadline
  inside it.
- `routes::answer`: the last case sets `Vary: Accept-Language`.
- `fetch::below`: an address is `prefix` and then exactly the expected
  number of plain segments (letters, digits, `. - + _ %`), none of them
  `.` or `..`, nothing after them; a release's page is
  `…/releases/tag/<tag>`, a file `…/releases/download/<tag>/<name>`.
  `plain_tag` refuses `.` and `..`.
- `islands::session::local`: one slash, then neither a slash nor a
  backslash; `go` asks it.
- `central-web`'s `main` sets a panic hook that prints a fixed line, as
  `central-service` does.
- `validate_origin` refuses a host that ends in a dot. The function is
  shared with the game's client and gateway through `central-api`: no
  type or signature changed; an origin with a trailing dot, which no
  gateway should have been given, is now refused where it was accepted.
- `docs/operations.md`: `header_up -Cookie` on the way to the feedback
  service, the 64 KiB bound on `/api/*`, the old block's headers named
  for `/admin*`, and that HSTS is sent without `includeSubDomains`.
- The privacy notice's sentence on counting by address (German and
  English by hand) no longer reads as a complete list of four: it says
  that Central also counts how many requests an address sends within a
  minute. The duration it gives was already true of those counts.

**Adversarial pass.**
- `a_release_s_links_stay_below_baylee_s_releases`: climbing addresses,
  a query, a fragment, an empty segment, a backslash, a third segment and
  a neighbouring repository are refused; a real asset's address and one
  with a percent-encoded octet pass. `tags_are_plain` has `.` and `..`.
- `addresses_and_status_codes`: a 404 under no language carries `Vary`
  and all security headers in three languages; one under a language
  carries no `Vary`.
- `only_a_path_on_this_site_is_gone_to`: `//host`, `/\host`, `\/host`,
  a scheme and an empty path are refused.
- `origins_and_public_metadata_cannot_redirect_or_smuggle_credentials`:
  the trailing dot, with and without a port.
- Not shown by a test: the deadline's answer with headers (the handler
  cannot be made to take ten seconds without a seam for it; the order of
  the two layers was read, and the header layer runs on whatever the
  inner service answers); the panic hook.

**A second look, the same day** (asked of a reviewer that read the whole
change; three of this section's own claims said more than the code did).
- *Release links.* `%2e%2e` is `..` to a browser, and `below` let it
  pass: the count of segments kept such an address inside Baylee's
  repository, but "cannot climb" was not true. A segment that is only
  dots, however they are spelled (`.`, `%2e`, `%2E`), is refused; the
  test has `%2e%2e`, `.%2E`, `%2E` and `...`.
- *The islands' redirect.* A browser drops a tab or a line break from an
  address before it reads it, so `/<tab>/host` is `//host`. `local`
  refuses any control character; the test has tab, line feed and
  carriage return. Still not reachable: every caller passes a fixed path.
- *Whose refusal it is* (`central-identity/src/limits.rs`). The budgets
  stopped counting a bearer as a session's after four `Rejected` answers
  in a row. A gateway asks about its players with its own credential
  (`GatewaySession`, `GatewayProfile`, `GatewayAuthorize`,
  `RedeemGateway`), and a made-up lease or ticket is answered with the
  same `Rejected`: four bad tickets from any player took the gateway's
  credential out of the vouched-for, and its next request then met the
  strangers' second, which seven addresses can keep full. Refusals of
  those four requests are no longer counted
  (`refusal_is_the_bearer_s`). What that costs: a gateway credential that
  did end keeps its place among the vouched-for until its last vouching
  answer is half an hour old, and spends its own bearer's budget
  meanwhile, as any vouched-for bearer can.
  `a_player_s_bad_ticket_is_not_held_against_the_gateway_that_asked` was
  seen failing with the change switched off (`assertion failed:
  vouched(&limits, &gateway, now)`), and holds that the gateway's own
  refused status still counts.
- *The trailing dot* refuses a gateway that was registered under such a
  spelling: the runbook has the one query to run before deploying
  (`docs/operations.md`, "An origin's host may not end in a dot").
- *The card numbers' explanations* (found by a test that lost a race, not
  by the review): where the browser has interest invokers, the keyboard's
  focus opens an explanation after half a second. Enter and then Escape
  within that half second leave it closed for a moment and then open, once;
  Escape closes it again. Left as it is; the test waits for the focus's
  own showing.

**Open after this section.**
- `central-web` relies on Caddy for the bound on connections and for
  slow headers; reachable only from the machine itself.
- The deploy hook's leads (S6, L14) and the split of the runtime's and
  the migrator's system user are operations on the owner's machine and
  were not changed.
- The nine translated privacy notices carry the new sentence by machine
  translation, unread by a speaker, like the rest of them.

## The website redeems a reset code and changes a password, 12 November 2026

Central and `central-client` could reset a login with an administrator's
code and change a signed-in account's password since 28.10.2026
([account reset](account-reset.md), [native login](native-login.md)
§"Password change"). The website could do neither, and its FAQ said so.
This change builds both on the website: five exports in
`central-web-crypto`, a page `/{lang}/reset`, and a form in the account
area's security section. Central is as it was: no request, no wire type, no
server crate, no migration and no dependency changed (`Cargo.lock` is
untouched).

Design pass. The two flows exist and were reviewed when they were built;
the questions were what a browser adds to them:
- **No new cryptography.** `reset_start`, `reset_keep` and `reset_new` do
  what `central-client`'s `reset` does, and `password_start` and
  `password_finish` what `Session::change_password` does, with the same
  `central-crypto` calls in the same order; every message is the wire
  type's. The epoch rule (a kept vault at the account's epoch, a new one at
  the next) and the minimum of eight characters are applied in the module,
  as the SDK applies them, so a page cannot ask for another. A reset's two
  finishes are two exports that consume one state: a reset started to keep
  cannot finish as a new vault, nor the other way round.
- **The browser has no master key.** After a sign-in the page keeps none,
  so it cannot wrap "the key it holds" as the client does. Two things
  follow. The reset page has two of the three ways: the recovery code, or a
  new vault; "still signed in" is the client's. And the password's form
  signs in once more with the current password and changes in the same
  Worker, which holds the sign-in's key for exactly that long. The price is
  one more sign-in start per change (three guesses where the client's is
  two; the right password gives them back) and a browser session replaced
  once more.
- **Where the secrets are.** The passwords and the recovery code go from
  the form's fields to the Worker and nowhere else; the fields carry no
  `name`, and the page's script stops a submit before the islands run, so
  no browser sends them as a form. The Worker ends when the flow drops it,
  on every way out. A new vault's recovery code comes back to the page, as
  a registration's does, to be shown once. The reset code is the one
  secret that is sent, in `ResetStart`, as the protocol has it.
- **What the page may say.** Central answers a name nobody has as an
  account that has not moved, and a wrong, expired, used or replaced code
  with `Rejected`. The page has one sentence for all of them and for a name
  that cannot be one. It stops at discovery for a name that is nobody's,
  which is what `Discover` tells anyone already.
- **A wrong recovery code must not cost the reset code.** The Worker reads
  the recovery code before `ResetStart` and opens the wrapper with it
  before `ResetFinish`. A code that does not open the wrapper, and an
  account that has no wrapper, stop after `ResetStart` and before the
  finish: Central has checked the reset code and has not spent it. A
  recovery code that is none at all stops before anything is sent: the
  reset code was neither checked nor used. The page has one sentence for
  the three, "Nothing was changed, and your reset code still works"; in the
  last case "still works" says more than the page knows (below, for the
  owner).
- **The warning before a new vault.** Without the recovery code the data
  is lost for good. The page says what that is (the account's settings,
  the deck backups, the chat history) and what stays, beside the choice,
  and takes the form only with the warning's box checked; nothing is sent
  before.
- **A lost answer.** A finish whose answer does not arrive may have
  happened. The page says so in both flows and calls it neither done nor
  refused. For a new vault it still shows the recovery code the Worker
  made, with what it is worth in either case: if the reset happened, that
  code is the only one, and it cannot be made again.
- **What the page says after a change** is what Central does
  (`central-identity`'s `password.rs` and its tests): every session of the
  account is deleted and one of the caller's kind inserted. So: signed in
  here, signed out everywhere else, the client included.
- **The link.** "Forgot your password?" under the sign-in form is a plain
  link; the site's script opens dialogs for the sign-in's and the
  registration's links only, and the reset page has no dialog of its own.
- Decided against: a code read from the page's address (it would stay in
  the browser's history); telling a wrong code from an unknown name; a
  "keep my data" without the recovery code on a signed-in browser (it has
  no key); a Worker kept alive between page loads to hold one; showing a
  new vault's code only after Central's yes when the answer is lost (the
  person would be left without any).

Implementation pass:
- `central-web-crypto` (`flow.rs`, `password_exports.rs`): the five
  exports and their handles (`Reset`, `ResetKept`, `PasswordChange`,
  `PasswordChanged`; a new vault answers the registration's `Enrolled`).
  States are consumed by the step that takes them, also on failure, and
  wiped when dropped. The crypto Worker's script has the five steps. The
  committed module was rebuilt in the pinned container
  ([xtask](xtask.md)).
- `islands/reset.rs`: the form's rules (`form_problem`), the refusals'
  mapping (`by_central`, `by_worker`, `at_finish`), the flow (a valid name
  and a parsed code, discovery, the Worker's start, `ResetStart`, the
  challenge's identity equal to the discovered one, the Worker's finish,
  `ResetFinish`), and the views. After a reset Central confirmed, a
  session this browser remembers is forgotten if Central refuses it, with
  the chat identity kept for it.
- `islands/password.rs`: the sign-in (`AccountSummary` for the session's
  own identity, `LoginStart`, the Worker's check, `LoginFinish`), the
  change, and the form. A wrong current password is said only where the
  Worker's KE2 check refused it. The change counts as done only when the
  wrapper Central answers is the one the Worker made.
- `islands/field.rs` (`labelled`: label, glyph, hint, eye), `clip.rs` (the
  clipboard), two glyphs. `Page::Reset`: not indexed, not listed, without
  the dialogs; `/reset` redirects under a language. The sign-in form's
  link; the FAQ's answer with its link.
- Words: 39 new messages and the changed `faq-password-a`;
  `security-password-later` is gone. English and German by hand. French,
  Italian and Brazilian Portuguese by machine, with this change: the
  gate's `no_language_is_english_under_another_name` allows a language
  fewer than 12 in 100 messages that are the English ones, and with the
  English text standing in, those three had 126, 118 and 114 of 913 where
  109 pass. The limit was left as it is. The six other languages carry the
  English text of these messages.
- The lines `docs/security-page.md` cites in files this change lengthened
  were moved by as much as the lines moved. Some were a few lines off
  before; they are as far off now.
- Nothing is logged, in the page or in the module. No lint was switched
  off; no `unsafe`.

Adversarial pass, each question with what answers it:
- *Does the page tell a guesser more than Central?* No. One sentence for
  an unknown name, a name that is none, an account that has not moved and
  every refusal of a code (unit test over every failure; in the browser
  with a stand-in for three of them; against a real Central for a used
  code).
- *Can a recovery code that is wrong burn the reset code?* No. Against a
  real Central the same code was refused with a foreign recovery code, no
  `reset_finish` was sent, the client's session stayed, and the code then
  worked with the right one. After a new vault the old recovery code is
  refused the same way and the new one works.
- *Can a new vault be asked for without the warning?* Not through the
  form: the rule is one function, tested, and the browser test sees no
  request before the box is checked.
- *Is "the vault stays" true?* Read from outside the page, by a sign-in
  over the native listener with the committed module: after a kept reset
  and after a password change the chat identity's fingerprint (derived
  from the master key) and the stored document with its revision are the
  same; after a new vault the epoch is one higher, the fingerprint is
  another and the document is a first one. That the fingerprint tells keys
  apart is seen in the same run.
- *Is "signed out everywhere" true?* A client's session, signed in over
  the native listener before, is live after the attempt with the foreign
  recovery code, and refused after the kept reset, after the new vault and
  after the password's change. (The run's third reset, kept with the new
  recovery code, was not followed by that question.) The page's own
  session is forgotten after a reset and kept after a change, where the
  list shows one session and a reload stays signed in.
- *Do the old password and the old code still work?* The old password is
  refused on the page and over the native listener after the kept reset
  and after the change, and on the page after the new vault; the used
  reset code is refused; the old recovery code opens nothing after a new
  vault.
- *Does a secret leave the Worker or stay in the browser?* Against a real
  Central, no request body the page sent to `/api/` after the
  registrations holds a password or a recovery code as text, with or
  without its dashes, and at the end neither storage holds one or a reset
  code. With a stand-in: the fields have no name, and a submit without
  script sends nothing and leaves the address without a query.
- *Can a stolen cookie change the password through the page?* No more
  than through Central: the page's sign-in fails in the Worker without the
  password, and `ChangePasswordFinish` needs the proof whatever a page
  does.
- *Someone signed in as one account resets another.* By reading, not by a
  test: the session is forgotten only when Central refuses it, so the
  first account stays signed in.
- *A lost answer.* With a stand-in that drops the reset's finish, the
  page says "No answer came back", not "done", and still shows the code. A
  refused finish shows no code.
- *Script in the origin* reads what is typed, here as on the sign-in page
  ([threat model](threat-model.md), "Website compromise"); the Worker does
  not help against that, and nothing here claims it.
- **The checks have teeth** (one change at a time, each undone). Caught by
  the unit tests: a code's refusal mapped to the sign-in's "wrong
  password"; the confirmation no longer required; a lost finish called a
  refusal (reset, and again for the password); any Worker refusal blamed
  on the recovery code; any Worker refusal blamed on the current password.
  Caught by the server's tests: the page indexed; the page given the
  sign-in dialogs. Caught in the browser with a stand-in, on a site built
  with the change and rebuilt without it afterwards: the shared field
  given a `name` (both specs), and a dead session left remembered after a
  reset. These are the mutants that were tried, not every decision.

Run on this change: `cargo xtask gate` passes (format, policy, clippy, the
workspace's tests, the doc tests, the release build), as do `cargo test -p
central-web -p central-web-server -p central-web-crypto` (117 tests),
`cargo xtask site`, `cargo xtask secrets` and, in the pinned container
under Rosetta, `cargo xtask wasm --check` (the committed module and glue
match a fresh build). In `web/`, `npm run lint` and `npm run typecheck`
pass. The gate, the tests, the site's build and the module's check ran on
the commit before this section's, with the documents of this section's
commit in the tree; the gate and the secret scan ran again on the last
commit.

The site suite (`web/e2e-site`, every spec, both projects) ran on a
loopback port of its own, 28797, because another session's server held the
suite's 28792; the configuration for that was not committed. 338 passed, 6
were skipped, 2 failed. The two are one test in both projects,
`site.spec.ts`'s "the gateway directory finds by name or address …": it
expects five tiles of the directory's numbers, and the page draws six
since the commit this change stands on (the players in a game). That test
and the directory are not part of this change and were left. In an
earlier run of the whole suite `account.spec.ts`'s three-second check of
a registration's haste failed once on the desktop while the machine was
busy, and passed alone and in the last run.

The real suite's two specs (`web/e2e-real/site-reset.spec.ts`,
`site-password.spec.ts`) ran against a throwaway Central with the suite's
PostgreSQL image in a container, a stand-in for the site's Caddy and
loopback ports of their own, with synthetic accounts: both passed on the
desktop and at the phone's size, each project against a Central started
afresh. Run together against one Central, the phone's reset met Central's
429 at its registration, as the registration budget has it. They ran on
the site as the commit that built the two pages left it. After them the
words of three languages, the site suite's 320 px checks, comments and
documents changed, nothing the two specs run. Every process and container
started for these runs was stopped, and the ports were seen closed.

Not tested, and recorded for the owner:
- **No page of the new site issues a reset code.** The run against a real
  Central sends the administrator's request with the administrator's
  session, as a page would. Until the new site's administration has
  "Reset login", a person gets a code only where the old build's admin
  page is still served.
- **"Your reset code still works"** is also said for a recovery code that
  is none at all, where the reset code was never sent. If the reset code
  is wrong too, the person learns that at the next attempt. The words were
  left; a second sentence for that case is the owner's to want.
- **The FAQ's answer** no longer says "or are still signed in to the
  Baylee client": the client has no reset form yet (M6), and the page
  cannot do that.
- **The password's lost answer**: that a lost finish is called "cannot
  tell" is a unit test of the mapping. No browser test drops
  `ChangePasswordFinish`'s answer, and the comparison of the wrapper
  Central answers with the one the Worker made has no test of its own.
- **A secret sent as bytes.** The run's search of the request bodies looks
  for the secrets as text. A password sent as an array of numbers would
  pass it; that no wire type has such a field is by reading.
- **The third reset's sessions** and the old password over the native
  listener after the new vault: not asked (above).
- **Someone signed in as one account who resets another**: by reading.
- **Browsers and aids.** Chromium only, desktop and a phone's size. No
  screen reader was run: the labels, the alert and status regions and the
  focus after each step are asserted in the page's tree, not heard. The
  clipboard was tried with its permission granted; the refusal's sentence
  is not reached by a test. The file to save is checked by its address,
  not downloaded.
- **Nine languages** have these messages by machine translation (fr, it
  and pt-br with the change, es, ja, ko, ru, zh-hans and zh-hant the same
  day, below); no speaker has read any of them.
- **A failed run's trace** of the real suite holds what was typed and the
  reset code as sent (`web/test-results-real`). Everything in it is
  synthetic; the folder of these runs was deleted.
- **Not run**: `cargo xtask postgres` (no database crate, migration or
  query was touched), `cargo xtask smoke`, `cargo audit` and `cargo deny
  check` (`Cargo.lock` and every `Cargo.toml` are untouched).
- **The registration's code step** has no copy button; the reset's has
  one (`clip.rs` is there for it). The account area's older requests
  word Central's `Rejected` as the sign-in's wrong name or password
  (`failure.into()`); the two new forms do not
  (`Problem::outside_sign_in`). `Debug` on the reset's outcome holds a
  new vault's recovery code, as the registration's `NewCode` does;
  nothing prints either.

### A second look, the same day

The session that took the change into the branch read it once more
before it did: the five exports beside `central-client`'s `reset.rs` and
`password.rs`, the two islands, the Worker's five new cases and the
route.

- **The exports add no step and leave none out.** Each calls what the SDK
  calls, in its order, and checks what it checks (the reset's epoch rule,
  the minimum length, the recovery wrapper's opening before anything is
  uploaded). The Worker's cases take their state out of its one variable
  before they use it, so no state serves twice.
- **One check the SDK makes was missing on the page.** After
  `ChangePasswordFinish` the SDK takes the answer only for the account
  and epoch it asked for and only with the wrapper it sent; the page
  compared the wrapper alone. It compares all three now (`as_asked`,
  with a test), and an answer that fails signs out the session it came
  with, as the SDK does and as the sign-in does (`auth.rs`). The sign-in
  the form makes first did compare the account, but kept the session of
  an answer for another one; it signs that out too. A Central that
  answers as specified never reaches either branch.
- **The reset page** checks the account and epoch `ResetChallenge` names
  against the discovered ones before the Worker is asked again, and ends
  the Worker before the finish is sent: nothing found.
- **The module.** The committed module was built by the change on its own
  base. On the merged tree, where `central-api` differs (an origin's
  trailing dot, above), `cargo xtask wasm --check` in the pinned container
  says `Committed WebAssembly module and glue match a fresh build.`
- **The words.** Six languages were translated by machine after the
  merge, with the vocabulary of each language's file; "vault" had no word
  in them and has one per language now. The checker compared ids, order,
  tags and paragraphs; nobody who speaks the languages read them.

- **The recovery code's copy button** is one piece now (`clip::copier`)
  and the registration's code step has it too, as `web-accounts.md` had
  said it would. The code goes to `navigator.clipboard.writeText` and
  nowhere else; a browser that refuses is told to the person.

- **The chat after a password change.** The chat opens without a password
  while the sign-in lasts (owner, 03.11.2026), from an identity this
  browser keeps sealed and bound to account, epoch and tag. A password
  change replaces the session twice and uses up the Worker's sign-in key,
  and no test opened the chat afterwards. The kept record depends on
  neither, and the master key stays, so it should open; the real suite's
  password spec now opens the chat after the change and once more after a
  reload and finds it open, with no password asked for, at both sizes.
- **Looking at the copy button found a step the dialog cut off.** In the
  registration dialog the step with the recovery code was wider than the
  dialog at a phone's width in German: the dialog's card and the step are
  grids, and a grid's column is as wide as its longest word
  ("Wiederherstellungscode"). The lead and the card's right edge were cut
  off; nothing was unreachable (the code wraps anywhere, the buttons sat
  left), and it was so since the dialogs were built on 11.11. Both columns
  are `minmax(0, 1fr)` now, the step's heading hyphenates and is a size
  smaller in the dialog. `dialogs.spec.ts` puts such a heading into the
  dialog at 320 px, once the German one and once a word nothing
  hyphenates; without the two columns the second fails. Looked at in
  German and Russian at 412 and 320 px.

- **A cache made again behind the page that removed it.** The site's
  browser suite failed once in `pwa.spec.ts` on the last full run, and
  the test repeated in parallel failed in 3 to 6 of 16 runs, alone in
  none: after a sign-out the next page removes the service worker and its
  cache (a reader gets nothing stored), but the removed worker still
  answers for that page, and a fetch that had found the cache present
  opened it again after the page deleted it. The copy held the site's
  public files only and went with the next page load; the sentence "a
  reader gets nothing stored" was not true for that while. It was so
  since 11.11. and is no part of the reset's change. Now the page tells
  the worker to retire: on `RETIRE` the worker answers no request from
  its cache any more, waits for the answers under way and deletes its
  caches; the page deletes them as before, after the worker's word or
  three seconds. The removal does not wait for the word: the previous
  site's worker (also `/sw.js`, scope `/`) knows no such message, and a
  copy of this worker without it was removed 80 to 375 ms after the
  reload began, its cache after the three seconds (or with the next
  page, which finds no worker and waits for none).
- **A worker handed back with no cache.** Looking at the above with two
  pages open showed a second thing, older than the fix: while a page
  that removed the worker is still open, Chromium gives a page that
  registers it again (signed in in another tab) the same worker back. It
  had no cache, the install that fills one does not run again, and so
  that person had no offline page and no stored files until the next
  build (4 runs of 4). A page that wants the worker now says so (`KEEP`),
  and the worker caches again and fills its cache if it has none; `KEEP`
  and `RETIRE` are taken one after the other, in the order they came, so
  the later one decides. Should the two pages cross within the same
  moment and the first delete what the second had filled, the second's
  next page fills it again.
- **What the two messages allow.** Both are taken from a window of this
  origin only, like the worker's other messages. `RETIRE` stops the
  caching and deletes the site's own caches; `KEEP` stores the offline
  pages and their files, which the install stores anyway. A script of
  this origin could always do both to the caches directly; neither
  message carries data, and neither touches the push state. A worker
  started again forgets that it was retired, finds no cache and makes
  none (the check that was there before).
- **How it was tested.** The evidence for the first point is the rate:
  the same test, repeated in parallel, failed 3 to 6 times in 16 before
  and has not failed since (the count on the head is in the pull
  request's text); a probe of the same steps found a cache left in 4 runs
  of 20 before and in none of 24 after. The test's added fetch through
  the removed worker would pass on the old code too and only guards what
  was there. For the second point a test opens the two pages, finds all
  offline pages stored for the second, more files after its reload, and
  the German offline page with the network off; without the page's
  `KEEP` it fails at both sizes. Chromium only: no other engine ran these
  tests, and what another browser does with a registration that is still
  in use was not looked at.

Checks on the merged tree. On `2a54baa`: `cargo xtask gate`, `postgres`,
`smoke`, `secrets` and `crypto-scope` and `cargo deny check` pass;
`cargo audit --deny warnings` fails on RUSTSEC-2026-0459 alone
(`libsqlite3-sys`, in the lockfile through `sqlx-sqlite` and in no build;
the owner's decision, as before); the site's browser suite, 348 passed, 4
skipped, none failed. Against a real Central on loopback (PostgreSQL 18,
Central's release binary, the site from the development server, synthetic
accounts), each project against a Central started afresh: the chat's
spec, the reset's and the password's pass at the desktop's and at a
phone's size; the registration in them copies its recovery code and
reads it back from the clipboard. What ran after the points above is in
the pull request's text.

Not looked at again: the points the list above names as not checked stay
as they are.

Engineering review, not an audit.

## The seven open findings, 12 November 2026

The review of 11.11.2026 left seven findings open, medium or worse (the
list in "The review's other findings"). On 12.11.2026 the owner decided
for the recommended variant of each, in one pull request. This section is
the record of that work: for each point what was thought through, what
was built, what was tried against it, and what stays possible. The points
keep the list's numbers. Built is not ended: each point says what it left.

What holds for all seven:

- **Nothing on the wire changed.** No request, answer or type of
  `central-api` was touched. The released client and the released gateway
  decode strictly, so every refusal is one that existed before: `Busy`
  (429 on the routes) and `Rejected`. How the released gateway and client
  take each was read in their source before a refusal was chosen
  (`baylee-account/src/http.rs`, `baylee-gateway/src/report/central.rs`,
  `baylee-client`, `social/central/poll.rs`; read-only).
- **No migration, no grant, no dependency.** `Cargo.lock` and every
  `Cargo.toml` are as they were. The migration number 000027 was kept for
  this work and is not used.
- **Nothing ran on a server.** No deployed Central, proxy, firewall or
  database was touched or asked. What follows ran on one developer
  machine, with synthetic accounts, on loopback or in containers that reach
  no outside network, and every container and process started for it was
  removed.
- **The counts are in one process's memory** unless the point names the
  store. A restart forgets them.

### 1. One address's bearers, and one account's gateways

Design pass. A vouched-for bearer drew on its own budget (128 control and
64 data requests a second) and then on the window all such bearers share
(512 and 256). Four bearers at their own budgets fill a window. Two
accounts hold four sessions. One account alone holds more: every
credential, heartbeat token and link token of its gateways was a bearer
of its own, and an account may register eight gateways with eight tokens
each. All of it from one address.
- **What a limit by address must not hurt.** A gateway alone at its
  address. Two gateways behind one address. A household's clients, each
  of which asks a few requests a minute. The official gateway if it
  reaches Central on its own host, where the listener knows no address.
- **(a) A place's share.** All the bearers of one place draw half of each
  shared window between them. Half and not a quarter: two bearers at one
  address then each keep their whole budget. A request without a place
  has no share and meets the other two budgets, so a full address never
  refuses the host's own gateway.
- **(b) By owner.** The count cannot know whose a token is; the store
  can. It is asked once, when an answer first vouches for a bearer, and
  outside the count's lock. A gateway's bearers are then one unit with
  every other gateway of the same account. A session stays a unit of its
  own, and so does the official gateway's bridge, which has no owner.
- **Failure modes looked at.** The store cannot say: the bearer is not
  vouched for yet, and the next answer asks again. A bearer that is
  already vouched for is marked again without asking, so a failing store
  takes no bearer's standing away. The table of places is bounded and
  cannot fill: a place's window is opened by a request that was served
  and is forgotten a second later, and the table holds more than two
  seconds of served requests can open (2,048; asserted at compile time).

Implementation pass: `central-identity/src/limits.rs` (`PLACE_CONTROL`
256, `PLACE_DATA` 128, the map `sharing`, `Unit::{Bearer, Owner}`,
`Vouched::owner`). A request is refused by the first of three budgets
without room, in the order own, place, shared, and nothing is spent of
any of them then. `answered` now says whether the store is to be asked;
`central-identity/src/lib.rs` asks `bearer_owner` and calls `vouch`.
`central-postgres/src/accounts/gateways.rs`, `bearer_owner`: one
statement, three lookups by a unique digest. The lists of which request
is of which class, and which answer vouches, were not touched.

Adversarial pass (unit tests with a clock of their own, and PostgreSQL 18
through the service the listeners serve):
- Four bearers at one address, each asking its whole budget in one
  instant: half a window is served and a session at another address is
  served beside them. A gateway alone, and two behind one address, keep
  their whole budgets. Six clients of a household are served within
  their address's share. A bearer without a place is served beside a
  full address. A place's window is forgotten after its second.
- Seen failing first, on the tree before (a): four bearers at one address
  (512 served where 256 were expected), the host's own gateway beside a
  full address (the address was not full: `Ok` where `Busy` was
  expected), and the forgetting (0 where 64).
- One account's gateway bearers: four tokens of two gateways, each from
  an address of its own, each asking 64 data requests in one instant.
  Sixty-four are served in all, and the account's own session beside
  them (`central-client/tests/support/strangers.rs`). Seen failing first
  with the owner ignored: 256 served where 64 were expected; and the unit
  test `one_account_s_gateway_bearers_are_counted_together`, 512 where
  128.
- On PostgreSQL, `bearer_owner` says the owner for a gateway's
  credential, its heartbeat token and a link token, whatever their
  state, and nobody for a session, a replaced credential and a token
  nobody issued (`central-postgres/tests/support/accounts/gateways.rs`).
- What the released gateway asks was read: a heartbeat and a look at
  revocations every 30 seconds, the terms every 60, one `RedeemGateway`
  for each sign-in, four requests in flight at the most, and tokens
  checked locally. One owner's gateways together stay far below 128
  control requests a second unless very many players sign in at once.

**What stays possible** (for the owner):
- Two addresses with two bearers each fill a shared window (2 x 2 x 128).
  One account has two sessions, so two accounts at two addresses do it.
  The list said four addresses for a quarter of a window; at the
  recommended half it is two. Accounts are open to register. This
  reviewer would still call it medium, narrower than it was: it took one
  address before. A wider shared window is a question of what the
  store's four connections serve.
- One account's two sessions at their full budgets use up their address's
  half. Everyone else behind that address is `Busy` meanwhile: a
  household, and also a campus or a carrier's shared address.
- All of one account's gateways share one bearer's budget.
- A process on Central's host has no place, so no share holds it.
- The first answer that vouches for a bearer costs the store one more
  statement.

### 2. Those without a session: a reserve for who asks little, and the firewall's file

Design pass. Every place may draw 600 requests a minute of the 32 a second
that all clients without a session share; four places that keep it up are
40 a second. Sign-in, registration and the public pages' data are `Busy`
meanwhile, for everyone.
- **(c) A reserve.** 8 requests a second are kept beside the 32. A place
  draws on them only when the 32 are spent, and only while it was served
  fewer than 30 requests in its minute: six sign-ins of five requests.
  Beside the 32 and not out of them, so nothing that was served before
  is refused now. What a place draws of the 8 counts toward its 30, so
  one place takes at most 30 of a minute's 480, and spending them all
  takes sixteen places.
- **IPv6.** A place is a /64. A subscriber with a /56 has 256 of them,
  each of which has "asked little". So how little a place asked is
  counted by the /48 around it, as the guesses at passwords are.
- **What it moves.** The most requests a second without a session is 40
  now. Two tables are sized by it and were checked: unfinished sign-ins
  (4,096 against 40 x 60 = 2,400) and registration's challenges, which
  grew from 40,000 to 50,000 (against 40 x 1,201 = 48,040). Both hold by
  a compile-time assertion that names the sum.
- **(a) The firewall.** `ops/central-limits.nft` bounds, for one source
  address (an IPv4 address or an IPv6 /64), new connections to the
  proxy's ports (300 a minute after a burst of 300) and connections held
  at once (512). It drops and never opens; it has a table of its own and
  never flushes another's; loopback is not counted. The reasons for the
  numbers are in the file's head. Nothing in the repository loads it.
- **What (a) is not.** The list said it "answers this, the next two
  points and floods of whole requests at once". That was too much. It
  counts connections. A client that keeps one connection and sends its
  requests over it (keep-alive, HTTP/2, HTTP/3) is not slowed, so four
  addresses still spend the 32. It ends what nothing bounded before: one
  source opening connections to the proxy without limit.

Implementation pass: `central-identity/src/limits.rs`
(`RESERVE_PER_SECOND` 8, `LIGHT_PER_MINUTE` 30, `PUBLIC_PER_SECOND` 40,
`block`, the window `spare` and the places `light`),
`central-identity/src/challenge.rs` (`HELD` 50,000),
`ops/central-limits.nft`, and [operations](operations.md) §"Connection
limits in front of the proxy" with how to load, check and remove it. A
request the listener knows no place of never draws on the reserve.

Adversarial pass:
- Four places spend the 32 and a fifth that asked little is served; a
  place draws the reserve only until its 30; a hundred /64s of one /48 draw 30
  between them. Seen failing first: the fifth place `Busy`; 0 served
  where 30 were expected; 80 where 30 (before the /48).
- The file, with nft 1.1.3 in a container without a network: checked
  (`nft -c`), loaded, loaded again (the same rules, once), listed,
  deleted. Between two containers on an internal network, IPv4: 300 of
  300 connections in a burst, then 12 of 20 in the four seconds after;
  held connections stop at 512; after ten are closed on both sides, ten
  of twelve new ones connect. A connection that only the client closed
  stayed counted until the server closed too.
- Not tried: the IPv6 rules with traffic (they were loaded and listed);
  UDP; the file on the server. It assumes that Caddy listens on the
  host's own addresses. A port published by Docker does not pass the
  `input` hook, and the server was not looked at for that.

**What stays possible:**
- Four addresses still spend the 32, and everyone who asks more than a
  little is `Busy` meanwhile: the website's pages that ask again and
  again, a client that retries. Sixteen places that each ask thirty a
  minute spend the 8 too. Many addresses still win; for IPv6 they must
  be in sixteen /48s.
- A limit on requests at the proxy is not built. Caddy's own build has
  none, to this review's knowledge.
- Still medium for a flood from many addresses. Against four addresses a
  person who signs in is now served.

### 3. Requests without the proxy's address

Design pass. The account listener binds loopback, and the proxy appends
the client it saw. A request without that hop is counted by no address.
On the deployed listener it means a proxy that stopped sending the header,
or a process on the host that talks to the listener directly. Nothing
told the operator. Refusing such requests would refuse every local run
and whatever the operator points at the listener; the list recommended to
keep the runbook's check and to leave the refusal to the owner. So: count
them, say so, refuse nothing.
- **What the line may hold.** The count. There is no address to write,
  and nothing of a request is written.
- **How often.** The first one, then at most one line an hour, so a flood
  of such requests is not a flood of lines.

Implementation pass: `central-http/src/unplaced.rs`; the account
listener's routes hold the counter only where the endpoint is the live
one (`native.rs`), so a development listener writes nothing;
`/health/live` is not counted. [Operations](operations.md) has the line,
what it means, and the proxy check written out with its expected output
(30 answers of `200` and one `429`, then a second address).

Adversarial pass: one unit test (the first is reported, then one an hour,
with the count, and the line is fixed but for the count). The wiring on
the deployed listener has no test of its own: it needs the service behind
the routes. The check's command was written from the wire's definition
and was not run against the deployed name.

**What stays possible:** such requests are served as before, so a process
on the host is held by no address's budget. A burst between two lines
shows only in the next line's count. Whether to refuse them on the
deployed listener is the owner's decision; the bridge would have to be
let through. Unchanged in what an attacker can do; an operator now sees
it.

### 4. The account listener's connections

Built as the list recommended: the firewall's file of point 2 bounds the
connections one source opens and holds. It does not end this point, and
the record should not read as if it did. The listener's 32 places for
late and large bodies are taken by requests the proxy passes on, and one
address has four of them (11.11.2026). Eight addresses still hold all 32.
The firewall counts a client's connections to the proxy, 512 for one
source, which is far above four. What it ends is one source holding the
proxy's own connections without bound. Open as it was: more places, or a
proxy that reads small bodies itself before it passes a request on
([operations](operations.md) names Caddy's `request_buffers`, from its
documentation and not tried), are the owner's to choose.

### 5. Reports

Design pass. An approved gateway is a stranger's server that an
administrator let in. It could fill the store (about 7.7 GiB an hour) and
could hold both upload slots with uploads that never finish.
- **(c), (d) Weighed before stored.** A filing is refused with `Busy`
  when its gateway would hold more than 2 GiB, the official gateway more
  than 8 GiB, or all together more than 16 GiB. A report counts as 64 KiB
  whatever its length, and each part as its announced length while it is
  stored or may still come (an hour). The level is asked of the reports
  themselves in the filing's transaction, so it cannot drift and a purge
  frees room. That needs no migration; the list had expected one.
- **Two filings and the last room.** One filing is weighed at a time,
  under a transaction's advisory lock that is tried and never waited
  for. A filing that finds another being weighed is `Busy`. The released
  gateway sends a filing again after 250 ms and after a second, and then
  tells its player the report failed.
- **The official gateway** has its own, larger bound. Most players play
  through it, and what the one-time import brought counts in it. The
  import itself is the operator's and is not weighed.
- **(a) Slots.** Four uploads at once, two of one gateway, which is what
  the released gateway sends at once. By reading the code, a part of 64
  MiB is held up to three times while it is stored: about 768 MiB for
  four at the same moment, 384 MiB with the two slots before.
- **(b) Unfinished uploads.** Thirty in an hour for one gateway, counted
  from the first; past it, its uploads are `Busy` until the hour is over.
  Unfinished is an upload that took a slot and did not store its part. A
  part its other upload stored meanwhile (409) and a store that failed
  (503) are held against nobody. The number comes from the released
  gateway: a part is sent at most five times, a report has two, so
  thirty is three reports of which every send failed.
- **Where the count lives.** Not in `limits.rs`: it is the listener's,
  beside the slots (`central-http/src/uploads.rs`).

Implementation pass: `central-postgres/src/accounts/reports/level.rs`
(`admit`, `levels`, `report_levels`; the constants are exported),
`central-http/src/uploads.rs` and `native.rs` (the semaphore is gone; a
slot that is dropped counts as unfinished, so a handler that is cancelled
is counted), `ops/report-levels.sql` for the operator.
[Reports](reports.md) §"What the reports may take" and
[operations](operations.md) §"What reports may take, and how to make
room" have the numbers, how to see each gateway's level and how room
comes back.

Adversarial pass:
- On PostgreSQL 18 (`central-postgres/tests/support/accounts/reports/level.rs`):
  31 largest reports leave a gateway exactly one report's room, and one
  byte more is refused; an announced part stops counting after its hour
  and a stored one does not; the official gateway's bound; all together;
  the operator's query gives the same numbers as the code; with the lock
  held elsewhere a filing is `Busy`; of four filings at once for the last
  room, one is stored.
- Seen failing first, with the weighing not wired: "one byte past a
  gateway's bound" was accepted (`level.rs:106`).
- Slots (unit tests, `uploads.rs`): one gateway holds two and another is
  served; unfinished uploads are counted for an hour; finished ones are
  not, however many; a dropped slot is free again and counted. Three of
  the four failed against the old behaviour.
- Over the real listener and PostgreSQL
  (`central-client/tests/support/reports.rs`): one gateway holds two
  uploads within the pace, its third is 429, another approved gateway's
  part is stored meanwhile, and the first gateway's part goes through
  once its connections are gone. Seen failing first with two slots in
  all: the other gateway's part was 429.
- Not measured: memory. Not tried over a socket: the hour's count of
  unfinished uploads (unit tests only).

**What stays possible:**
- Two approved gateways that each hold two slots hold all four, for up
  to a quarter of each hour with the count. It needs two approvals.
- The official gateway at its bound leaves room for four registered
  gateways; more approved gateways than that can fill the 16 GiB between
  them, and then everyone's reports are refused, the official one's too.
- 16 GiB counted is more than 16 GiB of disk: the files do not shrink
  when rows go, and the log, indexes and backups take their own. Reports
  in the trash still count.
- An honest gateway whose records never fit its line in a minute reaches
  thirty with six such records, and loses its small parts for the rest
  of that hour.
- A refusal writes no log line. The weighing reads every report row at
  each filing.
- This reviewer would call what stays low: it needs approved gateways,
  and an administrator can disable one.

### 6. Chat

Design pass. Nothing deletes a delivery, so a device's inbox of 5,000
that is full stays full. One contact could fill it alone. A message to a
group was refused whole while one member's device was full, so one
member, or one contact of a member, silenced a group for everyone.
- **The share.** A device of another account takes a delivery while its
  inbox holds fewer than 5,000 and fewer than 2,500 of them are this
  sender's. Half: the two sides of a conversation take the same, and it
  takes two senders to fill an inbox. The sender's own devices are held
  to the 5,000 only.
- **A group.** A device without room is left out and the message goes to
  the others. It is `Busy` when no device of another account has room,
  so a direct message is never accepted for nobody.
- **A block must not change the next answer.** Whether anyone else has
  room is asked of every other device, a blocker's too. Before, a full
  device was not looked at when its owner had blocked the sender, so a
  group message that a full device refused went through once that member
  blocked: the answer told the sender of the block. Setting or lifting a
  block does not change the next answer now. Over 2,500 messages a block
  can still show; see what stays possible.
- **The deletion was not built.** The list recommended the share and
  deleting on acknowledged delivery. The wire has no acknowledgement
  (`Inbox { device, after }` reads by a cursor), both clients read their
  history again from the start, and the privacy notice says history is
  not deleted automatically. It needs the owner's retention decision and
  an addition to the protocol, which this work was not to make.
- **`Busy`** is an answer both released clients know, and the one a full
  inbox and a sender's full outbox were answered with before. The website
  shows it. The released native client was read before it was chosen
  (`baylee-client`, `social/central/poll.rs` and `worker.rs`;
  `baylee-client-core`, `social/machine/sending.rs`): it takes `Busy` as
  a rate refusal, keeps the sealed message, shows no failure and sends it
  again, a second later and after a pause of two seconds that doubles to
  thirty. It has no last try. `Rejected` it ends at once, with its words
  for a sender who may not write there, which would be wrong for a full
  inbox and is what a refused direct message says. So `Busy` stayed, and
  what its not ending costs is under what stays possible.

Implementation pass: `central-postgres/src/accounts/social/chat/send.rs`
(`CHAT_SENDER_SHARE`, `room`, `Held::fits`): one query a send counts each
recipient device's deliveries and how many are the sender's. Everything
else of a send is as it was: the locks, the exact recipient set, the
retry that returns the original.

Adversarial pass (PostgreSQL 18,
`central-postgres/tests/support/accounts/chat/inboxes.rs`, and a unit
test of the rule):
- At one below the share a direct message is delivered; the next is
  `Busy`. Another member is still heard in that inbox. The sender's
  message to the group reaches the member with room and skips the other.
- A full device: the group's message still reaches the others, and the
  full inbox never passes its bound. With a block: sent as far as the
  sender can tell, nothing delivered to the blocker. With nobody left who
  has room: `Busy`, and the same answer after the block is lifted. The
  sender's own device full: `Busy`, as it was.
- A message sent again: the direct message that used up the share, and
  the group message that filled an inbox, are each sent a second time
  with the same bytes and answered with the first answer's id and time,
  not `Busy`; the counts show each delivered once.
- Seen failing first: the message past the share was accepted
  (`inboxes.rs:145`, as the file was then).
- Not tried: the query's plan with full inboxes of a large group. It
  reads every delivery of every recipient device, as the count before it
  did.

**What stays possible:**
- Two contacts, or one person with two accounts, fill an inbox for good,
  2,500 each.
- A sender at the share is refused in that inbox for good. That is also
  the busier side of an honest conversation: one person can send another
  2,500 messages, and then no more, until something is deleted.
- A member who is left out is not told, and neither is the sender.
- A `Busy` for a used-up share or a full inbox does not end, and the
  released client sends such a message again for as long as it runs,
  without showing a failure: at most about one request in two seconds
  from one client, each taken as far as the room query. It keeps at most
  64 such messages; while 64 wait, its later sends through Central are
  answered as busy by the client itself, in every conversation. This was
  so before at 5,000; one sender now meets it at 2,500 in one inbox.
  Central looks at a message's time after the room, so a waiting message
  is never `Rejected` for its age. Looking at the time first would end
  it in the released client ten minutes after it was written. That was
  not built: it changes what a user is shown, which is the owner's.
- A block can still show over 2,500 messages. A member who blocked the
  sender takes none of the sender's messages, so the sender's share of
  that member's inbox never fills and the device goes on counting as one
  with room. A sender who has used up the share with every other member
  of a group and is still answered as sent can conclude that some
  member's device takes nothing from them: one who blocked them, or one
  who joined late.
- The privacy notice does not mention a share.
- Narrower, and this reviewer would still call it medium until retention
  is decided: the group can no longer be silenced by one device, but an
  inbox can still be filled for good.

### 7. The deploy hook

Design pass. Root built as the build user and then took `ops/grants.sql`,
the binary and the website out of the tree the build had run in. The
grants went to `psql` as the database's superuser. A dependency's build
script or an install script of `npm ci` runs as the build user and can
write that tree. A rewritten grants file would have run with every right
the database has, and `psql` takes what follows a backslash in a file
for a command of its own, which can start a program.
- **From the commit.** The file is printed out of the revision's commit
  by git's plumbing (`cat-file blob`, with `--no-replace-objects`) after
  the checkout and before anything of the revision is built. The copy is
  root's from then on.
- **As the migrator.** `psql` runs as `central_migrator`, the login that
  owns the tables, with no start-up file and no password prompt. Before
  the unit is stopped the hook asks whether that login reaches the
  database and is no superuser; if not, it ends and nothing was changed.
- **SQL only.** A file that is empty or holds a backslash is refused,
  when it is staged and again before the switch.
- **Git as the build user, not as root.** Root running git in a
  repository another user can write is a hazard of its own (its settings
  can name programs). So the blob is as good as the build user's git.

Implementation pass: `ops/deploy-hook` (`as_grantor`, `grantor_ready`,
`plain_sql`; `DB_GRANTOR` in place of `DB_SUPERUSER`), a note in
`ops/grants.sql`, and [operations](operations.md) §"The grants' file and
login" with the steps before the first deploy. The store's fixture now
applies the grants through a helper that asserts the file holds no
backslash and the login is no superuser.

Adversarial pass:
- `shellcheck` 0.11.0 and `bash -n` pass.
- A local dry run with the hook's own three functions, cut out of the
  file, against a scratch clone and a throwaway PostgreSQL 18.6 container
  without a network. The commit's file came out byte for byte with the
  working tree rewritten, with the index rewritten, with a filter set in
  the repository's settings, and with a replacement object (which is
  read without `--no-replace-objects`: tried). A revision without the
  file fails. An empty file, a file with a `psql` command and a
  backslash in a comment are refused. The migrator reaches the database
  over the container's socket and is no superuser; a superuser, a login
  or a database that does not exist, and a container that does not
  exist are refused. The migrator applies a stand-in grants file to
  tables it made, twice, and the grant is recorded as its own. Refused
  to the migrator: making a role a superuser, making a role, `COPY` to a
  program and from a server file, `ALTER SYSTEM`, reading a server file,
  an untrusted extension. And `psql` does run a program that a file
  names, which is why such a file never reaches it.
- Both PostgreSQL fixtures apply the real `ops/grants.sql` as
  `central_migrator` after all migrations, and the store's startup check
  accepts the result.
- Not tested: the hook as a whole. It needs root, systemd, the build
  user and the server's container. The server's `pg_hba.conf` was not
  seen; the old hook reached the database over the same socket as the
  superuser, which is why the migrator is expected to. No deploy was
  made with it.
- Not seen either: whether the migrator may grant on every object the
  file names on the server. The fixtures' tables are the migrator's and
  the server's are expected to be, since the migrations make them with
  that login; but the superuser applied the grants there until now, so it
  was never asked. The hook applies them after the unit is stopped, and
  the check before the stop asks only for the login. The runbook
  therefore has the operator apply the deployed revision's grants by hand
  as the migrator before the first deploy, so that a refusal shows while
  the service runs. That step was written and not run.
- The hook does not see a grant that only warns. Tried against the
  throwaway PostgreSQL 18.6 with the hook's own call: on a table that is
  not the migrator's, a grant of a privilege the migrator holds but may
  not pass on prints a warning, grants nothing and ends with status 0; a
  grant of a privilege it does not hold ends with an error and status 3;
  a grant on its own table prints nothing. After a warning the hook
  would start the unit, the store's check at start would refuse to run
  without the privilege (it fails closed, and the fixture tests it), and
  the hook would put the previous binary back when its probe fails. So
  the runbook's step by hand passes only when `psql` prints nothing.
  The hook was not changed for it: telling a warning from a notice by
  its words depends on the server's language.

**What stays possible:**
- The binary and the website are what the build made, and the binary
  runs the migrations with the migrator's login. A build that was
  tampered with needs no grants file.
- The build user's git (its settings, its objects, a `git` earlier in
  its path) can be written by an earlier build as the tree can.
- `npm ci` still runs install scripts, and the build user is a person's
  login with a home and keys. The list's other two options were not
  among the recommended and were not built.
- Low: it is not reachable from the internet and needs a dependency
  that turned hostile.

### What was run, and what was not

On commit `0dcedc3` with nothing uncommitted, on macOS on Apple Silicon
(rustc 1.99.0), 12.11.2026, each once and in this order. What this
branch holds after that commit changes documents only: the words of this
section, and the runbook's and this record's lines on a grant that only
warns. `cargo fmt --all --check` and `cargo xtask secrets` were run again
on the last of those commits and pass.

- `cargo fmt --all --check` and `cargo clippy --workspace --all-targets`
  pass, the second without a warning.
- `cargo xtask gate` passes: format, policy, clippy, tests, doc tests and
  the release build; 605 tests passed, none failed, and the two it
  ignores are the PostgreSQL fixtures of the next line.
- `cargo xtask postgres` passes on PostgreSQL 18.6, both fixtures, and
  removed its container and listener.
- `cargo xtask smoke`, `cargo xtask secrets` (the tree and the whole
  history), `cargo xtask crypto-scope` and `cargo deny check` (0.20.2)
  pass.
- `cargo audit --deny warnings` (0.22.2) fails on RUSTSEC-2026-0459
  (`libsqlite3-sys` 0.37.0, an advisory of 30.10.2026) and on nothing
  else. This work did not bring it: `cargo audit` reads the lock file,
  and no manifest and no lock file was changed. It is not in the ignore
  lists and was not added to them; it is the owner's to decide, and the
  dependency job of CI runs the same command.

Not run: the other runners of CI (Linux, Windows, macOS on Intel);
`cargo xtask wasm --check` and the web app's checks, since nothing they
build was touched; `cargo xtask postgres --exercise-failure` and `smoke
--exercise-failure`. Nothing was run against a server, and the three
things an operator loads or installs (the firewall's file, the deploy
hook, the report levels' query) were tried only as each point says.

## An account's e-mail address and a reset code by mail, 12 November 2026

The wire for an account's optional e-mail address was set before
(`central_api::email`, `Request::Email`, `Request::ResetMail`,
`Response::Email`). This change builds the service behind it
([e-mail](email.md)): migration 000026 and its store, a new crate
`central-mail` (the three mails and the client of `baylee-mailer`), the
identity service's handlers, counts and worker, a per-address budget on
both listeners, a `mail` block in both launchers, and two functions in
`central-client`. No existing request, answer or struct changed, and
`AccountExport` is as it was. No third-party dependency was added or
changed: `Cargo.lock` gains `central-mail`'s own entry and its name in the
entries of the two crates that use it.

Design pass. The assets are the address (personal data Central did not
hold before), the confirmation code, the reset code, which takes a login,
and the mailer's token. A session is trusted for its own account. Nobody is
trusted on `ResetMail`. The mailer is trusted with addresses and codes; it
is on the same host and reached over loopback only.
- **What an address can do.** It resets the login and nothing else, as an
  administrator's code does. It decrypts nothing, signs in to nothing and
  is shown to nobody. Whoever reads the mailbox can take the login and,
  with a new vault, destroy the account's data. That is what the address
  is for; it is optional for that reason, and the document says so.
- **Who may name one.** A session whose password sign-in is under 30
  minutes old, the rule every change of who can reset a login follows.
  `resend` goes to an address already named so; `withdraw` and `remove`
  take power from a mailbox and give none, and need no fresh sign-in.
- **The confirmation code.** `MailCode` from `central-api` and nothing of
  our own: 40 bits, 15 minutes, given up at the fifth wrong code, its
  digest bound to the account. With 10 mails a day that is 50 guesses a
  day at 2^40, by a session that is signed in to the account already. The
  code is never stored, and was never in a link. (Since the second round
  of the mails, 12.11.2026, it is in the fragment of the mail's button and
  nowhere else in a link: "The code in the mail button's fragment" below.)
- **Mail first.** `set` and `resend` hand the mail over before the store
  keeps anything. The other order would have to undo a write when the
  mailer is down, and a `resend` would by then have ended a code that
  worked. So that the mail is not sent for nothing, everything the store
  would refuse is checked before it, and the store decides again after it.
- **No answer about other accounts.** Addresses are not unique, and no
  answer depends on whether another account holds one. `Conflict` is for
  the account's own confirmed address only.
- **`ResetMail` tells nothing.** It is answered `Done` for every name. Its
  handler awaits nothing and reads no store: it checks the tag, counts,
  queues and answers. The queue holds more than an hour's requests (a
  compile-time assertion), so it is never the reason for an answer. A text
  too long to be a name is counted and not kept. Without a mailer every
  name is `Rejected`, which says only what `status` tells any account.
- **Whose reset code.** The store has to know whether an outstanding code
  is an administrator's, and the accounts table is written by the previous
  binary too. So the mailed code's digest is kept a second time, beside
  the address; a code is a mailed one exactly when the two digests are
  equal, and the accounts table is unchanged. One statement makes both
  kinds, so both refuse the same accounts. The mailed code's own rules
  (one hour, never over an administrator's code that lasts, once in ten
  minutes, three in 24 hours) are conditions of that statement, under the
  account's row lock.
- **A code does not outlive its way to the person.** It is taken back, by
  its digest, when the mailer does not take the mail, and it ends when the
  address it went to is replaced or removed. An administrator's code is
  touched by neither.
- **The counts.** Per account, in a table of the mail module's own, not in
  the request budgets that sessions share. An attempt counts whatever the
  mailer says, so the count bounds what Central asks of the mailer. A full
  table refuses a new account and forgets nobody. `ResetMail` has 5 an hour
  for a client address on each listener and 30 an hour for everyone
  (changed the same day to 30 mails an hour; "A second look" below). The
  mailer's own limits are a second line, keyed by `account:` and the id.
- **The mails.** No text a person typed, except the sign-in name in a
  reset mail, cut down to a name's alphabet and escaped. No image, no
  script, nothing loaded from elsewhere, links to the configured site
  only. An address has passed `central_api::email::address` (ASCII, no
  control character) and travels in JSON; Central writes no mail header.
- **The mailer's client.** Plain HTTP to one numeric loopback address,
  through a resolver that answers that address and no other; no proxy, no
  redirect, no kept connection, a bounded answer, one request a mail with a
  random `Idempotency-Key`, never a second try. The token is wiped and
  never printed. There is no default address: without the block there is
  no mail, and a block that names anything else stops the start.
- **Notices** are queued and tried once. They never fail the request whose
  change they tell of.
- **Audit.** None. The audit records what administrators and the operator
  do. An account's own password change writes no event
  (`central-postgres`'s `password.rs` has none), so its own address writes
  none, and a mailed code, which no administrator made, writes none.
- Decided against: a link that confirms (a code in an address); unique
  addresses (an answer about other accounts); the store first; making or
  mailing the reset code on the request; a second try for a notice; a flag
  for the code's kind in the accounts table (the previous binary's
  statements write that row); the address in `AccountExport` (the released
  client decodes it strictly).

Implementation pass: looked at each boundary in turn.
- **Default deny.** Without a mailer `set`, `resend`, `confirm` and
  `ResetMail` are `Rejected` before anything is read or counted. `Email`
  needs a session of the listener's own kind and accepted terms; the
  cookie is read for it and for no request without a session.
- **Ownership.** Every statement of the address is keyed by the account
  behind the session; no request names an account. A confirmation code's
  digest holds the account's id, so one account's code is another's wrong
  one.
- **Limits.** Every number is a named constant with its reason, listed in
  the document; a test pins the mail module's constants to those numbers,
  and reads no document. The
  mailer's client had the listeners' own five seconds for a request; it
  has three now, and one to connect, so a mailer that does not answer is
  `Busy` and the listener does not cut the request between the mail and
  the store's write.
- **Cancellation.** A request that is cut after its mail was handed over
  leaves a mail whose code confirms nothing; asking again mails another.
  The worker works until nothing waits, since the queue is told once for
  many pieces, and on a stop it finishes the piece in hand: a reset code
  that was made is mailed or taken back, never left.
- **Secrets and diagnostics.** `Mail`, `Token`, `Client`, the store's
  `Email`, `Told` and `MailedReset` print nothing of what they hold; a
  queued request and the launcher's block have no Debug output at all.
  The service logs one fixed line when a mail was not handed over. No code
  and no token is ever part of an address.
- **The other listener's budgets.** The changes to `limits.rs` (one line:
  `ResetMail` is counted as a request without a session) and to
  `central-http` (one budget on each listener) add and reorder nothing.
- **What the wire commit left.** The service's lists of operations were
  two short (the test counted 67 of 69); both are extended, and `Email`
  takes the cookie.
- **Dependencies.** `ureq`, `serde`, `serde_json`, `zeroize` and
  `getrandom` at the versions and features the other crates use.
  `central-mail` is source-available and builds on `central-api` only; the
  three AGPL crates build on each other as before.

Adversarial pass, with what was tested.
- **One answer.** On real PostgreSQL, the encoded answer to `ResetMail` is
  the same bytes for a name that is nobody's, an account whose address
  only waits, an administrator's and an ordinary account, on both
  listeners, with and without a cookie, and for texts that are no name;
  afterwards exactly one mail was handed over, to the ordinary account.
- **An administrator's code.** A mailed request ten minutes after an
  administrator's code makes nothing, and the administrator's code then
  resets the login; taking a mailed code back does not touch it; once it
  is spent the same request mails one. In the store: an administrator's
  code replaces a mailed one and is then safe from the next mailed
  request.
- **How often.** In the store with times moved by the operator's login: at
  once and after nine minutes nothing, after ten a second code that ends
  the first, a third, then none until 24 hours after the first. In the
  service: the fifth mail of an account's hour is handed over and the
  sixth is not, on either listener; 30 requests for a reset code and then
  `Busy` for every name; through the real listeners five an hour for one
  address on each, and another address served.
- **Wrong codes.** Five give the address up and the right code then finds
  nothing; another account's code and a code past its end count nothing;
  a withdrawn address's code confirms nothing; a code confirms once.
- **A mailer that fails.** `Busy`, no answer and a refused address, on
  `set` and `resend`: the answer is `Busy` or `Rejected`, the mail was
  handed over once, and what waited before still waits and is confirmed by
  the code it had. For a reset code: the code that was in the mail starts
  no reset, and the try stays counted. A mailer that cannot run at all is
  a mail not sent.
- **A stale sign-in** names no address on either listener and no mail goes
  out; it still reads and resends. In the store's tests it also withdraws
  and removes.
- **No mailer.** `status`, `withdraw` and `remove` work and say `mail:
  false`; the rest is `Rejected` for every name, the right code included,
  and nothing was counted against the address that waited.
- **The client.** Against a stand-in mailer on an ephemeral loopback port,
  stopped and found closed afterwards: one request as the mailer's
  contract has it, each answer's outcome, a redirect not followed, a long
  head not read, a silent mailer ended within the bound, an address other
  than the configured one never reached. Endpoints that are not numeric
  loopback HTTP, sites that are no origin and tokens that are none are
  refused when the client is made, and again by each launcher's block.
- **The mails.** Every language renders every mail; no `<img`, no
  `<script`, no address outside the site's origin, the code in neither
  link; a hostile name and site are escaped; every language has every
  message with the same placeables, and a tag without words is English.
- **Redaction.** Tests print each of the types above and look for the
  address, the name and the codes.
- **The store.** On real PostgreSQL: the migration over 000025 with the
  previous binary's reset statements, the tables' CHECKs, the exact
  grants, deletion with the account, the sweep.
- **Mutants.** Fifteen changes to the mail module's counts and queues
  were each caught by its unit tests: a sixth mail in the hour, an
  eleventh in the day, a refused mail that forgets the account's count, a
  count that is not looked at, a reset request taken without a mailer, one
  whose language is not checked, a text too long for a name that is
  queued, an hour that is not counted, a queue of requests and one of
  notices without its bound, a notice queued without a mailer, notices
  before reset codes, a worker that is not told, a state that says mail
  where there is none, and a refused request that is counted. Three more
  were tried against the real store, one run of the fixture each: a mailed
  code that replaces an administrator's (caught where a mailed request
  must make nothing), a reset code that stays when its mail was not taken
  (caught where that code must start no reset), and a stale sign-in found
  out only after the mail (caught by the count of mails handed over).
  None survived.
- **Found on the way.** The secret scan took two synthetic tokens in the
  launchers' tests for keys; they are written differently now. A reason
  given for the 30 requests an hour, that the mailer's reserve holds as
  much, was not what the mailer's contract says and is gone.

Not tested.
- A real `baylee-mailer` and real delivery. The client was tested against
  a stand-in that speaks the contract's request and answers.
- That a proxy named in the environment is ignored. The agent is built
  without one; a test would have to change the process's environment.
- The worker's loop against a real identity service. The loop has its
  tests with a queue of the test's own, and `deliver_mail` is driven piece
  by piece by the fixture.
- Wrong codes sent at the same moment. The store counts them under the
  account's row lock; the tests send them one after another.
- The time `ResetMail` takes on a deployed host. The handler has no branch
  on an account; nothing was measured.
- The website's committed WebAssembly was not rebuilt or compared
  (`cargo xtask wasm --check`); `central-client` gained two functions that
  `central-web-crypto` does not call.

For the owner.
- The privacy notice names the address as "not available yet", speaks of
  reset links where Central mails codes, and knows a reset code's digest
  only as an administrator's (`legal/privacy.*.md` §§ 3 and 6). The texts
  were not changed here.
- The mailer's client waits one second to connect and three for a request,
  where two and five were planned: the listeners cut a request at five.
- Three addresses that ask on both listeners at their limit, or six on
  one, spend the 30 requests an hour that Central takes for everyone. The
  numbers are constants. (Changed the same day: "A second look" below.)
- Every `Email` action needs accepted terms, `remove` included. (Changed
  the same day: "A second look" below.)
- No mail says that a reset was finished or a password changed.
- Nine of the eleven languages held the English text; the same day they
  were translated by machine (Sonnet 5.5, by the owner's instruction),
  unread by a speaker.
- A refused `Email` request counts against the session's bearer in the
  request budgets, like other refusals; the line of work on those budgets
  may want it otherwise.

Checks. On the code of `964d272` (the documents after it change no code):
`cargo xtask gate` passes (format, policy, clippy, 634 tests, the doc tests
and the release build), and so do `cargo xtask postgres`, `cargo xtask
smoke`, `cargo xtask crypto-scope` and `cargo deny check`. `cargo audit
--deny warnings` fails on RUSTSEC-2026-0459 alone (`libsqlite3-sys`, as
before). `cargo xtask secrets` passes on the tree and the full history; its
first run stopped at the two test tokens named above. The fixture's
container was gone after every run, the three mutant runs that failed on
purpose included, and the smoke test found its listener closed.

Engineering review, not an audit.

### A second look, the same day

Read in the main session after the three pieces were put together (the
two reviewers that were to read them stopped early). It looked at the
request for a reset code by mail, the store's statements for the address
and for a mailed code, the mailer's client and the mails. Three changes:

- **The hour's 30 counted requests, now mails.** Three addresses asking
  about made-up names at their own limit on both listeners spent the 30
  requests an hour, and nobody's reset code was mailed until the hour
  passed: a feature anyone could switch off for everyone with three
  addresses (with IPv6, with one subscriber's prefix). The worker now
  counts a code when it makes one (`RESET_MAILS_PER_HOUR`, 30), and the
  request path counts nothing for all; the queue (256) is bounded and
  fills only while the store does not answer, since requests without a
  session share 40 a second in `limits.rs`. To keep the 30 spent someone
  needs accounts with confirmed addresses, about 240 of them at three mails
  a day each. Requests past the 30 are dropped, and the journal says so
  once an hour. The answer and its time on the request path still depend
  on nothing of any account.
- **Five minutes, not thirty.** An address can take the login, so a
  session stolen within thirty minutes of its sign-in could name the
  thief's address, confirm it and take the login by mail, a path from a
  stolen session to a taken account that did not exist before. The
  website proves the password once more right before it names an address,
  so the window is now five minutes (`EMAIL_FRESH_MINUTES`); a session
  taken later needs the password. It stays open for those five minutes
  and is in the accepted limits.
- **Reading and removing without new terms.** `status`, `withdraw` and
  `remove` no longer need the terms in force, as export and deletion do
  not: a person who will not accept new terms can take the address away.
  `set`, `resend` and `confirm` still need them.

Found sound by reading: the store checks the five minutes in the
transaction that writes, with the account's row locked; wrong codes are
counted under that lock, and a renewed code starts its count again with a
new code; a mailed code is told from an administrator's by the address
row holding its digest too, and every write of either digest keeps that
(an administrator's code replaces a mailed one, a mailed one never an
administrator's that lasts, a confirmed or removed address ends a mailed
code); the mailer's client connects only to the configured numeric
loopback address, with no proxy, no redirect and a bounded answer, and
never prints its token; every text in a mail is escaped, the language is
one of the eleven or English, and links go to the configured site.

Tested: the unit tests of the hour's mails and the queue, the terms rule
for each action, and the fixtures of both PostgreSQL suites with the
five minutes. What ran on the merged tree is in the pull request's text.

## Notices of a changed login to the account's address, 12 November 2026

Until this change Central told an account's e-mail address only that it
was replaced or removed. Three notices now go to the account's confirmed
address ([e-mail](email.md#notices-that-the-login-changed)): a reset code
set a new password (with `keep` or `new`), the password was changed, and
an administrator made a reset code. No change to `central-api`, no
migration, no new dependency and no change to `Cargo.lock`.
`central-postgres`'s `finish_reset`, `change_password` and `issue_reset`
return the confirmed address with their answers, and
`ISSUED_RESET_HOURS` is public, so the notice says the number the store
uses. The same work set the reset code in its mail at 15 px without added
space between its characters, so that it fits on one line of the card in
Gmail and mailbox.org, where it had broken in two.

Design pass. The assets are the confirmed address (personal data Central
holds already), the fact that the login changed, and the reset code, which
no notice may carry. The account's mailbox is trusted with what the
account's owner is told and nobody else is told anything.
- **What a notice is for.** It tells the owner of a change they did not
  make, in time to act: an administrator's code is ended by a password
  change, which the notice says; after a reset the owner asks an
  administrator for a code, and learns whether the recovery code still
  opens the data; after a password change the owner sets a new one with a
  reset code.
- **Who is told.** The confirmed address only. An address that waits for
  its code is nobody's yet: a session named it, and nothing proves the
  mailbox is the owner's until the code comes back.
- **When, and which address.** Only once the store committed the change,
  and to the address the account had at that moment. The store reads it in
  the change's own transaction, after the account's row is locked, which
  every write of the address locks first; `email_confirm` and
  `email_remove` read the address they tell the same way. Decided against:
  reading after the commit (one more `await` between the commit and the
  queue, where a request that is cut off loses the notice, and an address
  replaced meanwhile receives it), and reading in the worker (whoever took
  the login could remove or replace the address before the worker came to
  it; a password change gives its new session five minutes in which it
  may name one).
- **What it says.** No code; links only to the site's reset and security
  pages (and, since the second round, the site's start page in the
  footer); no administrator's name. **No account named**: the sign-in name
  is half of what a reset by mail needs, these mails go out unasked, and
  the confirmation mail and the older notices name none. The cost, in the
  accepted limits: a mailbox that two accounts confirmed is not told which
  account changed.
- **Not for a mailed code.** That code is made by the worker
  (`issue_mailed_reset`), not by `ResetLogin`; the reset mail is what the
  address learns.
- **Never a reason to fail or wait.** The notice is queued with the others
  and tried once, off the request. A full queue drops it, a mailer that
  does not take it leaves the change done.
- **The shared queue.** One account changing its password again and again
  could take the 64 places. The same notice to the same address of an
  account is now queued once while it waits, so one account holds at most
  one place for each kind; filling the queue needs many accounts with
  confirmed addresses and a slow mailer, which is in the accepted limits.
- **Audit.** No new event. An administrator's code has its event already;
  a reset and a password change are the account's own and have none, as
  before.
- Decided against: a notice for every sign-in; a second try for a notice;
  naming the administrator; a notice for a mailed code beside its mail.

Implementation pass: looked at each boundary in turn.
- **After the commit only.** `Mails::committed` takes the store's answer
  with the address: a refusal returns before anything else, a commit
  queues the notice and returns the store's answer. It is synchronous, so
  nothing is awaited between the store's return and the queue. The three
  callers pass the answer straight to it; `changed` (the guesses' places)
  still runs after a commit only.
- **The cut-off request.** The listeners wrap each request in a timeout
  and serve it on the connection's task (`central-http`'s `boundary.rs`,
  `server.rs`): a request cut off is dropped at its next `await`. After the
  commit returns there is none before the queue, so a cut there loses
  nothing; a cut during the commit can lose the notice of a change that
  committed, as it can for the two older notices. In the accepted limits.
- **The store.** One `SELECT` of `account_emails` by the account's id, in
  the transaction that changes the login, after its lock. Nothing else in
  those statements changed. A storage failure of that `SELECT` fails the
  change, as any statement of the transaction would; it then commits
  nothing. This is the one way the notice can change an answer, and it was
  chosen over a read after the commit (design pass).
- **Words that are true for every account.** An account may have no
  recovery code (a moved account's is optional, and a registered account
  may not have made one yet), so the notice of a kept reset says that a
  recovery code the account has still opens the data. The notice of a
  changed password points to a reset code, which no administrator's
  account gets; that is in the accepted limits.
- **Without a mailer** nothing is queued, and an address read is dropped.
- **Secrets and diagnostics.** `Told` prints nothing of what it holds, as
  before; `What` says which mail and, for a reset, whether a new vault
  began. No new log line. The notices carry no code: they are made without
  one, and `Mail::code` is `None` for them.
- **Words.** English and German by hand. The test that every language has
  its own words goes by id now (it went by position), allows exactly the
  nineteen new ids to be English in the nine other languages, and still
  holds German and the nine to their own words for every other id. The
  placeables test allows `hours`.
- **Lints.** No exemption; clippy with `-D warnings` over the workspace.
- **The worker** in `central-service` is unchanged: it reads `Delivery`
  only, and a notice is a `Notice` as before.

Adversarial pass, with what was tested.
- **A refusal tells nobody.** Unit: every refusal the store gives is
  answered as it is, with nothing queued and the worker not told; the
  closure that writes the mail is never called. Real PostgreSQL: a wrong
  current password, a spent code, a code asked for by no administrator and
  one for the administrator's own account each leave nothing waiting.
- **Nobody without an address or a mailer.** Unit: no address and no
  mailer queue nothing. Real PostgreSQL: an account whose address only
  waits changes its password and gets an administrator's code, and nothing
  waits; a Central without a mailer changes a password, makes a code and
  resets with it for an account with a confirmed address, and nothing
  waits and nothing is handed over.
- **Never on the request.** Unit: with a mailer that panics when asked,
  the change is answered and the notice waits; a full queue drops it and
  the answer stands. Real PostgreSQL: after each change's answer the
  mailer has been handed nothing; the notice goes out when the worker's
  step runs.
- **Each notice end to end.** Over the real listeners: a reset that keeps
  the data, an administrator's code and a reset with a new vault, told in
  that order to the address the account confirmed, in its language; a
  password change, told before the address's removal. Directly on a second
  service: a password change in German, an administrator's code, a reset
  that keeps and one that begins a new vault in Italian, each to the
  confirmed address with no code and no time to live.
- **A notice not taken.** The mailer refuses the notice of a new vault:
  `Failed`, nothing waits after it, and the account signs in with the new
  password at the new epoch.
- **A mailed code** gets its reset mail and no notice of a code beside it.
- **Once while it waits.** Unit: 128 password changes of one account hold
  one place; another kind, another address and another account each have
  their own; once taken, the same notice waits again.
- **The mails.** Every language renders the three notices; no image, no
  script, no address outside the site, no code, no name; the reset and the
  password notice link to the reset page, the administrator's to the
  security page; the German words say what each change was, and the keep
  and new resets differ. The reset code's box is at 15 px with no added
  space, the confirmation code's unchanged.
- **Mutants.** Seven changes were each caught by the unit tests, one run
  each: a notice queued twice while it waits, a notice queued for a
  refusal or for an account without an address, a committed change never
  told, a notice queued without a mailer, the reset code's box back at
  17 px, a new message left in English in German, and an old message left
  in English in French. Two more were tried against the real store, one fixture run
  each: a password change whose transaction reads no address (caught
  where the password notice must arrive) and a reset that reports the
  other vault (caught over the real listeners, where the reset that kept
  the data must say so). None survived; each run's container was gone
  afterwards.

Not tested.
- A `ResetFinish` the store refuses after a start that succeeded (two
  finishes racing for one code). The fixture's refused resets are refused
  at `ResetStart`; the unit test covers that a refusal from the store
  tells nobody.
- A request cut off between the store's commit and its return.
- A real mailer, real delivery, and how the notices look in real mail
  clients. The 15 px for the reset code follow the coordinator's
  measurement in Gmail and mailbox.org; no client was looked at here.
- The nine languages: they hold the English text.

For the owner.
- The nineteen messages of the three notices are English in fr, it, es,
  pt-br, ru, ja, ko, zh-hans and zh-hant until they are translated.
- The notices name no account (above). If a mailbox is shared by two
  accounts, it is not told which one changed.
- An administrator whose password was changed by someone else is told to
  set a new one with a reset code, which their account cannot get while it
  holds the role.
- The privacy notice (`legal/privacy.*.md` § 3) gives the address's
  purposes as confirming it, sending reset codes and telling the old
  address when it is replaced or removed. Telling the address of a reset,
  a password change and an administrator's code is a further use of the
  same address. The texts were not changed here.

Checks. On the code of `b33d07b` (the documents after it change no code):
`cargo fmt --all --check` and `cargo clippy --locked --workspace
--all-targets -- -D warnings` pass; `cargo xtask gate` passes (format,
policy, clippy, 673 tests with the doc tests, the release build); `cargo
xtask smoke` passes and found its listener closed; `cargo xtask postgres`
passes, both suites, and its container was gone afterwards; `cargo xtask
secrets` passes on the tree and the full history; `cargo xtask
crypto-scope` and `cargo xtask sqlite-scope` pass; `cargo deny check`
(0.20.2) and `cargo audit --deny warnings` (0.22.2, with the ignores in
`.cargo/audit.toml`) pass. One run of `cargo xtask postgres` before that
timed out: its 300 seconds include compiling the two test binaries, and
the machine was loaded by other work; with the binaries built first, the
same code passed. The same fixture had passed on `2d614e4` before the
words were changed.

Engineering review, not an audit.

## The new site makes a reset code, 12 November 2026

Until now an administrator could make a reset code
(`AdminRequest::ResetLogin`, [account reset](account-reset.md) "Issuing a
reset code") only on the old build's `/admin` page. This change gives the
new site a page for it, `/{lang}/account/admin` (`islands/admin.rs`).
Central is as it was: no request, answer, server crate, migration or
dependency changed, and `Cargo.toml` and `Cargo.lock` are untouched.

Design pass. The asset is the code: whoever holds it takes the login, and
with a new vault destroys the account's data. The other assets are the
administrator's password, which the page takes once more, and the list of
accounts with their sign-in names, which only an administrator may read.
Central is trusted to decide who may read and make; the page decides
nothing Central does not decide again.
- **Who sees the page.** The server cannot know the role, so the address
  answers everyone with the same frame, and the island shows the tool only
  to a session whose `AccountSummary` says `admin`. Anyone else gets one
  sentence, and the page asks Central nothing more. The navigation, the
  same for everyone, has no link to it; the overview links to it for an
  administrator only. Central refuses every administration request from
  anyone else whatever a page does, so the island's check hides a tool, it
  guards nothing. The address is under a language, never Caddy's `/admin*`
  of the old build.
- **The order of things.** Nothing that acts is sent before the person
  has opened the account, read the warning and confirmed it, and typed the
  password. The password is proven again in a Worker that is ended at
  once, as the e-mail address's form does, and the page then checks that
  the session is still the account it was opened as: a browser that
  signed in as someone else in another tab asks for no code. Only then
  `ResetLogin`.
- **The warning** says what the design says ([account reset](account-reset.md)):
  check out of band who asks, since the code takes the login; issuing ends
  nothing (the old password and every session stay); the data survives
  only with the recovery code or a device still signed in; a new code
  replaces an earlier one, a mailed one too; a code is for a forgotten
  password, not a taken-over account.
- **Where the code is.** It arrives in `ResetLogin`'s answer, is parsed as
  a `ResetCode` (which wipes its 20 bytes when dropped) and is held by the
  step that shows it. Leaving that step drops it: "Done", leaving the page,
  and `pagehide`, so that the back-forward cache keeps no page with it. It
  goes into no address, no storage, no log, no `console` and no field; the
  page offers no download, since a `data:` link is an address. It goes to
  the clipboard only when the administrator presses "Copy". The answer's
  text and the code's text on the page are copies that the browser frees
  when they go; a page cannot wipe them.
- **What a refusal may say.** Central answers every code it does not make
  with one `Rejected`, so the page cannot learn more from the status than
  that. It says what the account's row says before anything is sent (an
  administrator's account, one's own, a pending one, a moved one not in
  use), and after a refusal it asks Central what stands now:
  `AccountSummary` (session, role, the sign-in's time) and the account read
  again. A moved account in use whose move is not closed looks like any
  other in every answer; the page names the move for it while the sign-in
  is fresh by the browser's clock, and the stale sign-in otherwise. That
  guess picks between two sentences; it decides nothing.
- **A lost answer** (no network, a 5xx, an answer of another shape) may
  have made a code. The page says so, says that such a code cannot be
  shown and has already replaced any earlier one, reads the account again,
  and offers a new code, which replaces the lost one in turn.
- Decided against: a search in the address (`?q=`, as the old build had);
  reading `Account` for every row of a list (50 requests a page) to show
  the recovery code's state there; a download of the code.

Implementation pass:
- `islands/admin.rs`: the decisions are plain functions with unit tests
  (`refusal`, `explain`, `at_reset`, `at_sign_in`, `query`); the flow
  (`issue`, `why`) and the views around them. The search is checked by
  `AccountQuery::parse` before it is sent, so Central's `Rejected` for a
  malformed search never reads as a lost session. Only the newest list
  request's answer is drawn. Every list or account read that Central
  refuses is followed by `AccountSummary`, and only its refusal signs the
  page out.
- `islands/account.rs`: `Summary` keeps the session's account id; the
  administration's section is drawn only for `admin`; the overview's link.
  `data.rs`: `Section::Admin`, `Section::ROUTED`; the navigation is
  `Section::ALL` as before.
- No new dependency; the code's type is `central-api`'s. Nothing is
  logged. No lint was switched off; no `unsafe`.
- Words: 48 new messages, English and German by hand, the nine others the
  English text under `# English for now: to be translated.`

Adversarial pass, each question with what answers it:
- *Does anyone without the role get the tool, or make the page ask?* No:
  in the browser, a session without the role sees one sentence, no link on
  the overview or in the navigation, and the stand-in sees no
  administration request at all. The server's test finds no page of the
  area that links to the address.
- *Is anything sent before the warning is confirmed and the password
  typed?* No: the browser test counts the requests from the account's
  opening through the warning, its way back, the password's step with an
  empty password and its way back: none.
- *Is the password proven before the code, for the session's own
  account?* The browser test sees `account_summary`, `login_start` with the
  session's account, `login_finish`, then `reset_login`, in that order. A
  session that has become another account's meanwhile gets no
  `reset_login`. A wrong password is refused in the real Worker and the
  stand-in sees `account_summary` and `login_start` only.
- *Where does the code go?* In the browser: shown once, copied (the
  clipboard holds it), in no request body, no address, neither storage; and
  gone from the page's HTML after "Done" and after a `pagehide` event the
  test dispatches. A real return through the back-forward cache was not
  exercised. Against a real Central (`site-reset.spec.ts`, not run here)
  the same checks are written for the administrator's browser.
- *Can a refusal read as something it is not?* Unit tests: a lost answer
  is never a refusal, a refusal never a wrong password, and every refusal
  has its own sentence in every language. In the browser with a stand-in
  that refuses as Central's store does: each refusal the row tells before
  anything is sent, and each Central gives after the password (an
  unfinished move, a stale sign-in, an account gone, the role gone, an
  account that became an administrator's), `Busy` and `Forbidden`, and the
  three kinds of lost answer. Reading it over found one sentence that gave
  advice Central would refuse (one's own account: "ask another
  administrator", whose account Central makes no code for); it was changed
  to the password's change under Security.
- *Script in the origin* reads the code as it reads the page; nothing here
  claims otherwise ([threat model](threat-model.md), "Website compromise").
- **The checks have teeth** (one change at a time, each undone). Caught by
  the unit tests: a lost answer (no network) read as a refusal; a moved
  account not in use no longer told apart; a search sent without Central's
  rule. Caught by the server's tests: the administration put into the
  navigation. Caught in the browser with a stand-in, on a site and a
  server built with the change and rebuilt without it afterwards: the
  tool shown without the role; the check that the session is still the
  page's account removed; the `pagehide` listener removed; the password's
  proof skipped (both the order of the requests and the real Worker's
  refusal of a wrong password fail). These are the mutants that were
  tried, not every decision. A site rebuilt without its server shows a
  page whose island never starts: a browser mutant needs both rebuilt.

Run on this change: `cargo xtask gate` passes (format, policy, clippy,
tests, doc tests and the release build), as do `cargo xtask site`, `cargo
xtask secrets` (the tree and the history), `cargo fmt --all --check`,
`cargo clippy --locked --workspace --all-targets` and for
`central-web-islands` on `wasm32-unknown-unknown` (no warning), `cargo
test --locked -p central-web -p central-web-islands -p central-web-server`
and `cargo test --locked -p xtask` (with the test that no language is the
English under another name). In `web/`, `npm run typecheck` and `npm run
lint` pass, and `admin.spec.ts` type-checks under the strict settings
(`typecheck` does not include `e2e-site`). The site suite (`web/e2e-site`,
every spec, both projects, on its own port 28792) passed 402 tests and
skipped 6; after the mutants were undone, the sentence above changed and
the site and the server rebuilt, `admin.spec.ts` passed its 20 again. The
gate ran before that sentence's change, which touched only the words and
the spec. `web/e2e-real/site-reset.spec.ts`, which now makes its codes on
the page, type-checks and was not run: it needs a real Central,
PostgreSQL and the operator's appointment. `cargo xtask postgres`, `cargo
xtask smoke`, `cargo audit` and `cargo deny` were not run (no
dependency changed). This is an engineering review of one page, not an
audit.

## The site's strip, the official mark and the colour switch, 12 November 2026

The owner asked for three things on the website
([frontend rewrite](proposals/frontend-rewrite.md), §19).

**Pass 1, what changed.** The strip under the bar lost the release, the
checks, the last commit and the stars, and lays out what stays from a count
the server writes (`data-count`) in the stylesheet alone. No card, list or
chip calls a gateway official; Central's directory and API are unchanged.
The colour switch turns the scheme on screen into the other one and keeps a
choice only where it differs from the device's.

**Pass 2, the attacker's view.** `boot.js` still sets `data-theme` only from
the two values `light` and `dark` and writes only those or removes the key;
it adds a listener on a media query and no sink, origin, inline script or
style (the CSP is unchanged; the switch's position is set through the CSSOM
as before). `data-count` is a number the server counts from the snapshot,
not input. The strip links to nothing off the site any more. `directory.js`
offers only the chips the server wrote, so an old `?show=official` falls back
to every gateway; nothing in the snapshot or the wire changed.

**Pass 3, what was tested.** `pages.rs`: a cell for each figure a source
reports, under all sixteen sets of silent sources, none of GitHub's, and no
official mark in either language. `e2e-site`: the strip's lines measured for
all sixteen sets of silent sources at ten widths in all eleven languages,
axe and the 44 px targets; one press of the switch on a dark and a light
device, with nothing, light or dark kept, on both sizes, and the kept scheme
on the root before the body begins; no "official" on the start page or in
the directory. `tests/hover.rs` and `tests/contrast.rs` still hold.

## Ways to keep a recovery code, and an address at registration, 12 November 2026

The registration's step that shows the recovery code once offered copying
and a text file. On the owner's decision after the trade-off was put to
him ("Wie empfohlen + Mail an mich"), it now offers four more ways out of
the page and an e-mail address for the account
([web accounts](web-accounts.md#keeping-the-recovery-code)): a text file
with two lines of explanation, a print, the browser's password manager,
and a mail to oneself through the person's own mail program. The same work
gave the mails their new look ([e-mail](email.md#how-a-mail-looks)); that
changed no rule of theirs and is not part of this record. No change to
`central-api`, no migration, no new dependency, no change to `Cargo.lock`,
no `web-sys` feature added.

Design pass. The asset is the recovery code: with a reset code it resets
the login and keeps the encrypted data ([account reset](account-reset.md));
without one it does nothing, since Central hands its wrapper out only to a
reset. A reset code comes from an administrator or, by mail, from the
account's confirmed address. So every place the code is kept is weighed by
who can read it and whether they can also get a reset code. Central never
learns the code, before or after this change.
- **Made on the press, never before.** The file's `data:` link, the
  `mailto:` address and the print's sheet exist from the press of their
  button on; until then the code is in the page once, as the step's text,
  and in no attribute. Nothing is sent to Central, nothing logged, nothing
  written to the page's storage, nothing put into the page's address.
- **The file** lands where the browser saves files, possibly a folder a
  cloud keeps. Its two lines say what the code does and to keep it where
  only its owner reaches it.
- **The print** passes the operating system's print queue and, printed to
  PDF, becomes a file. Paper is the way that is easiest to keep offline.
- **The password manager** (Chromium's `PasswordCredential`): the browser
  asks the person, and may sync the entry to the browser's account. The
  entry is listed under "Baylee recovery code (<sign-in name>)", which is
  no sign-in name the site takes (a test holds it for every language), so
  it never replaces the entry of the account's password, and if the
  browser offers it on the sign-in form, the page refuses it before
  anything is sent. Whoever reads the manager usually reads the account's
  password there too, which signs in and opens the data already; the code
  adds little where the password is kept, and is a second place where it
  is not. Firefox and Safari offer no such interface; there is no button.
- **The mail to oneself** puts the code into a mailbox, its sent folder,
  the provider's servers and backups. Whoever reads that mailbox, if it is
  the account's address, can have a reset code mailed there and then, with
  the code, reset the login and open the data. The warning says so in two
  sentences, stands before the button and is its accessible description.
  The owner chose to offer it with that warning; it is the weakest of the
  ways and stands last and apart.
- **The address at registration** keeps the rule that naming an address
  needs a password sign-in less than five minutes old: the registration's
  own sign-in is one (`COALESCE(proven_at, created_at)` of the session
  `LoginFinish` just made), and Central, not the page, decides. A sign-in
  that has grown older is refused, and the page then asks for the
  password and signs in once more, as the account area does. Naming one
  needs Central's terms accepted, which a new account meets after its
  code, so the page checks the address with Central's rule and names it
  only after the code is kept and the terms are accepted. Nothing about it
  goes out while the field is empty.
- **The mail's address.** Only an address that `central_api::email::address`
  takes goes into the `mailto:`, percent-encoded; anything else leaves it
  empty and the mail program asks. No header can be added through it.
- Decided against: keeping the code for later or showing it again (shown
  once stays the rule); sending it by Central's mailer (Central would learn
  it); a QR code (one more picture of the code to be photographed).

Implementation pass: looked at each boundary in turn.
- **`keep.rs`.** Pure functions for the file's text, the `data:` address
  and the `mailto:` address, unit-tested; the browser's side behind
  `cfg(target_arch = "wasm32")` with empty stand-ins on the server. The
  print and the password manager are reached through `js_sys::Reflect`,
  as the clipboard is; the file through a link made, clicked and removed
  at once; the mail through `session::open` (`location.assign`), shared
  with the Impressum. No `innerHTML`, no script URL, no inline script: no
  new Trusted Types sink and nothing the CSP has to allow.
- **The print** draws the sheet into the step when pressed and asks for the
  print after the next frame; `pages.css` shows the sheet alone in print
  media, including from the registration's dialog, which a print hid
  before. The sheet stays until the step ends.
- **The password manager's answer** says only whether the browser took the
  offer; the page says that and asks the person to check. It never claims
  the code was saved.
- **The address's step** is the account area's section (`email.rs`,
  `Begin::Name`), so the code's field, the wrong-code and lost-answer
  handling and the form with the password are the ones already tested.
  `Set` without the password is sent once, when the step opens; a refusal
  that leaves a state to read draws the form with the address and asks for
  the password; any other refusal says what it was.
- **Secrets.** The code is held by the step's signals and the DOM as
  before, and in a closure for the password manager's offer; none of it
  outlives the step. `NewCode`'s `Debug` prints the code, as it did; it is
  printed nowhere.

Adversarial pass, with what was tested.
- **Unit** (`keep.rs`): the file is the title, the name, the code alone on
  its line and the two lines, in every language; the `data:` address
  encodes every reserved byte; the `mailto:` address carries the subject
  and body with CR LF and nothing that ends a field early, takes an
  address only if Central's rule does (a second `@` with `?cc=`, a CR LF
  with a `Bcc:` header, `localhost` all leave it empty), and encodes what
  a dot-atom may hold; the password manager's id is no sign-in name.
- **Browser** (`web/e2e-site/recovery.spec.ts`, desktop and phone, the
  real island against a stand-in Central and a canned Worker): before any
  press the page's markup holds the code once and no `mailto:`, no
  `data:` and no sheet; the mail's address is built on the press (seen as
  the browser's navigation) with the typed address, the subject and the
  body, and never enters the page; the file downloads with its content and
  its link is gone; the password manager is offered the code under its id
  where `PasswordCredential` exists, and there is no button where it does
  not; the print, in print media, shows the sheet alone, from the page and
  from the dialog; no request body holds the code; the address is named
  without a sign-in once more, after the terms where they are in force,
  and confirmed on the next step; a stale sign-in asks for the password;
  an address Central would refuse is refused before anything is sent;
  skipping sends nothing about an address; where Central sends no mail
  there is no field. Axe in both schemes, 44 px targets, 320 px and the
  keyboard's order on the steps; in German at 320 px, on the page and in
  the dialog (which hides what runs out of its card), nothing of either
  step reaches past the card. No console line holds the code or a
  `mailto:`.
- **Mutants** (one run each): the `mailto:` address built without
  Central's check of the address, and `&` left unencoded; both were caught
  by the unit tests. No mutant was run against the browser tests.

Not tested.
- The real password managers: Chrome's and Edge's prompt, sync, and what
  the sign-in form then suggests. The tests record the offer instead. One
  probe without the stand-in: Playwright's Chromium (build 1248) makes the
  credential from the id and the code as the page does, and
  `credentials.store` resolves in its full browser; its headless shell,
  which has no password service, refuses it, and the page would then say
  that the browser did not take the code.
  Firefox and Safari were not run; their lack of `PasswordCredential` is
  from MDN's compatibility data and was simulated in Chromium.
- A real mail program, and a web mail registered as the `mailto:` handler.
- Paper, and print to PDF: only print media were emulated.
- `e2e-real` (the registration's address against a real Central and the
  mailer stand-in): adapted and type-checked, not run.
- Screen readers.

For the owner.
- **A web mail as the `mailto:` handler.** Where a person made a web mail
  (Gmail, for one) the browser's handler for `mailto:`, the browser opens
  that site with the whole `mailto:` address in its URL: the code is then
  in the browser's history and the provider's request logs, and the step
  is left in the same tab before "I have saved my recovery code". A
  desktop mail program does not do that. The page cannot tell which one
  the browser uses.
- **The mail to oneself** is offered as decided, with its warning. It is
  safe only where the mailbox is not the account's address or is guarded
  as well as the account.
- **The password manager** may sync the code to the browser's account,
  and the browser may suggest the entry on the sign-in form.
- **The address is confirmed on its own step** after the code, and after
  Central's terms where they are in force, not on the code's step:
  Central refuses `Set` before the terms are accepted, and the code must
  come before the terms (a person who declines them keeps a stored code
  they saw).
- **A refused `Set` has more than one reason.** The step reads a refusal
  as a sign-in grown old and asks for the password once more, without
  saying why; if the mailer refused the address itself, the second `Set`
  is refused too and the section says that it did not work, as the
  account area does.
- **A moved account's code step** (after a sign-in) has the ways to keep
  the code and no e-mail field; it may have an address already.
- **Nine languages** carry the new words in English, and their
  `code-lead` still says to keep the code "away from this device"
  (English and German now say "where you will find it even without this
  device").
- **The privacy notice** has a second paragraph under "What your browser
  keeps" (`privacy-device-manager`): a code kept in the password manager,
  a mail, a file or on paper is kept there, never by the site, and
  clearing the site's data does not remove it. `legal/privacy.*.md` was not
  changed; no sentence of it became untrue (it says Central never learns
  the code).

Checks. On the code of `5217887` (what follows it changes the documents
and gives the browser tests a longer time): `cargo fmt --all --check`,
`cargo clippy --locked --workspace --all-targets -- -D warnings` and the
islands' `cargo clippy --locked -p central-web -p central-web-islands
--target wasm32-unknown-unknown --lib -- -D warnings` pass; `cargo test
--locked -p central-mail -p central-web -p central-web-islands -p
central-web-server` passes; `cargo xtask gate` passes (format, policy,
clippy, 680 tests with the doc tests, the release build); `cargo xtask
site` built the site; `cargo xtask secrets` passes on the tree and the
full history; `cargo xtask smoke` passes; `cargo xtask postgres` passes
and its container was gone afterwards; `cargo audit --deny warnings` and
`cargo deny check` pass. In `web/`, `npm run typecheck` and `npm run lint`
pass. The site's browser suite (`e2e-site`, both projects) ran in full
twice while other work loaded the machine: 393 passed and 7 ran out of
their 30 seconds the first time (four on `/de/signin` and `/de/register`,
none of them new), all seven passing when run again; 403 passed and one
new test ran out of its time the second time, after which the new tests
were given the longer time of the suite's other slow ones and passed
again with `dialogs.spec.ts` and `email.spec.ts` (62 passed).

Engineering review, not an audit.

## The code in the mail button's fragment, and the mails' logo, 12 November 2026

The owner asked that whoever comes to the page through the mail's button
find the code filled in ("wähle einen guten sicheren weg"). Until now the
rule was that no code is ever part of a link. The way chosen: the code
travels in the fragment of the button's link and nowhere else
([e-mail](email.md#the-code-in-the-buttons-link)). The reset mail's button
links to `{site}/{lang}/reset#code=RESET-…`; the confirmation mail has a
button now, to `{site}/{lang}/account/security#code=XXXX-XXXX`. The same
round made the mails' second look (no rule of theirs changed but this
one) and lets them show Baylee's logo where the mailer attaches it. No
change to `central-api`, no migration, no new dependency, no change to
`Cargo.lock`, no `web-sys` feature added.

Design pass. The assets are the two codes. A reset code with the sign-in
name takes the login (one hour, one use, the account's latest mailed one);
a confirmation code makes the mailed address the account's, for a session
already signed in to it (15 minutes, five tries). Both are in the mail
already, as text: whoever reads the mail has them, link or no link. So
the question is only what a link adds, and to whom.
- **The fragment and nothing else.** A browser sends no fragment to the
  server and puts none into a `Referer`, so the code is in no request, no
  log of Central's, the website's, Caddy's or a proxy's, and the site
  sends `Referrer-Policy: no-referrer` besides. A path or a query would be
  in all of them. Never in any other link of a mail: the footer's link and
  the notices' buttons carry none.
- **Taken once, removed at once.** The island reads `location.hash` when
  it starts, keeps only a code that its own parser takes (`ResetCode` on
  the reset page, `MailCode` on the security page, with at most 64
  letters, digits and hyphens before parsing), and removes any `#code=`
  fragment with `history.replaceState`: the current entry of the history
  is replaced, not added to, so going back or forward or reloading finds
  no code. A code of the other kind or a malformed one fills nothing and
  is removed the same way.
- **Never by itself.** Nothing is sent because of the fragment; the
  person presses the form's button. Mail providers and security gateways
  open links to scan them: such a visit runs the page, perhaps, and
  spends nothing, and a signed-out scanner's visit to the security page
  asks Central nothing at all.
- **The security page** has the code's field only while an address waits.
  The code is kept in the island's memory until the section's first read
  of Central's state and dropped there unless an address waits; signed
  out, without mail or with the state unread, it is dropped without a
  word. It never fills a field drawn later.
- **Not stored, not logged.** The code lives in the field as a typed one
  does. No `localStorage`, no console line, no request.
- Decided against: a query (`?code=`), which every server and proxy logs
  and a `Referer` carries; a code that submits itself (a scanner would
  spend it, and a person could not see what is sent); a separate token
  that stands for the code (one more secret with the same reach); putting
  the sign-in name into the reset link too (it would make the link alone
  a whole reset).

What remains, and why it is acceptable:
- **The browser's history may keep the first address.** The fragment is
  in the address bar from the navigation until the island runs, which
  waits for the page's largest paint; a browser may record that address
  in its history or sync it before the replacement. Without script it is
  never removed. The code there is one-use and short-lived (15 minutes,
  one hour), the history is the person's own, and the mail that holds the
  same code sits in the same person's mailbox, usually for longer.
- **Mail providers and link rewriters see the address.** Outlook's Safe
  Links, Gmail's link checks and similar gateways rewrite or fetch the
  link and see the whole address, fragment included, as they see the mail
  and its code in the text. They learn nothing they did not have, and
  their fetch submits nothing. Checked in Gmail's web view on 12.11.2026
  by a script that answered only yes or no: the reset mail's button
  carries a `data-saferedirecturl` that begins with
  `https://www.google.com/url?` and holds the whole link, `#code=`
  included, percent-encoded in its query. Gmail's web view opens that
  address when the link is clicked (its known behaviour; the link was not
  clicked here), so the browser's history may hold Google's address with
  the code as well, an entry on another site that the page cannot
  replace. It is recorded as a residual on the same grounds as the first
  bullet (one use, 15 minutes or one hour, the person's own history, a
  provider that holds the mail); the owner has not yet said whether it is
  accepted.
- **A link in a tab that shows the page already** is a move within the
  page, which runs no island; on such a change to `#code=` the fragment is
  removed as at the start (`fragment::watch`) and fills nothing. Loading
  the page anew would fill it, and was decided against: it would throw
  away a step the page shows once, a new vault's recovery code on the
  reset page. The person types the code from the mail there.
- **Whoever opens the mail's link on a shared computer** leaves the page
  with the code in its field until it is sent or the tab closed, as a
  typed code would be.

The logo. A mail could show it by a remote address (a tracker, and
nothing a reader loads by default) or as part of the mail: the mailer
attaches it under `Content-ID: <baylee-logo>` and the HTML names
`cid:baylee-logo`, so nothing is loaded from elsewhere. An older mailer
refuses the request field `inline`, so Central asks for it only when the
operator says the mailer has it (`"logo": true` in the `mail` block,
default false); deployed in the wrong order, every mail is refused and
logged as not handed over, and turning the flag off restores it. With the
flag off the request and the HTML are as before.

Implementation pass: looked at each boundary in turn.
- **`central-mail`.** The code-bearing address is built by one function
  (`with_code`), held in `Zeroizing` as the code is, and written into the
  button and the text's line under it; the HTML escapes it as everything
  else. The footer's start page and the notices' buttons take no code.
  The logo is one fixed `<img>` with a fixed `cid:`, no value of the mail
  in it.
- **`central-web`.** `islands/fragment.rs`: a pure, host-tested parser
  (`code`), and the browser's half through `js_sys::Reflect` (`take`,
  `watch`), no new `web-sys` feature. The reset island takes the fragment
  in `Form::early` beside the fields typed before it ran; the account
  island takes it on every page of the area and keeps it for the security
  page only, handing a `StoredValue` to the e-mail section, whose first
  `load` takes it (`Option::take`) before it awaits anything.
  `replaceState` is given the page's own address without its fragment,
  so it cannot point elsewhere.
- **Secrets.** The fragment's text is a `String` dropped at once; the
  parsed code fills the field's signal, as typing does. Nothing prints it.

Adversarial pass, with what was tested.
- **Unit** (`central-mail`): every language and every mail, the links are
  exactly the button's page (with `#code=` and that mail's own code for
  the two code mails, nothing for the notices) and the start page; no
  code before a `#`, no query, in the HTML and in the text; each code mail
  shows its code once in its box and once in its link; with the logo, the
  one image is the `cid:` one and the rest of the mail is unchanged; the
  client sends `inline` only with the flag; both documents take the flag,
  refuse a value that is no boolean and still refuse unknown fields.
- **Unit** (`central-web`, `fragment.rs`): each page takes its own kind,
  printed or as typed; the other kind, an encoded character, a second
  field, a space, a wrong letter, a full-width digit, an upper-case name
  and too long a text are no code; any `#code=` fragment is removed and a
  place on the page is not.
- **Browser** (`web/e2e-site`, `reset.spec.ts` and `email.spec.ts`, desktop
  and phone, the real islands against a stand-in Central): the field is
  filled, the address bar and `location.href` hold no fragment, the
  history's entry was replaced (its length grew by one, back and forward
  find no code, a reload fills nothing); no request to Central before the
  person sends the form, and no request the page makes after its
  navigation holds the code in its address or its `Referer` (the
  navigations themselves are left out: Playwright reports them with the
  fragment it was given, which a browser does not send); the form then
  sends it; a code of the other kind, a recovery code, a cut code, an
  encoded character, a second field and an empty one fill nothing and are
  removed; another fragment stays; a code typed before the islands ran
  stays; the link opened in a tab on the same page removes the fragment
  and leaves the page as it was, a new vault's recovery code still shown.
  On the security page: only `status` is read until the person
  confirms; no address that waits, no mail, a session Central no longer
  knows and no session remembered each drop the code (and the last asks
  Central nothing); a first read that fails drops it, so the field is
  empty after "Try again"; with no address that waits, an address named
  afterwards whose answer is lost (the field drawn without being
  cleared) has an empty field; another page of the area removes it and
  keeps nothing for the security page.
- **Mutants** (one build, both at once): the code filled where no address
  waits, and the code kept after a first read that failed. Each was
  caught by its own case, the field holding the code where it must be
  empty. No mutant was run against `central-mail` or `fragment.rs`.

Not tested.
- Real mail programs and gateways: Outlook's Safe Links, Gmail's checks,
  mailbox.org; the mails' second look and the logo in a real mail program
  (the owner saw the first round and a quick logo test only).
- Firefox and Safari; the history a browser syncs.
- `e2e-real` (the mails through the mailer stand-in against a real
  Central): its check of a mail's links was adapted to the new rule and
  type-checked, not run.

For the owner.
- **The sign-in name is still typed** on the reset page: the link carries
  only the code, so that the link alone is not a whole reset.
- **Deploy the mailer with the picture first**, then set `"logo": true`
  (operations, "Mail").
- **Nine languages** have the confirmation mail's new button in English.
- **A code that arrives while the page is open fills nothing.** Filling
  it there without a reload is possible on the reset page's form step and,
  by drawing the account area anew, on the security page; it was left out
  as the smaller change.

Checks, on the code of `b6c1a81`: `cargo fmt --all --check` and `cargo
clippy --locked --workspace --all-targets -- -D warnings` pass, and the
islands' `cargo clippy --locked -p central-web -p central-web-islands
--target wasm32-unknown-unknown --lib -- -D warnings`; `cargo test --locked
-p central-mail -p central-identity -p central-service -p central-web -p
central-web-islands -p central-web-server` passes; `cargo xtask site`
built the site. `cargo xtask gate` passed the format, the policy, both
advisory scopes and clippy, and every test of every crate but one test of
`xtask`'s own: `site::tests::no_language_is_english_under_another_name`
finds 127 of French's 1023 website messages English, over its 12 %. This
work changed no website words and no `xtask` code, so the test fails the
same on the commit this work began from; the gate stopped there, and its
last two steps were run by hand: the doc tests and the release build pass.
In `web/`, `npm run typecheck` and `npm run lint` pass (`typecheck` does
not cover `e2e-site`; the new tests there were checked against the
strict settings separately and add no error). The site's browser suite
(`e2e-site`, both projects): 527 passed, 4 skipped, and one failed,
`admin.spec.ts`'s first administrator test, on finding ten animations
with reduced motion asked for in the dark scheme; it fails the same with
this work's website sources set back to the commit it began from. The
mails were written in English and German, with and without the logo, and
looked at in Chromium at 360 and 640 px, as drawn and inverted. Not run:
`cargo xtask postgres`, `smoke`, `secrets`, `cargo audit` and `cargo deny`
(no dependency and no migration changed; the service only reads one more
optional field), and `e2e-real`.

Engineering review, not an audit.

## The reset page's new recovery code kept the registration's ways, and stillness reaches a view transition, 12 November 2026

The reset page's step that shows a new vault's recovery code offered a
copy button and a `data:` link that stood in the page, the code in its
address, from the moment the step was drawn. It now offers what the
registration's step offers ("Ways to keep a recovery code" above):
`keep::ways` in `islands/reset.rs`. Carrying the owner's decision for the
registration over to the reset page was this session's call, not a new
decision of the owner's. No change to `central-api`, no migration, no
dependency.

Design pass. The asset is the same recovery code with the same reach;
the step shows it once, after a reset that ended every session. The
registration's record holds for each way. Two differences: the reset page
knows no e-mail address, so the mail to oneself leaves the recipient to
the mail program (`mailto:` with an empty address, as the registration
does when no valid address is typed); and the file and the sheet name the
sign-in name the reset was for, which the person typed into the form.
Taking the old link away removes an address that held the code before
anyone asked for a file. Two layout rules met on the way: a check row's
label could not be narrower than its longest word ("Wiederherstellungs-
code" ran 6 px out of the card at 320 px; the column is now
`minmax(0, 1fr)` and the label breaks and hyphenates), and the
reduced-motion rules did not reach the pseudo-elements of a view
transition, so one that began while motion was welcome kept the
browser's own fades (ten animations; the open finding of the record
above). Each of `::view-transition-group`, `-image-pair`, `-old` and
`-new` now has `animation: none` under reduced motion, in a rule of its
own.

Implementation pass. `Outcome` carries the sign-in name, lower-cased as
it was sent; `code_view` renders the code once as text and `keep::ways`
beside it; nothing else of the step changed (the box that confirms the
code is kept, the way on only after it). The reset island no longer uses
`clip.rs` directly. No new `web-sys` feature; the islands build for
`wasm32-unknown-unknown` without warnings.

Verification pass. `web/e2e-site/reset.spec.ts`: the new vault's step
offers copy, file, print and mail; before any click the page holds the
code once and no `data:text`, `mailto:`, download link or sheet; the file
is saved as `baylee-recovery-code.txt`, begins with the title, the sign-in
name and the code, and its link is gone again; the mail opens a `mailto:`
without an address whose body is the file's text; axe, the targets and
320 px pass; in German at 320 px, at a desktop's and a phone's size,
nothing of the step reaches past its card, both schemes pass the colour
check with reduced motion, and the print shows the sheet alone (six lines,
the code among them) and the screen hides it again. A probe that left a
page and asked for reduced motion during the view transition found ten
animations before the rule and none after. The site's browser suite and
`e2e-real`'s `site-reset` (a new vault redeemed against a real Central)
ran on this work as reported in the pull request.

Engineering review, not an audit.

## Profile pictures, 12 November 2026

An account may set a profile picture that everyone sees, signed in or not
([profile pictures](profile-pictures.md); the owner's decisions and the
packages he accepted in the [proposal](dependency-proposal/profile-picture.md)
§0 to §0.8). New: migration 000027, the crates `central-picture` (the
protocol and the service's checks) and `central-picture-codec` (every
decoder and encoder, linked only by the worker binary), routes on the
browser listener, wire variants, moderation, the website's card, and
ffmpeg as a native runtime dependency of the worker's host.

Design pass. The assets are the service's secrets and database, which a
stranger's file must not reach; the people shown in pictures; the
reporters, whom the reported never learn; and the moderators' decisions.
- **Containment.** Every decoder runs in a fresh process per upload, which
  holds no secret, has no network and no file but a private `/tmp`, and is
  bounded in memory, tasks and time by its systemd unit; the service links
  no codec. The worker's answer is checked by the service as if written by
  an attacker: a PNG master of exactly three chunks with right CRCs, an
  AVIF of exactly three boxes, a sequence of exactly four whose sample
  tables agree; anything else refuses the picture.
- **ffmpeg** (owner, §0.7 and §0.8) is a child of the worker, inside the
  same unit, and is fed only frames the worker decoded, bounded and
  averaged itself, as raw pixels: its parsers never see a stranger's file
  (ffprobe 8.0.1 crashed on a partial file in the research). No shell, an
  absolute path from the worker's command line, no environment, the pipe
  in and one file out in a fresh directory only the worker may enter, a
  deadline. Its failure keeps the still; it never fails the upload, and
  the service logs a fixed line so a broken sandbox is seen.
- **Bounds before work:** upload sizes from `Content-Length`, admission and
  budgets before the body, header readers of our own before any decoder,
  an AV1 sequence header bound before `rav1d`, an SVG scan before `usvg`,
  frame, play, canvas and composited-pixel bounds for animations.
- **Visibility:** public URLs under a 16-byte random key, new at every
  upload, never an account's id; keys learned only from answers that name
  the account already, with the lookup budget and blocks; the same 404 for
  every refusal; at most an hour in a cache.
- **Reports** name a picture by the digest of its master and its sequence,
  so the animation shown is what was reported; administrators see the
  animation and can open it alone.

Implementation pass. Read for `unsafe` (none: every target root forbids
it, the policy check passes), panics that cross the process (decoders'
panics become `malformed`; `rav1d`'s abort ends only the worker), lengths
and offsets (every box and chunk reader checks its size against what is
left), logging (no picture, key, address or account in any line; the
worker prints nothing but its answer), and the ffmpeg runner: the frame
writer is never waited for beyond the deadline, so a child that holds the
pipe cannot hold the worker; the directory is removed on every path; the
probe and every attempt share one deadline inside the service's.

Verification pass. Hostile files on every input path and per format
(bombs, lying headers, cuts, broken checksums, polyglots, scripts and
references in SVG, frame bombs and huge canvases), a mutation fuzzer for
every decoder entry point with no finding (`fuzz/README.md`), stand-in
encoders for every failure of ffmpeg, the real `libaom-av1` in Ubuntu
26.04's ffmpeg for durations, tracks and colour, the HTTP routes byte for
byte, the PostgreSQL fixture with the grants both ways, and the website at
320 px in both schemes. Not verified: ffmpeg inside the production unit's
system-call filter and limits (an operator's check after deploy,
[operations](operations.md#profile-pictures)), anything on x86-64, and
Firefox and Safari playing the sequences. This is an engineering review,
not an audit.

### An independent review, 12 November 2026 (addendum)

An independent reviewer probed the build. Findings and what changed:
- **M1, slots held for free.** Two accounts could hold both conversion
  slots with bodies that stall or trickle: the slot was taken at
  admission and the budget charged after the body. Fixed: the slot is
  taken only once the whole body has arrived (bodies stay bounded by the
  listener's places per address).
- **M2, reports lost on re-upload.** Removing a picture and uploading the
  same file again left every open report `gone`. Fixed in the store: a
  picture with the same digest reopens its `gone` reports, unless their
  reporter opened another. Second half (later the same day): each report
  opens its account's case (`PictureAdmin::Case`), where an administrator
  removes the current picture by its digest without an open report, under
  the same lock, fresh sign-in, role check and an audit of the reason code
  alone; keeping still needs a report. Tested on PostgreSQL and in the
  browser.
- **L1, workers outliving their slot.** The slot was freed at the
  service's deadline or when the request dropped, while the worker ran on.
  Fixed: the slot moves onto the conversion's thread and lives until the
  worker ended; the worker ends itself at 19 s; `RuntimeMaxSec=20`.
- **L2, SVG entities.** Entity expansion passed the element and depth
  scan (25 KB took 548 MiB and 9.3 s). Fixed: any internal subset and any
  `<!ENTITY` are refused before a parser runs; the probes are a test.
- **L3, looser answer checks** than the worker writes. Fixed (later the
  same day): a still, and a sequence's still item, hold exactly one AV1
  item or one and its alpha auxiliary; every `ispe` is 256 × 256; colour
  only `nclx`; no Exif, XMP, ICC or other box; every AV1 sequence header
  allows at most 256 px. Hostile answers through a stand-in worker are
  refused; real `ravif` and libaom output passes. A compromised worker can
  still choose any pixels within those bounds.
- **Info items:**
  - fixed:
    - Caddy's 64 KiB body bound would have refused uploads in
      production; a `handle /api/v1/picture` block now comes first;
    - the unit has `NoExecPaths=/` with `ExecPaths=` for the worker,
      ffmpeg and the libraries, `PrivateIPC=yes` and a 64 MiB `/tmp`;
    - pictures are cached `private`;
    - the key lookup now needs a display name, as `Lookup` does;
    - the fuzzer now counts panics;
    - a flaky encoder test: a busy just-written stand-in is spawned again;
  - not fixed: `rav1d`'s threads, which `avif-decode` 3.0.0 offers no
    setting for.
- **Still not verified:** the unit's new settings, under systemd, with
  ffmpeg.


## The administration's other pages on the new site, 12 November 2026

The old build's administration (`/admin`, the accounts, one account, the
gateways, the audit log, the legacy store) is rebuilt on the new site
under `/{lang}/account/admin/…` ([frontend
rewrite](proposals/frontend-rewrite.md) §25), so that the old build can go
(the owner, 12.11.2026). Central is as it was: no request, answer, server
crate, migration or dependency changed; `Cargo.lock` is untouched.

Design pass. The assets are what an administrator reads (every account's
sign-in name, ids, the audit log, the legacy store's names) and the four
changes the pages make: the administrator role (whoever holds it sees
and changes everything), a tag (it moves a handle, and loses an account
its gateway-bound whisper history), a gateway's approval (the public
list, and disabling ends its players' sign-ins), and a token's
revocation. Central decides who may read and change, and refuses a
change without a password sign-in under 30 minutes old; the pages decide
nothing Central does not decide again.
- **Who sees the pages.** As for the reset code's page: the server cannot
  know the role, every address answers everyone with the same frame, and
  the island draws a page, and the administration's own navigation, only
  for a session whose `AccountSummary` says `admin`. Anyone else gets one
  sentence and no administration request. No page the server renders
  links to the administration.
- **One account's address** carries an id the visitor chooses. It is read
  strictly (32 lowercase hex digits) and the page is rendered for each
  request and never kept: the table of rendered pages stays bounded by
  the pages there are. The cost moves to the processor: it is the one
  address anyone, signed in or not, can have rendered afresh with every
  request (a frame and "Loading …", like any account page), where every
  other answer is rendered once; limiting requests is Caddy's, as for
  any address. An id is no secret (`AGENTS.md`); the reset code's page
  takes an account by the fragment (`#account=`), which never reaches the
  site, and removes it once read. A link someone else made opens that
  account on the reset code's page for reading only: nothing is made
  without the warning and the password, as for an account opened from
  the list.
- **The order of a change.** Nothing that changes is sent from a button:
  it opens a confirmation that says what follows; only the confirmation's
  own button sends, after `AccountSummary` (another account in this
  browser sends nothing; no role sends nothing), and where the sign-in is
  older than 29 minutes by the browser's clock, after the password proven
  once more in a Worker ended at once, for the session's own account.
  The confirmation gives way to the work before the request leaves, and a
  page that is working takes no other change: a double press sends one.
- **What a refusal may say.** Central answers each refusal with one
  `Rejected` (a tag also with `Conflict`). The page says what stands
  afterwards, in this order: the role, whether what was changed still
  exists, then the sign-in's age, for which it asks the password once and
  never again for the same attempt. An answer that did not arrive may have
  made the change: the page never calls it a refusal, reads what it shows
  again and says that it cannot tell; where that read fails too, it says
  to load the page.
- Decided against: the search in the address (as for the reset code's
  page); the navigation's counts (every page, the reset code's included,
  would ask Central for them).

Implementation pass:
- `islands/admin/common.rs`: the decisions are plain functions with unit
  tests (`before`, `at_change`, `explain`, `came_text`, `unread`,
  `action_text`); the flow (`send`, `send_after`, `Flow`). Reads go
  through `read`, whose `Rejected` is a lost session only where
  `AccountSummary` is refused too. Only the newest list request is drawn.
- `data.rs`: `Section`'s administration variants, `id_hex`/`id_from_hex`;
  `site/mod.rs`: `Page::path`, `Page::kept`; `central-web-server`'s
  `routes.rs` skips the table for a page not kept.
- No new dependency. Nothing is logged; the password is a signal's
  string, emptied when it is sent and dropped after the Worker's sign-in.
  No lint was switched off; no `unsafe`.

Adversarial pass, each question with what answers it:
- *Does anyone without the role get a page, the navigation or a request?*
  No: the browser test opens all seven pages without the role and sees
  one sentence each, no navigation, no link into the administration in
  the frame, and no administration request; the server's test finds no
  link to it in any page it renders.
- *Is anything sent before the confirmation?* No: the tests count the
  requests from each button through its confirmation and its way back
  (none), and after it exactly one change, also for a double press.
- *Is the password proven before a change on a stale sign-in, for the
  session's own account?* The tests see `account_summary` only, then
  after the password `account_summary`, `login_start`, `login_finish`,
  `account_summary` and the one change; with Central counting the
  sign-in old where the browser does not, the refusal is explained and
  the password asked for, with no second change. A browser signed in as
  another account meanwhile sends no change.
- *Can a refusal or a lost answer read as something it is not?* Unit
  tests: a lost answer never a refusal, a refusal never asking for the
  password twice, every outcome its own sentence in every language. In
  the browser: an account and a gateway gone meanwhile, the role gone,
  `Busy`, a taken tag, and the three kinds of lost answer (none; another
  kind; 503), with the read afterwards showing what Central holds.
- *Can ids fill the server's table?* The server's test asks for twenty
  ids and finds the table unchanged.
- **The checks have teeth** (one change at a time, each undone, the site
  and the server rebuilt): one account's page reported as kept, and the
  server keeping every page whatever it reports (both caught by the
  server's test, the second by its count of kept pages); the pages drawn
  without the role; a change sent from its button without the
  confirmation (both caught in the browser).

Run on this change: `cargo fmt --all --check`; `cargo clippy` for
`central-web` and `central-web-islands` on `wasm32-unknown-unknown` (no
warning); `cargo test -p central-web -p central-web-server -p xtask`
(with the English share); `cargo xtask secrets` (the tree and the
history); `cargo xtask site`; the site's whole browser suite
(`web/e2e-site`, 596 passed, 4 skipped, both projects, the 74 of
`admin*.spec.ts` among them); the new specs type-checked under the strict
settings and `oxlint --deny-warnings e2e-site`. Two findings of `cargo
clippy --workspace --all-targets` with the gate's `-D warnings` are in
code this change did not touch, both from the BIMI mark (d41fce5):
`site/shell.rs`'s `footer` is 104 lines, and `xtask`'s `BIMI_SHA256` is
never used outside its tests. So `cargo xtask gate` as committed stops at
clippy on them; with both set aside locally (an `allow` each, not
committed), it passes: format, policy, the crypto and SQLite scopes,
clippy, tests, doc tests and the release build. Not run: `e2e-real` (no
real Central here; `site.ts`'s `issue` was read, and its locators still
find one element each), `cargo xtask postgres`, `smoke`, `cargo audit`,
`cargo deny` (no dependency changed).

Engineering review, not an audit.


## The new site moderates bug and crash reports, 12 November 2026

Until now an administrator triaged reports (`docs/reports.md` §"Triage")
only on the old build's `/admin/feedback`. This change rebuilds it on the
new site, `/{lang}/account/admin/feedback` and one report at
`/{lang}/account/admin/feedback/<id in hex>` (`islands/feedback/`,
[frontend rewrite](proposals/frontend-rewrite.md) §26), so the old build
can go. Central is as it was: no request, answer, route, server crate,
migration or dependency changed; `Cargo.toml` and `Cargo.lock` are
untouched, and no `web-sys` feature was added.

Design pass. The assets are the reports (a player's words, a pseudonym,
what the client sent: a screenshot, the game, the log, the system; a
game's record), which only an administrator may read; the triage, which
only an administrator may change; and the administrator's password,
which deleting for good may take once more. The trust boundary is the
report: a game client wrote it, so every byte of it is hostile to the
page. Central decides who reads and changes; the page decides nothing
Central does not decide again.
- **Who sees the pages.** As the reset code's page (above): the server
  answers everyone with the same frame and cannot know the role; the
  island shows the tool only where `AccountSummary` says `admin`, and asks
  Central nothing more otherwise. No page links to the addresses. Every
  report's address is one page without an id, so the server keeps and
  varies nothing by it; anything but 32 lowercase hex digits is a 404.
- **A report is text.** Every field is drawn as a Leptos text node: the
  words, the crash and its place, the gateway's names and address, the
  builds, the game id, labels, notes, history details, saved lists' names,
  the client part's every value and every key of its tree. No `inner_html`
  exists, and the site's Trusted Types would refuse one. The gateway's
  address is shown, never linked. The only addresses built from report
  data are the old build's two: Baylee's issue by its number (1 to
  10,000,000), and GitHub's new-issue form from the administrator's own
  summary, a category and a build that reads as a version; both go to the
  one repository, in a new tab without opener or referrer. The old
  build's card chips with Scryfall's pictures and links are not rebuilt:
  the site's `img-src 'self'` admits no other origin, and a link to
  another site built from a client's data was not needed.
- **The client part** is untrusted gzip: read from Central's own route no
  further than its announced bound, its magic checked, inflated by the
  browser's `DecompressionStream` and cut off past 16 MiB while it
  streams, decoded as strict UTF-8, parsed as a JSON object; every reader
  takes its shape or nothing. The window's picture is shown only as a PNG
  whose signature and `IHDR` (1 to 16,384 px a side) were checked, drawn
  into a canvas by `createImageBitmap` from its bytes: no `blob:` or
  `data:` address, so `img-src 'self'` stays. The tree draws a fold's
  children only when it opens, 200 at a time, strings past 240
  characters cut until asked, base64 counted, not shown. The record is
  handed to the browser as Central keeps it, never unpacked.
- **Addresses.** The list's filter is in the address (it was in the old
  build's, and a saved list is that query string); a report's address
  holds its id and that filter. Nothing a report says goes into an
  address, a storage, a log or the console. The CSV is made in the page
  and handed over through a `blob:` address that is given back after 30
  seconds; a cell that begins with `=`, `+`, `-`, `@`, a tab or a carriage
  return is defused with an apostrophe.
- **Deleting for good** cannot be undone, and is the one triage change
  Central wants a fresh password sign-in for (`AdminRequest::fresh`).
  Nothing is sent before the question is answered; the page asks
  `AccountSummary` and, where the sign-in is older than 29 minutes by the
  browser's clock, takes the password once more in a Worker ended at once
  and checks that the session is still the page's account; then one
  request, however often the button is pressed. Central refuses a stale
  sign-in and a report out of the trash with the same 401: the page tells
  them apart by reading the reports again (still in the trash: the
  sign-in; else: deleted nothing), not by its own clock.
- **A lost answer** (no network, a 5xx, an answer of another shape) is
  never a refusal: a change is read again and said as possibly made, a
  note's draft stays, a purge reads the list again.
- Decided against: rendering a report's Markdown or links; the drawer
  and the single-key shortcuts (a second way to the same requests, and
  single keys need WCAG 2.1.4's off switch); a quiet re-read every 30
  seconds; widening the CSP for Scryfall or for `blob:` images.

Implementation pass:
- `islands/feedback/`: `query` (the address, the filter, views, pills),
  `csv`, `text` (ids, alias, sizes, times, the chart's days, history
  words), `github`, `client` (the readers) are plain functions with unit
  tests; `api` holds every request, each answer taken only in its
  request's shape, and the part route and the browser's pieces through
  `js_sys`; `list`, `report`, `chart`, `tree`, `shared` are the views.
  Every filter goes through `ReportFilter`'s bounds, every change through
  `ReportChange::validate`, every label through `valid_label`, a saved
  list through `valid_view`, before it is sent; element ids and focus
  selectors are built from the 16-byte id only.
- `data.rs`: `Section::Feedback` and `Section::Report` (in `ROUTED`, not
  in `ALL`); `site/mod.rs`: the id-less match; `islands/account.rs`: the
  two sections for `admin` only; `pages/account.rs`: their titles.
- No new dependency or feature, no `unsafe`, no lint switched off.
  Nothing is logged.

Adversarial pass, each question with what answers it:
- *Does anyone without the role get the tool, or make the page ask?* No:
  in the browser both addresses show one sentence and the stand-in sees
  no administration request and no part asked for. The server's test
  finds no page of the area linking to them and no id in the pages.
- *Does a report's markup run or become an element?* The stand-in puts
  `<img onerror>` and `<script>` in every text field Central keeps and in
  the client part's values and keys: the browser tests find them as text
  in the list, the report's sections, the notes, the history, the saved
  lists' names, the crash groups and the tree, no `img[src=x]`, no
  `[onerror]`, no inline script, the marker never set, and no console
  message carrying it. The gateway's `javascript:` address is code text;
  no `javascript:` link exists.
- *Is anything sent before deleting for good is confirmed, and is it sent
  once?* The question and its "Cancel" send nothing; a double press sends
  one `reports_bulk` (list) and one `purge_report` (report); a stale
  sign-in asks the password first (`account_summary`, `login_start`,
  `login_finish`, then the one request), and the password is in no
  request body. A sign-in Central counts as stale while the browser's
  clock does not is found by the reports still in the trash, and the
  password is asked.
- *Does each action send exactly its request?* The browser tests compare
  every request with Central's serialized shape: the list's page with the
  whole filter, the facets, a view, the search, every chip and field, the
  order, the page and its size, saved lists, each selection's action,
  marks, each triage change with the revision it was read at, notes,
  trash, restore and purge. A label, a duplicate or an issue Central would
  refuse is not sent.
- *Can a refusal or a lost answer read as something else?* A role taken
  away, a session gone, Central unavailable, a lost list change, a lost
  purge, a conflict and a lost triage change: each is said as what it is,
  in the browser.
- *Does the CSV run a formula?* Unit tests, and the downloaded file in
  the browser: the cell starts with an apostrophe.
- **The checks have teeth** (one change at a time, each undone). Caught by
  the unit tests: the CSV no longer defusing a formula; a space written as
  `%20` (old saved lists stop matching); no answer read as a refusal.
  Caught in the browser on a site rebuilt with each: the moderation shown
  without the role; deleting for good without its question; a second
  press sending a second request. These are the mutants that were tried,
  not every decision.
- *Script in the origin* reads the reports as it reads the page; nothing
  here claims otherwise ([threat model](threat-model.md), "Website
  compromise").

Run on this change: `cargo fmt --all --check`; `cargo clippy --workspace
--all-targets` finds nothing in this change (it stops at `shell.rs`'s
`footer`, 104 lines, unchanged since `d41fce5`, and warns of xtask's
unused `BIMI_SHA256`, also not this change's); `cargo clippy -p
central-web -p central-web-islands --target wasm32-unknown-unknown --lib
-- -D warnings`; `cargo test -p central-web -p central-web-server`;
`cargo xtask site`; in `web/`, `npm run lint` and the specs type-checked
under the strict settings; `web/e2e-site/feedback*.spec.ts` (17 tests,
both projects, 34 passed) and `admin.spec.ts` and `account.spec.ts` with
them. `cargo xtask gate` fails, on three findings of which one is this
change's: its clippy step stops at the footer above and at xtask's
`BIMI_SHA256`, used by tests only (both as at `d41fce5`); with those two
silenced in the working tree only, never committed, policy, the crypto
and SQLite scopes and clippy pass, and the tests stop at
`no_language_is_english_under_another_name`: the nine languages carry the
moderation's 297 messages in English (24 to 27 in 100, the limit 12)
until they are translated. Every other test of the workspace
(`--no-fail-fast`, that one skipped), the doc tests and the release build
pass. Nothing ran against a real Central. This is an engineering review
of two pages, not an audit.

12.11.2026: the old React website (the `web/` app, its tests and its build) was removed from the repository, as the owner decided; the server-rendered site in `crates/central-web` replaces it, administration included. The records above name that app as it was.

## The administration's islands in a module of their own, 12 November 2026

Every page with an island loaded one WebAssembly module, and since the
administration and the feedback moderation were rebuilt (above) it held
them: 913 KB Brotli for the contact page, the sign-in dialogs, the area
and the messenger. This change gives the administration its own module,
which only `/{lang}/account/admin…` loads ([frontend
rewrite](proposals/frontend-rewrite.md) §27). No request, answer, route,
header, migration, dependency or tool changed; `Cargo.lock` is
untouched; the only new switch is a feature of a first-party crate.

Design pass. Nothing new is held or sent; what changes is which code a
page runs. The invariants: the server renders the same frame for
everyone on the administration's pages, and the tool shows only where
`AccountSummary` says `admin`, with nothing more asked of Central
otherwise; every page loads exactly one islands module, by a hashed
address with its SRI value from `manifest.json`, under the unchanged CSP
(`script-src 'self' 'wasm-unsafe-eval'`, no inline script); a mail's
code in the address's fragment is removed on every page of the area, the
administration's included; the public module holds none of the
administration's code.
- **One frame, two islands.** `Account` and the new `Administration`
  share `account::frame` (the summary read, signed out, failure, the
  fragment's code taken and watched), so the administration's pages
  keep the area's handling of the address; `Administration` checks the
  role first and draws §18's refusal without it, as `Account` did.
- **Features never meet in a build.** `cargo xtask site` builds the two
  modules in two cargo runs; the build fails if the public module holds
  an administration's mark or the administration's lacks one, so a
  regression is a failed build, not a quietly larger page. The account
  id that only the administration uses is not compiled into the public
  module.

Implementation pass. The feature only adds code to the browser's half;
the server compiles everything, as before. Each module is checked for
local paths as before. No `unsafe`, no lint allowance: what became dead
in the public build was given the feature's `cfg`, not an `allow`.

Adversarial pass.
- *Does a public page fetch the administration?* The server test checks
  every language's pages with islands for the public script only and no
  administration's island; the browser test watches the requests of the
  contact page, the sign-in dialog, the area and the messenger.
- *Does an administration's page fetch the public module, or fail to
  draw?* The same tests the other way; the browser test also sees the
  overview, the reset code's page and the feedback's list work, and a
  page refused to someone without the role ask Central no
  administration's request.
- **The checks have teeth.** With the feedback's pages given the public
  module, the server test fails. The build's guard compares bytes: the
  one module of `fe100ce` holds both marks, so it would have refused that
  module as a public one; the xtask test keeps the marks inside the
  administration's sources.

Run on this change: `cargo fmt --all --check`; `cargo clippy --workspace
--all-targets -- -D warnings`; `cargo clippy -p central-web -p
central-web-islands --target wasm32-unknown-unknown --lib -- -D
warnings`, and the same with `--all-features` in a run of its own;
`cargo test -p central-web -p central-web-server -p xtask`; `cargo build
-p central-web-server` and `cargo xtask site` (its guard passing on both
modules); in `web/`, `npm ci` and `npm run check`; the whole
`web/e2e-site` suite (`--workers=4`: 644 passed, 4 skipped by their own
conditions, `modules.spec.ts` among the passed); `cargo xtask gate`
(format, policy, both advisory scopes, clippy, tests, doc tests and the
release build: passed); `cargo xtask postgres`, `cargo xtask smoke` and
`cargo xtask secrets`; `cargo audit --deny warnings` (0.22.2) and `cargo
deny check` (0.20.2). Nothing ran against a real Central, and CI's
`site` job, which now builds two modules and lints one more, did not
run: nothing was pushed. This is an engineering review of a build
change, not an audit.

## Session tokens and the website's release combined, 13 November 2026

PR #63 (signed session tokens, `c42/session-tokens`) and PR #64 (the
server-rendered website, mail, open registration, budgets by address) were
written side by side from one base and merged into one branch
(`claude/central-combined`). Neither line had been deployed; production is at
migration 000022.

Design pass: what each line assumed of the other. #63 makes a native
session's bearer a signed Central token, verified before dispatch, and keeps
a gateway's bare credential and a client's signed session apart
(`credential_operation`); every native request names its protocol. #64 counts
requests by address and by bearer, vouches for a bearer once an answer shows
the store knows it, and puts the gateway's pulse on the heartbeat's path.
Invariants kept: no check of either line is dropped; the protocol refusal and
the bearer's kind come before every budget, the budgets before dispatch; the
budgets still see a session by its capability, never by its signed text.

Implementation pass, the places where the two met:

- *Migrations.* #63's 000023 became `m20261111_000024_session_tokens`, between
  #64's 000023 (open registration) and 000025 (gateway activity); the upgrade
  fixture runs 000023 to 000026 in that order over one chain. No grant names
  the tables 000024 drops; the runtime's grant list is the union (38 tables).
- *The native handler.* The protocol layer and the key set stay as #63 wrote
  them; `process` reads the bearer (`present`), decodes, charges the address's
  budget, then `handle_from` with the place. `handle_from` takes `Presented`
  and applies the credential gate before `limits.request`.
- *Vouching for an issued session.* A native answer carries the signed token,
  not the capability the budgets count by, so a sign-in, a password change and
  a pairing now vouch for the capability where it is issued
  (`IdentityService::issued`) instead of reading it from the answer.
- *Which answers vouch.* `GatewayToken`, `RegisterInvalidate` and
  `GatewayRevocations` replace the ticket and lease operations: the store
  refuses each for a dead bearer (`gateway_grant` locks a live native session;
  the other two authenticate the credential or compare the bridge). The
  exemption for refusals of a player's lease or ticket went with those
  operations: every refusal counts now. `credential_operation` names #64's six
  operations: `GatewayPulse` is a gateway's, `PublicFigures`,
  `RegisterChallenge`, `RegisterChallengeFor`, `ResetMail` and `Email` are not.
- *The pulse* answers as the heartbeat does, `GatewayHeartbeat {
  last_revocation }`, as #64's note expected; `central_client::gateway_pulse`
  returns that number.
- *Starting.* `--check-accounts-stdin` refuses a `mail` block the service would
  refuse; the deploy hook asks it after #64's checks of the grants and their
  login, before anything stops. The signing key is read before a port is bound.
- *Names.* #63's `push::Delivery` became `GatewayOutcome`; #64's mail
  `Delivery` keeps its name. The two `Http::raw` test helpers became `raw` and
  `written`.

Adversarial pass, what was found:

- **A shared signature budget.** Made-up signed tokens spend none of #64's
  budgets but each costs one of the 1,024 signature checks a second all native
  sessions share, which has no share by address. One address can keep native
  sessions at `Busy` while it sends them; sign-in, the website and gateways'
  credentials are not touched. Left open and written down ([native
  login](native-login.md#a-bearer-nobody-issued)).
- **The protocol refusal** comes after #64's boundary has read a small body
  within its bounds, not before any read; the comment and the docs say so now.
- **The strangers' flood** (`tests/support/strangers.rs`) sent made-up bare
  tokens to client operations, which the credential gate now refuses before
  any budget. It now presents capabilities as if their signature had verified,
  which over a socket is the token of a session that ended, so it still
  reaches the budgets it tests. All the fixture's services share one signing
  key, as a restart restores it.
- **Stale statements** that both lines left: a lease's updatable expiry in the
  sessions runbook, tickets and leases in the deletion and objects docs and the
  account area's session list. Corrected.
- **Callers that would have been refused at run time** without a compile
  error: the report uploads of the client fixture and `web/e2e-real` lacked
  the protocol header, and `web/e2e-real` read the session token as bytes.

Run on the merge of #64: `cargo fmt --all --check`; `cargo clippy --workspace
--all-targets -- -D warnings`; `cargo clippy -p central-web -p
central-web-islands --target wasm32-unknown-unknown --lib -- -D warnings`,
and again with `--features admin`; `cargo xtask crypto-scope`; the unit tests
of `central-identity`, `central-http`, `central-service` and
`central-client`; `cargo xtask postgres` (both fixtures, the combined chain
and the strangers' flood included: passed); `cargo audit --deny warnings`
(0.22.2), `cargo deny check` (0.20.2); in `web/`, `npm ci` and `npm run
check`. The rest of the checks ran once more after #65 and the rebuilt
browser crypto, and are recorded with them. Nothing ran against a real
gateway or a real Central. This is the merging agent's engineering review,
not an independent audit, and the owner's release gates still apply.

**Then PR #65** (profile pictures, stacked on #64): its migration stays 000027,
after 000026 (26 migrations in all; the runtime's grant list has 42 tables).
`Request::Picture` is a client's operation for `credential_operation`.
`--check-accounts-stdin` uses #65's `outside()`, so a picture socket the
service would refuse is refused before a deploy stops anything, as a mailer
is; the hook builds the worker with the other two binaries, each named, and
installs it after its checks. #65 adds no route to the account listener, so
nothing of it escapes the protocol layer; its upload route is the browser
listener's. Its SDK stand-in issues a signed token.
