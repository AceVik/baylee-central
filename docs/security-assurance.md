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
||||||| parent of c0dd0a7 (Registration schema: account source, beta keys, session kinds (migration 000007))
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
