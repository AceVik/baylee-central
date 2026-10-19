# Implementation status

2026-10-19. No production deployment or real-user processing.

## Implemented foundation

- central-policy: dependency-free configuration and versioned training preferences.
- central-http: local-only bounded HTTP library; liveness and deliberately negative
  readiness endpoints. No login, vault, admin or training endpoint pretends to work.
- central-service: explicit --dev launcher with generic diagnostics.
- central-postgres / central-migration: SeaORM 2.0.4 development adapter, explicit
  transactional migrations, restricted runtime role and database-generated UUIDv7.
- central-crypto: isolated native-client OPAQUE/master-key wrapping prototype;
  optional server primitives, no HTTP/persistence/session integration.
- Pinned Rust toolchain and dependency inventories, compiler unsafe rejection
  probe, lint/format/tests, vulnerability/license/source and secret checks.
- Native HTTP, malformed boundary and cancellation/shutdown tests.
- Developer commands and library boundaries documented in dev-mode.md.

The owner accepted HTTP, crypto and SeaORM/SQLx/TLS dependency internals through safe APIs.
Crypto details, exact limitations and tests are in crypto-protocol.md. Database
choice is PostgreSQL 18+ with database-generated UUIDv7 and SeaORM 2 migrations.
Its separate inventory is recorded in storage-proposal; current use remains
local development only, with no production database connector.

## Validation

Windows local: full foundation/crypto format, Clippy, unit/integration/doc tests,
release build, compiler guard, RustSec, cargo-deny and secret checks are required.
Current suite has 44 ordinary Rust tests: 15 crypto, 25 HTTP/policy, two PostgreSQL
configuration and two process-level CLI tests, plus three compile-fail API contracts
and a separate real-database integration test. A published registration/login
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
credentials. Current native CI evidence is linked from draft PR #1; a previous
revision's green run does not certify later changes.

## Remaining M1/M2 and release gates

M1 is not entirely complete: the PostgreSQL fixture exists; object-store fixtures,
production connector/TLS validation and capacity SLO measurements remain. M2 has an isolated crypto spike, but browser SSO,
complete protocol state machines, schemas, recovery and independent review remain.
The current service is usable for local development, not production.

No deployed authentication/MFA/recovery, vault sync, legacy account migration,
production database connector, website UI, gateway registration/admin or human training pipeline
exists yet. Legal, independent security and operational release gates remain.
