# Implementation status

2026-10-20. Deployed at `https://central.baylee.app` from `ff0aac2` (see
[operations](operations.md)); no account is enrolled and no real-user data is
processed yet.

Current reviewed integration: public `main` at
`43d89eed8cc89cd6ebc992e6de0f5e5610d54c0f`. The sibling native client/gateway
integration is tracked in [Baylee PR #390](https://github.com/AceVik/baylee/pull/390);
its current revision and merge state must be checked separately. The owner will
continue production provisioning from the MacBook. See [the handoff](handoff.md)
for that decision and the required staging sequence, and [operations](operations.md)
for the deployed launcher and verified-TLS connector. The development launchers
are not production entry points.

## Integration hardening milestone

The current implementation adds a gateway registry with separate redemption and
heartbeat capabilities, operator-only origin approval, scoped administrators and
revocable gateway leases. A successful login now replaces all prior sessions for
the account, including on the same device. A database uniqueness constraint and
post-lock authorization recheck enforce this under concurrency. Encrypted private
objects support client-side chat archives across devices; migrations 000004 and
000005 add these tables and close the legacy bridge's reset authority at cutover.
See [gateway registry](gateway-registry.md) and [private objects](private-objects.md)
for APIs, exact runtime grants, quotas and confidentiality limitations.

Local evidence includes repeated real PostgreSQL/native-HTTP test passes (recorded
27.20 seconds native SDK and 78.69 seconds database suite), deterministic
revocation-versus-mutation races, registry authorization, encrypted object restore,
quota rollback and bounded tombstone reclamation. The full local policy, Clippy,
unit/integration/doc and release suite passed; later transport-size and independent
request-budget changes received focused tests and workspace Clippy. RustSec,
cargo-deny, working-tree/history secret scans and owned-service smoke passed.
These are recorded local results. Protected CI evidence must match the exact
revision under review; the older runs listed below do not certify a later change.

Request budgets separate unauthenticated/login traffic (32 per second, with the
existing eight login starts per account/minute), data operations (256 per second,
64 per bearer), and session/revocation control (512 per second, 128 per bearer).
All buckets remain bounded and dispatch still authenticates every capability.
These development limits are not a production capacity benchmark. The native
wire limit includes worst-case JSON expansion of a maximum ciphertext and its
enrollment envelope. No first-party unsafe or new external dependency was added.

## Implemented foundation

The native-login libraries are implemented for isolated development: central-api defines
the fixed live/dev endpoint policy and wire messages; central-client provides
transport-independent OPAQUE login and authenticated encrypted-document restore;
central-identity composes bounded one-use challenges with PostgreSQL accounts,
sessions, enrollment and gateway tickets. An optional HTTP library route serves
this protocol. The CLI's explicit `--dev-accounts-stdin` mode serves accounts
using persisted operator-supplied setup and bridge keys. The sibling Baylee
integration has a real synthetic gateway/Central/PostgreSQL migration test:
preferences and bounded deck/history export, restored-digest cutover, old-session
revocation, fresh-device restore and action-scoped admission. Baylee records native
client UI and integration validation in its `docs/central-login.md` and PR #390.
Full data migration and production deployment remain incomplete.

Recorded native-login milestone evidence: real HTTP + PostgreSQL 18.6 + client SDK covers enrollment,
second-device restore, wrong passwords, consumed failed proofs, wrong gateway
audiences, replay, session revocation, concurrent writes, lost commit responses
and corrupt ciphertext. The isolated PostgreSQL script runs both the prior
database suite and this new end-to-end suite and verifies owned-resource cleanup.
The full local format/policy/Clippy/test/doc/release suite passed this milestone.
That milestone added migrations 000002/000003; the current registry also includes
000004/000005 described above. No real data was migrated.

- central-policy: dependency-free configuration and versioned training preferences.
- central-http: local-only bounded HTTP library; liveness and deliberately negative
  readiness endpoints, plus the optional native account protocol.
- central-service: explicit --dev / --dev-accounts-stdin launchers with generic diagnostics.
- central-postgres / central-migration: SeaORM 2.0.4 development adapter, explicit
  transactional migrations, restricted runtime role and database-generated UUIDv7.
- central-crypto: native OPAQUE/master-key wrapping and encrypted objects,
  composed with persistence/session/HTTP libraries through safe APIs.
- Pinned Rust toolchain and dependency inventories, compiler unsafe rejection
  probe, lint/format/tests, vulnerability/license/source and secret checks.
- Native HTTP, malformed boundary and cancellation/shutdown tests.
- Developer commands and library boundaries documented in dev-mode.md.
- Terms of use ([terms](terms.md), migration 000006): Central's compiled-in
  terms (in force only once `legal/terms.*.md` name a version other than
  `draft`), gateways' immutable published versions, append-only acceptances,
  the 403 refusal of unaccepted sessions and `GatewayAuthorize` for gateways.
  Deploy: stop, migrate, grant the three tables (operations.md), start.

The owner accepted HTTP, crypto and SeaORM/SQLx/TLS dependency internals through safe APIs.
Crypto details, exact limitations and tests are in crypto-protocol.md. Database
choice is PostgreSQL 18+ with database-generated UUIDv7 and SeaORM 2 migrations.
Its separate inventory is recorded in storage-proposal; current use remains
local development plus the verified-TLS deployed connector.

## Registration and browser sign-in (in progress)

The server side of [web accounts](web-accounts.md) lands in three PRs.

- **Migration 000007 and the store.** Accounts carry a `source` (`legacy` or
  `registered`) paired with `legacy_id`; legacy rules hold legacy accounts only,
  so a registered account uses private objects after `ConfirmRestore` and the
  bridge's redemption answers `legacy_id: None` for it. The recovery wrapper is
  stored for registered accounts. Beta keys (`beta_keys`, digests only) are
  issued with `--issue-beta-keys-stdin` ([operations](operations.md)); the
  runtime role may only spend their uses. Sessions have a kind, one per account
  and kind, and a browser session lives in its own digest domain. Tests: a
  prior-version upgrade of the exact migration SQL with legacy data, the schema
  constraints, the grant contract and both session kinds over PostgreSQL 18.
  Deploy: stop, migrate, grant (`ops/grants.sql`), start.
- **Wire and identity.** `Register { username, beta_key }` reserves a
  registered account (key before name, `Conflict` only with a valid key);
  `EnrollmentFinish.recovery` carries the recovery wrapper, required for
  registered and refused for legacy accounts, and old clients still decode.
  `IdentityService::handle_browser` serves the browser operations only, issues
  browser sessions apart from the response (`Response::BrowserSession`) and
  keeps sign-in attempts on their surface. Tests: every key refusal, reservations
  that give their use back, revocation, reclaimed names, races for a key's last
  use and for one name, the sweep of abandoned registrations, and the website's
  flow ending in a native admission at the official gateway with no legacy ID.
- **The browser listener.** `browser.listen` in `central.json` starts
  `POST /api/v1` for `baylee.app` ([operations](operations.md)): exact `Host`
  and `Origin`, same-origin fetch metadata, JSON, `X-Baylee-Central: 1`, no
  `Authorization`, loopback peers only, the browser operations only, no CORS
  answer; the `__Host-central` cookie carries the session; per-address budgets
  from the proxy's last `X-Forwarded-For` hop. Tests: every refusal without a
  database (each before the service is called), the cookie's exact attributes
  and parsing, budgets per address and per IPv6 /64, and the website's whole
  flow over the real listener and PostgreSQL, ending in a native admission.
- **The username, and local runs.** `BrowserSession` and the browser
  surface's `BrowserStatus` carry the account's canonical username (the native
  `SessionStatus` is unchanged). `--dev-accounts-stdin` can start the browser
  listener for a numeric loopback page origin, every other check unchanged;
  the deployed launcher keeps `baylee.app` fixed.
- **The gateway directory on the website.** `GatewayDirectory` is served on
  the browser listener without a session (60 a minute per address), and
  `baylee.app/gateways` reads it there. Migration 000009 adds the official
  gateway's row, without owner or credential; the operator lists it, and the
  bridge's heartbeat moves it alone ([gateway registry](gateway-registry.md),
  [operations](operations.md#the-official-gateway)).
- **Not yet deployed.** Deploy: migrate, grant, add `browser.listen`, pass
  `/api/*` in the `baylee.app` site block, issue beta keys. Central's own terms
  are still `draft`, so none are in force.

## Validation and historical milestone evidence

Windows local: full foundation/crypto format, Clippy, unit/integration/doc tests,
release build, compiler guard, RustSec, cargo-deny and secret checks are required.
The suite includes crypto, HTTP/policy, SDK/API, PostgreSQL configuration and
process-level CLI tests, compile-fail API contracts, and real-database/native
account integration tests. A published registration/login
vector is checked on both peers. Password change, identity substitution,
corrupted/replayed proof, malformed messages, all wrapper bits and KDF work-buffer
wiping are covered.
Real TCP smoke verifies endpoint state, Host/Origin rejection and pre-header
timeout, then checks exact owned-process exit and listener closure.

Foundation commit 5b19a0e passed Linux and Windows GitHub CI:
https://github.com/AceVik/baylee-central/actions/runs/39244714058
Crypto commit 73fa00c passed Linux, Windows and macOS ARM64/Intel CI:
https://github.com/AceVik/baylee-central/actions/runs/39247411399
The database milestone additionally has a dedicated Linux PostgreSQL job; its
latest commit results must be checked separately. The prior database commit c77df87
passed all seven jobs: https://github.com/AceVik/baylee-central/actions/runs/39249498079

The subsequent hardening pass adds canonical OPAQUE framing/epoch binding, strict
runtime grants, real transaction cancellation, raw HTTP attack tests, generated-code
compiler probes and a path-dependency policy regression. Details and limitations
are recorded in security-assurance.md. CI now tests PR revisions and main pushes
without duplicate feature-branch push runs; manual workflow dispatch remains available.
All-feature documentation and release builds are checked on all four native targets.
The preceding hardening revision b747895 passed all seven jobs:
https://github.com/AceVik/baylee-central/actions/runs/39254827503

The iterative review adds fixed zeroized AEAD buffers, complete C.1.1 login
interoperability checks, guarded migration history selection, closed raw migrator
APIs, strict environment-name handling, malformed-body classification and bounded
test cleanup. Policy fixtures cover hidden/root sources, linked sources, excluded
targets and case-sensitive dependency features. See security-assurance.md for
reproduced findings, repeated review scope and exact validation limits.
Secret history has its own configuration without working-tree path exceptions;
its scanner self-test verifies forced commits and subsequent deletions of synthetic
credentials. That hardening milestone's CI evidence is linked from PR #1; a previous
revision's green run does not certify later changes.

The earlier native account revision `5bb53ba` passed all seven CI jobs (Linux, Windows,
macOS ARM64/Intel, PostgreSQL, dependency and secret checks):
https://github.com/AceVik/baylee-central/actions/runs/39272471661
This is test evidence, not independent security review or production readiness.

## Remaining implementation and release gates

A deployed launcher (`--accounts-stdin`, `--migrate-stdin`, `--generate-secrets`)
and a verified-TLS database connector now exist; see [operations](operations.md).
The live endpoint and legacy gateway are compiled in, secrets arrive on standard
input from a supervisor-opened root-owned file, and the database must present a
certificate from the configured authority for a loopback or private address.
Backup/restore drills, deployment isolation, per-client abuse controls and
capacity SLO measurements remain. The request budgets above are bounds, not
evidence of production capacity.

Native OPAQUE login, account persistence, gateway registration/admin APIs,
exclusive sessions, guarded legacy migration and encrypted private-object/chat
storage are implemented and tested with synthetic data. They have not been
deployed for real users. Browser SSO, password change/reset/recovery service
and UI flows and MFA remain unfinished (sessions renew, up to 12 hours); registration and
browser sign-in are in progress (above). Live discovery exposes enrollment state
and requires its documented review before public use.

Private deck CRUD/history and the durable replay archive are not implemented in
the sibling client: its encrypted legacy deck snapshot is not an operational
library, replays are not exported, and the 192-KiB / 2,048-row source limit still
blocks larger migrations. Baylee's `docs/central-private-library-plan.md` defines
the remaining typed storage, gameplay handoff and acceptance tests. The generic
encrypted-object API does not close those product requirements by itself.

Human training ingestion remains disabled pending legal basis, rights,
anonymization and preference-enforcement review. Independent security and
operational release gates remain; local tests or a green CI run are not a
production launch approval.
