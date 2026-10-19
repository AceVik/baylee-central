# Implementation status

2026-10-19. No production deployment or real-user processing.

## Implemented foundation

- central-policy: dependency-free configuration and versioned training preferences.
- central-http: local-only bounded HTTP library; liveness and deliberately negative
  readiness endpoints. No login, vault, admin or training endpoint pretends to work.
- central-service: explicit --dev launcher with generic diagnostics.
- central-crypto: isolated native-client OPAQUE/master-key wrapping prototype;
  optional server primitives, no HTTP/persistence/session integration.
- Pinned Rust toolchain and dependency inventories, compiler unsafe rejection
  probe, lint/format/tests, vulnerability/license/source and secret checks.
- Native HTTP, malformed boundary and cancellation/shutdown tests.
- Developer commands and library boundaries documented in dev-mode.md.

The owner accepted HTTP and crypto dependency internals through safe APIs.
Crypto details, exact limitations and tests are in crypto-protocol.md. Database
choice is PostgreSQL 18+ with database-generated UUIDv7 and SeaORM 2 migrations.
Its separate dependency inventory is being reviewed before runtime adoption.

## Validation

Windows local: full foundation/crypto format, Clippy, unit/integration/doc tests,
release build, compiler guard, RustSec, cargo-deny and secret checks are required.
Current crypto suite has 10 tests, plus 19 HTTP/policy tests. Published registration
vector, password change, identity substitution, corrupted/replayed proof, malformed
messages, all wrapper bytes and KDF work-buffer wiping are covered.
Real TCP smoke verifies endpoint state, Host/Origin rejection and pre-header
timeout, then checks exact owned-process exit and listener closure.

Foundation commit 5b19a0e passed Linux and Windows GitHub CI:
https://github.com/AceVik/baylee-central/actions/runs/39244714058
The next CI matrix adds macOS ARM64 and Intel. Results for the latest commit must
be checked before claiming those platforms passed.

## Remaining M1/M2 and release gates

M1 is not entirely complete: isolated database/object-store fixtures and capacity
SLO measurements remain. M2 has an isolated crypto spike, but browser SSO,
complete protocol state machines, schemas, recovery and independent review remain.
The current service is usable for local development, not production.

No deployed authentication/MFA/recovery, vault sync, legacy account migration,
database adapter, website UI, gateway registration/admin or human training pipeline
exists yet. Legal, independent security and operational release gates remain.
