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
