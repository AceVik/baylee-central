# Implementation status

2026-10-19. No production deployment or real-user processing.

## Implemented foundation

- central-policy: dependency-free configuration and versioned training preferences.
- central-http: local-only bounded HTTP library; liveness and deliberately negative
  readiness endpoints. No login, vault, admin or training endpoint pretends to work.
- central-service: explicit --dev launcher with generic diagnostics.
- Pinned Rust toolchain, lockfile, lint/format/tests, compiler unsafe rejection
  probe, approved dependency list, vulnerability/license/source and secret checks.
- Native HTTP, malformed boundary and cancellation/shutdown tests.
- Developer commands and library boundaries documented in dev-mode.md.

The owner accepted the documented HTTP dependency internals through safe APIs.
The application uses a subset of the proposed 49 packages. Hyper and Hyper Util
were already in that approved graph; they are direct dependencies to enforce
header timeouts and bounded connections. No new feature beyond that proposal is
needed. Crypto and database adoption still require their concrete review.

## Validation

Local Windows: format, Clippy with warnings denied, unit/integration tests,
release build, compiler guard, RustSec and cargo-deny checks have passed.
Real TCP smoke covers endpoint state, Host/Origin rejection and pre-header timeout,
then verifies the exact started process and listener have exited.
Final CI and secret-scan results are recorded in the milestone commit/report.

## Remaining M1 and next gates

M1 is not entirely complete: isolated database/object-store fixtures and capacity
SLO measurements remain after storage dependencies/design are approved. The
transport foundation is usable for development, not production.

M2 authentication/crypto design, test vectors and dependency acceptance come next.
No OPAQUE, MFA, recovery, vault, legacy account migration, storage, website UI,
gateway registration/admin or human training pipeline has been implemented here.
Legal review, independent security review and operational release gates remain.
