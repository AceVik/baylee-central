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
