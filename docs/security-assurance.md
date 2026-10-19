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

These are distinct passes by the implementing agent, not independent audits.
Release still needs independent review and operational/legal gates.

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
